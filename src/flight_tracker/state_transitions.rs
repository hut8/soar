use crate::Fix;
use crate::flights::TimeoutPhase;
use crate::flights_repo::FlightsRepository;
use crate::ogn_aprs_aircraft::AircraftType;
use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, error, info, trace, warn};
use uuid::Uuid;

use super::aircraft_tracker;
use super::altitude::calculate_altitude_agl;
use super::altitude::calculate_altitude_offset_ft;
use super::flight_lifecycle::{complete_flight, create_flight};
use super::towing;
use super::{CurrentFlightState, FlightProcessorContext};

/// Helper function to update last_fix_at timestamp in database
/// Logs error if update fails but doesn't propagate the error
async fn update_flight_timestamp(
    flights_repo: &FlightsRepository,
    flight_id: Uuid,
    timestamp: chrono::DateTime<chrono::Utc>,
) {
    let start = std::time::Instant::now();
    if let Err(e) = flights_repo.update_last_fix_at(flight_id, timestamp).await {
        error!(
            "Failed to update last_fix_at for flight {}: {}",
            flight_id, e
        );
    }
    metrics::histogram!("aprs.aircraft.flight_update_last_fix_ms")
        .record(start.elapsed().as_micros() as f64 / 1000.0);
}

/// Determine if aircraft should be active based on fix data
/// This checks ground speed first, then altitude (if available)
pub(crate) fn should_be_active(fix: &Fix) -> bool {
    // Special case: If no altitude data and speed < 80 knots, consider inactive
    // This handles cases where altitude data is missing but we can still infer ground state from speed
    if fix.altitude_agl_feet.is_none() && fix.altitude_msl_feet.is_none() {
        let speed_knots = fix.ground_speed_knots.unwrap_or(0.0);
        if speed_knots < 80.0 {
            // No altitude data and slow speed - likely on ground
            return false;
        }
        // No altitude data but high speed - assume active/airborne
        return true;
    }

    // Check ground speed - >= 25 knots means active
    let speed_indicates_active = fix.ground_speed_knots.map(|s| s >= 25.0).unwrap_or(false);

    if speed_indicates_active {
        return true;
    }

    // Speed is low - check altitude to see if still airborne
    // Don't register landing if AGL altitude is >= 250 feet (hovering helicopter, slow glider)
    if let Some(altitude_agl) = fix.altitude_agl_feet
        && altitude_agl >= 250
    {
        // Still too high to land - remain active
        return true;
    }

    // Speed is low and either altitude is unavailable or < 250 feet AGL
    false
}

/// Process state transition for an aircraft and return updated fix with flight_id
pub(crate) async fn process_state_transition(
    ctx: &FlightProcessorContext<'_>,
    mut fix: Fix,
) -> Result<Fix> {
    let is_active = should_be_active(&fix);

    // Fetch aircraft once for use throughout the function
    let aircraft_lookup_start = std::time::Instant::now();
    let aircraft = ctx
        .aircraft_repo
        .get_aircraft_by_id(fix.aircraft_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Aircraft {} not found", fix.aircraft_id))?;
    metrics::histogram!("aprs.aircraft.aircraft_lookup_ms")
        .record(aircraft_lookup_start.elapsed().as_micros() as f64 / 1000.0);

    let is_towtug = aircraft.aircraft_type_ogn == Some(AircraftType::TowTug);

    // Get current flight state
    let current_flight_state = {
        let flights = ctx.active_flights.read().await;
        flights.get(&fix.aircraft_id).cloned()
    };

    match (current_flight_state, is_active) {
        // Case 1: Active flight exists and current fix is active
        (Some(mut state), true) => {
            // Check if callsign has changed - if both current flight and new fix have callsigns
            // and they differ, this indicates a different aircraft/flight. End current flight and create new one.
            let should_end_flight = if let Some(new_callsign) = &fix.flight_number {
                if let Ok(Some(current_flight)) =
                    ctx.flights_repo.get_flight_by_id(state.flight_id).await
                {
                    if let Some(current_callsign) = &current_flight.callsign {
                        if current_callsign != new_callsign {
                            info!(
                                "Aircraft {} active flight {} has callsign '{}' but new fix has callsign '{}' - ending flight and creating new one",
                                fix.aircraft_id, state.flight_id, current_callsign, new_callsign
                            );
                            true
                        } else {
                            false
                        }
                    } else {
                        false // Current flight has no callsign, OK to continue
                    }
                } else {
                    false // Couldn't fetch flight, continue anyway
                }
            } else {
                false // New fix has no callsign, OK to continue
            };

            if should_end_flight {
                // End the current flight
                if let Err(e) = complete_flight(ctx, &aircraft, state.flight_id, &fix).await {
                    error!(
                        "Failed to complete flight {} due to callsign change: {}",
                        state.flight_id, e
                    );
                }

                // Create a new flight for this fix
                let new_flight_id = Uuid::new_v4();
                match create_flight(
                    ctx,
                    &fix,
                    new_flight_id,
                    false, // Don't skip airport lookup - this is a new flight
                )
                .await
                {
                    Ok(flight_id) => {
                        info!(
                            "Created new flight {} for device {} with callsign {:?}",
                            flight_id, fix.aircraft_id, fix.flight_number
                        );

                        // Assign the new flight_id to this fix
                        fix.flight_id = Some(flight_id);

                        // Create new active flight state
                        let mut new_state =
                            CurrentFlightState::new(flight_id, fix.timestamp, is_active);
                        new_state.last_altitude_msl_ft = fix.altitude_msl_feet;
                        new_state.last_climb_fpm = fix.climb_fpm;
                        new_state.last_position = (fix.latitude, fix.longitude);

                        // Add to active flights
                        let mut flights = ctx.active_flights.write().await;
                        flights.insert(fix.aircraft_id, new_state);
                    }
                    Err(e) => {
                        error!(
                            "Failed to create new flight for device {} after callsign change: {}",
                            fix.aircraft_id, e
                        );
                    }
                }
            } else {
                // Callsign hasn't changed (or not applicable), continue existing flight
                trace!(
                    "Aircraft {} has active flight {} - continuing flight",
                    fix.aircraft_id, state.flight_id
                );

                // Assign existing flight_id to this fix
                fix.flight_id = Some(state.flight_id);

                // Calculate time gap from last fix in the same flight
                let time_gap_seconds =
                    (fix.timestamp - state.last_fix_timestamp).num_seconds() as i32;
                fix.time_gap_seconds = Some(time_gap_seconds);

                // Update last_fix_at in database
                update_flight_timestamp(ctx.flights_repo, state.flight_id, fix.timestamp).await;

                // For towplanes: track climb rate and check for tow release
                if is_towtug && let Some(climb_fpm) = fix.climb_fpm {
                    // Update aircraft tracker with climb rate and check for release
                    let mut trackers = ctx.aircraft_trackers.write().await;
                    let tracker = trackers.entry(fix.aircraft_id).or_insert_with(|| {
                        aircraft_tracker::AircraftTracker::new(
                            aircraft_tracker::AircraftState::Active,
                        )
                    });

                    let climb_fpm_f32 = climb_fpm as f32;
                    tracker.update_climb_rate(climb_fpm_f32);
                    tracker.current_flight_id = Some(state.flight_id);

                    // Check if tow has been released
                    if towing::check_tow_release(tracker, Some(climb_fpm_f32)) {
                        // Record the release
                        if let Some(towing_info) = &tracker.towing_info {
                            if let Some(altitude_ft) = fix.altitude_msl_feet {
                                info!(
                                    "Recording tow release for glider {} at {}ft MSL",
                                    towing_info.glider_device_id, altitude_ft
                                );

                                if let Err(e) = ctx
                                    .flights_repo
                                    .update_tow_release(
                                        towing_info.glider_flight_id,
                                        altitude_ft,
                                        fix.timestamp,
                                    )
                                    .await
                                {
                                    error!(
                                        "Failed to record tow release for glider {}: {}",
                                        towing_info.glider_device_id, e
                                    );
                                }
                            }

                            // Clear towing info after release
                            tracker.towing_info = None;
                            tracker.climb_rate_history.clear();
                        }
                    }
                }

                // Update the state with this fix
                state.update(fix.timestamp, is_active);
                state.last_altitude_msl_ft = fix.altitude_msl_feet;
                state.last_climb_fpm = fix.climb_fpm;
                state.last_position = (fix.latitude, fix.longitude);

                // Write back updated state
                let mut flights = ctx.active_flights.write().await;
                flights.insert(fix.aircraft_id, state);
            }
        }

        // Case 2: No flight and fix is active - need to create a flight
        (None, true) => {
            // First, check if we should resume a recently timed-out flight (flight coalescing).
            // This handles the case where an aircraft temporarily goes out of receiver range
            // (e.g., trans-atlantic flight) and then comes back into range. Without coalescing,
            // we would create two separate flights. With coalescing, we resume the original flight.
            if let Ok(Some(timed_out_flight)) = ctx
                .flights_repo
                .find_recent_timed_out_flight(fix.aircraft_id)
                .await
            {
                // Check if callsigns differ - if both flights have callsigns and they differ,
                // do NOT coalesce (this indicates a different aircraft/flight)
                let should_coalesce = match (&timed_out_flight.callsign, &fix.flight_number) {
                    (Some(prev_callsign), Some(new_callsign)) if prev_callsign != new_callsign => {
                        debug!(
                            "Aircraft {} came back into range but callsigns differ (previous: '{}', new: '{}') - NOT coalescing, will create new flight",
                            fix.aircraft_id, prev_callsign, new_callsign
                        );
                        metrics::counter!("flight_tracker.coalesce.callsign_mismatch").increment(1);
                        false
                    }
                    _ => true, // Coalesce if either has no callsign, or callsigns match
                };

                if should_coalesce {
                    // Check 2: Calculate distance between last fix and new fix
                    let last_fix = ctx
                        .fixes_repo
                        .get_last_fix_for_flight(timed_out_flight.id)
                        .await
                        .ok()
                        .flatten();

                    let distance_km = if let Some(last) = &last_fix {
                        crate::flights::haversine_distance(
                            last.latitude,
                            last.longitude,
                            fix.latitude,
                            fix.longitude,
                        ) / 1000.0 // Convert meters to km
                    } else {
                        0.0
                    };

                    // Check 3: Phase-based validation - determine if this looks like a landing + new takeoff
                    let looks_like_landing = match timed_out_flight.timeout_phase {
                        Some(TimeoutPhase::Descending) => {
                            // Aircraft was descending when it timed out
                            // Check if new fix is at low altitude climbing (indicates new takeoff)
                            let new_fix_low_altitude =
                                fix.altitude_agl_feet.map(|agl| agl < 1000).unwrap_or(false);

                            let new_fix_climbing =
                                fix.climb_fpm.map(|climb| climb > 300).unwrap_or(false);

                            let far_apart = distance_km > 100.0; // More than 100km away

                            // If at low altitude, climbing, and far from timeout position → landed
                            new_fix_low_altitude && new_fix_climbing && far_apart
                        }
                        Some(TimeoutPhase::Cruising) => {
                            // Aircraft was cruising - only reject if VERY far apart
                            // This allows trans-Atlantic flights
                            distance_km > 3000.0 // More than 3000km suggests different flight
                        }
                        Some(TimeoutPhase::Climbing) => {
                            // Climbing phase timeout is unusual, be conservative
                            distance_km > 500.0
                        }
                        Some(TimeoutPhase::Unknown) | None => {
                            // Unknown phase - use conservative distance threshold
                            distance_km > 500.0
                        }
                    };

                    // Override: Same position always resumes (glider/helicopter case)
                    let should_resume = distance_km < 1.0 || !looks_like_landing;

                    if !should_resume {
                        info!(
                            "Aircraft {} NOT coalescing - probable landing detected. \
                             Phase: {:?}, Distance: {:.1}km, New alt AGL: {:?}ft, New climb: {:?}fpm",
                            fix.aircraft_id,
                            timed_out_flight.timeout_phase,
                            distance_km,
                            fix.altitude_agl_feet,
                            fix.climb_fpm
                        );
                        metrics::counter!("flight_tracker.coalesce.rejected.probable_landing")
                            .increment(1);
                        metrics::histogram!("flight_tracker.coalesce.rejected.distance_km")
                            .record(distance_km);
                        // Don't resume - fall through to create new flight
                    } else {
                        // Resume the timed-out flight
                        let flight_id = timed_out_flight.id;
                        info!(
                            "Aircraft {} resuming flight {} after timeout. \
                             Phase: {:?}, Distance: {:.1}km, Gap: {}s",
                            fix.aircraft_id,
                            flight_id,
                            timed_out_flight.timeout_phase,
                            distance_km,
                            (fix.timestamp - timed_out_flight.last_fix_at).num_seconds()
                        );

                        metrics::counter!("flight_tracker.coalesce.resumed").increment(1);
                        metrics::histogram!("flight_tracker.coalesce.resumed.distance_km")
                            .record(distance_km);

                        // Clear the timeout in the database
                        if let Err(e) = ctx.flights_repo.clear_timeout(flight_id).await {
                            warn!("Failed to clear timeout for flight {}: {}", flight_id, e);
                        }

                        // Update last_fix_at to current fix timestamp
                        if let Err(e) = ctx
                            .flights_repo
                            .update_last_fix_at(flight_id, fix.timestamp)
                            .await
                        {
                            warn!(
                                "Failed to update last_fix_at for resumed flight {}: {}",
                                flight_id, e
                            );
                        }

                        // Add flight back to ctx.active_flights map with updated position/altitude
                        let mut state =
                            CurrentFlightState::new(flight_id, fix.timestamp, is_active);
                        state.last_altitude_msl_ft = fix.altitude_msl_feet;
                        state.last_climb_fpm = fix.climb_fpm;
                        state.last_position = (fix.latitude, fix.longitude);
                        {
                            let mut flights = ctx.active_flights.write().await;
                            flights.insert(fix.aircraft_id, state);
                        }

                        // Assign fix to the resumed flight
                        fix.flight_id = Some(flight_id);

                        // Calculate time gap from the last fix in the resumed flight
                        // This will capture the gap during the timeout period
                        let time_gap_seconds =
                            (fix.timestamp - timed_out_flight.last_fix_at).num_seconds() as i32;
                        fix.time_gap_seconds = Some(time_gap_seconds);

                        // Return the fix with the resumed flight_id
                        return Ok(fix);
                    }
                }
            }

            // No recent timed-out flight to resume, so create a new flight
            // Check if this is a takeoff or mid-flight appearance
            // We need to query recent fixes to determine this
            metrics::counter!("flight_tracker.coalesce.no_timeout_flight").increment(1);
            let recent_fixes = ctx
                .fixes_repo
                .get_fixes_for_aircraft(fix.aircraft_id, Some(3), None)
                .await
                .unwrap_or_default();

            // Calculate AGL altitude to determine if we're on the ground
            let agl_altitude = calculate_altitude_agl(ctx.elevation_db, &fix).await;

            // Determine if this is a takeoff:
            // 1. If we have 3+ recent fixes and they're all inactive, it's a takeoff
            // 2. If the current fix is at ground level (AGL < 100 ft), it's a takeoff
            let is_takeoff = (recent_fixes.len() >= 3
                && recent_fixes.iter().all(|f| !should_be_active(f)))
                || agl_altitude.map(|agl| agl < 100).unwrap_or(false);

            let flight_id = Uuid::now_v7();

            if is_takeoff {
                // Case 2a: Taking off - last fixes were inactive
                debug!(
                    "Aircraft {} is taking off (recent fixes were inactive) - creating flight {} with airport lookup",
                    fix.aircraft_id, flight_id
                );

                // Calculate altitude offset and check if reasonable for takeoff
                let altitude_offset = calculate_altitude_offset_ft(ctx.elevation_db, &fix).await;
                if let Some(offset) = altitude_offset
                    && offset.abs() > 250
                {
                    info!(
                        "Flight is initialized while airborne: {} ft for device {} at ({:.6}, {:.6}). Aircraft should be on ground at takeoff.",
                        offset, fix.aircraft_id, fix.latitude, fix.longitude
                    );
                }

                // Create flight state and add to map BEFORE creating in database
                let mut state = CurrentFlightState::new(flight_id, fix.timestamp, is_active);
                state.last_altitude_msl_ft = fix.altitude_msl_feet;
                state.last_climb_fpm = fix.climb_fpm;
                state.last_position = (fix.latitude, fix.longitude);
                {
                    let mut flights = ctx.active_flights.write().await;
                    flights.insert(fix.aircraft_id, state);
                }

                // Create flight WITH airport/runway lookup
                match create_flight(
                    ctx, &fix, flight_id, false, // DO perform airport/runway lookup
                )
                .await
                {
                    Ok(_) => {
                        fix.flight_id = Some(flight_id);
                        metrics::counter!("flight_tracker.flight_created.takeoff").increment(1);

                        // If this is a towplane taking off, spawn towing detection task
                        if is_towtug {
                            debug!(
                                "Towplane {} taking off - spawning towing detection task",
                                fix.aircraft_id
                            );
                            towing::spawn_towing_detection_task(
                                fix.aircraft_id,
                                flight_id,
                                ctx.fixes_repo.clone(),
                                ctx.flights_repo.clone(),
                                ctx.aircraft_repo.clone(),
                                Arc::clone(ctx.aircraft_trackers),
                            );
                        }
                    }
                    Err(e) => {
                        error!(
                            "Failed to create flight for aircraft {}: {}",
                            fix.aircraft_id, e
                        );
                        let mut flights = ctx.active_flights.write().await;
                        flights.remove(&fix.aircraft_id);
                    }
                }
            } else {
                // Case 2b: First fix is active OR recent fixes were also active - appearing mid-flight
                debug!(
                    "Aircraft {} appearing in-flight (first fix or recent fixes active) - creating flight {} without airport lookup (ground speed: {:?} kts, altitude MSL: {:?} ft)",
                    fix.aircraft_id, flight_id, fix.ground_speed_knots, fix.altitude_msl_feet
                );

                // Create flight state and add to map
                let mut state = CurrentFlightState::new(flight_id, fix.timestamp, is_active);
                state.last_altitude_msl_ft = fix.altitude_msl_feet;
                state.last_climb_fpm = fix.climb_fpm;
                state.last_position = (fix.latitude, fix.longitude);
                {
                    let mut flights = ctx.active_flights.write().await;
                    flights.insert(fix.aircraft_id, state);
                }

                // Create flight WITHOUT airport/runway lookup (skip_airport_runway_lookup = true)
                match create_flight(
                    ctx, &fix, flight_id,
                    true, // SKIP airport/runway lookup for mid-flight appearance
                )
                .await
                {
                    Ok(_) => {
                        fix.flight_id = Some(flight_id);
                        metrics::counter!("flight_tracker.flight_created.airborne").increment(1);
                        // Note: last_fix_at is already set during flight creation
                    }
                    Err(e) => {
                        error!(
                            "Failed to create flight for aircraft {}: {}",
                            fix.aircraft_id, e
                        );
                        let mut flights = ctx.active_flights.write().await;
                        flights.remove(&fix.aircraft_id);
                    }
                }
            }
        }

        // Case 3a: No flight and fix is inactive - idle aircraft on ground
        (None, false) => {
            trace!(
                "Aircraft {} is idle on ground with no active flight",
                fix.aircraft_id
            );
            // Just save the fix without a flight_id
        }

        // Case 3b: Flight exists but fix is inactive - landing or still airborne?
        (Some(mut state), false) => {
            let flight_id = state.flight_id;

            // Calculate AGL to determine if we're actually landing or just slow at altitude
            let agl = calculate_altitude_agl(ctx.elevation_db, &fix).await;

            match agl {
                Some(altitude_agl) if altitude_agl >= 250 => {
                    // Case 3b2: Still airborne (>= 250 ft AGL) - slow moving aircraft
                    let speed_knots = fix.ground_speed_knots.unwrap_or(0.0);
                    info!(
                        "Aircraft {} slow but still airborne at {} ft AGL ({:.1} knots) - continuing flight {}",
                        fix.aircraft_id, altitude_agl, speed_knots, flight_id
                    );

                    // Keep the flight active, assign flight_id to fix
                    fix.flight_id = Some(flight_id);

                    // Calculate time gap from last fix in the same flight
                    let time_gap_seconds =
                        (fix.timestamp - state.last_fix_timestamp).num_seconds() as i32;
                    fix.time_gap_seconds = Some(time_gap_seconds);

                    // Update last_fix_at in database
                    update_flight_timestamp(ctx.flights_repo, flight_id, fix.timestamp).await;

                    // Update altitude_agl_feet on the fix
                    fix.altitude_agl_feet = Some(altitude_agl);

                    // Update state (still treat as active even though speed is low)
                    state.update(fix.timestamp, true); // Force active since airborne
                    state.last_altitude_msl_ft = fix.altitude_msl_feet;
                    state.last_climb_fpm = fix.climb_fpm;
                    state.last_position = (fix.latitude, fix.longitude);

                    let mut flights = ctx.active_flights.write().await;
                    flights.insert(fix.aircraft_id, state);
                }
                _ => {
                    // Case 3b1: Landing (< 250 ft AGL OR elevation unknown)
                    // Update state with inactive fix
                    state.update(fix.timestamp, false);
                    state.last_altitude_msl_ft = fix.altitude_msl_feet;
                    state.last_climb_fpm = fix.climb_fpm;
                    state.last_position = (fix.latitude, fix.longitude);

                    // Check if we have 5 consecutive inactive fixes
                    if state.has_five_consecutive_inactive() {
                        debug!(
                            "Aircraft {} landing after 5 consecutive inactive fixes (AGL: {:?} ft) - completing flight {}",
                            fix.aircraft_id, agl, flight_id
                        );

                        // Assign flight_id to this landing fix
                        fix.flight_id = Some(flight_id);

                        // Calculate time gap from last fix in the same flight
                        let time_gap_seconds =
                            (fix.timestamp - state.last_fix_timestamp).num_seconds() as i32;
                        fix.time_gap_seconds = Some(time_gap_seconds);

                        // Update altitude_agl_feet if we have it
                        if let Some(altitude_agl) = agl {
                            fix.altitude_agl_feet = Some(altitude_agl);
                        }

                        // Complete flight (includes airport/runway lookup for landing)
                        // Note: complete_flight will update both landing fields AND last_fix_at in a single UPDATE
                        // IMPORTANT: For spurious flights, complete_flight will remove from ctx.active_flights BEFORE deleting
                        // to prevent race condition where new fixes arrive and get assigned the spurious flight_id
                        // For normal landings, we remove from ctx.active_flights AFTER complete_flight finishes
                        let flight_completed =
                            match complete_flight(ctx, &aircraft, flight_id, &fix).await {
                                Ok(completed) => completed,
                                Err(e) => {
                                    warn!("Failed to complete flight {}: {}", flight_id, e);
                                    true // Assume completed on error to proceed with cleanup
                                }
                            };

                        // If flight was deleted as spurious, clear the flight_id from the fix
                        if !flight_completed {
                            info!(
                                "Flight {} was spurious - clearing flight_id from landing fix for device {}",
                                flight_id, fix.aircraft_id
                            );
                            fix.flight_id = None;
                        }

                        // Remove from active flights map AFTER complete_flight finishes (for normal landings)
                        // Note: For spurious flights, this was already done inside complete_flight
                        {
                            let mut flights = ctx.active_flights.write().await;
                            flights.remove(&fix.aircraft_id);
                        }

                        // Clean up the aircraft lock after flight completion
                        {
                            let mut locks = ctx.aircraft_locks.write().await;
                            if locks.remove(&fix.aircraft_id).is_some() {
                                trace!(
                                    "Cleaned up aircraft lock for aircraft {} after landing",
                                    fix.aircraft_id
                                );
                            }
                        }
                    } else {
                        // Not enough consecutive inactive fixes yet - keep flight active
                        debug!(
                            "Aircraft {} inactive (AGL: {:?} ft) but waiting for more inactive fixes ({}/5) - continuing flight {}",
                            fix.aircraft_id,
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

                        // Calculate time gap from last fix in the same flight
                        let time_gap_seconds =
                            (fix.timestamp - state.last_fix_timestamp).num_seconds() as i32;
                        fix.time_gap_seconds = Some(time_gap_seconds);

                        // Update last_fix_at in database
                        update_flight_timestamp(ctx.flights_repo, flight_id, fix.timestamp).await;

                        // Update altitude_agl_feet if we have it
                        if let Some(altitude_agl) = agl {
                            fix.altitude_agl_feet = Some(altitude_agl);
                        }

                        // Keep the updated state in the map
                        let mut flights = ctx.active_flights.write().await;
                        flights.insert(fix.aircraft_id, state);
                    }
                }
            }
        }
    }

    Ok(fix)
}
