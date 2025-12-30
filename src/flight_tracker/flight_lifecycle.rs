use crate::Fix;
use crate::aircraft::Aircraft;
use crate::flights::{Flight, TimeoutPhase};
use anyhow::Result;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::ActiveFlightsMap;
use super::FlightProcessorContext;
use super::altitude::calculate_altitude_offset_ft;
use super::geometry::haversine_distance;
use super::location::{
    create_or_find_location, create_start_end_location, find_nearby_airport,
    get_airport_location_id,
};
use super::runway::determine_runway_identifier;

/// Create a new flight for takeoff
/// Accepts a pre-generated flight_id to prevent race conditions
pub(crate) async fn create_flight(
    ctx: &FlightProcessorContext<'_>,
    fix: &Fix,
    flight_id: Uuid,
    skip_airport_runway_lookup: bool,
) -> Result<Uuid> {
    // Fetch aircraft first as we need it for Flight creation
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
                "Error fetching aircraft {} for flight {}: {}",
                fix.aircraft_id, flight_id, e
            );
            return Err(anyhow::anyhow!("Failed to fetch aircraft: {}", e));
        }
    };

    let mut flight = if skip_airport_runway_lookup {
        // Mid-flight appearance - no takeoff observed
        let mut flight = Flight::new_airborne_from_fix_with_id(fix, &aircraft, flight_id);

        // Create start location with Photon reverse geocoding for airborne detection point
        flight.start_location_id = create_start_end_location(
            ctx.locations_repo,
            fix.latitude,
            fix.longitude,
            "start (airborne)",
        )
        .await;

        flight
    } else {
        // Actual takeoff - calculate altitude offset and look up airport/runway
        let takeoff_altitude_offset_ft = calculate_altitude_offset_ft(ctx.elevation_db, fix).await;

        let departure_airport_id =
            find_nearby_airport(ctx.airports_repo, fix.latitude, fix.longitude).await;

        let takeoff_location_id = create_or_find_location(
            ctx.airports_repo,
            ctx.locations_repo,
            fix.latitude,
            fix.longitude,
            departure_airport_id,
        )
        .await;

        let takeoff_runway_info = determine_runway_identifier(
            ctx.fixes_repo,
            ctx.runways_repo,
            &aircraft,
            fix.timestamp,
            fix.latitude,
            fix.longitude,
            departure_airport_id,
        )
        .await;

        let takeoff_runway = takeoff_runway_info.map(|(runway, _)| runway);

        // Set start_location_id: use airport's location_id if at airport and it exists,
        // otherwise create new location with Photon reverse geocoding
        let start_location_id = if let Some(airport_id) = departure_airport_id {
            // Check if airport has a location_id
            match get_airport_location_id(ctx.airports_repo, airport_id).await {
                Some(location_id) => {
                    debug!(
                        "Using airport {}'s existing location_id {} for takeoff",
                        airport_id, location_id
                    );
                    Some(location_id)
                }
                None => {
                    // Airport doesn't have location_id yet, reverse geocode the coordinates
                    create_start_end_location(
                        ctx.locations_repo,
                        fix.latitude,
                        fix.longitude,
                        "start (takeoff)",
                    )
                    .await
                }
            }
        } else {
            // Not at an airport, reverse geocode the coordinates
            create_start_end_location(
                ctx.locations_repo,
                fix.latitude,
                fix.longitude,
                "start (takeoff)",
            )
            .await
        };

        let mut flight =
            Flight::new_with_takeoff_from_fix_with_id(fix, &aircraft, flight_id, fix.timestamp);
        flight.departure_airport_id = departure_airport_id;
        flight.takeoff_location_id = takeoff_location_id;
        flight.start_location_id = start_location_id;
        flight.takeoff_runway_ident = takeoff_runway;
        flight.takeoff_altitude_offset_ft = takeoff_altitude_offset_ft;
        flight
    };

    // Copy aircraft's club_id to the flight
    flight.club_id = aircraft.club_id;

    ctx.flights_repo.create_flight(flight).await?;

    debug!(
        "Created flight {} for device {} (takeoff at {:.6}, {:.6})",
        flight_id, fix.aircraft_id, fix.latitude, fix.longitude
    );

    Ok(flight_id)
}

/// Timeout a flight that has not received beacons for 1+ hour
/// Sets end_location_id with reverse geocoded location of last known position
/// Sets timed_out_at to the last_fix_at value from the flight
pub(crate) async fn timeout_flight(
    ctx: &FlightProcessorContext<'_>,
    active_flights: &ActiveFlightsMap,
    flight_id: Uuid,
    aircraft_id: Uuid,
) -> Result<()> {
    debug!(
        "Timing out flight {} for device {} (no beacons for 1+ hour)",
        flight_id, aircraft_id
    );

    // Get current flight state to determine phase
    let flight_phase = {
        let flights = active_flights.read().await;
        flights
            .get(&aircraft_id)
            .map(|state| state.determine_flight_phase())
            .unwrap_or(super::FlightPhase::Unknown)
    };

    let timeout_phase = match flight_phase {
        super::FlightPhase::Climbing => TimeoutPhase::Climbing,
        super::FlightPhase::Cruising => TimeoutPhase::Cruising,
        super::FlightPhase::Descending => TimeoutPhase::Descending,
        super::FlightPhase::Unknown => TimeoutPhase::Unknown,
    };

    debug!("Flight {} phase at timeout: {:?}", flight_id, timeout_phase);

    // Fetch last fix to get coordinates for reverse geocoding
    let last_fix = ctx
        .fixes_repo
        .get_fixes_for_flight(flight_id, Some(1))
        .await?
        .into_iter()
        .next();

    // Create end location with reverse geocoding if we have the last fix
    let end_location_id = if let Some(fix) = last_fix {
        create_start_end_location(
            ctx.locations_repo,
            fix.latitude,
            fix.longitude,
            "end (timeout)",
        )
        .await
    } else {
        debug!(
            "No fixes found for timed out flight {}, skipping end location creation",
            flight_id
        );
        None
    };

    // Mark flight as timed out in database WITH phase information and end location
    // The timeout timestamp will be set to the current last_fix_at value atomically
    match ctx
        .flights_repo
        .timeout_flight_with_phase(flight_id, timeout_phase, end_location_id)
        .await
    {
        Ok(true) => {
            // Calculate and update bounding box now that flight is timed out
            ctx.flights_repo
                .calculate_and_update_bounding_box(flight_id)
                .await?;

            // Remove from active flights
            let mut flights = active_flights.write().await;
            flights.remove(&aircraft_id);

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
            error!("Failed to timeout flight {}: {}", flight_id, e);
            Err(e)
        }
    }
}

/// Update flight with landing information
/// Returns Ok(true) if flight was completed normally, Ok(false) if flight was deleted as spurious
pub(crate) async fn complete_flight(
    ctx: &FlightProcessorContext<'_>,
    device: &Aircraft,
    flight_id: Uuid,
    fix: &Fix,
) -> Result<bool> {
    // OPTIMIZATION: Fetch ALL fixes for this flight ONCE at the beginning
    // This single fetch is reused for:
    // 1. Spurious flight detection
    // 2. Total distance calculation
    // 3. Maximum displacement calculation
    // Previously, we were fetching fixes 3 separate times, causing 9+ second delays for long flights
    let flight_fixes = ctx.fixes_repo.get_fixes_for_flight(flight_id, None).await?;

    let arrival_airport_id =
        find_nearby_airport(ctx.airports_repo, fix.latitude, fix.longitude).await;

    let landing_location_id = create_or_find_location(
        ctx.airports_repo,
        ctx.locations_repo,
        fix.latitude,
        fix.longitude,
        arrival_airport_id,
    )
    .await;

    let landing_runway_info = determine_runway_identifier(
        ctx.fixes_repo,
        ctx.runways_repo,
        device,
        fix.timestamp,
        fix.latitude,
        fix.longitude,
        arrival_airport_id,
    )
    .await;

    let (landing_runway, landing_runway_inferred) = match landing_runway_info {
        Some((runway, was_inferred)) => (Some(runway), Some(was_inferred)),
        None => (None, None),
    };
    let landing_altitude_offset_ft = calculate_altitude_offset_ft(ctx.elevation_db, fix).await;

    // Set end_location_id: use airport's location_id if at airport and it exists,
    // otherwise create new location with Photon reverse geocoding
    let end_location_id = if let Some(airport_id) = arrival_airport_id {
        // Check if airport has a location_id
        match get_airport_location_id(ctx.airports_repo, airport_id).await {
            Some(location_id) => {
                debug!(
                    "Using airport {}'s existing location_id {} for landing",
                    airport_id, location_id
                );
                Some(location_id)
            }
            None => {
                // Airport doesn't have location_id yet, reverse geocode the coordinates
                create_start_end_location(
                    ctx.locations_repo,
                    fix.latitude,
                    fix.longitude,
                    "end (landing)",
                )
                .await
            }
        }
    } else {
        // Not at an airport, reverse geocode the coordinates
        create_start_end_location(
            ctx.locations_repo,
            fix.latitude,
            fix.longitude,
            "end (landing)",
        )
        .await
    };

    // Fetch the flight to compute distance metrics
    let flight = match ctx.flights_repo.get_flight_by_id(flight_id).await? {
        Some(f) => f,
        None => {
            error!("Flight {} not found when completing", flight_id);
            return Err(anyhow::anyhow!("Flight not found"));
        }
    };

    // Check if this is a spurious flight (too short or no altitude variation)
    // Use pre-fetched flight_fixes instead of fetching again
    if let Some(takeoff_time) = flight.takeoff_time {
        let duration_seconds = (fix.timestamp - takeoff_time).num_seconds();

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

        // Check for excessive altitude (> 100,000 feet indicates bad data)
        let has_excessive_altitude = if !flight_fixes.is_empty() {
            flight_fixes
                .iter()
                .filter_map(|f| f.altitude_msl_feet)
                .any(|alt| alt > 100_000)
        } else {
            false
        };

        // Calculate average speed for sanity check
        let average_speed_mph = if duration_seconds > 0 {
            // Calculate total distance using haversine formula
            let mut total_distance_meters = 0.0;
            for i in 1..flight_fixes.len() {
                let prev = &flight_fixes[i - 1];
                let curr = &flight_fixes[i];
                total_distance_meters += haversine_distance(
                    prev.latitude,
                    prev.longitude,
                    curr.latitude,
                    curr.longitude,
                );
            }
            let total_distance_miles = total_distance_meters / 1609.34;
            let duration_hours = duration_seconds as f64 / 3600.0;
            Some(total_distance_miles / duration_hours)
        } else {
            None
        };

        // Check if average speed is > 1000 mph (indicates bad data)
        let has_excessive_speed = average_speed_mph
            .map(|speed| speed > 1000.0)
            .unwrap_or(false);

        // Check if flight is spurious
        let is_spurious = duration_seconds < 120
            || altitude_range.map(|range| range < 50).unwrap_or(false)
            || max_agl_altitude.map(|agl| agl < 100).unwrap_or(false)
            || has_excessive_altitude
            || has_excessive_speed;

        if is_spurious {
            // Determine the specific reason(s) for spurious classification
            let mut reasons = Vec::new();
            if duration_seconds < 120 {
                reasons.push(format!("duration too short ({}s < 120s)", duration_seconds));
            }
            if altitude_range.map(|range| range < 50).unwrap_or(false) {
                reasons.push(format!(
                    "altitude range too small ({:?}ft < 50ft)",
                    altitude_range
                ));
            }
            if max_agl_altitude.map(|agl| agl < 100).unwrap_or(false) {
                reasons.push(format!(
                    "max AGL too low ({:?}ft < 100ft)",
                    max_agl_altitude
                ));
            }
            if has_excessive_altitude {
                reasons.push("excessive altitude (>100,000ft)".to_string());
            }
            if has_excessive_speed {
                reasons.push(format!(
                    "excessive speed ({:?}mph > 1000mph)",
                    average_speed_mph
                ));
            }

            warn!(
                "Spurious flight {} detected - reasons: [{}]. Duration={}s, altitude_range={:?}ft, max_agl={:?}ft, avg_speed={:?}mph. Deleting.",
                fix.aircraft_id,
                reasons.join(", "),
                duration_seconds,
                altitude_range,
                max_agl_altitude,
                average_speed_mph
            );

            // CRITICAL: Remove from active_flights FIRST to prevent race condition
            // where new fixes arrive and get assigned this flight_id while we're deleting it
            {
                let mut flights = ctx.active_flights.write().await;
                flights.remove(&fix.aircraft_id);
            }

            // Clear flight_id from all associated fixes
            if let Err(e) = ctx.fixes_repo.clear_flight_id(flight_id).await {
                error!("Failed to clear flight_id from fixes: {}", e);
            }

            // Delete the flight
            match ctx.flights_repo.delete_flight(flight_id).await {
                Ok(_) => {
                    info!("Deleted spurious flight {}", flight_id);
                    return Ok(false); // Return false to indicate flight was deleted
                }
                Err(e) => {
                    error!("Failed to delete spurious flight {}: {}", flight_id, e);
                    return Err(e);
                }
            }
        }
    }

    // Calculate total distance flown (using cached fixes for performance)
    let total_distance_meters = flight
        .total_distance(ctx.fixes_repo, Some(&flight_fixes))
        .await
        .ok()
        .flatten();

    // Calculate maximum displacement (using cached fixes for performance)
    let maximum_displacement_meters = flight
        .maximum_displacement(ctx.fixes_repo, ctx.airports_repo, Some(&flight_fixes))
        .await
        .ok()
        .flatten();

    ctx.flights_repo
        .update_flight_landing(
            flight_id,
            fix.timestamp,
            arrival_airport_id,
            landing_location_id,
            end_location_id, // Reverse geocoded end location
            landing_altitude_offset_ft,
            landing_runway,
            total_distance_meters,
            maximum_displacement_meters,
            landing_runway_inferred, // Track whether runway was inferred from heading or looked up
            Some(fix.timestamp),     // last_fix_at - update in same query to avoid two UPDATEs
        )
        .await?;

    // Calculate and update bounding box now that flight is complete
    ctx.flights_repo
        .calculate_and_update_bounding_box(flight_id)
        .await?;

    debug!(
        "Completed flight {} with landing at {:.6}, {:.6}",
        flight_id, fix.latitude, fix.longitude
    );

    // Send email notifications to users watching this aircraft
    let pool_clone = ctx.pool.clone();
    let device_id_opt = device.id;
    let device_address = device.address;

    tokio::spawn(async move {
        use crate::aircraft_repo::AircraftRepository;
        use crate::fixes_repo::FixesRepository;
        use crate::flights_repo::FlightsRepository;
        use crate::users_repo::UsersRepository;
        use crate::watchlist_repo::WatchlistRepository;

        // Get device_id, return early if not available
        let device_id = match device_id_opt {
            Some(id) => id,
            None => {
                tracing::warn!("Aircraft has no ID, cannot send email notifications");
                return;
            }
        };

        // Get users who want email notifications for this aircraft
        let watchlist_repo = WatchlistRepository::new(pool_clone.clone());
        match watchlist_repo.get_users_for_aircraft_email(device_id).await {
            Ok(user_ids) if !user_ids.is_empty() => {
                tracing::info!(
                    "Sending flight completion emails to {} users for aircraft {}",
                    user_ids.len(),
                    device_address
                );

                // Get flight data for KML generation
                let fixes_repo = FixesRepository::new(pool_clone.clone());
                let flight_repo = FlightsRepository::new(pool_clone.clone());
                let aircraft_repo = AircraftRepository::new(pool_clone.clone());
                let users_repo = UsersRepository::new(pool_clone.clone());

                // Get full aircraft info
                let aircraft = match aircraft_repo.get_aircraft_by_address(device_address).await {
                    Ok(Some(d)) => d,
                    _ => {
                        tracing::error!("Failed to get aircraft for KML generation");
                        metrics::counter!("watchlist.emails.failed_total").increment(1);
                        return;
                    }
                };

                // Get flight for KML generation
                let flight = match flight_repo.get_flight_by_id(flight_id).await {
                    Ok(Some(f)) => f,
                    _ => {
                        tracing::error!("Failed to get flight for KML generation");
                        metrics::counter!("watchlist.emails.failed_total").increment(1);
                        return;
                    }
                };

                // Generate KML
                let kml_content = match flight.make_kml(&fixes_repo, Some(&aircraft)).await {
                    Ok(kml) => kml,
                    Err(e) => {
                        tracing::error!("Failed to generate KML: {}", e);
                        metrics::counter!("watchlist.emails.failed_total").increment(1);
                        return;
                    }
                };

                // Generate KML filename
                let takeoff_time_str = flight
                    .takeoff_time
                    .map(|t| t.format("%Y%m%d-%H%M%S").to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let kml_filename = format!("flight-{}-{}.kml", takeoff_time_str, device_address);

                // Send emails
                let email_service = match crate::email::EmailService::new() {
                    Ok(svc) => svc,
                    Err(e) => {
                        tracing::error!("Failed to create email service: {}", e);
                        sentry::capture_message(
                            &format!(
                                "Failed to create email service for flight completion notifications: {}",
                                e
                            ),
                            sentry::Level::Error,
                        );
                        metrics::counter!("watchlist.emails.failed_total").increment(1);
                        return;
                    }
                };

                for user_id in user_ids {
                    match users_repo.get_by_id(user_id).await {
                        Ok(Some(user)) => {
                            // Only send email if user has an email address (not a pilot-only user)
                            if let Some(email) = &user.email {
                                let to_name = format!("{} {}", user.first_name, user.last_name);
                                match email_service
                                    .send_flight_completion_email(
                                        email,
                                        &to_name,
                                        flight_id,
                                        &device_address.to_string(),
                                        kml_content.clone(),
                                        &kml_filename,
                                    )
                                    .await
                                {
                                    Ok(_) => {
                                        tracing::info!("Sent flight completion email to {}", email);
                                        metrics::counter!("watchlist.emails.sent_total")
                                            .increment(1);
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to send email to {}: {}", email, e);
                                        sentry::capture_message(
                                            &format!(
                                                "Failed to send flight completion email to {}: {}",
                                                email, e
                                            ),
                                            sentry::Level::Error,
                                        );
                                        metrics::counter!("watchlist.emails.failed_total")
                                            .increment(1);
                                    }
                                }
                            }
                        }
                        _ => {
                            tracing::error!(
                                "Failed to get user {} for email notification",
                                user_id
                            );
                            metrics::counter!("watchlist.emails.failed_total").increment(1);
                        }
                    }
                }
            }
            Ok(_) => {
                // No users watching this aircraft with email enabled
                tracing::debug!("No email watchers for aircraft {}", device_address);
            }
            Err(e) => {
                tracing::error!("Failed to get watchlist users: {}", e);
                sentry::capture_message(
                    &format!(
                        "Failed to get watchlist users for flight completion notifications: {}",
                        e
                    ),
                    sentry::Level::Error,
                );
                metrics::counter!("watchlist.emails.failed_total").increment(1);
            }
        }
    });

    metrics::counter!("flight_tracker.flight_ended.landed_total").increment(1);

    Ok(true) // Return true to indicate flight was completed normally
}
