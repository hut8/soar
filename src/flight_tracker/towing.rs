use crate::aircraft_repo::AircraftCache;
use crate::aircraft_types::AircraftCategory;
use crate::flights_repo::FlightsRepository;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, sleep};
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::AircraftStatesMap;
use super::geometry::haversine_distance;

const VICINITY_RADIUS_METERS: f64 = 500.0; // 0.5 km
const INITIAL_SEARCH_DELAY_SECS: u64 = 10;
const RETRY_SEARCH_DELAY_SECS: u64 = 10;

/// Information about a towing operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TowingInfo {
    pub glider_device_id: Uuid,
    pub glider_flight_id: Uuid,
    pub tow_started: chrono::DateTime<chrono::Utc>,
}

/// Categories that obviously cannot be towed — everything else is a valid candidate.
/// This exclusion-based approach is critical because many gliders have incorrect
/// categories (landplane, unknown, or NULL) in the OGN/FAA data.
const EXCLUDED_TOW_CATEGORIES: &[AircraftCategory] = &[
    AircraftCategory::TowTug,
    AircraftCategory::Helicopter,
    AircraftCategory::Balloon,
    AircraftCategory::Drone,
    AircraftCategory::Airship,
    AircraftCategory::SkydiverParachute,
    AircraftCategory::StaticObstacle,
];

/// Spawn a task to detect towing after a towplane takes off
pub fn spawn_towing_detection_task(
    towplane_aircraft_id: Uuid,
    towplane_flight_id: Uuid,
    flights_repo: FlightsRepository,
    aircraft_cache: AircraftCache,
    aircraft_states: AircraftStatesMap,
) {
    tokio::spawn(async move {
        sleep(Duration::from_secs(INITIAL_SEARCH_DELAY_SECS)).await;

        match find_towed_glider(
            towplane_aircraft_id,
            towplane_flight_id,
            &flights_repo,
            &aircraft_cache,
            &aircraft_states,
        )
        .await
        {
            Ok(Some(towing_info)) => {
                info!(
                    towplane = %towplane_aircraft_id,
                    glider = %towing_info.glider_device_id,
                    glider_flight = %towing_info.glider_flight_id,
                    "Towing detected"
                );
                metrics::counter!("flight_tracker.towing.detected_total").increment(1);

                // Set towing_info on the towplane's state
                if let Some(mut state) = aircraft_states.get_mut(&towplane_aircraft_id) {
                    state.towing_info = Some(towing_info);
                }
            }
            Ok(None) => {
                debug!(
                    towplane = %towplane_aircraft_id,
                    "No tow detected for towplane"
                );
            }
            Err(e) => {
                warn!(
                    towplane = %towplane_aircraft_id,
                    error = %e,
                    "Towing detection failed"
                );
            }
        }
    });
}

/// Find a glider being towed by the given towplane.
/// Returns None if no glider found or multiple gliders found after retries.
async fn find_towed_glider(
    towplane_aircraft_id: Uuid,
    towplane_flight_id: Uuid,
    flights_repo: &FlightsRepository,
    aircraft_cache: &AircraftCache,
    aircraft_states: &AircraftStatesMap,
) -> Result<Option<TowingInfo>> {
    // Get the towplane's current position from in-memory state
    let (towplane_lat, towplane_lng, towplane_time) =
        match get_towplane_position(towplane_aircraft_id, aircraft_states) {
            Some(pos) => pos,
            None => {
                debug!(
                    towplane = %towplane_aircraft_id,
                    "No recent fix in state for towplane - cannot detect towing"
                );
                return Ok(None);
            }
        };

    // Retry up to 3 times if multiple candidates detected
    for attempt in 0..3 {
        if attempt > 0 {
            debug!(
                towplane = %towplane_aircraft_id,
                attempt = attempt + 1,
                "Multiple candidates detected - retrying after delay"
            );
            sleep(Duration::from_secs(RETRY_SEARCH_DELAY_SECS)).await;
        }

        let candidates = find_nearby_candidates(
            towplane_aircraft_id,
            towplane_lat,
            towplane_lng,
            aircraft_cache,
            aircraft_states,
        )
        .await;

        match candidates.len() {
            0 => {
                debug!(
                    towplane = %towplane_aircraft_id,
                    "No candidates found near towplane"
                );
                metrics::counter!("flight_tracker.towing.no_candidates_total").increment(1);
                return Ok(None);
            }
            1 => {
                let (glider_aircraft_id, glider_flight_id, distance) = candidates[0];

                debug!(
                    towplane = %towplane_aircraft_id,
                    glider = %glider_aircraft_id,
                    distance_m = distance,
                    "Single candidate found - linking tow"
                );

                // Update the database to link glider to towplane
                if let Err(e) = flights_repo
                    .update_towing_info(
                        glider_flight_id,
                        towplane_aircraft_id,
                        towplane_flight_id,
                        towplane_time,
                    )
                    .await
                {
                    warn!(
                        glider = %glider_aircraft_id,
                        error = %e,
                        "Failed to update towing info in database"
                    );
                }

                return Ok(Some(TowingInfo {
                    glider_device_id: glider_aircraft_id,
                    glider_flight_id,
                    tow_started: towplane_time,
                }));
            }
            n => {
                warn!(
                    towplane = %towplane_aircraft_id,
                    count = n,
                    "Multiple candidates found near towplane"
                );
            }
        }
    }

    // After 3 attempts, still ambiguous
    warn!(
        towplane = %towplane_aircraft_id,
        "Could not disambiguate tow candidates after 3 attempts"
    );
    metrics::counter!("flight_tracker.towing.ambiguous_total").increment(1);
    Ok(None)
}

/// Get the towplane's latest position from in-memory state.
fn get_towplane_position(
    towplane_aircraft_id: Uuid,
    aircraft_states: &AircraftStatesMap,
) -> Option<(f64, f64, chrono::DateTime<chrono::Utc>)> {
    let state = aircraft_states.get(&towplane_aircraft_id)?;
    let last_fix = state.recent_fixes.back()?;
    let age = chrono::Utc::now() - last_fix.received_at;
    if age.num_seconds() > 30 {
        return None;
    }
    Some((last_fix.lat, last_fix.lng, last_fix.received_at))
}

/// Find nearby aircraft that could be towed by the towplane.
///
/// Phase 1 (sync): scan in-memory aircraft states for nearby active aircraft.
/// Phase 2 (async): look up aircraft category from cache, exclude non-towable types.
/// Note: `get_by_id` can fall back to DB on cache miss, but in practice all aircraft
/// in `aircraft_states` were cached on their first fix, so this is almost always a hit.
async fn find_nearby_candidates(
    towplane_aircraft_id: Uuid,
    towplane_lat: f64,
    towplane_lng: f64,
    aircraft_cache: &AircraftCache,
    aircraft_states: &AircraftStatesMap,
) -> Vec<(Uuid, Uuid, f64)> {
    let now = chrono::Utc::now();

    // Phase 1: synchronous scan of in-memory state
    let nearby: Vec<(Uuid, Uuid, f64)> = aircraft_states
        .iter()
        .filter_map(|entry| {
            let aircraft_id = *entry.key();
            let state = entry.value();

            // Skip the towplane itself
            if aircraft_id == towplane_aircraft_id {
                return None;
            }

            // Must have an active flight
            let flight_id = state.current_flight_id?;

            // Must have a recent fix
            let last_fix = state.recent_fixes.back()?;
            let age = now - last_fix.received_at;
            if age.num_seconds() > 30 {
                return None;
            }

            // Must be within vicinity
            let distance =
                haversine_distance(towplane_lat, towplane_lng, last_fix.lat, last_fix.lng);
            if distance > VICINITY_RADIUS_METERS {
                return None;
            }

            Some((aircraft_id, flight_id, distance))
        })
        .collect();

    // Phase 2: filter by aircraft category (async cache lookups)
    let mut candidates = Vec::new();
    for (aircraft_id, flight_id, distance) in nearby {
        let excluded = match aircraft_cache.get_by_id(aircraft_id).await {
            Ok(Some(aircraft)) => {
                if let Some(category) = aircraft.aircraft_category {
                    EXCLUDED_TOW_CATEGORIES.contains(&category)
                } else {
                    // No category — could be a glider, include it
                    false
                }
            }
            Ok(None) => {
                // Unknown aircraft — include as candidate
                false
            }
            Err(e) => {
                warn!(
                    aircraft = %aircraft_id,
                    error = %e,
                    "Failed to look up aircraft for tow candidate filtering"
                );
                // On error, include rather than exclude
                false
            }
        };

        if !excluded {
            debug!(
                aircraft = %aircraft_id,
                distance_m = distance,
                "Tow candidate found"
            );
            candidates.push((aircraft_id, flight_id, distance));
        }
    }

    candidates
}

/// Check if a towplane has released its tow based on climb rate transition
/// Returns true if release detected
pub fn check_tow_release(state: &super::AircraftState, current_climb_fpm: Option<i32>) -> bool {
    // Only check if we have towing info
    let _towing_info = match &state.towing_info {
        Some(info) => info,
        None => return false,
    };

    // Calculate average climb rate from recent fixes (last 5 with altitude data)
    let recent_altitudes: Vec<(chrono::DateTime<chrono::Utc>, i32)> = state
        .recent_fixes
        .iter()
        .rev()
        .filter_map(|f| Some((f.received_at, f.altitude_msl_ft?)))
        .take(5)
        .collect();

    if recent_altitudes.len() < 2 {
        return false;
    }

    // Calculate average climb rate from recent fixes
    let mut climb_rates = Vec::new();
    for i in 1..recent_altitudes.len() {
        let (t1, alt1) = recent_altitudes[i - 1];
        let (t2, alt2) = recent_altitudes[i];
        let time_diff_secs = (t2 - t1).num_seconds();
        if time_diff_secs > 0 {
            let climb_rate_fpm = ((alt2 - alt1) as f64 / time_diff_secs as f64) * 60.0;
            climb_rates.push(climb_rate_fpm as f32);
        }
    }

    if climb_rates.is_empty() {
        return false;
    }

    let avg_climb_rate: f32 = climb_rates.iter().sum::<f32>() / climb_rates.len() as f32;

    // Was climbing (avg > 100 fpm) and now descending (current < -100 fpm)
    // This indicates the towplane has released the glider and is descending back
    let was_climbing = avg_climb_rate > 100.0;
    let now_descending = current_climb_fpm.map(|rate| rate < -100).unwrap_or(false);

    if was_climbing && now_descending {
        info!(
            "Tow release detected: towplane {} was climbing (avg: {:.0} fpm) now descending ({:.0} fpm)",
            state
                .current_flight_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            avg_climb_rate,
            current_climb_fpm.unwrap_or(0)
        );
        return true;
    }

    false
}
