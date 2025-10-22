use crate::Fix;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::geometry::haversine_distance;

/// Simplified aircraft state - either idle or active
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AircraftState {
    Idle,   // Stationary or moving slowly (< 20 knots)
    Active, // Moving fast (>= 20 knots) on ground or airborne
}

/// Aircraft tracker with simplified state management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AircraftTracker {
    pub state: AircraftState,
    pub current_flight_id: Option<Uuid>,
    pub last_update: DateTime<Utc>,
    pub last_position: Option<(f64, f64)>, // (lat, lon) for calculating speed
    pub last_position_time: Option<DateTime<Utc>>,
    pub last_fix_timestamp: Option<DateTime<Utc>>, // Track last processed fix to avoid duplicates
    pub towed_by_device_id: Option<Uuid>, // For gliders: device_id of towplane (if being towed)
    pub tow_released: bool, // For gliders: whether tow release has been detected/recorded
    pub takeoff_runway_inferred: Option<bool>, // Track whether takeoff runway was inferred (for determining runways_inferred field)
}

impl AircraftTracker {
    pub fn new(initial_state: AircraftState) -> Self {
        Self {
            state: initial_state,
            current_flight_id: None,
            last_update: Utc::now(),
            last_position: None,
            last_position_time: None,
            last_fix_timestamp: None,
            towed_by_device_id: None,
            tow_released: false,
            takeoff_runway_inferred: None,
        }
    }

    /// Determine if aircraft should be considered active based on fix
    pub fn should_be_active(&self, fix: &Fix) -> bool {
        // Special case: If no altitude data and speed < 80 knots, consider inactive
        // This handles cases where altitude data is missing but we can still infer ground state from speed
        if fix.altitude_agl.is_none() && fix.altitude_msl_feet.is_none() {
            // Calculate speed (from fix or from position changes)
            let speed_knots = if let Some(ground_speed_knots) = fix.ground_speed_knots {
                ground_speed_knots
            } else if let (Some((last_lat, last_lon)), Some(last_time)) =
                (self.last_position, self.last_position_time)
            {
                let time_diff = fix.timestamp.signed_duration_since(last_time);
                if time_diff.num_seconds() > 0 {
                    let distance_meters =
                        haversine_distance(last_lat, last_lon, fix.latitude, fix.longitude);
                    let speed_ms = distance_meters / time_diff.num_seconds() as f64;
                    speed_ms * 1.94384 // m/s to knots
                } else {
                    0.0
                }
            } else {
                0.0
            };

            if speed_knots < 80.0 {
                // No altitude data and slow speed - likely on ground
                return false;
            }
            // No altitude data but high speed - assume active/airborne
            return true;
        }

        // Check ground speed first
        let speed_indicates_active = if let Some(ground_speed_knots) = fix.ground_speed_knots {
            ground_speed_knots >= 20.0
        } else {
            // If no ground speed, calculate from position changes
            if let (Some((last_lat, last_lon)), Some(last_time)) =
                (self.last_position, self.last_position_time)
            {
                let time_diff = fix.timestamp.signed_duration_since(last_time);
                if time_diff.num_seconds() > 0 {
                    let distance_meters =
                        haversine_distance(last_lat, last_lon, fix.latitude, fix.longitude);
                    let speed_ms = distance_meters / time_diff.num_seconds() as f64;
                    let speed_knots = speed_ms * 1.94384; // m/s to knots

                    speed_knots >= 20.0
                } else {
                    // Can't calculate speed, use current state
                    self.state == AircraftState::Active
                }
            } else {
                // No previous position, use current state
                self.state == AircraftState::Active
            }
        };

        // If speed indicates active, aircraft is active
        if speed_indicates_active {
            return true;
        }

        // Speed indicates not active (potential landing)
        // But don't register landing if AGL altitude is >= 250 feet
        // Only land if altitude is unavailable OR altitude is < 250 feet AGL
        if let Some(altitude_agl) = fix.altitude_agl
            && altitude_agl >= 250
        {
            // Still too high to land - remain active
            return true;
        }

        // Either altitude is unavailable, or altitude is < 250 feet
        // Speed is low, so aircraft should be idle (landing)
        false
    }

    pub fn update_position(&mut self, fix: &Fix) {
        self.last_position = Some((fix.latitude, fix.longitude));
        self.last_position_time = Some(fix.timestamp);
        self.last_fix_timestamp = Some(fix.timestamp);
        self.last_update = Utc::now();
    }

    /// Check if this fix is a duplicate (within 1 second of the last processed fix)
    pub fn is_duplicate_fix(&self, fix: &Fix) -> bool {
        if let Some(last_timestamp) = self.last_fix_timestamp {
            let time_diff = fix.timestamp.signed_duration_since(last_timestamp);
            time_diff.num_seconds().abs() < 1
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devices::AddressType;
    use chrono::Utc;

    #[test]
    fn test_aircraft_state_transitions() {
        let tracker = AircraftTracker::new(AircraftState::Idle);
        assert_eq!(tracker.state, AircraftState::Idle);

        // Create a fix with high ground speed
        let mut fix = Fix {
            id: uuid::Uuid::new_v4(),
            source: "TEST".to_string(),
            aprs_type: "APRS".to_string(),
            via: vec![],
            timestamp: Utc::now(),
            received_at: Utc::now(),
            latitude: 40.0,
            longitude: -74.0,
            altitude_msl_feet: Some(1000),
            altitude_agl: None,
            device_address: 0x123456,
            address_type: AddressType::Icao,
            aircraft_type_ogn: None,
            flight_id: None,
            flight_number: None,
            registration: None,
            squawk: None,
            ground_speed_knots: Some(50.0), // 50 knots - should be active
            track_degrees: None,
            climb_fpm: None,
            turn_rate_rot: None,
            snr_db: None,
            bit_errors_corrected: None,
            freq_offset_khz: None,
            gnss_horizontal_resolution: None,
            gnss_vertical_resolution: None,
            unparsed_data: None,
            device_id: uuid::Uuid::new_v4(),
            is_active: true, // 50 knots is active
            receiver_id: None,
            aprs_message_id: None,
        };

        assert!(tracker.should_be_active(&fix));

        // Test with low speed
        fix.ground_speed_knots = Some(15.0); // 15 knots - should be idle (below 20 knot threshold)
        assert!(!tracker.should_be_active(&fix));
    }
}
