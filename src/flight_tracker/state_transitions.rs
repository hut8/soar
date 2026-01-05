use crate::Fix;
use crate::flights_repo::FlightsRepository;
use crate::ogn_aprs_aircraft::AircraftType;
use anyhow::Result;
use std::sync::Arc;
use tracing::{error, info, trace};
use uuid::Uuid;

use super::altitude::calculate_altitude_agl;
use super::flight_lifecycle::{create_flight_fast, spawn_complete_flight};
use super::towing;
use super::{AircraftState, FlightProcessorContext};

/// Helper function to update last_fix_at timestamp in database
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
pub fn should_be_active(fix: &Fix) -> bool {
    // Special case: If no altitude data and speed < 80 knots, consider inactive
    if fix.altitude_agl_feet.is_none() && fix.altitude_msl_feet.is_none() {
        let speed_knots = fix.ground_speed_knots.unwrap_or(0.0);
        if speed_knots < 80.0 {
            return false;
        }
        return true;
    }

    // Check ground speed - >= 25 knots means active
    if let Some(speed) = fix.ground_speed_knots
        && speed >= 25.0
    {
        return true;
    }

    // Check AGL altitude - >= 250 ft means active (airborne)
    if let Some(agl) = fix.altitude_agl_feet
        && agl >= 250
    {
        return true;
    }

    // Low speed and low/no altitude = inactive (on ground)
    false
}

/// Process state transition for an aircraft and return updated fix with flight_id
pub(crate) async fn process_state_transition(
    ctx: &FlightProcessorContext<'_>,
    mut fix: Fix,
) -> Result<Fix> {
    let is_active = should_be_active(&fix);

    // Fetch aircraft
    let aircraft_lookup_start = std::time::Instant::now();
    let aircraft = ctx
        .aircraft_repo
        .get_aircraft_by_id(fix.aircraft_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Aircraft {} not found", fix.aircraft_id))?;
    metrics::histogram!("aprs.aircraft.aircraft_lookup_ms")
        .record(aircraft_lookup_start.elapsed().as_micros() as f64 / 1000.0);

    let is_towtug = aircraft.aircraft_type_ogn == Some(AircraftType::TowTug);

    // Get or create aircraft state, add this fix to history
    let current_flight_id = {
        let mut state = ctx
            .aircraft_states
            .entry(fix.aircraft_id)
            .or_insert_with(|| AircraftState::new(&fix, is_active));

        // Add fix to history (if not the initial fix that created the state)
        if state.recent_fixes.len() > 1
            || (state.recent_fixes.len() == 1
                && state.recent_fixes.back().unwrap().timestamp != fix.timestamp)
        {
            state.add_fix(&fix, is_active);
        }

        state.current_flight_id
    };

    match (current_flight_id, is_active) {
        // Case 1: Has active flight AND fix is active -> Continue flight
        (Some(flight_id), true) => {
            // Check for callsign change
            let should_end_flight = if let Some(new_callsign) = &fix.flight_number {
                if let Ok(Some(current_flight)) = ctx.flights_repo.get_flight_by_id(flight_id).await
                {
                    if let Some(current_callsign) = &current_flight.callsign {
                        current_callsign != new_callsign
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            if should_end_flight {
                // End current flight (non-blocking), start new one
                spawn_complete_flight(ctx, &aircraft, flight_id, &fix);

                let new_flight_id = Uuid::now_v7();
                match create_flight_fast(ctx, &fix, new_flight_id, false).await {
                    Ok(flight_id) => {
                        fix.flight_id = Some(flight_id);
                        if let Some(mut state) = ctx.aircraft_states.get_mut(&fix.aircraft_id) {
                            state.current_flight_id = Some(flight_id);
                        }
                    }
                    Err(e) => {
                        error!("Failed to create new flight after callsign change: {}", e);
                        fix.flight_id = None;
                    }
                }
            } else {
                // Continue existing flight
                fix.flight_id = Some(flight_id);

                // Calculate time gap
                if let Some(state) = ctx.aircraft_states.get(&fix.aircraft_id)
                    && let Some(last_fix_time) = state.last_fix_timestamp()
                {
                    let gap_seconds = (fix.timestamp - last_fix_time).num_seconds() as i32;
                    fix.time_gap_seconds = Some(gap_seconds);
                }

                update_flight_timestamp(ctx.flights_repo, flight_id, fix.timestamp).await;

                // Check for tow release (towtugs only)
                if is_towtug
                    && let Some(state) = ctx.aircraft_states.get(&fix.aircraft_id)
                    && towing::check_tow_release(&state, fix.climb_fpm)
                {
                    // Record tow release
                    if let Some(towing_info) = &state.towing_info
                        && let Some(altitude_ft) = fix.altitude_msl_feet
                    {
                        let _ = ctx
                            .flights_repo
                            .update_tow_release(
                                towing_info.glider_flight_id,
                                altitude_ft,
                                fix.timestamp,
                            )
                            .await;
                    }

                    // Clear towing info
                    if let Some(mut state) = ctx.aircraft_states.get_mut(&fix.aircraft_id) {
                        state.towing_info = None;
                    }
                }
            }
        }

        // Case 2: No active flight AND fix is active -> Create new flight
        (None, true) => {
            // Check if we should resume a timed-out flight
            if let Ok(Some(timed_out_flight)) = ctx
                .flights_repo
                .find_recent_timed_out_flight(fix.aircraft_id)
                .await
            {
                // Check callsign mismatch
                let should_coalesce = match (&timed_out_flight.callsign, &fix.flight_number) {
                    (Some(prev), Some(new)) if prev != new => {
                        metrics::counter!("flight_tracker.coalesce.callsign_mismatch_total")
                            .increment(1);
                        false
                    }
                    _ => true,
                };

                if should_coalesce {
                    let gap_seconds = (fix.timestamp - timed_out_flight.last_fix_at).num_seconds();
                    let gap_hours = gap_seconds as f64 / 3600.0;

                    if gap_hours >= 18.0 {
                        // Too long - create new flight
                        metrics::counter!("flight_tracker.coalesce.rejected.hard_limit_18h_total")
                            .increment(1);
                    } else {
                        // Resume the flight
                        info!(
                            "Resuming timed-out flight {} after {:.1}h gap",
                            timed_out_flight.id, gap_hours
                        );

                        fix.flight_id = Some(timed_out_flight.id);

                        // Clear timeout and update last_fix_at
                        let _ = ctx
                            .flights_repo
                            .resume_timed_out_flight(timed_out_flight.id, fix.timestamp)
                            .await;

                        // Update state
                        if let Some(mut state) = ctx.aircraft_states.get_mut(&fix.aircraft_id) {
                            state.current_flight_id = Some(timed_out_flight.id);
                        }

                        metrics::counter!("flight_tracker.coalesce.resumed_total").increment(1);
                        return Ok(fix);
                    }
                }
            }

            // Create new flight - check if takeoff or mid-flight
            let is_takeoff = if let Some(state) = ctx.aircraft_states.get(&fix.aircraft_id) {
                // Check last 3 fixes - if all inactive, it's a takeoff
                state.last_n_inactive(3)
            } else {
                // No recent fixes - check AGL
                calculate_altitude_agl(ctx.elevation_db, &fix)
                    .await
                    .map(|agl| agl < 100)
                    .unwrap_or(false)
            };

            let flight_id = Uuid::now_v7();

            match create_flight_fast(ctx, &fix, flight_id, !is_takeoff).await {
                Ok(flight_id) => {
                    fix.flight_id = Some(flight_id);

                    // Update state
                    if let Some(mut state) = ctx.aircraft_states.get_mut(&fix.aircraft_id) {
                        state.current_flight_id = Some(flight_id);
                    }

                    if is_takeoff {
                        metrics::counter!("flight_tracker.flight_created.takeoff_total")
                            .increment(1);

                        // Spawn towing detection for towtugs
                        if is_towtug {
                            towing::spawn_towing_detection_task(
                                fix.aircraft_id,
                                flight_id,
                                ctx.fixes_repo.clone(),
                                ctx.flights_repo.clone(),
                                ctx.aircraft_repo.clone(),
                                Arc::clone(ctx.aircraft_states),
                            );
                        }
                    } else {
                        metrics::counter!("flight_tracker.flight_created.mid_flight_total")
                            .increment(1);
                    }
                }
                Err(e) => {
                    error!("Failed to create flight: {}", e);
                    fix.flight_id = None;
                }
            }
        }

        // Case 3: Has active flight BUT fix is inactive -> Check if landing
        (Some(flight_id), false) => {
            // Calculate AGL to determine if actually landing or just slow at altitude
            let agl = calculate_altitude_agl(ctx.elevation_db, &fix).await;

            match agl {
                Some(altitude_agl) if altitude_agl >= 250 => {
                    // Still airborne - continue flight
                    fix.flight_id = Some(flight_id);

                    if let Some(state) = ctx.aircraft_states.get(&fix.aircraft_id)
                        && let Some(last_fix_time) = state.last_fix_timestamp()
                    {
                        fix.time_gap_seconds =
                            Some((fix.timestamp - last_fix_time).num_seconds() as i32);
                    }

                    update_flight_timestamp(ctx.flights_repo, flight_id, fix.timestamp).await;
                }
                _ => {
                    // Low altitude or unknown - check for 5 consecutive inactive
                    let should_land = if let Some(state) = ctx.aircraft_states.get(&fix.aircraft_id)
                    {
                        state.has_five_consecutive_inactive()
                    } else {
                        false
                    };

                    if should_land {
                        // Landing confirmed - complete flight (non-blocking)
                        fix.flight_id = Some(flight_id);

                        spawn_complete_flight(ctx, &aircraft, flight_id, &fix);

                        // Clear current_flight_id
                        if let Some(mut state) = ctx.aircraft_states.get_mut(&fix.aircraft_id) {
                            state.current_flight_id = None;
                        }

                        metrics::counter!("flight_tracker.flight_ended.landed_total").increment(1);
                    } else {
                        // Not yet 5 inactive - keep flight active
                        fix.flight_id = Some(flight_id);
                        update_flight_timestamp(ctx.flights_repo, flight_id, fix.timestamp).await;
                    }
                }
            }
        }

        // Case 4: No active flight AND fix is inactive -> Aircraft on ground, do nothing
        (None, false) => {
            trace!(
                "Aircraft {} on ground (inactive fix, no flight)",
                fix.aircraft_id
            );
            // Just save the fix without a flight_id
        }
    }

    Ok(fix)
}
