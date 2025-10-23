use crate::Fix;
use crate::airports_repo::AirportsRepository;
use crate::elevation::ElevationDB;
use crate::fixes_repo::FixesRepository;
use crate::flights_repo::FlightsRepository;
use crate::locations_repo::LocationsRepository;
use crate::runways_repo::RunwaysRepository;
use anyhow::Result;
use tracing::{error, info, trace, warn};
use uuid::Uuid;

use super::altitude::calculate_altitude_agl;
use super::altitude::calculate_altitude_offset_ft;
use super::flight_lifecycle::{complete_flight, create_flight};
use super::{ActiveFlightsMap, CurrentFlightState};

/// Helper function to update last_fix_at timestamp in database
/// Logs error if update fails but doesn't propagate the error
async fn update_flight_timestamp(
    flights_repo: &FlightsRepository,
    flight_id: Uuid,
    timestamp: chrono::DateTime<chrono::Utc>,
) {
    if let Err(e) = flights_repo.update_last_fix_at(flight_id, timestamp).await {
        error!(
            "Failed to update last_fix_at for flight {}: {}",
            flight_id, e
        );
    }
}

/// Determine if aircraft should be active based on fix data
/// This checks ground speed first, then altitude (if available)
pub(crate) fn should_be_active(fix: &Fix) -> bool {
    // Special case: If no altitude data and speed < 80 knots, consider inactive
    // This handles cases where altitude data is missing but we can still infer ground state from speed
    if fix.altitude_agl.is_none() && fix.altitude_msl_feet.is_none() {
        let speed_knots = fix.ground_speed_knots.unwrap_or(0.0);
        if speed_knots < 80.0 {
            // No altitude data and slow speed - likely on ground
            return false;
        }
        // No altitude data but high speed - assume active/airborne
        return true;
    }

    // Check ground speed - >= 20 knots means active
    let speed_indicates_active = fix.ground_speed_knots.map(|s| s >= 20.0).unwrap_or(false);

    if speed_indicates_active {
        return true;
    }

    // Speed is low - check altitude to see if still airborne
    // Don't register landing if AGL altitude is >= 250 feet (hovering helicopter, slow glider)
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

    // Get current flight state
    let current_flight_state = {
        let flights = active_flights.read().await;
        flights.get(&fix.device_id).cloned()
    };

    match (current_flight_state, is_active) {
        // Case 1: Active flight exists and current fix is active
        (Some(mut state), true) => {
            trace!(
                "Device {} has active flight {} - continuing flight",
                fix.device_id, state.flight_id
            );

            // Assign existing flight_id to this fix
            fix.flight_id = Some(state.flight_id);

            // Update last_fix_at in database
            update_flight_timestamp(flights_repo, state.flight_id, fix.timestamp).await;

            // Update the state with this fix
            state.update(fix.timestamp, is_active);

            // Write back updated state
            let mut flights = active_flights.write().await;
            flights.insert(fix.device_id, state);
        }

        // Case 2: No flight and fix is active - need to create a flight
        (None, true) => {
            // Check if this is a takeoff or mid-flight appearance
            // We need to query recent fixes to determine this
            let recent_fixes = fixes_repo
                .get_fixes_for_device(fix.device_id, Some(3))
                .await
                .unwrap_or_default();

            // If we have 3+ recent fixes and they're all inactive, this is a takeoff
            let is_takeoff =
                recent_fixes.len() >= 3 && recent_fixes.iter().all(|f| !should_be_active(f));

            let flight_id = Uuid::new_v4();

            if is_takeoff {
                // Case 2a: Taking off - last fixes were inactive
                info!(
                    "Device {} is taking off (recent fixes were inactive) - creating flight {} with airport lookup",
                    fix.device_id, flight_id
                );

                // Calculate altitude offset and check if reasonable for takeoff
                let altitude_offset = calculate_altitude_offset_ft(elevation_db, &fix).await;
                if let Some(offset) = altitude_offset
                    && offset.abs() > 250
                {
                    warn!(
                        "Large altitude offset at takeoff: {} ft for device {} at ({:.6}, {:.6}). Aircraft should be on ground at takeoff.",
                        offset, fix.device_id, fix.latitude, fix.longitude
                    );
                }

                // Create flight state and add to map BEFORE creating in database
                let state = CurrentFlightState::new(flight_id, fix.timestamp, is_active);
                {
                    let mut flights = active_flights.write().await;
                    flights.insert(fix.device_id, state);
                }

                // Create flight WITH airport/runway lookup
                match create_flight(
                    flights_repo,
                    airports_repo,
                    locations_repo,
                    runways_repo,
                    fixes_repo,
                    elevation_db,
                    &fix,
                    flight_id,
                    false, // DO perform airport/runway lookup
                )
                .await
                {
                    Ok(_) => {
                        fix.flight_id = Some(flight_id);
                        // Note: last_fix_at is already set during flight creation
                    }
                    Err(e) => {
                        warn!("Failed to create flight: {}", e);
                        let mut flights = active_flights.write().await;
                        flights.remove(&fix.device_id);
                    }
                }
            } else {
                // Case 2b: First fix is active OR recent fixes were also active - appearing mid-flight
                info!(
                    "Device {} appearing in-flight (first fix or recent fixes active) - creating flight {} without airport lookup (ground speed: {:?} kts, altitude MSL: {:?} ft)",
                    fix.device_id, flight_id, fix.ground_speed_knots, fix.altitude_msl_feet
                );

                // Create flight state and add to map
                let state = CurrentFlightState::new(flight_id, fix.timestamp, is_active);
                {
                    let mut flights = active_flights.write().await;
                    flights.insert(fix.device_id, state);
                }

                // Create flight WITHOUT airport/runway lookup (skip_airport_runway_lookup = true)
                match create_flight(
                    flights_repo,
                    airports_repo,
                    locations_repo,
                    runways_repo,
                    fixes_repo,
                    elevation_db,
                    &fix,
                    flight_id,
                    true, // SKIP airport/runway lookup for mid-flight appearance
                )
                .await
                {
                    Ok(_) => {
                        fix.flight_id = Some(flight_id);
                        // Note: last_fix_at is already set during flight creation
                    }
                    Err(e) => {
                        warn!("Failed to create flight: {}", e);
                        let mut flights = active_flights.write().await;
                        flights.remove(&fix.device_id);
                    }
                }
            }
        }

        // Case 3a: No flight and fix is inactive - idle aircraft on ground
        (None, false) => {
            trace!(
                "Device {} is idle on ground with no active flight",
                fix.device_id
            );
            // Just save the fix without a flight_id
        }

        // Case 3b: Flight exists but fix is inactive - landing or still airborne?
        (Some(mut state), false) => {
            let flight_id = state.flight_id;

            // Calculate AGL to determine if we're actually landing or just slow at altitude
            let agl = calculate_altitude_agl(elevation_db, &fix).await;

            match agl {
                Some(altitude_agl) if altitude_agl >= 250 => {
                    // Case 3b2: Still airborne (>= 250 ft AGL) - slow moving aircraft
                    info!(
                        "Device {} slow but still airborne at {} ft AGL - continuing flight {}",
                        fix.device_id, altitude_agl, flight_id
                    );

                    // Keep the flight active, assign flight_id to fix
                    fix.flight_id = Some(flight_id);

                    // Update last_fix_at in database
                    update_flight_timestamp(flights_repo, flight_id, fix.timestamp).await;

                    // Update altitude_agl on the fix
                    fix.altitude_agl = Some(altitude_agl);

                    // Update state (still treat as active even though speed is low)
                    state.update(fix.timestamp, true); // Force active since airborne

                    let mut flights = active_flights.write().await;
                    flights.insert(fix.device_id, state);
                }
                _ => {
                    // Case 3b1: Landing (< 250 ft AGL OR elevation unknown)
                    // Update state with inactive fix
                    state.update(fix.timestamp, false);

                    // Check if we have 5 consecutive inactive fixes
                    if state.has_five_consecutive_inactive() {
                        info!(
                            "Device {} landing after 5 consecutive inactive fixes (AGL: {:?} ft) - completing flight {}",
                            fix.device_id, agl, flight_id
                        );

                        // Assign flight_id to this landing fix
                        fix.flight_id = Some(flight_id);

                        // Update altitude_agl if we have it
                        if let Some(altitude_agl) = agl {
                            fix.altitude_agl = Some(altitude_agl);
                        }

                        // Remove from active flights map immediately
                        {
                            let mut flights = active_flights.write().await;
                            flights.remove(&fix.device_id);
                        }

                        // Complete flight in background (includes airport/runway lookup for landing)
                        // Note: complete_flight will update both landing fields AND last_fix_at in a single UPDATE
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
                    } else {
                        // Not enough consecutive inactive fixes yet - keep flight active
                        info!(
                            "Device {} inactive (AGL: {:?} ft) but waiting for more inactive fixes ({}/5) - continuing flight {}",
                            fix.device_id,
                            agl,
                            state
                                .recent_fix_history
                                .iter()
                                .filter(|&&active| !active)
                                .count(),
                            flight_id
                        );

                        // Assign flight_id to this fix
                        fix.flight_id = Some(flight_id);

                        // Update last_fix_at in database
                        update_flight_timestamp(flights_repo, flight_id, fix.timestamp).await;

                        // Update altitude_agl if we have it
                        if let Some(altitude_agl) = agl {
                            fix.altitude_agl = Some(altitude_agl);
                        }

                        // Keep the updated state in the map
                        let mut flights = active_flights.write().await;
                        flights.insert(fix.device_id, state);
                    }
                }
            }
        }
    }

    Ok(fix)
}
