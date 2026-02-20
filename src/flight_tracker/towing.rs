use crate::Fix;
use crate::aircraft_repo::AircraftRepository;
use crate::aircraft_types::AircraftCategory;
use crate::fixes_repo::FixesRepository;
use crate::flights_repo::FlightsRepository;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, sleep};
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::AircraftStatesMap;
use super::geometry::haversine_distance;

const VICINITY_RADIUS_METERS: f64 = 500.0; // 0.5 km
#[allow(dead_code)]
const INITIAL_SEARCH_DELAY_SECS: u64 = 10;
#[allow(dead_code)]
const RETRY_SEARCH_DELAY_SECS: u64 = 10;

/// Information about a towing operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TowingInfo {
    pub glider_device_id: Uuid,
    pub glider_flight_id: Uuid,
    pub tow_started: chrono::DateTime<chrono::Utc>,
}

/// Spawn a task to detect towing after a towplane takes off
///
/// DISABLED: This feature has been temporarily disabled due to performance issues.
/// The previous implementation queried the database for every active aircraft to check
/// if they were gliders (2 DB queries per aircraft * N active flights = very expensive).
///
/// TODO: Re-implement with a more efficient approach (e.g., spatial index, caching)
#[allow(unused_variables)]
pub fn spawn_towing_detection_task(
    towplane_aircraft_id: Uuid,
    towplane_flight_id: Uuid,
    fixes_repo: FixesRepository,
    flights_repo: FlightsRepository,
    aircraft_repo: AircraftRepository,
    aircraft_states: AircraftStatesMap,
) {
    // DISABLED - do nothing
    debug!(
        "Towing detection is currently disabled for towplane {} (flight {})",
        towplane_aircraft_id, towplane_flight_id
    );
}

/// Find a glider being towed by the given towplane
/// Returns None if no glider found or multiple gliders found (after retries)
///
/// DISABLED - see spawn_towing_detection_task
#[allow(dead_code)]
async fn find_towed_glider(
    towplane_aircraft_id: Uuid,
    fixes_repo: &FixesRepository,
    flights_repo: &FlightsRepository,
    aircraft_repo: &AircraftRepository,
    aircraft_states: &AircraftStatesMap,
) -> Result<Option<TowingInfo>> {
    // Get the latest fix for the towplane
    let towplane_fix = match fixes_repo
        .get_latest_fix_for_aircraft(
            towplane_aircraft_id,
            chrono::Utc::now() - chrono::Duration::seconds(30),
        )
        .await?
    {
        Some(fix) => fix,
        None => {
            debug!(
                "No recent fix found for towplane {} - cannot detect towing",
                towplane_aircraft_id
            );
            return Ok(None);
        }
    };

    // Retry up to 3 times if multiple gliders detected
    for attempt in 0..3 {
        if attempt > 0 {
            debug!(
                "Multiple gliders detected for towplane {} - waiting {} seconds (attempt {}/3)",
                towplane_aircraft_id, RETRY_SEARCH_DELAY_SECS, attempt
            );
            sleep(Duration::from_secs(RETRY_SEARCH_DELAY_SECS)).await;
        }

        let candidate_gliders = find_nearby_gliders(
            &towplane_fix,
            towplane_aircraft_id,
            aircraft_states,
            fixes_repo,
            aircraft_repo,
        )
        .await?;

        match candidate_gliders.len() {
            0 => {
                debug!(
                    "No gliders found near towplane {} - not towing",
                    towplane_aircraft_id
                );
                return Ok(None);
            }
            1 => {
                let (glider_aircraft_id, glider_flight_id) = candidate_gliders[0];

                // Update the database to link glider to towplane
                if let Err(e) = flights_repo
                    .update_towing_info(
                        glider_flight_id,
                        towplane_aircraft_id,
                        towplane_fix
                            .flight_id
                            .expect("Towplane must have flight_id"),
                        towplane_fix.received_at,
                    )
                    .await
                {
                    warn!(
                        "Failed to update towing info in database for glider {}: {}",
                        glider_aircraft_id, e
                    );
                }

                return Ok(Some(TowingInfo {
                    glider_device_id: glider_aircraft_id,
                    glider_flight_id,
                    tow_started: towplane_fix.received_at,
                }));
            }
            n => {
                warn!(
                    "Multiple gliders ({}) found near towplane {} - waiting to disambiguate",
                    n, towplane_aircraft_id
                );
                // Continue to next attempt
            }
        }
    }

    // After 3 attempts, still multiple gliders - give up
    warn!(
        "Could not disambiguate multiple gliders for towplane {} after 3 attempts",
        towplane_aircraft_id
    );
    Ok(None)
}

/// Find gliders with active flights near the towplane
///
/// DISABLED - see spawn_towing_detection_task
///
/// This implementation was problematic because it:
/// 1. Iterated over ALL active aircraft (could be 100+)
/// 2. Made 2 database queries PER aircraft:
///    - get_latest_fix_for_aircraft (fixes table query)
///    - get_aircraft_by_id (aircraft table query)
/// 3. For 100 active flights, that's 200 DB queries just to detect one tow!
///
/// TODO: Reimplement using:
/// - Spatial index on fixes table
/// - Cached aircraft type in memory (not in DB)
/// - Only query for gliders within bounding box of towplane
#[allow(dead_code)]
async fn find_nearby_gliders(
    towplane_fix: &Fix,
    towplane_aircraft_id: Uuid,
    aircraft_states: &AircraftStatesMap,
    fixes_repo: &FixesRepository,
    aircraft_repo: &AircraftRepository,
) -> Result<Vec<(Uuid, Uuid)>> {
    let mut candidate_gliders = Vec::new();

    // Get all aircraft with active flights
    let active_aircraft: Vec<(Uuid, Uuid)> = aircraft_states
        .iter()
        .filter_map(|entry| {
            let aircraft_id = *entry.key();
            let state = entry.value();

            // Skip aircraft without active flights
            let flight_id = state.current_flight_id?;

            // Skip the towplane itself
            if aircraft_id == towplane_aircraft_id {
                return None;
            }

            Some((aircraft_id, flight_id))
        })
        .collect();

    // Check each aircraft to see if it's a glider near the towplane
    for (aircraft_id, flight_id) in active_aircraft {
        // Get the latest fix for this aircraft
        let aircraft_fix = match fixes_repo
            .get_latest_fix_for_aircraft(
                aircraft_id,
                chrono::Utc::now() - chrono::Duration::seconds(30),
            )
            .await?
        {
            Some(fix) => fix,
            None => continue,
        };

        // Check if this is a glider (aircraft_category is on aircraft, not fix)
        let is_glider = aircraft_repo
            .get_aircraft_by_id(aircraft_id)
            .await
            .ok()
            .flatten()
            .and_then(|a| a.aircraft_category)
            == Some(AircraftCategory::Glider);

        if !is_glider {
            continue;
        }

        // Calculate distance
        let distance = haversine_distance(
            towplane_fix.latitude,
            towplane_fix.longitude,
            aircraft_fix.latitude,
            aircraft_fix.longitude,
        );

        if distance <= VICINITY_RADIUS_METERS {
            debug!(
                "Found glider {} at {:.0}m from towplane {}",
                aircraft_id, distance, towplane_aircraft_id
            );
            candidate_gliders.push((aircraft_id, flight_id));
        }
    }

    Ok(candidate_gliders)
}

/// Check if a towplane has released its tow based on climb rate transition
/// Returns true if release detected
#[allow(dead_code)]
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
