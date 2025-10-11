use crate::Fix;
use crate::fixes_repo::FixesRepository;
use crate::flights_repo::FlightsRepository;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::aircraft_tracker::{AircraftState, AircraftTracker};
use super::geometry::angular_difference;
use super::geometry::haversine_distance;

/// Detect if a glider taking off is being towed by a nearby towplane
/// Returns the towplane's (device_id, flight_id, current_altitude) if found
pub(crate) async fn detect_towing_at_takeoff(
    aircraft_trackers: &Arc<RwLock<HashMap<Uuid, AircraftTracker>>>,
    fixes_repo: &FixesRepository,
    glider_device_id: &Uuid,
    glider_fix: &Fix,
) -> Option<(Uuid, Uuid, i32)> {
    // Only check for towing if this is a glider
    use crate::ogn_aprs_aircraft::AircraftType;
    if glider_fix.aircraft_type_ogn != Some(AircraftType::Glider) {
        return None;
    }

    // Get all currently active aircraft trackers
    let active_flights = {
        let trackers = aircraft_trackers.read().await;
        trackers
            .iter()
            .filter_map(|(device_id, tracker)| {
                // Skip ourselves and aircraft without active flights
                if device_id == glider_device_id || tracker.current_flight_id.is_none() {
                    return None;
                }
                // Only consider aircraft that are active (flying)
                if tracker.state != AircraftState::Active {
                    return None;
                }
                Some((*device_id, tracker.current_flight_id.unwrap()))
            })
            .collect::<Vec<_>>()
    };

    if active_flights.is_empty() {
        return None;
    }

    // Get recent fixes for potential towplanes (within last 30 seconds)
    let time_window_start = glider_fix.timestamp - chrono::Duration::seconds(30);

    for (towplane_device_id, towplane_flight_id) in active_flights {
        // Get the most recent fix for this potential towplane
        match fixes_repo
            .get_latest_fix_for_device(towplane_device_id, time_window_start)
            .await
        {
            Ok(Some(towplane_fix)) => {
                // Check if aircraft type suggests it's a towplane
                let is_likely_towplane = match towplane_fix.aircraft_type_ogn {
                    Some(AircraftType::TowTug) => true,
                    Some(AircraftType::RecipEngine) => true,
                    Some(AircraftType::JetTurboprop) => false,
                    Some(AircraftType::Glider) => false, // Gliders don't tow gliders
                    _ => true,                           // Unknown types could be towplanes
                };

                if !is_likely_towplane {
                    continue;
                }

                // Calculate distance between glider and potential towplane
                let distance_meters = haversine_distance(
                    glider_fix.latitude,
                    glider_fix.longitude,
                    towplane_fix.latitude,
                    towplane_fix.longitude,
                );

                // Check if they're close enough to be towing (within 200 meters / ~650 feet)
                if distance_meters <= 200.0 {
                    // Check altitude difference (should be similar, within 200 feet)
                    if let (Some(glider_alt), Some(towplane_alt)) =
                        (glider_fix.altitude_msl_feet, towplane_fix.altitude_msl_feet)
                    {
                        let altitude_diff = (glider_alt - towplane_alt).abs();
                        if altitude_diff <= 200 {
                            info!(
                                "Detected towing: glider {} is being towed by towplane {} (distance: {:.0}m, alt diff: {}ft)",
                                glider_device_id,
                                towplane_device_id,
                                distance_meters,
                                altitude_diff
                            );
                            return Some((towplane_device_id, towplane_flight_id, towplane_alt));
                        }
                    }
                }
            }
            Ok(None) => continue,
            Err(e) => {
                debug!(
                    "Failed to get latest fix for potential towplane {}: {}",
                    towplane_device_id, e
                );
                continue;
            }
        }
    }

    None
}

/// Check if a glider flight has been released from tow
/// This is called periodically during active flight to detect separation
pub(crate) async fn check_tow_release(
    fixes_repo: &FixesRepository,
    glider_device_id: &Uuid,
    _glider_flight_id: &Uuid,
    glider_fix: &Fix,
    towplane_device_id: &Uuid,
) -> bool {
    // Get recent fix for towplane (within last 10 seconds)
    let time_window = glider_fix.timestamp - chrono::Duration::seconds(10);

    match fixes_repo
        .get_latest_fix_for_device(*towplane_device_id, time_window)
        .await
    {
        Ok(Some(towplane_fix)) => {
            // Calculate horizontal distance
            let distance_meters = haversine_distance(
                glider_fix.latitude,
                glider_fix.longitude,
                towplane_fix.latitude,
                towplane_fix.longitude,
            );

            // Calculate 3D distance if we have altitudes
            let separation_feet = if let (Some(glider_alt), Some(towplane_alt)) =
                (glider_fix.altitude_msl_feet, towplane_fix.altitude_msl_feet)
            {
                let horizontal_feet = distance_meters * 3.28084; // meters to feet
                let vertical_feet = (glider_alt - towplane_alt).abs() as f64;
                (horizontal_feet.powi(2) + vertical_feet.powi(2)).sqrt()
            } else {
                distance_meters * 3.28084 // Just horizontal distance in feet
            };

            // Release detected if separation > 500 feet
            if separation_feet > 500.0 {
                info!(
                    "Tow release detected for glider {}: separated {:.0} feet from towplane {}",
                    glider_device_id, separation_feet, towplane_device_id
                );
                return true;
            }

            // Also check for diverging headings (one or both turned significantly)
            if let (Some(glider_track), Some(towplane_track)) =
                (glider_fix.track_degrees, towplane_fix.track_degrees)
            {
                let heading_diff = angular_difference(glider_track as f64, towplane_track as f64);
                if heading_diff > 45.0 && distance_meters > 100.0 {
                    info!(
                        "Tow release detected for glider {}: diverged {:.0}° from towplane {} (distance: {:.0}m)",
                        glider_device_id, heading_diff, towplane_device_id, distance_meters
                    );
                    return true;
                }
            }

            false
        }
        Ok(None) => {
            // No recent fix from towplane - might have landed or lost signal
            // Consider this a release if we haven't seen them for 30+ seconds
            warn!(
                "Lost contact with towplane {} for glider {} - assuming release",
                towplane_device_id, glider_device_id
            );
            true
        }
        Err(e) => {
            debug!(
                "Error checking tow release for glider {}: {}",
                glider_device_id, e
            );
            false
        }
    }
}

/// Record tow release in the database
pub(crate) async fn record_tow_release(
    flights_repo: &FlightsRepository,
    glider_flight_id: &Uuid,
    release_fix: &Fix,
) -> Result<()> {
    if let Some(altitude_ft) = release_fix.altitude_msl_feet {
        info!(
            "Recording tow release for flight {} at {}ft MSL",
            glider_flight_id, altitude_ft
        );

        flights_repo
            .update_tow_release(*glider_flight_id, altitude_ft, release_fix.timestamp)
            .await?;
    }
    Ok(())
}
