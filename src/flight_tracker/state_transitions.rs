use crate::Fix;
use crate::airports_repo::AirportsRepository;
use crate::elevation::ElevationDB;
use crate::fixes_repo::FixesRepository;
use crate::flights_repo::FlightsRepository;
use crate::locations_repo::LocationsRepository;
use crate::runways_repo::RunwaysRepository;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::aircraft_tracker::{AircraftState, AircraftTracker};
use super::altitude::calculate_altitude_offset_ft;
use super::flight_lifecycle::{complete_flight, create_airborne_flight, create_flight};
use super::towing::{check_tow_release, record_tow_release};
use super::utils::format_device_address_with_type;

/// Process state transition for an aircraft and return updated fix with flight_id
#[allow(clippy::too_many_arguments)]
pub(crate) async fn process_state_transition(
    flights_repo: &FlightsRepository,
    airports_repo: &AirportsRepository,
    locations_repo: &LocationsRepository,
    runways_repo: &RunwaysRepository,
    fixes_repo: &FixesRepository,
    elevation_db: &ElevationDB,
    aircraft_trackers: &Arc<RwLock<HashMap<Uuid, AircraftTracker>>>,
    mut fix: Fix,
) -> Result<Fix> {
    // Determine the new state first
    let should_be_active = {
        let trackers = aircraft_trackers.read().await;
        match trackers.get(&fix.device_id) {
            Some(tracker) => tracker.should_be_active(&fix),
            None => {
                // New aircraft - determine initial state
                let ground_speed = fix.ground_speed_knots.unwrap_or(0.0);
                ground_speed >= 20.0
            }
        }
    };

    let new_state = if should_be_active {
        AircraftState::Active
    } else {
        AircraftState::Idle
    };

    // Check if this is an existing aircraft and get the old state
    let (is_existing, old_state, current_flight_id) = {
        let trackers = aircraft_trackers.read().await;
        match trackers.get(&fix.device_id) {
            Some(tracker) => (true, tracker.state.clone(), tracker.current_flight_id),
            None => (false, AircraftState::Idle, None), // Default values for new aircraft
        }
    };

    if is_existing {
        // Handle existing aircraft
        match (old_state, &new_state) {
            (AircraftState::Idle, AircraftState::Active) => {
                // Takeoff detected - update state FIRST to prevent race condition
                debug!(
                    "Takeoff detected for aircraft {}",
                    format_device_address_with_type(
                        fix.device_address_hex().as_ref(),
                        fix.address_type
                    )
                );

                // Check altitude offset to validate this is a real takeoff
                // If offset > 250 ft, the altitude data is likely unreliable
                let altitude_offset = calculate_altitude_offset_ft(elevation_db, &fix).await;
                let skip_flight_creation = if let Some(offset) = altitude_offset {
                    if offset.abs() > 250 {
                        warn!(
                            "Skipping flight creation for aircraft {} due to large altitude offset ({} ft) - altitude data may be unreliable",
                            format_device_address_with_type(
                                fix.device_address_hex().as_ref(),
                                fix.address_type
                            ),
                            offset
                        );
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };

                // Update tracker state immediately to prevent duplicate flight creation
                let mut trackers = aircraft_trackers.write().await;
                if let Some(tracker) = trackers.get_mut(&fix.device_id) {
                    tracker.update_position(&fix);
                    tracker.state = new_state.clone();
                }
                drop(trackers); // Release lock immediately

                // Only create flight if altitude data is reliable
                if !skip_flight_creation {
                    // Create flight in background (similar to landing)
                    let flights_repo_clone = flights_repo.clone();
                    let airports_repo_clone = airports_repo.clone();
                    let locations_repo_clone = locations_repo.clone();
                    let runways_repo_clone = runways_repo.clone();
                    let fixes_repo_clone = fixes_repo.clone();
                    let elevation_db_clone = elevation_db.clone();
                    let trackers_clone = Arc::clone(aircraft_trackers);
                    let takeoff_fix = fix.clone();
                    tokio::spawn(async move {
                        match create_flight(
                            &flights_repo_clone,
                            &airports_repo_clone,
                            &locations_repo_clone,
                            &runways_repo_clone,
                            &fixes_repo_clone,
                            &elevation_db_clone,
                            &trackers_clone,
                            &takeoff_fix,
                        )
                        .await
                        {
                            Ok(flight_id) => {
                                // Update tracker with the flight_id
                                let mut trackers = trackers_clone.write().await;
                                if let Some(tracker) = trackers.get_mut(&takeoff_fix.device_id) {
                                    tracker.current_flight_id = Some(flight_id);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to create flight for takeoff: {}", e);
                            }
                        }
                    });
                }

                // Set flight_id on the fix (it will be set later by the background task)
                // For now, we don't have it yet, so leave it as None
            }
            (AircraftState::Active, AircraftState::Idle) => {
                // Landing detected
                debug!(
                    "Landing detected for aircraft {}",
                    format_device_address_with_type(
                        fix.device_address_hex().as_ref(),
                        fix.address_type
                    )
                );

                // Update tracker state FIRST to prevent race condition
                let mut trackers = aircraft_trackers.write().await;
                if let Some(tracker) = trackers.get_mut(&fix.device_id) {
                    tracker.update_position(&fix);
                    tracker.current_flight_id = None;
                    tracker.state = new_state;
                    // Reset towing state for next flight
                    tracker.towed_by_device_id = None;
                    tracker.tow_released = false;
                }
                drop(trackers); // Release lock immediately

                // Complete flight in background if there was an active flight
                if let Some(flight_id) = current_flight_id {
                    // Keep the flight_id on the fix since it was part of this flight
                    fix.flight_id = Some(flight_id);

                    // Spawn background task to complete flight (don't await)
                    let flights_repo_clone = flights_repo.clone();
                    let airports_repo_clone = airports_repo.clone();
                    let locations_repo_clone = locations_repo.clone();
                    let runways_repo_clone = runways_repo.clone();
                    let fixes_repo_clone = fixes_repo.clone();
                    let elevation_db_clone = elevation_db.clone();
                    let trackers_clone = Arc::clone(aircraft_trackers);
                    let landing_fix = fix.clone();
                    tokio::spawn(async move {
                        if let Err(e) = complete_flight(
                            &flights_repo_clone,
                            &airports_repo_clone,
                            &locations_repo_clone,
                            &runways_repo_clone,
                            &fixes_repo_clone,
                            &elevation_db_clone,
                            &trackers_clone,
                            flight_id,
                            &landing_fix,
                        )
                        .await
                        {
                            warn!("Failed to complete flight for landing: {}", e);
                        }
                    });
                }
            }
            _ => {
                // No state change, just update position
                // If there's an ongoing flight, keep its flight_id
                if let Some(flight_id) = current_flight_id {
                    fix.flight_id = Some(flight_id);

                    // Check if this is a glider being towed that hasn't been released yet
                    let (is_being_towed, towplane_id, already_released) = {
                        let trackers = aircraft_trackers.read().await;
                        if let Some(tracker) = trackers.get(&fix.device_id) {
                            (
                                tracker.towed_by_device_id.is_some(),
                                tracker.towed_by_device_id,
                                tracker.tow_released,
                            )
                        } else {
                            (false, None, false)
                        }
                    };

                    if is_being_towed
                        && !already_released
                        && let Some(towplane_device_id) = towplane_id
                    {
                        // Check for tow release
                        if check_tow_release(
                            fixes_repo,
                            &fix.device_id,
                            &flight_id,
                            &fix,
                            &towplane_device_id,
                        )
                        .await
                        {
                            // Record the release
                            if let Err(e) = record_tow_release(flights_repo, &flight_id, &fix).await
                            {
                                warn!(
                                    "Failed to record tow release for flight {}: {}",
                                    flight_id, e
                                );
                            } else {
                                // Mark as released in tracker
                                let mut trackers = aircraft_trackers.write().await;
                                if let Some(tracker) = trackers.get_mut(&fix.device_id) {
                                    tracker.tow_released = true;
                                }
                            }
                        }
                    }
                }

                let mut trackers = aircraft_trackers.write().await;
                if let Some(tracker) = trackers.get_mut(&fix.device_id) {
                    tracker.update_position(&fix);
                    tracker.state = new_state;
                }
            }
        }
    } else {
        // New aircraft - only create a flight if we detect a true takeoff (idle→active)
        // For aircraft first seen in Active state, just track them without creating a flight
        // A flight will be created later when they transition from active→idle→active (true takeoff)
        let mut new_tracker = AircraftTracker::new(new_state.clone());
        new_tracker.update_position(&fix);

        // Insert tracker FIRST to prevent race condition with duplicate flight creation
        let mut trackers = aircraft_trackers.write().await;
        trackers.insert(fix.device_id, new_tracker);
        info!(
            "Started tracking aircraft {} in {:?} state",
            fix.device_id, new_state
        );
        drop(trackers); // Release lock immediately

        if new_state == AircraftState::Active {
            debug!(
                "New in-flight aircraft detected: {}",
                format_device_address_with_type(
                    fix.device_address_hex().as_ref(),
                    fix.address_type
                )
            );
            // Create flight in background to avoid blocking and prevent race condition
            let flights_repo_clone = flights_repo.clone();
            let fixes_repo_clone = fixes_repo.clone();
            let trackers_clone = Arc::clone(aircraft_trackers);
            let airborne_fix = fix.clone();
            tokio::spawn(async move {
                match create_airborne_flight(&flights_repo_clone, &fixes_repo_clone, &airborne_fix)
                    .await
                {
                    Ok(flight_id) => {
                        // Update tracker with the flight_id
                        let mut trackers = trackers_clone.write().await;
                        if let Some(tracker) = trackers.get_mut(&airborne_fix.device_id) {
                            tracker.current_flight_id = Some(flight_id);
                        }
                        info!(
                            "Created airborne flight {} for aircraft {} (no takeoff data)",
                            flight_id,
                            format_device_address_with_type(
                                airborne_fix.device_address_hex().as_ref(),
                                airborne_fix.address_type
                            )
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to create airborne flight for {}: {}",
                            airborne_fix.device_id, e
                        );
                    }
                }
            });
            // Note: flight_id will be set on subsequent fixes for this aircraft
        }
    }

    Ok(fix)
}
