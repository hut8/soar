use crate::Fix;
use crate::aircraft::Aircraft;
use crate::aircraft_repo::AircraftRepository;
use crate::airports_repo::AirportsRepository;
use crate::fixes_repo::FixesRepository;
use crate::flights::{Flight, SpuriousFlightReason, TimeoutPhase};
use crate::flights_repo::FlightsRepository;
use crate::locations_repo::LocationsRepository;
use crate::runways_repo::RunwaysRepository;
use anyhow::Result;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use tracing::{Instrument, debug, error, info_span, warn};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use uuid::Uuid;

use super::FlightProcessorContext;
use super::altitude::calculate_altitude_offset_ft;
use super::geometry::haversine_distance;
use super::location::{
    create_or_find_location, create_start_end_location, find_nearby_airport,
    get_airport_location_id,
};
use super::runway::determine_runway_identifier;
use crate::elevation::ElevationDB;

type PgPool = Pool<ConnectionManager<PgConnection>>;

/// Buffer before takeoff_time when querying fixes for a flight.
/// Accounts for fixes that may have been recorded slightly before the detected takeoff.
const FIXES_TIME_RANGE_START_BUFFER_MINUTES: i64 = 5;

/// Buffer after landing fix timestamp when clearing flight_id from fixes.
/// Ensures we capture any fixes that arrived slightly after the landing fix.
const FIXES_TIME_RANGE_END_BUFFER_MINUTES: i64 = 1;

/// Create a new flight FAST without blocking on slow operations (runway detection, geocoding)
/// Returns flight_id immediately and spawns background task to enrich the flight record
///
/// This is the primary entry point for flight creation - it ensures fix processing isn't blocked
/// by slow database queries or HTTP API calls.
pub(crate) async fn create_flight_fast(
    ctx: &FlightProcessorContext<'_>,
    fix: &Fix,
    flight_id: Uuid,
    skip_airport_runway_lookup: bool,
) -> Result<Uuid> {
    let start = std::time::Instant::now();

    // Fetch aircraft (fast - cached)
    let aircraft = match ctx.aircraft_repo.get_aircraft_by_id(fix.aircraft_id).await {
        Ok(Some(aircraft)) => aircraft,
        Ok(None) => {
            warn!(
                "Aircraft {} not found when creating flight {}",
                fix.aircraft_id, flight_id
            );
            return Err(anyhow::anyhow!("Aircraft not found"));
        }
        Err(e) => {
            error!(
                aircraft_id = %fix.aircraft_id, flight_id = %flight_id, error = %e,
                "Error fetching aircraft for flight"
            );
            return Err(anyhow::anyhow!("Failed to fetch aircraft: {}", e));
        }
    };

    // Create minimal flight record WITHOUT slow operations
    let mut flight = if skip_airport_runway_lookup {
        // Mid-flight appearance - minimal data
        Flight::new_airborne_from_fix_with_id(fix, &aircraft, flight_id)
    } else {
        // Takeoff - calculate altitude offset (fast, uses elevation db) but skip runway/location
        let takeoff_altitude_offset_ft = calculate_altitude_offset_ft(ctx.elevation_db, fix).await;

        // Quick airport lookup (fast, uses spatial index)
        let departure_airport_id =
            find_nearby_airport(ctx.airports_repo, fix.latitude, fix.longitude).await;

        let mut flight =
            Flight::new_with_takeoff_from_fix_with_id(fix, &aircraft, flight_id, fix.received_at);
        flight.departure_airport_id = departure_airport_id;
        flight.takeoff_altitude_offset_ft = takeoff_altitude_offset_ft;

        // NOTE: We intentionally skip:
        // - takeoff_location_id (will be enriched in background)
        // - start_location_id (will be enriched in background)
        // - takeoff_runway_ident (will be enriched in background)

        flight
    };

    // Copy aircraft's club_id to the flight
    flight.club_id = aircraft.club_id;

    // INSERT flight immediately (fast)
    ctx.flights_repo.create_flight(flight).await?;

    metrics::histogram!("flight_tracker.create_flight_fast.latency_ms")
        .record(start.elapsed().as_micros() as f64 / 1000.0);

    debug!(
        "Created flight {} FAST for aircraft {} (will enrich in background)",
        flight_id, fix.aircraft_id
    );

    // Spawn background task to enrich flight with runway/location data (SLOW operations)
    if skip_airport_runway_lookup {
        // Airborne flight - just geocode the start location
        spawn_flight_enrichment_airborne(ctx, fix.clone(), flight_id);
    } else {
        // Takeoff flight - full enrichment including runway detection
        spawn_flight_enrichment_on_creation(ctx, fix.clone(), aircraft, flight_id);
    }

    Ok(flight_id)
}

/// Spawn background task to enrich flight with runway and location data
/// This runs AFTER the flight is created and fix is processed, so it doesn't block the pipeline
fn spawn_flight_enrichment_on_creation(
    ctx: &FlightProcessorContext<'_>,
    fix: Fix,
    aircraft: Aircraft,
    flight_id: Uuid,
) {
    let flights_repo = ctx.flights_repo.clone();
    let fixes_repo = ctx.fixes_repo.clone();
    let runways_repo = ctx.runways_repo.clone();
    let airports_repo = ctx.airports_repo.clone();
    let locations_repo = ctx.locations_repo.clone();
    let magnetic_service = ctx.magnetic_service.clone();

    // Create a new root span to prevent trace accumulation
    let span = info_span!(parent: None, "flight_enrichment_creation", %flight_id);
    let _ = span.set_parent(opentelemetry::Context::new());

    tokio::spawn(
        async move {
            let start = std::time::Instant::now();

            // Fetch flight to get departure_airport_id (we set this in fast path)
            let flight = match flights_repo.get_flight_by_id(flight_id).await {
                Ok(Some(f)) => f,
                Ok(None) => {
                    warn!(
                        "Flight {} not found during background enrichment",
                        flight_id
                    );
                    return;
                }
                Err(e) => {
                    error!(flight_id = %flight_id, error = %e, "Failed to fetch flight for enrichment");
                    return;
                }
            };

            let departure_airport_id = flight.departure_airport_id;

            // SLOW: Determine runway (queries fixes table for 40-second window)
            let takeoff_runway_info = determine_runway_identifier(
                &fixes_repo,
                &runways_repo,
                &magnetic_service,
                &aircraft,
                fix.received_at,
                fix.latitude,
                fix.longitude,
                departure_airport_id,
            )
            .await;

            let takeoff_runway = takeoff_runway_info.map(|(runway, _)| runway);

            // SLOW: Create location via geocoding (HTTP API call to Pelias)
            let start_location_id = if let Some(airport_id) = departure_airport_id {
                match get_airport_location_id(&airports_repo, airport_id).await {
                    Some(location_id) => Some(location_id),
                    None => {
                        create_start_end_location(
                            &locations_repo,
                            fix.latitude,
                            fix.longitude,
                            "start (no airport location)",
                        )
                        .await
                    }
                }
            } else {
                create_start_end_location(
                    &locations_repo,
                    fix.latitude,
                    fix.longitude,
                    "start (no airport)",
                )
                .await
            };

            let takeoff_location_id = create_or_find_location(
                &airports_repo,
                &locations_repo,
                fix.latitude,
                fix.longitude,
                departure_airport_id,
            )
            .await;

            // Update flight with enriched data
            if let Err(e) = flights_repo
                .update_flight_takeoff_enrichment(
                    flight_id,
                    takeoff_runway,
                    start_location_id,
                    takeoff_location_id,
                    fix.received_at,
                )
                .await
            {
                error!(
                    flight_id = %flight_id, error = %e,
                    "Failed to update flight with enrichment data"
                );
            }

            metrics::histogram!("flight_tracker.enrich_flight_on_creation.latency_ms")
                .record(start.elapsed().as_micros() as f64 / 1000.0);

            debug!(
                "Enriched flight {} with runway/location data in background",
                flight_id
            );
        }
        .instrument(span),
    );
}

/// Spawn background task to enrich airborne flight with start location only
/// This is a simpler version for flights that started mid-air (no runway detection needed)
fn spawn_flight_enrichment_airborne(ctx: &FlightProcessorContext<'_>, fix: Fix, flight_id: Uuid) {
    let flights_repo = ctx.flights_repo.clone();
    let locations_repo = ctx.locations_repo.clone();

    // Create a new root span to prevent trace accumulation
    let span = info_span!(parent: None, "flight_enrichment_airborne", %flight_id);
    let _ = span.set_parent(opentelemetry::Context::new());

    tokio::spawn(
        async move {
            let start = std::time::Instant::now();

            // SLOW: Create location via geocoding (HTTP API call to Pelias)
            let start_location_id = create_start_end_location(
                &locations_repo,
                fix.latitude,
                fix.longitude,
                "start (airborne)",
            )
            .await;

            // Update flight with start location
            if let Err(e) = flights_repo
                .update_flight_start_location(flight_id, start_location_id, fix.received_at)
                .await
            {
                error!(
                    flight_id = %flight_id, error = %e,
                    "Failed to update airborne flight with start location"
                );
            }

            metrics::histogram!("flight_tracker.enrich_flight_airborne.latency_ms")
                .record(start.elapsed().as_micros() as f64 / 1000.0);

            debug!(
                "Enriched airborne flight {} with start location in background",
                flight_id
            );
        }
        .instrument(span),
    );
}

/// Timeout a flight that has not received beacons for 1+ hour
/// Sets end_location_id with reverse geocoded location of last known position
/// Sets timed_out_at to the last_fix_at value from the flight
pub(crate) async fn timeout_flight(
    ctx: &FlightProcessorContext<'_>,
    flight_id: Uuid,
    aircraft_id: Uuid,
) -> Result<()> {
    debug!(
        "Timing out flight {} for device {} (no beacons for 1+ hour)",
        flight_id, aircraft_id
    );

    // Get current flight phase from in-memory state
    // Climb rate is already calculated and stored in AircraftState
    let flight_phase = if let Some(state) = ctx.aircraft_states.get(&aircraft_id) {
        state.determine_flight_phase()
    } else {
        super::FlightPhase::Unknown
    };

    let timeout_phase = match flight_phase {
        super::FlightPhase::Climbing => TimeoutPhase::Climbing,
        super::FlightPhase::Cruising => TimeoutPhase::Cruising,
        super::FlightPhase::Descending => TimeoutPhase::Descending,
        super::FlightPhase::Unknown => TimeoutPhase::Unknown,
    };

    debug!("Flight {} phase at timeout: {:?}", flight_id, timeout_phase);

    // Get last fix coordinates from in-memory state for reverse geocoding
    // IMPORTANT: Extract data and release DashMap lock BEFORE any async operations
    // to avoid holding synchronous locks across await points (causes deadlocks)
    let (last_fix_coords, has_state) = if let Some(state) = ctx.aircraft_states.get(&aircraft_id) {
        (state.last_fix().map(|f| (f.lat, f.lng)), true)
    } else {
        (None, false)
    };
    // DashMap lock is now released - safe to do async work

    let end_location_id = match (last_fix_coords, has_state) {
        (Some((lat, lng)), _) => {
            create_start_end_location(ctx.locations_repo, lat, lng, "end (timeout)").await
        }
        (None, true) => {
            debug!(
                "No fixes in aircraft state for timed out flight {}, skipping end location creation",
                flight_id
            );
            None
        }
        (None, false) => {
            debug!(
                "No aircraft state found for timed out flight {}, skipping end location creation",
                flight_id
            );
            None
        }
    };

    // Mark flight as timed out in database WITH phase information and end location
    // The timeout timestamp will be set to the current last_fix_at value atomically
    match ctx
        .flights_repo
        .timeout_flight_with_phase(flight_id, timeout_phase, end_location_id)
        .await
    {
        Ok(true) => {
            // FIRST: Clear the in-memory current_flight_id/current_callsign for this aircraft.
            // This must happen before any fallible operations to prevent state desync
            // where the flight is timed out in the DB but still referenced as active in memory.
            if let Some(mut state) = ctx.aircraft_states.get_mut(&aircraft_id) {
                state.current_flight_id = None;
                state.current_callsign = None;
            }

            // THEN: Non-fatal bounding box calculation
            if let Err(e) = ctx
                .flights_repo
                .calculate_and_update_bounding_box(flight_id)
                .await
            {
                error!(
                    flight_id = %flight_id, error = %e,
                    "Failed to calculate bounding box for timed-out flight"
                );
            }

            metrics::counter!("flight_tracker.flight_ended.timed_out_total").increment(1);
            let phase_label = match timeout_phase {
                TimeoutPhase::Climbing => "climbing",
                TimeoutPhase::Cruising => "cruising",
                TimeoutPhase::Descending => "descending",
                TimeoutPhase::Unknown => "unknown",
            };
            metrics::counter!("flight_tracker.timeout.phase_total", "phase" => phase_label)
                .increment(1);

            Ok(())
        }
        Ok(false) => {
            // Flight already completed/deleted - benign race between timeout checker and landing/spurious deletion
            tracing::debug!(
                "Flight {} already completed or deleted (benign race with landing/spurious deletion)",
                flight_id
            );
            Ok(())
        }
        Err(e) => {
            error!(flight_id = %flight_id, error = %e, "Failed to timeout flight");
            Err(e)
        }
    }
}

/// Complete flight FAST without blocking on slow operations (runway detection, geocoding)
/// Returns immediately and spawns background task to enrich the flight record
///
/// Spawn flight completion as a background task (non-blocking)
/// This prevents flight completion (which can take 30+ seconds due to geocoding)
/// from blocking the fix processing pipeline
pub(crate) fn spawn_complete_flight(
    ctx: &FlightProcessorContext<'_>,
    device: &Aircraft,
    flight_id: Uuid,
    fix: &Fix,
) {
    // Clone everything needed for the background task
    let fixes_repo = ctx.fixes_repo.clone();
    let flights_repo = ctx.flights_repo.clone();
    let aircraft_repo = ctx.aircraft_repo.clone();
    let airports_repo = ctx.airports_repo.clone();
    let locations_repo = ctx.locations_repo.clone();
    let runways_repo = ctx.runways_repo.clone();
    let elevation_db = ctx.elevation_db.clone();
    let magnetic_service = ctx.magnetic_service.clone();
    let pool = ctx.pool.clone();

    let device_clone = device.clone();
    let fix_clone = fix.clone();

    let span = info_span!(parent: None, "flight_completion_background", %flight_id);
    let _ = span.set_parent(opentelemetry::Context::new());

    tokio::spawn(
        async move {
            if let Err(e) = complete_flight_in_background(
                &fixes_repo,
                &flights_repo,
                &aircraft_repo,
                &airports_repo,
                &locations_repo,
                &runways_repo,
                &elevation_db,
                &magnetic_service,
                pool,
                &device_clone,
                flight_id,
                &fix_clone,
            )
            .await
            {
                error!(
                    flight_id = %flight_id, error = %e,
                    "Background flight completion failed"
                );
            }
        }
        .instrument(span),
    );
}

#[allow(clippy::too_many_arguments)]
async fn complete_flight_in_background(
    fixes_repo: &FixesRepository,
    flights_repo: &FlightsRepository,
    _aircraft_repo: &AircraftRepository,
    airports_repo: &AirportsRepository,
    locations_repo: &LocationsRepository,
    runways_repo: &RunwaysRepository,
    elevation_db: &ElevationDB,
    magnetic_service: &crate::magnetic::MagneticService,
    pool: PgPool,
    device: &Aircraft,
    flight_id: Uuid,
    fix: &Fix,
) -> Result<bool> {
    let start = std::time::Instant::now();

    // Fetch the flight first to get takeoff_time for proper time-range queries
    let flight = match flights_repo.get_flight_by_id(flight_id).await {
        Ok(Some(f)) => f,
        Ok(None) => {
            error!(flight_id = %flight_id, "Flight not found when completing");
            return Err(anyhow::anyhow!("Flight not found"));
        }
        Err(e) => {
            error!(flight_id = %flight_id, error = %e, "Failed to fetch flight");
            return Err(e);
        }
    };

    // Use flight's actual time range for partition pruning (not "now - 24h" which
    // fails if processing old queued messages from soar-ingest)
    let fixes_start_time = flight
        .takeoff_time
        .map(|t| t - chrono::Duration::minutes(FIXES_TIME_RANGE_START_BUFFER_MINUTES))
        .unwrap_or_else(|| chrono::Utc::now() - chrono::Duration::hours(24));

    // OPTIMIZATION: Fetch ALL fixes for this flight ONCE (needed for spurious detection & distance calcs)
    let flight_fixes = fixes_repo
        .get_fixes_for_flight(flight_id, None, fixes_start_time, None)
        .await?;

    // Quick airport lookup (fast - spatial index)
    let arrival_airport_id = find_nearby_airport(airports_repo, fix.latitude, fix.longitude).await;

    // Calculate altitude offset (fast - elevation db)
    let landing_altitude_offset_ft = calculate_altitude_offset_ft(elevation_db, fix).await;

    // Check if this is a spurious flight
    if let Some(takeoff_time) = flight.takeoff_time {
        let duration_seconds = (fix.received_at - takeoff_time).num_seconds();

        let altitude_range = if !flight_fixes.is_empty() {
            let altitudes: Vec<i32> = flight_fixes
                .iter()
                .filter_map(|f| f.altitude_msl_feet)
                .collect();

            if !altitudes.is_empty() {
                let max_alt = altitudes.iter().max().unwrap();
                let min_alt = altitudes.iter().min().unwrap();
                Some(max_alt - min_alt)
            } else {
                None
            }
        } else {
            None
        };

        let max_agl_altitude = if !flight_fixes.is_empty() {
            flight_fixes
                .iter()
                .filter_map(|f| f.altitude_agl_feet)
                .max()
        } else {
            None
        };

        let has_excessive_altitude = if !flight_fixes.is_empty() {
            flight_fixes
                .iter()
                .filter_map(|f| f.altitude_msl_feet)
                .any(|alt| alt > 100_000)
        } else {
            false
        };

        let mut total_distance_meters = 0.0;
        for i in 1..flight_fixes.len() {
            let prev = &flight_fixes[i - 1];
            let curr = &flight_fixes[i];
            total_distance_meters +=
                haversine_distance(prev.latitude, prev.longitude, curr.latitude, curr.longitude);
        }

        let average_speed_mph = if duration_seconds > 0 {
            let total_distance_miles = total_distance_meters / 1609.34;
            let duration_hours = duration_seconds as f64 / 3600.0;
            Some(total_distance_miles / duration_hours)
        } else {
            None
        };

        let has_excessive_speed = average_speed_mph
            .map(|speed| speed > 1000.0)
            .unwrap_or(false);

        const MIN_DISPLACEMENT_METERS: f64 = 500.0;
        let has_insufficient_displacement = total_distance_meters < MIN_DISPLACEMENT_METERS;

        // Check if this flight is from ADS-B/SBS (has explicit on_ground status)
        // For these sources, we skip heuristic-based spurious detection since they
        // transmit actual airborne/on-ground status. We only check for data corruption.
        let has_transponder = fix.has_transponder_data();

        // For APRS/OGN: apply heuristic checks (we infer activity from sensor data)
        // For ADS-B/SBS: only apply data corruption checks (excessive altitude/speed)
        let is_spurious = if has_transponder {
            // ADS-B/SBS: only data corruption checks
            has_excessive_altitude || has_excessive_speed
        } else {
            // APRS/OGN: full heuristic checks
            duration_seconds < 120
                || altitude_range.map(|range| range < 50).unwrap_or(false)
                || max_agl_altitude.map(|agl| agl < 100).unwrap_or(false)
                || has_excessive_altitude
                || has_excessive_speed
                || has_insufficient_displacement
        };

        if is_spurious {
            // Archive spurious flight instead of deleting it
            let mut reason_enums = Vec::new();
            let mut reason_descriptions = Vec::new();
            if duration_seconds < 120 {
                reason_enums.push(SpuriousFlightReason::DurationTooShort);
                reason_descriptions
                    .push(format!("duration too short ({}s < 120s)", duration_seconds));
            }
            if altitude_range.map(|range| range < 50).unwrap_or(false) {
                reason_enums.push(SpuriousFlightReason::AltitudeRangeInsufficient);
                reason_descriptions.push(format!(
                    "altitude range too small ({:?}ft < 50ft)",
                    altitude_range
                ));
            }
            if max_agl_altitude.map(|agl| agl < 100).unwrap_or(false) {
                reason_enums.push(SpuriousFlightReason::MaxAglTooLow);
                reason_descriptions.push(format!(
                    "max AGL too low ({:?}ft < 100ft)",
                    max_agl_altitude
                ));
            }
            if has_excessive_altitude {
                reason_enums.push(SpuriousFlightReason::ExcessiveAltitude);
                reason_descriptions.push("excessive altitude (>100,000ft)".to_string());
            }
            if has_excessive_speed {
                reason_enums.push(SpuriousFlightReason::ExcessiveSpeed);
                reason_descriptions.push(format!(
                    "excessive speed ({:.1} mph > 1000 mph)",
                    average_speed_mph.unwrap()
                ));
            }
            if has_insufficient_displacement {
                reason_enums.push(SpuriousFlightReason::DisplacementTooLow);
                reason_descriptions.push(format!(
                    "displacement too low ({:.0}m < {:.0}m)",
                    total_distance_meters, MIN_DISPLACEMENT_METERS
                ));
            }

            warn!(
                "Spurious flight {} detected for aircraft {:06X} - reasons: [{}]. Archiving.",
                flight_id,
                device.address,
                reason_descriptions.join(", ")
            );

            // NOTE: current_flight_id is already cleared by the caller (in state_transitions.rs)
            // before spawning this background task, so we don't need to touch aircraft_states here

            // Clear flight_id from all associated fixes before archiving the flight
            // Use flight's actual time range for partition pruning (not "now - 24h" which
            // fails if processing old queued messages from soar-ingest)
            // This is safe now because the landing fix has already been inserted into the database
            // (we spawn this background task AFTER the fix is inserted to avoid race conditions)
            let clear_start =
                takeoff_time - chrono::Duration::minutes(FIXES_TIME_RANGE_START_BUFFER_MINUTES);
            let clear_end =
                fix.received_at + chrono::Duration::minutes(FIXES_TIME_RANGE_END_BUFFER_MINUTES);
            if let Err(e) = fixes_repo
                .clear_flight_id(flight_id, clear_start, clear_end)
                .await
            {
                error!(error = %e, "Failed to clear flight_id from fixes");
            }

            // Archive the flight to spurious_flights table, then delete from flights
            flights_repo
                .archive_spurious_flight(flight_id, reason_enums, reason_descriptions)
                .await?;

            metrics::counter!("flight_tracker.spurious_flights_archived_total").increment(1);

            return Ok(false); // Return false to indicate flight was archived as spurious
        }
    }

    // Calculate total distance flown (using cached fixes for performance)
    let total_distance_meters = flight
        .total_distance(fixes_repo, Some(&flight_fixes))
        .await
        .ok()
        .flatten();

    // Calculate maximum displacement from takeoff (using cached fixes for performance)
    let maximum_displacement_meters = flight
        .maximum_displacement(fixes_repo, airports_repo, Some(&flight_fixes))
        .await
        .ok()
        .flatten();

    // Update flight with MINIMAL landing data (NO runway, NO geocoded locations yet)
    flights_repo
        .update_flight_landing(
            flight_id,
            fix.received_at, // landing_time
            arrival_airport_id,
            None, // landing_location_id - will be enriched in background
            None, // end_location_id - will be enriched in background
            landing_altitude_offset_ft,
            None, // landing_runway_ident - will be enriched in background
            total_distance_meters,
            maximum_displacement_meters,
            None,                  // runways_inferred - will be enriched in background
            Some(fix.received_at), // last_fix_at
        )
        .await?;

    // Calculate and update bounding box now that flight is complete
    flights_repo
        .calculate_and_update_bounding_box(flight_id)
        .await?;

    metrics::histogram!("flight_tracker.complete_flight_fast.latency_ms")
        .record(start.elapsed().as_micros() as f64 / 1000.0);

    debug!(
        "Completed flight {} FAST for aircraft {} (will enrich in background)",
        flight_id, device.address
    );

    // Spawn background task to enrich flight with runway/location data (SLOW operations)
    spawn_flight_enrichment_on_completion_direct(
        fixes_repo,
        flights_repo,
        airports_repo,
        locations_repo,
        runways_repo,
        magnetic_service,
        fix.clone(),
        device.clone(),
        flight_id,
    );

    // Spawn email notification task
    let pool_clone = pool.clone();
    let device_id_opt = device.id;
    let device_address = device.address;
    let device_address_type = device.primary_address_type();

    let email_span = info_span!(parent: None, "flight_email_notification", %flight_id);
    let _ = email_span.set_parent(opentelemetry::Context::new());

    tokio::spawn(
        async move {
            use crate::aircraft_repo::AircraftRepository;
            use crate::fixes_repo::FixesRepository;
            use crate::flights_repo::FlightsRepository;
            use crate::users_repo::UsersRepository;
            use crate::watchlist_repo::WatchlistRepository;

            let device_id = match device_id_opt {
                Some(id) => id,
                None => {
                    tracing::warn!("Aircraft has no ID, cannot send email notifications");
                    return;
                }
            };

            let watchlist_repo = WatchlistRepository::new(pool_clone.clone());
            match watchlist_repo.get_users_for_aircraft_email(device_id).await {
                Ok(user_ids) if !user_ids.is_empty() => {
                    tracing::info!(
                        "Sending flight completion emails to {} users for aircraft {}",
                        user_ids.len(),
                        device_address
                    );

                    let fixes_repo = FixesRepository::new(pool_clone.clone());
                    let flight_repo = FlightsRepository::new(pool_clone.clone());
                    let aircraft_repo = AircraftRepository::new(pool_clone.clone());
                    let airports_repo =
                        crate::airports_repo::AirportsRepository::new(pool_clone.clone());
                    let users_repo = UsersRepository::new(pool_clone.clone());

                    let aircraft = match aircraft_repo
                        .get_aircraft_by_address(device_address, device_address_type)
                        .await
                    {
                        Ok(Some(d)) => d,
                        _ => {
                            tracing::error!("Failed to get aircraft for KML generation");
                            metrics::counter!("watchlist.emails.failed_total").increment(1);
                            return;
                        }
                    };

                    let flight = match flight_repo.get_flight_by_id(flight_id).await {
                        Ok(Some(f)) => f,
                        _ => {
                            tracing::error!("Failed to get flight for KML generation");
                            metrics::counter!("watchlist.emails.failed_total").increment(1);
                            return;
                        }
                    };

                    let kml_content = match flight.make_kml(&fixes_repo, Some(&aircraft)).await {
                        Ok(kml) => kml,
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to generate KML");
                            metrics::counter!("watchlist.emails.failed_total").increment(1);
                            return;
                        }
                    };

                    let igc_content = match flight.make_igc(&fixes_repo, Some(&aircraft)).await {
                        Ok(igc) => igc,
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to generate IGC");
                            metrics::counter!("watchlist.emails.failed_total").increment(1);
                            return;
                        }
                    };

                    // Get airport information for email
                    let departure_airport = match flight.departure_airport_id {
                        Some(id) => airports_repo.get_airport_by_id(id).await.ok().flatten(),
                        None => None,
                    };
                    let arrival_airport = match flight.arrival_airport_id {
                        Some(id) => airports_repo.get_airport_by_id(id).await.ok().flatten(),
                        None => None,
                    };

                    // Build aircraft email data
                    let aircraft_email_data = crate::email::AircraftEmailData {
                        id: aircraft.id.unwrap_or(uuid::Uuid::nil()),
                        registration: aircraft.registration.clone(),
                        aircraft_model: aircraft.aircraft_model.clone(),
                        hex_address: aircraft.aircraft_address_hex().unwrap_or_default(),
                    };

                    // Build flight email data
                    let flight_email_data = crate::email::FlightEmailData {
                        flight_id,
                        aircraft: aircraft_email_data.clone(),
                        takeoff_time: flight.takeoff_time,
                        landing_time: flight.landing_time,
                        departure_airport: departure_airport.as_ref().map(|a| a.ident.clone()),
                        departure_airport_name: departure_airport.as_ref().map(|a| a.name.clone()),
                        arrival_airport: arrival_airport.as_ref().map(|a| a.ident.clone()),
                        arrival_airport_name: arrival_airport.as_ref().map(|a| a.name.clone()),
                        duration_seconds: flight.duration().map(|d| d.num_seconds()),
                        total_distance_meters: flight.total_distance_meters,
                        max_displacement_meters: flight.maximum_displacement_meters,
                        takeoff_runway: flight.takeoff_runway_ident.clone(),
                        landing_runway: flight.landing_runway_ident.clone(),
                        detected_airborne: flight.takeoff_time.is_none(),
                        timed_out: flight.timed_out_at.is_some(),
                    };

                    // Build filename with format: flight-YYYYMMDD-HHMMSS-REGISTRATION-HEXADDR.ext
                    // If no registration, just: flight-YYYYMMDD-HHMMSS-HEXADDR.ext
                    let takeoff_time_str = flight
                        .takeoff_time
                        .map(|t| t.format("%Y%m%d-%H%M%S").to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    let registration_part = aircraft_email_data.filename_component();
                    let hex_addr = aircraft.aircraft_address_hex().unwrap_or_default();
                    let kml_filename = format!(
                        "flight-{}{}-{}.kml",
                        takeoff_time_str, registration_part, hex_addr
                    );
                    let igc_filename = format!(
                        "flight-{}{}-{}.igc",
                        takeoff_time_str, registration_part, hex_addr
                    );

                    let email_service = match crate::email::EmailService::new() {
                        Ok(service) => service,
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to initialize email service");
                            metrics::counter!("watchlist.emails.failed_total").increment(1);
                            return;
                        }
                    };

                    for user_id in user_ids {
                        match users_repo.get_by_id(user_id).await {
                            Ok(Some(user)) => {
                                if let Some(email) = &user.email {
                                    let to_name = format!("{} {}", user.first_name, user.last_name);
                                    match email_service
                                        .send_flight_completion_email(
                                            email,
                                            &to_name,
                                            &flight_email_data,
                                            kml_content.clone(),
                                            &kml_filename,
                                            igc_content.clone(),
                                            &igc_filename,
                                        )
                                        .await
                                    {
                                        Ok(_) => {
                                            tracing::info!(
                                                "Sent flight completion email to {}",
                                                email
                                            );
                                            metrics::counter!("watchlist.emails.sent_total")
                                                .increment(1);
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                email = %email, error = %e,
                                                "Failed to send email"
                                            );
                                            metrics::counter!("watchlist.emails.failed_total")
                                                .increment(1);
                                        }
                                    }
                                }
                            }
                            _ => {
                                tracing::error!(
                                    user_id = %user_id,
                                    "Failed to get user for email notification"
                                );
                                metrics::counter!("watchlist.emails.failed_total").increment(1);
                            }
                        }
                    }
                }
                Ok(_) => {
                    tracing::debug!("No email watchers for aircraft {}", device_address);
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to get watchlist users");
                    metrics::counter!("watchlist.emails.failed_total").increment(1);
                }
            }
        }
        .instrument(email_span),
    );

    metrics::counter!("flight_tracker.flight_ended.landed_total").increment(1);

    Ok(true) // Return true to indicate flight was completed normally
}

/// Enrich a flight that was completed during startup orphan cleanup.
///
/// This fetches the flight from DB, calculates distance/displacement from fixes,
/// finds the arrival airport, calculates altitude offset, updates the flight record,
/// and spawns geocoding/runway enrichment in background.
///
/// Skips email notifications and spurious detection (these are old/recovered flights).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn enrich_flight_on_startup(
    flight_id: Uuid,
    fixes_repo: &FixesRepository,
    flights_repo: &FlightsRepository,
    aircraft_repo: &AircraftRepository,
    airports_repo: &AirportsRepository,
    locations_repo: &LocationsRepository,
    runways_repo: &RunwaysRepository,
    elevation_db: &ElevationDB,
    magnetic_service: &crate::magnetic::MagneticService,
) {
    let start = std::time::Instant::now();

    // 1. Fetch the flight (already has landing_time = last_fix_at from startup cleanup)
    let flight = match flights_repo.get_flight_by_id(flight_id).await {
        Ok(Some(f)) => f,
        Ok(None) => {
            warn!(
                "Flight {} not found during startup enrichment (may have been deleted)",
                flight_id
            );
            return;
        }
        Err(e) => {
            error!(
                flight_id = %flight_id, error = %e,
                "Failed to fetch flight for startup enrichment"
            );
            return;
        }
    };

    // 2. Fetch all fixes using partition pruning
    let fixes_start_time = flight
        .takeoff_time
        .map(|t| t - chrono::Duration::minutes(FIXES_TIME_RANGE_START_BUFFER_MINUTES))
        .unwrap_or_else(|| flight.created_at - chrono::Duration::hours(1));

    let flight_fixes = match fixes_repo
        .get_fixes_for_flight(flight_id, None, fixes_start_time, None)
        .await
    {
        Ok(fixes) => fixes,
        Err(e) => {
            error!(
                flight_id = %flight_id, error = %e,
                "Failed to fetch fixes for startup enrichment"
            );
            return;
        }
    };

    // 3. Calculate total distance and maximum displacement
    let total_distance_meters = flight
        .total_distance(fixes_repo, Some(&flight_fixes))
        .await
        .ok()
        .flatten();

    let maximum_displacement_meters = flight
        .maximum_displacement(fixes_repo, airports_repo, Some(&flight_fixes))
        .await
        .ok()
        .flatten();

    // 4. Find arrival airport from last fix coordinates
    let (arrival_airport_id, landing_altitude_offset_ft, last_fix_time) =
        if let Some(last_fix) = flight_fixes.last() {
            let airport_id =
                find_nearby_airport(airports_repo, last_fix.latitude, last_fix.longitude).await;
            let alt_offset = calculate_altitude_offset_ft(elevation_db, last_fix).await;
            (airport_id, alt_offset, Some(last_fix.received_at))
        } else {
            (None, None, None)
        };

    // 5. Update the flight record (landing_time already set, add enrichment data)
    let landing_time_val = flight.landing_time.unwrap_or(flight.last_fix_at);
    // Clamp last_fix_time to be at least landing_time_val to satisfy check_landing_near_last_fix constraint.
    // During startup enrichment, landing_time may have been set by complete_orphaned_startup_flights
    // to a value like takeoff_time + 1s, while the actual last fix record's received_at could be
    // much earlier (e.g., fix records were pruned or missing).
    let last_fix_time = last_fix_time.map(|t| t.max(landing_time_val));
    if let Err(e) = flights_repo
        .update_flight_landing(
            flight_id,
            landing_time_val,
            arrival_airport_id,
            None, // landing_location_id - will be enriched in background
            None, // end_location_id - will be enriched in background
            landing_altitude_offset_ft,
            None, // landing_runway_ident - will be enriched in background
            total_distance_meters,
            maximum_displacement_meters,
            None,          // runways_inferred
            last_fix_time, // last_fix_at
        )
        .await
    {
        error!(
            flight_id = %flight_id, error = %e,
            "Failed to update flight during startup enrichment"
        );
        return;
    }

    // 6. Calculate and update bounding box
    if let Err(e) = flights_repo
        .calculate_and_update_bounding_box(flight_id)
        .await
    {
        error!(
            flight_id = %flight_id, error = %e,
            "Failed to calculate bounding box during startup enrichment"
        );
    }

    // 7. Spawn geocoding/runway enrichment in background (if we have a last fix)
    if !flight_fixes.is_empty() {
        let last_fix = flight_fixes.last().unwrap();

        // Fetch aircraft for runway detection
        let aircraft = match aircraft_repo.get_aircraft_by_id(last_fix.aircraft_id).await {
            Ok(Some(a)) => Some(a),
            _ => None,
        };

        if let Some(aircraft) = aircraft {
            spawn_flight_enrichment_on_completion_direct(
                fixes_repo,
                flights_repo,
                airports_repo,
                locations_repo,
                runways_repo,
                magnetic_service,
                last_fix.clone(),
                aircraft,
                flight_id,
            );
        }
    }

    metrics::histogram!("flight_tracker.enrich_flight_on_startup.latency_ms")
        .record(start.elapsed().as_micros() as f64 / 1000.0);

    debug!(
        "Startup enrichment completed for flight {} in {:.2}s",
        flight_id,
        start.elapsed().as_secs_f64()
    );
}

/// Spawn background task to enrich flight with runway and location data on completion
/// This runs AFTER the flight is completed and fix is processed, so it doesn't block the pipeline
/// Direct version of spawn_flight_enrichment_on_completion that takes individual repos
/// Used by complete_flight_in_background to avoid context reconstruction
#[allow(clippy::too_many_arguments)]
fn spawn_flight_enrichment_on_completion_direct(
    fixes_repo: &FixesRepository,
    flights_repo: &FlightsRepository,
    airports_repo: &AirportsRepository,
    locations_repo: &LocationsRepository,
    runways_repo: &RunwaysRepository,
    magnetic_service: &crate::magnetic::MagneticService,
    fix: Fix,
    aircraft: Aircraft,
    flight_id: Uuid,
) {
    let flights_repo = flights_repo.clone();
    let fixes_repo = fixes_repo.clone();
    let runways_repo = runways_repo.clone();
    let airports_repo = airports_repo.clone();
    let locations_repo = locations_repo.clone();
    let magnetic_service = magnetic_service.clone();

    // Create a new root span to prevent trace accumulation
    let span = info_span!(parent: None, "flight_enrichment_completion", %flight_id);
    let _ = span.set_parent(opentelemetry::Context::new());

    tokio::spawn(
        async move {
            let start = std::time::Instant::now();

            // Fetch flight to get arrival_airport_id (we set this in fast path)
            let flight = match flights_repo.get_flight_by_id(flight_id).await {
                Ok(Some(f)) => f,
                Ok(None) => {
                    warn!(
                        "Flight {} not found during background enrichment on completion",
                        flight_id
                    );
                    return;
                }
                Err(e) => {
                    error!(
                        flight_id = %flight_id, error = %e,
                        "Failed to fetch flight for enrichment on completion"
                    );
                    return;
                }
            };

            let arrival_airport_id = flight.arrival_airport_id;

            // SLOW: Determine runway (queries fixes table for 40-second window)
            let landing_runway_info = determine_runway_identifier(
                &fixes_repo,
                &runways_repo,
                &magnetic_service,
                &aircraft,
                fix.received_at,
                fix.latitude,
                fix.longitude,
                arrival_airport_id,
            )
            .await;

            let (landing_runway, landing_runway_inferred) = match landing_runway_info {
                Some((runway, was_inferred)) => (Some(runway), Some(was_inferred)),
                None => (None, None),
            };

            // SLOW: Create location via geocoding (HTTP API call to Pelias)
            let end_location_id = if let Some(airport_id) = arrival_airport_id {
                match get_airport_location_id(&airports_repo, airport_id).await {
                    Some(location_id) => Some(location_id),
                    None => {
                        create_start_end_location(
                            &locations_repo,
                            fix.latitude,
                            fix.longitude,
                            "end (no airport location)",
                        )
                        .await
                    }
                }
            } else {
                create_start_end_location(
                    &locations_repo,
                    fix.latitude,
                    fix.longitude,
                    "end (no airport)",
                )
                .await
            };

            // Update flight with enriched landing data
            if let Err(e) = flights_repo
                .update_flight_landing_enrichment(
                    flight_id,
                    landing_runway,
                    landing_runway_inferred,
                    end_location_id,
                    None, // landing_location_id - not set during enrichment
                    fix.received_at,
                )
                .await
            {
                error!(
                    flight_id = %flight_id, error = %e,
                    "Failed to update flight with enriched landing data"
                );
            }

            metrics::histogram!("flight_tracker.enrichment.landing.latency_ms")
                .record(start.elapsed().as_micros() as f64 / 1000.0);

            debug!(
                "Flight {} landing enrichment completed in {:.2}s",
                flight_id,
                start.elapsed().as_secs_f64()
            );
        }
        .instrument(span),
    );
}
