//! Geofence detection logic
//!
//! Checks if aircraft positions are within geofence boundaries and detects exit events.

use crate::Fix;
use crate::geofence::{Geofence, GeofenceLayer};

use super::geometry::haversine_distance;

/// Nautical miles to meters conversion factor
const NM_TO_METERS: f64 = 1852.0;

/// Result of checking a fix against a geofence
#[derive(Debug, Clone)]
pub enum GeofenceCheckResult {
    /// Aircraft is inside the geofence at this layer
    Inside { layer: GeofenceLayer },
    /// Aircraft has exited the geofence from this layer
    Outside { exited_layer: GeofenceLayer },
    /// No layer covers this altitude (aircraft is above or below all layers)
    NoLayerAtAltitude,
    /// Cannot determine - missing altitude data
    MissingAltitude,
}

/// Check if a fix is within a geofence boundary
///
/// The geofence is a stacked cylinder ("upside-down birthday cake") where each layer
/// has its own altitude range and radius. The check:
/// 1. Finds the layer(s) that contain the aircraft's altitude
/// 2. Calculates the horizontal distance from the geofence center
/// 3. Returns whether the aircraft is inside or outside
///
/// Returns `GeofenceCheckResult::Inside` if the aircraft is within any layer's radius
/// at its altitude, `Outside` if it's outside the applicable layer's radius,
/// `NoLayerAtAltitude` if no layer covers this altitude, or `MissingAltitude` if
/// the fix doesn't have altitude data.
pub fn check_fix_against_geofence(fix: &Fix, geofence: &Geofence) -> GeofenceCheckResult {
    // Get altitude MSL (required for layer matching)
    let altitude_ft = match fix.altitude_msl_feet {
        Some(alt) => alt,
        None => return GeofenceCheckResult::MissingAltitude,
    };

    // Find all layers that contain this altitude
    let matching_layers: Vec<&GeofenceLayer> = geofence
        .layers
        .iter()
        .filter(|l| l.contains_altitude(altitude_ft))
        .collect();

    if matching_layers.is_empty() {
        return GeofenceCheckResult::NoLayerAtAltitude;
    }

    // Calculate horizontal distance from geofence center
    let distance_m = haversine_distance(
        fix.latitude,
        fix.longitude,
        geofence.center_latitude,
        geofence.center_longitude,
    );

    // Check if inside ANY matching layer (most permissive - if inside any layer, aircraft is inside)
    for layer in &matching_layers {
        let radius_m = layer.radius_nm * NM_TO_METERS;
        if distance_m <= radius_m {
            return GeofenceCheckResult::Inside {
                layer: (*layer).clone(),
            };
        }
    }

    // Outside all matching layers - return the first (should typically only be one layer per altitude)
    GeofenceCheckResult::Outside {
        exited_layer: matching_layers[0].clone(),
    }
}

/// Check if an aircraft has transitioned from inside to outside a geofence
///
/// This is used for exit detection - we only want to trigger an alert when
/// the aircraft transitions from inside to outside (not when it was never inside).
pub fn has_exited_geofence(
    was_inside: bool,
    current_check: &GeofenceCheckResult,
) -> Option<GeofenceLayer> {
    if !was_inside {
        return None;
    }

    match current_check {
        GeofenceCheckResult::Outside { exited_layer } => Some(exited_layer.clone()),
        _ => None,
    }
}

/// Determine if a check result indicates the aircraft is inside the geofence
pub fn is_inside(result: &GeofenceCheckResult) -> bool {
    matches!(result, GeofenceCheckResult::Inside { .. })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_fix(lat: f64, lng: f64, altitude_msl_ft: Option<i32>) -> Fix {
        Fix {
            id: Uuid::new_v4(),
            source: "TEST".to_string(),
            latitude: lat,
            longitude: lng,
            altitude_msl_feet: altitude_msl_ft,
            altitude_agl_feet: None,
            flight_number: None,
            squawk: None,
            ground_speed_knots: Some(100.0),
            track_degrees: None,
            climb_fpm: None,
            turn_rate_rot: None,
            source_metadata: None,
            flight_id: None,
            aircraft_id: Uuid::new_v4(),
            received_at: Utc::now(),
            is_active: true,
            receiver_id: Some(Uuid::new_v4()),
            raw_message_id: Uuid::new_v4(),
            altitude_agl_valid: false,
            time_gap_seconds: None,
        }
    }

    fn create_test_geofence() -> Geofence {
        // Create a geofence centered at 0,0 with two layers:
        // Layer 1: 0-5000 ft MSL, 5 nm radius
        // Layer 2: 5000-10000 ft MSL, 10 nm radius
        Geofence {
            id: Uuid::new_v4(),
            name: "Test Geofence".to_string(),
            description: None,
            center_latitude: 0.0,
            center_longitude: 0.0,
            max_radius_meters: 10.0 * NM_TO_METERS,
            layers: vec![
                GeofenceLayer::new(0, 5000, 5.0),
                GeofenceLayer::new(5000, 10000, 10.0),
            ],
            owner_user_id: Uuid::new_v4(),
            club_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_inside_lower_layer() {
        let geofence = create_test_geofence();
        // Aircraft at center, 3000 ft MSL (within layer 1)
        let fix = create_test_fix(0.0, 0.0, Some(3000));

        let result = check_fix_against_geofence(&fix, &geofence);
        assert!(matches!(result, GeofenceCheckResult::Inside { .. }));
    }

    #[test]
    fn test_inside_upper_layer() {
        let geofence = create_test_geofence();
        // Aircraft at center, 7000 ft MSL (within layer 2)
        let fix = create_test_fix(0.0, 0.0, Some(7000));

        let result = check_fix_against_geofence(&fix, &geofence);
        assert!(matches!(result, GeofenceCheckResult::Inside { .. }));
    }

    #[test]
    fn test_outside_lower_layer() {
        let geofence = create_test_geofence();
        // Aircraft 6 nm east of center (outside 5nm radius), 3000 ft MSL
        // 6 nm = 6 * 1852m / 111320 m per degree = ~0.0998 degrees
        let lng_offset = 6.0 * NM_TO_METERS / 111320.0;
        let fix = create_test_fix(0.0, lng_offset, Some(3000));

        let result = check_fix_against_geofence(&fix, &geofence);
        assert!(matches!(result, GeofenceCheckResult::Outside { .. }));
    }

    #[test]
    fn test_inside_upper_layer_larger_radius() {
        let geofence = create_test_geofence();
        // Aircraft 6 nm east of center, 7000 ft MSL (inside 10nm radius of layer 2)
        let lng_offset = 6.0 * NM_TO_METERS / 111320.0;
        let fix = create_test_fix(0.0, lng_offset, Some(7000));

        let result = check_fix_against_geofence(&fix, &geofence);
        assert!(matches!(result, GeofenceCheckResult::Inside { .. }));
    }

    #[test]
    fn test_no_layer_at_altitude() {
        let geofence = create_test_geofence();
        // Aircraft at 15000 ft (above all layers)
        let fix = create_test_fix(0.0, 0.0, Some(15000));

        let result = check_fix_against_geofence(&fix, &geofence);
        assert!(matches!(result, GeofenceCheckResult::NoLayerAtAltitude));
    }

    #[test]
    fn test_missing_altitude() {
        let geofence = create_test_geofence();
        let fix = create_test_fix(0.0, 0.0, None);

        let result = check_fix_against_geofence(&fix, &geofence);
        assert!(matches!(result, GeofenceCheckResult::MissingAltitude));
    }

    #[test]
    fn test_exit_detection() {
        let layer = GeofenceLayer::new(0, 5000, 5.0);
        let outside_result = GeofenceCheckResult::Outside {
            exited_layer: layer.clone(),
        };

        // Was inside, now outside -> exit detected
        let exit = has_exited_geofence(true, &outside_result);
        assert!(exit.is_some());

        // Was not inside, now outside -> no exit (never entered)
        let no_exit = has_exited_geofence(false, &outside_result);
        assert!(no_exit.is_none());

        // Was inside, now inside -> no exit
        let inside_result = GeofenceCheckResult::Inside { layer };
        let still_inside = has_exited_geofence(true, &inside_result);
        assert!(still_inside.is_none());
    }

    #[test]
    fn test_layer_boundary_at_ceiling() {
        let geofence = create_test_geofence();
        // Aircraft exactly at layer 1 ceiling (5000 ft) - should be inside layer 1
        let fix = create_test_fix(0.0, 0.0, Some(5000));

        let result = check_fix_against_geofence(&fix, &geofence);
        // Should be inside (both layers contain 5000 ft)
        assert!(matches!(result, GeofenceCheckResult::Inside { .. }));
    }

    #[test]
    fn test_layer_boundary_at_floor() {
        let geofence = create_test_geofence();
        // Aircraft exactly at layer 1 floor (0 ft) - should be inside
        let fix = create_test_fix(0.0, 0.0, Some(0));

        let result = check_fix_against_geofence(&fix, &geofence);
        assert!(matches!(result, GeofenceCheckResult::Inside { .. }));
    }
}
