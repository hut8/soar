use crate::Fix;
use crate::airports_repo::AirportsRepository;
use crate::device_repo::DeviceRepository;
use crate::elevation::ElevationDB;
use crate::fixes_repo::FixesRepository;
use crate::flights::Flight;
use crate::flights_repo::FlightsRepository;
use crate::locations_repo::LocationsRepository;
use crate::runways_repo::RunwaysRepository;
use anyhow::Result;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::ActiveFlightsMap;
use super::altitude::calculate_altitude_offset_ft;
use super::geometry::haversine_distance;
use super::location::{create_or_find_location, find_nearby_airport};
use super::runway::determine_runway_identifier;

/// Create a new flight for takeoff
/// Accepts a pre-generated flight_id to prevent race conditions
#[allow(clippy::too_many_arguments)]
pub(crate) async fn create_flight(
    flights_repo: &FlightsRepository,
    device_repo: &DeviceRepository,
    airports_repo: &AirportsRepository,
    locations_repo: &LocationsRepository,
    runways_repo: &RunwaysRepository,
    fixes_repo: &FixesRepository,
    elevation_db: &ElevationDB,
    fix: &Fix,
    flight_id: Uuid,
    skip_airport_runway_lookup: bool,
) -> Result<Uuid> {
    let mut flight = if skip_airport_runway_lookup {
        // Mid-flight appearance - no takeoff observed
        Flight::new_airborne_from_fix_with_id(fix, flight_id)
    } else {
        // Actual takeoff - calculate altitude offset and look up airport/runway
        let takeoff_altitude_offset_ft = calculate_altitude_offset_ft(elevation_db, fix).await;

        let departure_airport_id =
            find_nearby_airport(airports_repo, fix.latitude, fix.longitude).await;

        let takeoff_location_id = create_or_find_location(
            airports_repo,
            locations_repo,
            fix.latitude,
            fix.longitude,
            departure_airport_id,
        )
        .await;

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

        let takeoff_runway = takeoff_runway_info.map(|(runway, _)| runway);

        let mut flight = Flight::new_with_takeoff_from_fix_with_id(fix, flight_id, fix.timestamp);
        flight.departure_airport_id = departure_airport_id;
        flight.takeoff_location_id = takeoff_location_id;
        flight.takeoff_runway_ident = takeoff_runway;
        flight.takeoff_altitude_offset_ft = takeoff_altitude_offset_ft;
        flight
    };

    flight.device_address_type = fix.address_type;

    // Look up the device's club_id and copy it to the flight
    match device_repo.get_device_by_uuid(fix.device_id).await {
        Ok(Some(device)) => {
            flight.club_id = device.club_id;
        }
        Ok(None) => {
            warn!(
                "Device {} not found when creating flight {}",
                fix.device_id, flight_id
            );
        }
        Err(e) => {
            warn!(
                "Failed to fetch device {} when creating flight {}: {}",
                fix.device_id, flight_id, e
            );
        }
    }

    flights_repo.create_flight(flight).await?;

    info!(
        "Created flight {} for device {} (takeoff at {:.6}, {:.6})",
        flight_id, fix.device_id, fix.latitude, fix.longitude
    );

    Ok(flight_id)
}

/// Timeout a flight that has not received beacons for 1+ hour
/// Does NOT set landing location - this is a timeout, not a landing
/// Sets timed_out_at to the last_fix_at value from the flight
pub(crate) async fn timeout_flight(
    flights_repo: &FlightsRepository,
    active_flights: &ActiveFlightsMap,
    flight_id: Uuid,
    device_id: Uuid,
) -> Result<()> {
    info!(
        "Timing out flight {} for device {} (no beacons for 1+ hour)",
        flight_id, device_id
    );

    // Fetch the flight to get the last_fix_at timestamp
    let flight = match flights_repo.get_flight_by_id(flight_id).await? {
        Some(f) => f,
        None => {
            error!("Flight {} not found when timing out", flight_id);
            // Remove from active flights even if flight doesn't exist
            let mut flights = active_flights.write().await;
            flights.remove(&device_id);
            return Ok(());
        }
    };

    // Use last_fix_at as the timeout time
    let timeout_time = flight.last_fix_at;

    // Mark flight as timed out in database
    match flights_repo.timeout_flight(flight_id, timeout_time).await {
        Ok(true) => {
            info!(
                "Successfully timed out flight {} at {}",
                flight_id, timeout_time
            );

            // Calculate and update bounding box now that flight is timed out
            flights_repo
                .calculate_and_update_bounding_box(flight_id)
                .await?;

            // Remove from active flights
            let mut flights = active_flights.write().await;
            flights.remove(&device_id);

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
#[allow(clippy::too_many_arguments)]
pub(crate) async fn complete_flight(
    flights_repo: &FlightsRepository,
    airports_repo: &AirportsRepository,
    locations_repo: &LocationsRepository,
    runways_repo: &RunwaysRepository,
    fixes_repo: &FixesRepository,
    elevation_db: &ElevationDB,
    active_flights: &ActiveFlightsMap,
    flight_id: Uuid,
    fix: &Fix,
) -> Result<bool> {
    let arrival_airport_id = find_nearby_airport(airports_repo, fix.latitude, fix.longitude).await;

    let landing_location_id = create_or_find_location(
        airports_repo,
        locations_repo,
        fix.latitude,
        fix.longitude,
        arrival_airport_id,
    )
    .await;

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

    let (landing_runway, landing_runway_inferred) = match landing_runway_info {
        Some((runway, was_inferred)) => (Some(runway), Some(was_inferred)),
        None => (None, None),
    };
    let landing_altitude_offset_ft = calculate_altitude_offset_ft(elevation_db, fix).await;

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
        let flight_fixes = fixes_repo.get_fixes_for_flight(flight_id, None).await?;

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
                fix.device_id,
                reasons.join(", "),
                duration_seconds,
                altitude_range,
                max_agl_altitude,
                average_speed_mph
            );

            // CRITICAL: Remove from active_flights FIRST to prevent race condition
            // where new fixes arrive and get assigned this flight_id while we're deleting it
            {
                let mut flights = active_flights.write().await;
                flights.remove(&fix.device_id);
            }

            // Clear flight_id from all associated fixes
            if let Err(e) = fixes_repo.clear_flight_id(flight_id).await {
                error!("Failed to clear flight_id from fixes: {}", e);
            }

            // Delete the flight
            match flights_repo.delete_flight(flight_id).await {
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

    // Calculate total distance flown
    let total_distance_meters = flight.total_distance(fixes_repo).await.ok().flatten();

    // Calculate maximum displacement
    let maximum_displacement_meters = flight
        .maximum_displacement(fixes_repo, airports_repo)
        .await
        .ok()
        .flatten();

    flights_repo
        .update_flight_landing(
            flight_id,
            fix.timestamp,
            arrival_airport_id,
            landing_location_id,
            landing_altitude_offset_ft,
            landing_runway,
            total_distance_meters,
            maximum_displacement_meters,
            landing_runway_inferred, // Track whether runway was inferred from heading or looked up
            Some(fix.timestamp),     // last_fix_at - update in same query to avoid two UPDATEs
        )
        .await?;

    // Calculate and update bounding box now that flight is complete
    flights_repo
        .calculate_and_update_bounding_box(flight_id)
        .await?;

    info!(
        "Completed flight {} with landing at {:.6}, {:.6}",
        flight_id, fix.latitude, fix.longitude
    );

    Ok(true) // Return true to indicate flight was completed normally
}
