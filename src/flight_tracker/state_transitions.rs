use crate::Fix;
use crate::aircraft::Aircraft;
use crate::aircraft_types::AircraftCategory;
use anyhow::Result;
use std::sync::Arc;
use tracing::{error, info, trace, warn};
use uuid::Uuid;

use super::altitude::calculate_altitude_agl;
use super::flight_lifecycle::create_flight_fast;
use super::geometry::haversine_distance;
use super::towing;
use super::{AircraftState, FlightProcessorContext};

/// Pending background work to be executed after fix insertion
#[derive(Debug, Clone)]
pub enum PendingBackgroundWork {
    /// No background work needed
    None,
    /// Complete a flight (landing, callsign change, or gap detected)
    CompleteFlight {
        flight_id: Uuid,
        aircraft: Box<Aircraft>,
        fix: Box<Fix>,
    },
}

/// Result of processing a state transition
pub struct StateTransitionResult {
    /// The updated fix with flight_id assigned
    pub fix: Fix,
    /// Background work to spawn after the fix is inserted
    pub pending_work: PendingBackgroundWork,
}

/// Normalize a callsign: trim whitespace and treat empty strings as None.
fn normalize_callsign(callsign: &Option<String>) -> Option<&str> {
    callsign
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
}

/// Determine if a callsign change warrants ending the current flight.
///
/// Returns `true` if the fix has a callsign that differs from the current in-memory callsign.
/// Returns `false` if either callsign is absent (None, empty, or whitespace-only).
fn is_callsign_change(current_callsign: &Option<String>, fix_callsign: &Option<String>) -> bool {
    match (
        normalize_callsign(current_callsign),
        normalize_callsign(fix_callsign),
    ) {
        (Some(current), Some(new)) => current != new,
        _ => false,
    }
}

/// Update in-memory callsign when first learned on an existing flight.
///
/// When a flight is created without a callsign (e.g. APRS source), the first fix
/// that carries a non-empty callsign should record it so subsequent changes are detected.
/// Returns the new callsign value for the state.
fn learn_callsign(
    current_callsign: &Option<String>,
    fix_callsign: &Option<String>,
) -> Option<String> {
    if current_callsign.is_none() {
        normalize_callsign(fix_callsign).map(|s| s.to_string())
    } else {
        current_callsign.clone()
    }
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
/// Returns StateTransitionResult containing the fix and any pending background work
/// that should be spawned AFTER the fix is inserted into the database.
pub(crate) async fn process_state_transition(
    ctx: &FlightProcessorContext<'_>,
    mut fix: Fix,
) -> Result<StateTransitionResult> {
    // ADS-B: trust transponder's on_ground field (already set as is_active = !on_ground)
    // APRS: use speed/AGL heuristic
    let is_active = if fix.has_transponder_data() {
        fix.is_active
    } else {
        should_be_active(&fix)
    };
    let mut pending_work = PendingBackgroundWork::None;

    // Fetch aircraft (from in-memory cache, falling back to DB on miss)
    let aircraft_lookup_start = std::time::Instant::now();
    let aircraft = ctx
        .aircraft_cache
        .get_by_id(fix.aircraft_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Aircraft {} not found", fix.aircraft_id))?;
    metrics::histogram!("aprs.aircraft.aircraft_lookup_ms")
        .record(aircraft_lookup_start.elapsed().as_micros() as f64 / 1000.0);

    let is_towtug = aircraft.aircraft_category == Some(AircraftCategory::TowTug);

    // Get or create aircraft state, add this fix to history
    let current_flight_id = {
        let mut state = ctx
            .aircraft_states
            .entry(fix.aircraft_id)
            .or_insert_with(|| AircraftState::new(&fix, is_active));

        // Add fix to history (if not the initial fix that created the state)
        if state.recent_fixes.len() > 1
            || (state.recent_fixes.len() == 1
                && state.recent_fixes.back().unwrap().received_at != fix.received_at)
        {
            state.add_fix(&fix, is_active);
        }

        state.current_flight_id
    };

    match (current_flight_id, is_active) {
        // Case 1: Has active flight AND fix is active -> Continue flight
        (Some(flight_id), true) => {
            // Check for callsign change (using in-memory state, not DB)
            let should_end_flight = ctx
                .aircraft_states
                .get(&fix.aircraft_id)
                .map(|state| is_callsign_change(&state.current_callsign, &fix.flight_number))
                .unwrap_or(false);

            if should_end_flight {
                // Set preliminary landing_time before creating new flight to prevent
                // two active flights for the same aircraft (unique index violation)
                match ctx
                    .flights_repo
                    .set_preliminary_landing_time(flight_id, fix.received_at)
                    .await
                {
                    Ok(true) => {
                        // Old flight is now inactive - safe to create new one
                        pending_work = PendingBackgroundWork::CompleteFlight {
                            flight_id,
                            aircraft: Box::new(aircraft.clone()),
                            fix: Box::new(fix.clone()),
                        };

                        let new_flight_id = Uuid::now_v7();
                        match create_flight_fast(ctx, &fix, &aircraft, new_flight_id, false).await {
                            Ok(flight_id) => {
                                fix.flight_id = Some(flight_id);
                                if let Some(mut state) =
                                    ctx.aircraft_states.get_mut(&fix.aircraft_id)
                                {
                                    state.current_flight_id = Some(flight_id);
                                    state.current_callsign = fix.flight_number.clone();
                                }
                            }
                            Err(e) => {
                                error!(error = %e, "Failed to create new flight after callsign change");
                                // Old flight already ended in DB — clear stale state
                                if let Some(mut state) =
                                    ctx.aircraft_states.get_mut(&fix.aircraft_id)
                                {
                                    state.current_flight_id = None;
                                    state.current_callsign = None;
                                }
                                fix.flight_id = None;
                            }
                        }
                    }
                    Ok(false) => {
                        // No rows updated — flight already timed out or landed in DB.
                        // Clear stale in-memory state so next fix creates a new flight.
                        warn!(
                            "Flight {} already ended in DB (timed out or landed), clearing stale in-memory state",
                            flight_id
                        );
                        if let Some(mut state) = ctx.aircraft_states.get_mut(&fix.aircraft_id) {
                            state.current_flight_id = None;
                        }
                        fix.flight_id = None;
                    }
                    Err(e) => {
                        // DB error - keep old flight active in memory to maintain sync
                        // Next fix will re-evaluate the callsign change
                        warn!(
                            "Could not set landing time for flight {} before callsign change, will retry: {}",
                            flight_id, e
                        );
                        fix.flight_id = Some(flight_id);
                    }
                }
            } else {
                // Check for long time/distance gap that indicates aircraft landed
                // NOTE: Current fix has already been added to state, so we check the PREVIOUS fix
                let should_end_due_to_gap = if let Some(state) =
                    ctx.aircraft_states.get(&fix.aircraft_id)
                {
                    if let Some(prev_fix_time) = state.previous_fix_timestamp() {
                        let gap_seconds = (fix.received_at - prev_fix_time).num_seconds();

                        // Only check if gap is significant (>30 minutes)
                        if gap_seconds > 1800 {
                            if let Some(prev_fix) = state.previous_fix() {
                                let actual_distance_m = haversine_distance(
                                    prev_fix.lat,
                                    prev_fix.lng,
                                    fix.latitude,
                                    fix.longitude,
                                );
                                let last_speed_knots =
                                    prev_fix.ground_speed_knots.unwrap_or(0.0) as f64;

                                // If aircraft was flying (>25 knots)
                                if last_speed_knots > 25.0 {
                                    let last_speed_ms = last_speed_knots * 0.514444;
                                    let expected_distance_m = gap_seconds as f64 * last_speed_ms;
                                    let min_expected_distance_m = expected_distance_m * 0.3;

                                    // Aircraft moved way too little for the time/speed - must have landed
                                    if actual_distance_m < min_expected_distance_m {
                                        info!(
                                            "Ending flight {}: moved too little ({:.1}km in {:.1}h at {:.0} knots, expected min {:.1}km)",
                                            flight_id,
                                            actual_distance_m / 1000.0,
                                            gap_seconds as f64 / 3600.0,
                                            last_speed_knots,
                                            min_expected_distance_m / 1000.0
                                        );
                                        metrics::counter!(
                                            "flight_tracker.flight_ended.gap_suggests_landing_total"
                                        )
                                        .increment(1);
                                        true
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                if should_end_due_to_gap {
                    // Set preliminary landing_time before creating new flight to prevent
                    // two active flights for the same aircraft (unique index violation)
                    match ctx
                        .flights_repo
                        .set_preliminary_landing_time(flight_id, fix.received_at)
                        .await
                    {
                        Ok(true) => {
                            // Old flight is now inactive - safe to create new one
                            pending_work = PendingBackgroundWork::CompleteFlight {
                                flight_id,
                                aircraft: Box::new(aircraft.clone()),
                                fix: Box::new(fix.clone()),
                            };

                            let new_flight_id = Uuid::now_v7();
                            match create_flight_fast(ctx, &fix, &aircraft, new_flight_id, false)
                                .await
                            {
                                Ok(flight_id) => {
                                    fix.flight_id = Some(flight_id);
                                    if let Some(mut state) =
                                        ctx.aircraft_states.get_mut(&fix.aircraft_id)
                                    {
                                        state.current_flight_id = Some(flight_id);
                                        state.current_callsign = fix.flight_number.clone();
                                    }
                                }
                                Err(e) => {
                                    error!(error = %e, "Failed to create new flight after gap");
                                    // Old flight already ended in DB — clear stale state
                                    if let Some(mut state) =
                                        ctx.aircraft_states.get_mut(&fix.aircraft_id)
                                    {
                                        state.current_flight_id = None;
                                        state.current_callsign = None;
                                    }
                                    fix.flight_id = None;
                                }
                            }
                        }
                        Ok(false) => {
                            // No rows updated — flight already timed out or landed in DB.
                            // Clear stale in-memory state so next fix creates a new flight.
                            warn!(
                                "Flight {} already ended in DB (timed out or landed), clearing stale in-memory state",
                                flight_id
                            );
                            if let Some(mut state) = ctx.aircraft_states.get_mut(&fix.aircraft_id) {
                                state.current_flight_id = None;
                            }
                            fix.flight_id = None;
                        }
                        Err(e) => {
                            // DB error - keep old flight active in memory to maintain sync
                            // Next fix will re-evaluate the gap
                            warn!(
                                "Could not set landing time for flight {} before gap-based split, will retry: {}",
                                flight_id, e
                            );
                            fix.flight_id = Some(flight_id);
                        }
                    }
                } else {
                    // Continue existing flight
                    fix.flight_id = Some(flight_id);

                    // Update in-memory callsign when first learned on an existing flight.
                    // Without this, current_callsign stays None and subsequent fixes with
                    // a different callsign won't trigger a flight split.
                    if let Some(mut state) = ctx.aircraft_states.get_mut(&fix.aircraft_id) {
                        state.current_callsign =
                            learn_callsign(&state.current_callsign, &fix.flight_number);
                    }

                    // Calculate time gap
                    if let Some(state) = ctx.aircraft_states.get(&fix.aircraft_id)
                        && let Some(last_fix_time) = state.last_fix_timestamp()
                    {
                        let gap_seconds = (fix.received_at - last_fix_time).num_seconds() as i32;
                        fix.time_gap_seconds = Some(gap_seconds);
                    }
                }

                // Check for tow release (towtugs only)
                // IMPORTANT: Extract data and release DashMap lock BEFORE any async operations
                // to avoid holding synchronous locks across await points (causes deadlocks)
                let tow_release_info = if is_towtug {
                    if let Some(state) = ctx.aircraft_states.get(&fix.aircraft_id) {
                        if towing::check_tow_release(&state, fix.climb_fpm) {
                            state.towing_info.as_ref().map(|ti| ti.glider_flight_id)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                // DashMap lock is now released - safe to do async work

                if let Some(glider_flight_id) = tow_release_info {
                    // Record tow release
                    if let Some(altitude_ft) = fix.altitude_msl_feet {
                        let _ = ctx
                            .flights_repo
                            .update_tow_release(glider_flight_id, altitude_ft, fix.received_at)
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
            // Before creating a new flight, check if there's an orphaned active flight in DB.
            // This handles cases where in-memory state got out of sync with the database
            // (e.g., set_preliminary_landing_time failed but current_flight_id was cleared,
            // or state was lost after a restart).
            match ctx
                .flights_repo
                .get_active_flight_for_aircraft(fix.aircraft_id)
                .await
            {
                Ok(Some((existing_flight_id, existing_callsign))) => {
                    // Check callsign compatibility before adopting.
                    // If the orphaned flight has a different callsign than the current fix,
                    // it's a different operation (e.g., SAS1465 vs SAS1470) and we should
                    // end the orphaned flight and create a new one instead of adopting it.
                    let callsign_compatible = match (&existing_callsign, &fix.flight_number) {
                        (Some(existing), Some(new)) => existing == new,
                        _ => true, // If either is None, allow adoption
                    };

                    if callsign_compatible {
                        info!(
                            "Found orphaned active flight {} for aircraft {}, adopting it",
                            existing_flight_id, fix.aircraft_id
                        );
                        fix.flight_id = Some(existing_flight_id);
                        if let Some(mut state) = ctx.aircraft_states.get_mut(&fix.aircraft_id) {
                            state.current_flight_id = Some(existing_flight_id);
                            // Prefer the DB callsign over the fix's callsign to avoid
                            // clearing in-memory state when a fix has no callsign
                            state.current_callsign =
                                existing_callsign.or(fix.flight_number.clone());
                        }
                        metrics::counter!("flight_tracker.orphaned_flight_adopted_total")
                            .increment(1);
                        return Ok(StateTransitionResult { fix, pending_work });
                    } else {
                        // Callsign mismatch - end the orphaned flight and create a new one
                        info!(
                            "Orphaned active flight {} has callsign '{:?}' but fix has '{:?}', ending it instead of adopting",
                            existing_flight_id, existing_callsign, fix.flight_number
                        );
                        match ctx
                            .flights_repo
                            .set_preliminary_landing_time(existing_flight_id, fix.received_at)
                            .await
                        {
                            Ok(true) => {
                                pending_work = PendingBackgroundWork::CompleteFlight {
                                    flight_id: existing_flight_id,
                                    aircraft: Box::new(aircraft.clone()),
                                    fix: Box::new(fix.clone()),
                                };
                            }
                            Ok(false) => {
                                // Orphaned flight was already timed out or landed concurrently.
                                // Safe to fall through to create a new flight.
                                warn!(
                                    "Orphaned flight {} already ended in DB, proceeding to create new flight",
                                    existing_flight_id
                                );
                            }
                            Err(e) => {
                                // DB error - orphaned flight stays active, so adopt it
                                // to avoid unique index violation when creating a new flight
                                warn!(
                                    "Could not set landing time for orphaned flight {} before callsign-mismatch split, adopting instead: {}",
                                    existing_flight_id, e
                                );
                                fix.flight_id = Some(existing_flight_id);
                                if let Some(mut state) =
                                    ctx.aircraft_states.get_mut(&fix.aircraft_id)
                                {
                                    state.current_flight_id = Some(existing_flight_id);
                                    state.current_callsign =
                                        existing_callsign.or(fix.flight_number.clone());
                                }
                                return Ok(StateTransitionResult { fix, pending_work });
                            }
                        }
                        metrics::counter!("flight_tracker.orphaned_flight_callsign_mismatch_total")
                            .increment(1);
                        // Fall through to create a new flight
                    }
                }
                Ok(None) => {
                    // No existing active flight - proceed to create a new one
                }
                Err(e) => {
                    // DB query failed - proceed to create and let it fail naturally if needed
                    warn!("Failed to check for existing active flight: {}", e);
                }
            }

            // Create new flight - check if takeoff or mid-flight
            // IMPORTANT: Extract data and release DashMap lock BEFORE any async operations
            // to avoid holding synchronous locks across await points (causes deadlocks)
            let is_takeoff = if fix.has_transponder_data() {
                // ADS-B: any prior inactive fix means on_ground→airborne transition = takeoff
                if let Some(state) = ctx.aircraft_states.get(&fix.aircraft_id) {
                    state.last_n_inactive(1)
                } else {
                    false // No prior state — mid-flight appearance
                }
            } else {
                // APRS: check in-memory state first, release DashMap ref, then do async work
                let history_result = ctx.aircraft_states.get(&fix.aircraft_id).map(|state| {
                    if state.recent_fixes.len() >= 4 {
                        // Enough history - check if last 3 fixes before current were inactive
                        Some(state.last_n_inactive(3))
                    } else {
                        // Not enough history - need AGL check (async)
                        None
                    }
                });
                // DashMap ref is now dropped - safe to do async work

                match history_result {
                    Some(Some(is_inactive)) => is_inactive,
                    Some(None) | None => {
                        // Not enough history or no state - fall back to AGL check
                        // If AGL < 100 ft, aircraft is on/near ground = takeoff
                        calculate_altitude_agl(ctx.elevation_db, &fix)
                            .await
                            .map(|agl| agl < 100)
                            .unwrap_or(false)
                    }
                }
            };

            let flight_id = Uuid::now_v7();

            match create_flight_fast(ctx, &fix, &aircraft, flight_id, !is_takeoff).await {
                Ok(flight_id) => {
                    fix.flight_id = Some(flight_id);

                    // Update state
                    if let Some(mut state) = ctx.aircraft_states.get_mut(&fix.aircraft_id) {
                        state.current_flight_id = Some(flight_id);
                        state.current_callsign = fix.flight_number.clone();
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
                        metrics::counter!("flight_tracker.flight_created.airborne_total")
                            .increment(1);
                    }
                }
                Err(e) => {
                    error!(error = %e, "Failed to create flight");
                    fix.flight_id = None;
                }
            }
        }

        // Case 3: Has active flight BUT fix is inactive -> Check if landing
        (Some(flight_id), false) => {
            if fix.has_transponder_data() {
                // ADS-B: transponder on_ground is authoritative — land immediately
                // Set preliminary landing_time to prevent race where a new fix triggers
                // flight creation before CompleteFlight background task runs
                match ctx
                    .flights_repo
                    .set_preliminary_landing_time(flight_id, fix.received_at)
                    .await
                {
                    Ok(true) => {
                        // Landing time set - safe to clear state
                        fix.flight_id = Some(flight_id);
                        pending_work = PendingBackgroundWork::CompleteFlight {
                            flight_id,
                            aircraft: Box::new(aircraft.clone()),
                            fix: Box::new(fix.clone()),
                        };
                        if let Some(mut state) = ctx.aircraft_states.get_mut(&fix.aircraft_id) {
                            state.current_flight_id = None;
                        }
                        metrics::counter!("flight_tracker.flight_ended.landed_total").increment(1);
                    }
                    Ok(false) => {
                        // Flight already timed out or landed in DB — clear stale in-memory state
                        warn!(
                            "Flight {} already ended in DB (timed out or landed), clearing stale in-memory state",
                            flight_id
                        );
                        if let Some(mut state) = ctx.aircraft_states.get_mut(&fix.aircraft_id) {
                            state.current_flight_id = None;
                            state.current_callsign = None;
                        }
                        fix.flight_id = None;
                    }
                    Err(e) => {
                        // DB error - keep flight active in memory to maintain sync
                        // Next inactive fix will retry the landing
                        warn!(
                            "Failed to set landing time for flight {} (will retry): {}",
                            flight_id, e
                        );
                        fix.flight_id = Some(flight_id);
                    }
                }
            } else {
                // APRS: use AGL check + 5-fix debounce (heuristic-based is_active can flicker)
                let agl = calculate_altitude_agl(ctx.elevation_db, &fix).await;

                match agl {
                    Some(altitude_agl) if altitude_agl >= 250 => {
                        // Still airborne - continue flight
                        fix.flight_id = Some(flight_id);

                        if let Some(state) = ctx.aircraft_states.get(&fix.aircraft_id)
                            && let Some(last_fix_time) = state.last_fix_timestamp()
                        {
                            fix.time_gap_seconds =
                                Some((fix.received_at - last_fix_time).num_seconds() as i32);
                        }
                    }
                    _ => {
                        // Low altitude or unknown - check for 5 consecutive inactive
                        let should_land =
                            if let Some(state) = ctx.aircraft_states.get(&fix.aircraft_id) {
                                state.has_five_consecutive_inactive()
                            } else {
                                false
                            };

                        if should_land {
                            // Set preliminary landing_time to prevent race where a new fix
                            // triggers flight creation before CompleteFlight background task runs
                            match ctx
                                .flights_repo
                                .set_preliminary_landing_time(flight_id, fix.received_at)
                                .await
                            {
                                Ok(true) => {
                                    // Landing confirmed
                                    fix.flight_id = Some(flight_id);
                                    pending_work = PendingBackgroundWork::CompleteFlight {
                                        flight_id,
                                        aircraft: Box::new(aircraft.clone()),
                                        fix: Box::new(fix.clone()),
                                    };
                                    if let Some(mut state) =
                                        ctx.aircraft_states.get_mut(&fix.aircraft_id)
                                    {
                                        state.current_flight_id = None;
                                    }
                                    metrics::counter!("flight_tracker.flight_ended.landed_total")
                                        .increment(1);
                                }
                                Ok(false) => {
                                    // Flight already timed out or landed in DB — clear stale state
                                    warn!(
                                        "Flight {} already ended in DB (timed out or landed), clearing stale in-memory state",
                                        flight_id
                                    );
                                    if let Some(mut state) =
                                        ctx.aircraft_states.get_mut(&fix.aircraft_id)
                                    {
                                        state.current_flight_id = None;
                                        state.current_callsign = None;
                                    }
                                    fix.flight_id = None;
                                }
                                Err(e) => {
                                    // DB error - keep flight active in memory to maintain sync
                                    // Next inactive fix will retry the landing
                                    warn!(
                                        "Failed to set landing time for flight {} (will retry): {}",
                                        flight_id, e
                                    );
                                    fix.flight_id = Some(flight_id);
                                }
                            }
                        } else {
                            // Not yet 5 inactive - keep flight active
                            fix.flight_id = Some(flight_id);
                        }
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

    Ok(StateTransitionResult { fix, pending_work })
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_callsign_change tests ---

    #[test]
    fn callsign_change_both_present_and_different() {
        assert!(is_callsign_change(
            &Some("WMT437".to_string()),
            &Some("WMT2574".to_string())
        ));
    }

    #[test]
    fn callsign_change_both_present_and_same() {
        assert!(!is_callsign_change(
            &Some("WMT437".to_string()),
            &Some("WMT437".to_string())
        ));
    }

    #[test]
    fn callsign_change_current_none_fix_has_value() {
        assert!(!is_callsign_change(&None, &Some("WMT437".to_string())));
    }

    #[test]
    fn callsign_change_current_has_value_fix_none() {
        assert!(!is_callsign_change(&Some("WMT437".to_string()), &None));
    }

    #[test]
    fn callsign_change_both_none() {
        assert!(!is_callsign_change(&None, &None));
    }

    // --- learn_callsign tests ---

    #[test]
    fn learn_callsign_from_none_to_value() {
        let result = learn_callsign(&None, &Some("WMT437".to_string()));
        assert_eq!(result, Some("WMT437".to_string()));
    }

    #[test]
    fn learn_callsign_does_not_overwrite_existing() {
        let result = learn_callsign(&Some("WMT437".to_string()), &Some("WMT2574".to_string()));
        assert_eq!(result, Some("WMT437".to_string()));
    }

    #[test]
    fn learn_callsign_both_none_stays_none() {
        let result = learn_callsign(&None, &None);
        assert_eq!(result, None);
    }

    #[test]
    fn learn_callsign_existing_with_no_fix_callsign_unchanged() {
        let result = learn_callsign(&Some("WMT437".to_string()), &None);
        assert_eq!(result, Some("WMT437".to_string()));
    }

    // --- Combined scenario: the bug that was fixed ---

    /// Reproduces the exact scenario from the bug report:
    /// 1. Flight starts with no callsign (current_callsign = None)
    /// 2. First fix arrives with callsign "WMT437" → should be learned
    /// 3. Second fix arrives with callsign "WMT2574" → should detect a change
    ///
    /// Before the fix, step 2 would NOT update current_callsign, so step 3
    /// would also see None and not detect the change.
    #[test]
    fn callsign_mismatch_bug_scenario() {
        let mut current_callsign: Option<String> = None;

        // Fix 1: aircraft sends callsign "WMT437" for the first time
        let fix1_callsign = Some("WMT437".to_string());
        assert!(
            !is_callsign_change(&current_callsign, &fix1_callsign),
            "First callsign on a flight should not trigger a change"
        );
        current_callsign = learn_callsign(&current_callsign, &fix1_callsign);
        assert_eq!(
            current_callsign,
            Some("WMT437".to_string()),
            "Callsign should be learned after first fix"
        );

        // Fix 2: same callsign - no change
        let fix2_callsign = Some("WMT437".to_string());
        assert!(
            !is_callsign_change(&current_callsign, &fix2_callsign),
            "Same callsign should not trigger a change"
        );
        current_callsign = learn_callsign(&current_callsign, &fix2_callsign);
        assert_eq!(current_callsign, Some("WMT437".to_string()));

        // Fix 3: DIFFERENT callsign - must detect the change
        let fix3_callsign = Some("WMT2574".to_string());
        assert!(
            is_callsign_change(&current_callsign, &fix3_callsign),
            "Different callsign must trigger a flight split"
        );
    }

    // --- Empty/whitespace callsign edge cases ---

    #[test]
    fn empty_callsign_treated_as_none_for_change_detection() {
        // Empty string on fix should not trigger a change from a real callsign
        assert!(!is_callsign_change(
            &Some("WMT437".to_string()),
            &Some("".to_string())
        ));
    }

    #[test]
    fn empty_callsign_not_learned() {
        let result = learn_callsign(&None, &Some("".to_string()));
        assert_eq!(result, None, "Empty callsign should not be stored");
    }

    #[test]
    fn whitespace_callsign_not_learned() {
        let result = learn_callsign(&None, &Some("  ".to_string()));
        assert_eq!(
            result, None,
            "Whitespace-only callsign should not be stored"
        );
    }

    #[test]
    fn whitespace_callsign_treated_as_none_for_change_detection() {
        assert!(!is_callsign_change(
            &Some("WMT437".to_string()),
            &Some("  ".to_string())
        ));
    }

    #[test]
    fn callsign_with_whitespace_is_trimmed_when_learned() {
        let result = learn_callsign(&None, &Some(" WMT437 ".to_string()));
        assert_eq!(result, Some("WMT437".to_string()));
    }
}
