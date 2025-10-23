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
    use crate::flight_tracker::state_transitions::should_be_active;
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

        assert!(should_be_active(&fix));

        // Test with low speed
        fix.ground_speed_knots = Some(15.0); // 15 knots - should be idle (below 20 knot threshold)
        assert!(!should_be_active(&fix));
    }
}
