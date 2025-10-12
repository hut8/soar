use crate::Fix;
use crate::airports_repo::AirportsRepository;
use crate::elevation::ElevationDB;
use crate::fixes_repo::FixesRepository;
use crate::flights::Flight;
use crate::flights_repo::FlightsRepository;
use crate::locations_repo::LocationsRepository;
use crate::runways_repo::RunwaysRepository;
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::aircraft_tracker::AircraftTracker;
use super::altitude::calculate_altitude_offset_ft;
use super::location::{create_or_find_location, find_nearby_airport};
use super::runway::determine_runway_identifier;
use super::towing::detect_towing_at_takeoff;
use super::utils::format_device_address_with_type;

/// Create a new flight for aircraft already airborne (no takeoff data)
pub(crate) async fn create_airborne_flight(
    flights_repo: &FlightsRepository,
    fixes_repo: &FixesRepository,
    fix: &Fix,
) -> Result<Uuid> {
    info!("Creating airborne flight from fix: {:?}", fix);
    let mut flight = Flight::new_airborne_from_fix(fix);
    flight.device_address_type = fix.address_type;
    // No departure airport since we don't know where they took off from

    let flight_id = flight.id;

    match flights_repo.create_flight(flight).await {
        Ok(_) => {
            info!(
                "Created airborne flight {} for aircraft {} (first seen at {:.6}, {:.6})",
                flight_id,
                format_device_address_with_type(
                    fix.device_address_hex().as_ref(),
                    fix.address_type
                ),
                fix.latitude,
                fix.longitude
            );

            // Update existing fixes for this device to associate them with the new flight
            // Use a time range from 10 minutes ago to now to catch recent fixes
            let lookback_time = fix.timestamp - chrono::Duration::minutes(10);
            if let Err(e) = fixes_repo
                .update_flight_id_by_device_and_time(
                    fix.device_id,
                    flight_id,
                    lookback_time,
                    None, // No end time - update all fixes from lookback_time onward
                )
                .await
            {
                warn!(
                    "Failed to update existing fixes with flight_id {} for aircraft {}: {}",
                    flight_id, fix.device_id, e
                );
            }

            Ok(flight_id)
        }
        Err(e) => {
            error!(
                "Failed to create airborne flight for aircraft {}: {}",
                fix.device_id, e
            );
            Err(e)
        }
    }
}

/// Create a new flight for takeoff
#[allow(clippy::too_many_arguments)]
pub(crate) async fn create_flight(
    flights_repo: &FlightsRepository,
    airports_repo: &AirportsRepository,
    locations_repo: &LocationsRepository,
    runways_repo: &RunwaysRepository,
    fixes_repo: &FixesRepository,
    elevation_db: &ElevationDB,
    aircraft_trackers: &Arc<RwLock<HashMap<Uuid, AircraftTracker>>>,
    fix: &Fix,
    skip_airport_runway_lookup: bool,
) -> Result<Uuid> {
    // Calculate takeoff altitude offset (difference between reported altitude and true elevation)
    let takeoff_altitude_offset_ft = calculate_altitude_offset_ft(elevation_db, fix).await;

    // Only look up airport and runway if altitude data is reliable
    let (departure_airport_id, takeoff_location_id, takeoff_runway, takeoff_was_inferred) =
        if skip_airport_runway_lookup {
            info!(
                "Skipping airport/runway lookup for aircraft {} due to unreliable altitude data",
                format_device_address_with_type(
                    fix.device_address_hex().as_ref(),
                    fix.address_type
                )
            );
            (None, None, None, None)
        } else {
            let departure_airport_id =
                find_nearby_airport(airports_repo, fix.latitude, fix.longitude).await;

            // Create or find a location for the takeoff point
            let takeoff_location_id = create_or_find_location(
                airports_repo,
                locations_repo,
                fix.latitude,
                fix.longitude,
                departure_airport_id,
            )
            .await;

            // Determine takeoff runway and whether it was inferred
            // Pass the departure airport to optimize runway search
            let takeoff_runway_info = determine_runway_identifier(
                fixes_repo,
                runways_repo,
                &fix.device_id,
                fix.timestamp,
                fix.latitude,
                fix.longitude,
                departure_airport_id,
            )
            .await;

            let (takeoff_runway, takeoff_was_inferred) = match takeoff_runway_info {
                Some((runway, was_inferred)) => (Some(runway), Some(was_inferred)),
                None => (None, None),
            };

            (
                departure_airport_id,
                takeoff_location_id,
                takeoff_runway,
                takeoff_was_inferred,
            )
        };

    let mut flight = Flight::new_with_takeoff_from_fix(fix, fix.timestamp);
    flight.device_address_type = fix.address_type;
    flight.departure_airport_id = departure_airport_id;
    flight.takeoff_location_id = takeoff_location_id;
    flight.takeoff_runway_ident = takeoff_runway.clone();
    flight.takeoff_altitude_offset_ft = takeoff_altitude_offset_ft;

    let flight_id = flight.id;

    match flights_repo.create_flight(flight).await {
        Ok(_) => {
            info!(
                "Created flight {} for aircraft {} (takeoff at {:.6}, {:.6}{})",
                flight_id,
                fix.device_id,
                fix.latitude,
                fix.longitude,
                if departure_airport_id.is_some() {
                    format!(" from airport ID {}", departure_airport_id.unwrap())
                } else {
                    String::new()
                }
            );

            // Update existing fixes for this device to associate them with the new flight
            // Use a time range from 10 minutes ago to now to catch recent fixes
            let lookback_time = fix.timestamp - chrono::Duration::minutes(10);
            if let Err(e) = fixes_repo
                .update_flight_id_by_device_and_time(
                    fix.device_id,
                    flight_id,
                    lookback_time,
                    None, // No end time - update all fixes from lookback_time onward
                )
                .await
            {
                warn!(
                    "Failed to update existing fixes with flight_id {} for aircraft {}: {}",
                    flight_id, fix.device_id, e
                );
            }

            // Store takeoff runway source in tracker for later use when landing
            {
                let mut trackers = aircraft_trackers.write().await;
                if let Some(tracker) = trackers.get_mut(&fix.device_id) {
                    tracker.takeoff_runway_inferred = takeoff_was_inferred;
                }
            }

            // Check if this is a glider being towed
            if let Some((towplane_device_id, towplane_flight_id, _)) =
                detect_towing_at_takeoff(aircraft_trackers, fixes_repo, &fix.device_id, fix).await
            {
                // Update the flight with towing information
                if let Err(e) = flights_repo
                    .update_towing_info(flight_id, towplane_device_id, towplane_flight_id)
                    .await
                {
                    warn!(
                        "Failed to update towing info for flight {}: {}",
                        flight_id, e
                    );
                } else {
                    // Update tracker to remember we're being towed
                    let mut trackers = aircraft_trackers.write().await;
                    if let Some(tracker) = trackers.get_mut(&fix.device_id) {
                        tracker.towed_by_device_id = Some(towplane_device_id);
                        tracker.tow_released = false;
                    }
                }
            }

            Ok(flight_id)
        }
        Err(e) => {
            error!(
                "Failed to create flight for aircraft {}: {}",
                fix.device_id, e
            );
            Err(e)
        }
    }
}

/// Timeout a flight that has not received beacons for 5+ minutes
/// Does NOT set landing location - this is a timeout, not a landing
pub(crate) async fn timeout_flight(
    flights_repo: &FlightsRepository,
    aircraft_trackers: &Arc<RwLock<HashMap<Uuid, AircraftTracker>>>,
    flight_id: Uuid,
    device_id: Uuid,
) -> Result<()> {
    let timeout_time = Utc::now();

    info!(
        "Timing out flight {} for device {} (no beacons for 5+ minutes)",
        flight_id, device_id
    );

    // Mark flight as timed out in database
    match flights_repo.timeout_flight(flight_id, timeout_time).await {
        Ok(true) => {
            info!("Successfully timed out flight {}", flight_id);

            // Clear tracker state
            let mut trackers = aircraft_trackers.write().await;
            if let Some(tracker) = trackers.get_mut(&device_id) {
                tracker.state = super::aircraft_tracker::AircraftState::Idle;
                tracker.current_flight_id = None;
                // Reset towing state
                tracker.towed_by_device_id = None;
                tracker.tow_released = false;
            }

            Ok(())
        }
        Ok(false) => {
            warn!(
                "Flight {} was not found when attempting to timeout",
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
#[allow(clippy::too_many_arguments)]
pub(crate) async fn complete_flight(
    flights_repo: &FlightsRepository,
    airports_repo: &AirportsRepository,
    locations_repo: &LocationsRepository,
    runways_repo: &RunwaysRepository,
    fixes_repo: &FixesRepository,
    elevation_db: &ElevationDB,
    aircraft_trackers: &Arc<RwLock<HashMap<Uuid, AircraftTracker>>>,
    flight_id: Uuid,
    fix: &Fix,
) -> Result<()> {
    let arrival_airport_id = find_nearby_airport(airports_repo, fix.latitude, fix.longitude).await;

    // Create or find a location for the landing point
    let landing_location_id = create_or_find_location(
        airports_repo,
        locations_repo,
        fix.latitude,
        fix.longitude,
        arrival_airport_id,
    )
    .await;

    // Determine landing runway and whether it was inferred
    // Pass the arrival airport to optimize runway search
    let landing_runway_info = determine_runway_identifier(
        fixes_repo,
        runways_repo,
        &fix.device_id,
        fix.timestamp,
        fix.latitude,
        fix.longitude,
        arrival_airport_id,
    )
    .await;

    let (landing_runway, landing_was_inferred) = match landing_runway_info {
        Some((runway, was_inferred)) => (Some(runway), Some(was_inferred)),
        None => (None, None),
    };

    // Get the takeoff runway source from the tracker
    let takeoff_runway_inferred = {
        let trackers = aircraft_trackers.read().await;
        trackers
            .get(&fix.device_id)
            .and_then(|tracker| tracker.takeoff_runway_inferred)
    };

    // Calculate landing altitude offset (difference between reported altitude and true elevation)
    let landing_altitude_offset_ft = calculate_altitude_offset_ft(elevation_db, fix).await;

    // Determine runways_inferred based on takeoff and landing runway sources
    // Logic:
    // - NULL if both takeoff and landing runways are null
    // - true if both were inferred from heading
    // - false if both were looked up in database
    // - NULL if mixed sources (one inferred, one from database, or one is unknown)
    let runways_inferred = match (takeoff_runway_inferred, landing_was_inferred) {
        (Some(true), Some(true)) => Some(true),    // Both inferred
        (Some(false), Some(false)) => Some(false), // Both from database
        _ => None,                                 // Mixed, unknown, or one/both are null
    };

    // Fetch the flight to compute distance metrics
    let flight = match flights_repo.get_flight_by_id(flight_id).await? {
        Some(f) => f,
        None => {
            error!("Flight {} not found when completing", flight_id);
            return Err(anyhow::anyhow!("Flight not found"));
        }
    };

    // Check if this is a spurious flight (too short or no altitude variation)
    if let Some(takeoff_time) = flight.takeoff_time {
        let duration_seconds = (fix.timestamp - takeoff_time).num_seconds();

        // Get all fixes for this flight to check altitude range
        let flight_fixes = fixes_repo.get_fixes_for_flight(flight_id, None).await?;

        let altitude_range = if !flight_fixes.is_empty() {
            let altitudes: Vec<i32> = flight_fixes
                .iter()
                .filter_map(|f| f.altitude_msl_feet)
                .collect();

            if altitudes.is_empty() {
                None
            } else {
                let max_alt = altitudes.iter().max().unwrap();
                let min_alt = altitudes.iter().min().unwrap();
                Some(max_alt - min_alt)
            }
        } else {
            None
        };

        // Check max AGL altitude if elevation data is available
        let max_agl_altitude = if !flight_fixes.is_empty() {
            flight_fixes.iter().filter_map(|f| f.altitude_agl).max()
        } else {
            None
        };

        // Check if flight is spurious:
        // - Duration < 60 seconds OR
        // - Altitude range < 50 feet OR
        // - If elevation data is available, max AGL < 100 feet
        let is_spurious = duration_seconds < 60
            || altitude_range.map(|range| range < 50).unwrap_or(false)
            || max_agl_altitude.map(|agl| agl < 100).unwrap_or(false);

        if is_spurious {
            warn!(
                "Detected spurious flight {}: duration={}s, altitude_range={:?}ft, max_agl={:?}ft. Deleting flight. Fix was {:?}",
                flight_id, duration_seconds, altitude_range, max_agl_altitude, fix
            );

            // Clear flight_id from all associated fixes
            match fixes_repo.clear_flight_id(flight_id).await {
                Ok(count) => {
                    info!("Cleared flight_id from {} fixes", count);
                }
                Err(e) => {
                    error!("Failed to clear flight_id from fixes: {}", e);
                }
            }

            // Delete the flight
            match flights_repo.delete_flight(flight_id).await {
                Ok(true) => {
                    info!("Deleted spurious flight {}", flight_id);
                    return Ok(());
                }
                Ok(false) => {
                    warn!("Flight {} was already deleted", flight_id);
                    return Ok(());
                }
                Err(e) => {
                    error!("Failed to delete spurious flight {}: {}", flight_id, e);
                    return Err(e);
                }
            }
        }
    }

    // Calculate total distance flown
    let total_distance_meters = match flight.total_distance(fixes_repo).await {
        Ok(dist) => dist,
        Err(e) => {
            warn!(
                "Failed to calculate total distance for flight {}: {}",
                flight_id, e
            );
            None
        }
    };

    // Calculate maximum displacement (only for local flights)
    let maximum_displacement_meters =
        match flight.maximum_displacement(fixes_repo, airports_repo).await {
            Ok(disp) => disp,
            Err(e) => {
                warn!(
                    "Failed to calculate maximum displacement for flight {}: {}",
                    flight_id, e
                );
                None
            }
        };

    match flights_repo
        .update_flight_landing(
            flight_id,
            fix.timestamp,
            arrival_airport_id,
            landing_location_id,
            landing_altitude_offset_ft,
            landing_runway.clone(),
            total_distance_meters,
            maximum_displacement_meters,
            runways_inferred,
        )
        .await
    {
        Ok(_) => {
            info!(
                "Completed flight {} with landing at {:.6}, {:.6}{}",
                flight_id,
                fix.latitude,
                fix.longitude,
                if arrival_airport_id.is_some() {
                    format!(" at airport ID {}", arrival_airport_id.unwrap())
                } else {
                    String::new()
                }
            );
            Ok(())
        }
        Err(e) => {
            error!("Failed to update flight {} with landing: {}", flight_id, e);
            Err(e)
        }
    }
}
