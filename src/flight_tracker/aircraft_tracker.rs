use crate::Fix;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use uuid::Uuid;

use super::towing::TowingInfo;

/// Simplified aircraft state - either idle or active
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AircraftState {
    Idle,   // Stationary or moving slowly (< 25 knots)
    Active, // Moving fast (>= 25 knots) on ground or airborne
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
    pub takeoff_runway_inferred: Option<bool>, // Track whether takeoff runway was inferred (for determining runways_inferred field)
    /// For towplanes: information about the glider being towed (if any)
    pub towing_info: Option<TowingInfo>,
    /// For towplanes: moving window of last 5 climb rates (in fpm) to detect release
    pub climb_rate_history: VecDeque<f32>,
}

impl AircraftTracker {
    #[allow(dead_code)]
    pub fn new(initial_state: AircraftState) -> Self {
        Self {
            state: initial_state,
            current_flight_id: None,
            last_update: Utc::now(),
            last_position: None,
            last_position_time: None,
            last_fix_timestamp: None,
            takeoff_runway_inferred: None,
            towing_info: None,
            climb_rate_history: VecDeque::with_capacity(5),
        }
    }

    #[allow(dead_code)]
    pub fn update_position(&mut self, fix: &Fix) {
        self.last_position = Some((fix.latitude, fix.longitude));
        self.last_position_time = Some(fix.timestamp);
        self.last_fix_timestamp = Some(fix.timestamp);
        self.last_update = Utc::now();
    }

    /// Check if this fix is a duplicate (within 1 second of the last processed fix)
    #[allow(dead_code)]
    pub fn is_duplicate_fix(&self, fix: &Fix) -> bool {
        if let Some(last_timestamp) = self.last_fix_timestamp {
            let time_diff = fix.timestamp.signed_duration_since(last_timestamp);
            time_diff.num_seconds().abs() < 1
        } else {
            false
        }
    }

    /// Update climb rate history for towplane tow release detection
    /// Maintains a moving window of the last 5 climb rates
    #[allow(dead_code)]
    pub fn update_climb_rate(&mut self, climb_fpm: f32) {
        if self.climb_rate_history.len() >= 5 {
            self.climb_rate_history.pop_front();
        }
        self.climb_rate_history.push_back(climb_fpm);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight_tracker::state_transitions::should_be_active;
    use chrono::Utc;

    #[test]
    fn test_aircraft_state_transitions() {
        let tracker = AircraftTracker::new(AircraftState::Idle);
        assert_eq!(tracker.state, AircraftState::Idle);

        // Create a fix with high ground speed
        let mut fix = Fix {
            id: uuid::Uuid::now_v7(),
            source: "TEST".to_string(),
            aprs_type: "APRS".to_string(),
            via: vec![],
            timestamp: Utc::now(),
            received_at: Utc::now(),
            latitude: 40.0,
            longitude: -74.0,
            altitude_msl_feet: Some(1000),
            altitude_agl_feet: None,
            flight_id: None,
            flight_number: None,
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
            device_id: uuid::Uuid::now_v7(),
            is_active: true, // 50 knots is active
            receiver_id: None,
            aprs_message_id: None,
            altitude_agl_valid: false,
        };

        assert!(should_be_active(&fix));

        // Test with low speed
        fix.ground_speed_knots = Some(15.0); // 15 knots - should be idle (below 25 knot threshold)
        assert!(!should_be_active(&fix));
    }
}
