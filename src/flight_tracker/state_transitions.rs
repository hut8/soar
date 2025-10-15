use crate::Fix;
use crate::airports_repo::AirportsRepository;
use crate::elevation::ElevationDB;
use crate::fixes_repo::FixesRepository;
use crate::flights_repo::FlightsRepository;
use crate::locations_repo::LocationsRepository;
use crate::runways_repo::RunwaysRepository;
use anyhow::Result;
use chrono::Utc;
use tracing::{debug, warn};
use uuid::Uuid;

use super::ActiveFlightsMap;
use super::altitude::calculate_altitude_offset_ft;
use super::flight_lifecycle::{complete_flight, create_flight};

/// Determine if aircraft should be active based on fix data
fn should_be_active(fix: &Fix) -> bool {
    // Check ground speed
    let speed_indicates_active = fix.ground_speed_knots.map(|s| s >= 20.0).unwrap_or(false);

    if speed_indicates_active {
        return true;
    }

    // Speed is low - check altitude to see if still airborne
    // Don't register landing if AGL altitude is >= 250 feet
    if let Some(altitude_agl) = fix.altitude_agl
        && altitude_agl >= 250
    {
        // Still too high to land - remain active
        return true;
    }

    // Speed is low and either altitude is unavailable or < 250 feet AGL
    false
}

/// Process state transition for an aircraft and return updated fix with flight_id
/// active_flights: device_id -> (flight_id, last_fix_timestamp, last_update_time)
#[allow(clippy::too_many_arguments)]
pub(crate) async fn process_state_transition(
    flights_repo: &FlightsRepository,
    airports_repo: &AirportsRepository,
    locations_repo: &LocationsRepository,
    runways_repo: &RunwaysRepository,
    fixes_repo: &FixesRepository,
    elevation_db: &ElevationDB,
    active_flights: &ActiveFlightsMap,
    mut fix: Fix,
) -> Result<Fix> {
    let is_active = should_be_active(&fix);
    let current_flight_id = {
        let flights = active_flights.read().await;
        flights
            .get(&fix.device_id)
            .map(|(flight_id, _, _)| *flight_id)
    };

    match (current_flight_id, is_active) {
        (None, true) => {
            // Device should be active but has no flight - create one
            debug!("Creating new flight for device {}", fix.device_id);

            // Generate flight_id
            let flight_id = Uuid::new_v4();

            // Check altitude offset to determine if we should skip airport/runway lookup
            let altitude_offset = calculate_altitude_offset_ft(elevation_db, &fix).await;
            let skip_airport_runway_lookup = altitude_offset
                .map(|offset| {
                    if offset.abs() > 250 {
                        warn!(
                            "Large altitude offset ({} ft) for device {} - skipping airport/runway lookup",
                            offset, fix.device_id
                        );
                        true
                    } else {
                        false
                    }
                })
                .unwrap_or(false);

            // Add to active flights map BEFORE creating flight in database
            {
                let mut flights = active_flights.write().await;
                flights.insert(fix.device_id, (flight_id, fix.timestamp, Utc::now()));
            }

            // Create flight synchronously
            match create_flight(
                flights_repo,
                airports_repo,
                locations_repo,
                runways_repo,
                fixes_repo,
                elevation_db,
                &fix,
                flight_id,
                skip_airport_runway_lookup,
            )
            .await
            {
                Ok(_) => {
                    // Flight created successfully - set flight_id on fix
                    fix.flight_id = Some(flight_id);
                }
                Err(e) => {
                    warn!("Failed to create flight: {}", e);
                    // Remove from active flights map on error
                    let mut flights = active_flights.write().await;
                    flights.remove(&fix.device_id);
                }
            }
        }
        (Some(flight_id), false) => {
            // Device has flight but should now be idle - complete the flight
            debug!(
                "Completing flight {} for device {}",
                flight_id, fix.device_id
            );

            // Keep flight_id on this fix since it's part of the flight
            fix.flight_id = Some(flight_id);

            // Remove from active flights map immediately
            {
                let mut flights = active_flights.write().await;
                flights.remove(&fix.device_id);
            }

            // Complete flight in background (don't block)
            let flights_repo_clone = flights_repo.clone();
            let airports_repo_clone = airports_repo.clone();
            let locations_repo_clone = locations_repo.clone();
            let runways_repo_clone = runways_repo.clone();
            let fixes_repo_clone = fixes_repo.clone();
            let elevation_db_clone = elevation_db.clone();
            let landing_fix = fix.clone();
            tokio::spawn(async move {
                if let Err(e) = complete_flight(
                    &flights_repo_clone,
                    &airports_repo_clone,
                    &locations_repo_clone,
                    &runways_repo_clone,
                    &fixes_repo_clone,
                    &elevation_db_clone,
                    flight_id,
                    &landing_fix,
                )
                .await
                {
                    warn!("Failed to complete flight {}: {}", flight_id, e);
                }
            });
        }
        (Some(flight_id), true) => {
            // Device has flight and is still active - just update
            fix.flight_id = Some(flight_id);

            // Update last fix timestamp and last update time
            {
                let mut flights = active_flights.write().await;
                if let Some(entry) = flights.get_mut(&fix.device_id) {
                    entry.1 = fix.timestamp;
                    entry.2 = Utc::now();
                }
            }
        }
        (None, false) => {
            // Device has no flight and should be idle - check if we need to create an airborne flight
            // Only create airborne flight if we've never seen this device before
            // For now, just do nothing - fixes without flights are fine
            debug!("Device {} is idle with no active flight", fix.device_id);
        }
    }

    Ok(fix)
}
