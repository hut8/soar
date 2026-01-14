use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use uuid::Uuid;

use super::towing::TowingInfo;
use crate::Fix;

/// Compact fix data for in-memory state tracking
/// Much smaller than the full Fix struct - only the fields needed for flight decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactFix {
    pub timestamp: DateTime<Utc>,
    pub lat: f64,
    pub lng: f64,
    pub altitude_msl_ft: Option<i32>,
    pub altitude_agl_ft: Option<i32>,
    pub ground_speed_knots: Option<f32>,
    pub is_active: bool, // Pre-computed from speed/altitude using should_be_active()
}

impl CompactFix {
    /// Create a CompactFix from a full Fix
    pub fn from_fix(fix: &Fix, is_active: bool) -> Self {
        Self {
            timestamp: fix.timestamp,
            lat: fix.latitude,
            lng: fix.longitude,
            altitude_msl_ft: fix.altitude_msl_feet,
            altitude_agl_ft: fix.altitude_agl_feet,
            ground_speed_knots: fix.ground_speed_knots,
            is_active,
        }
    }
}

/// Unified aircraft state tracker
/// Keeps state for ANY aircraft with a fix in the last 18 hours
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AircraftState {
    /// Recent fix history (last 10 fixes, oldest first)
    /// This is enough for all decisions:
    /// - Takeoff detection (check if last 3 are inactive)
    /// - Landing detection (check if last 5 are inactive)
    /// - Flight phase determination (calculate climb rate from altitude changes)
    /// - Flight coalescing (validate distance/speed consistency)
    pub recent_fixes: VecDeque<CompactFix>,

    /// Current flight ID, if aircraft is flying
    pub current_flight_id: Option<Uuid>,

    /// Current flight callsign (for detecting callsign changes without DB query)
    pub current_callsign: Option<String>,

    /// Last timed-out flight info (for resumption without DB query)
    pub last_timed_out_flight_id: Option<Uuid>,
    pub last_timed_out_callsign: Option<String>,
    pub last_timed_out_at: Option<DateTime<Utc>>, // When the flight timed out

    /// Wall clock time of last update (for cleanup and timeout detection)
    pub last_update_time: DateTime<Utc>,

    /// Towplane-specific data (only populated for towtugs)
    pub towing_info: Option<TowingInfo>,

    /// Whether takeoff runway was inferred (for determining runways_inferred field)
    pub takeoff_runway_inferred: Option<bool>,
}

impl AircraftState {
    /// Create a new AircraftState with an initial fix
    pub fn new(fix: &Fix, is_active: bool) -> Self {
        let mut recent_fixes = VecDeque::with_capacity(10);
        recent_fixes.push_back(CompactFix::from_fix(fix, is_active));

        Self {
            recent_fixes,
            current_flight_id: None,
            current_callsign: None,
            last_timed_out_flight_id: None,
            last_timed_out_callsign: None,
            last_timed_out_at: None,
            last_update_time: Utc::now(),
            towing_info: None,
            takeoff_runway_inferred: None,
        }
    }

    /// Create a new AircraftState for restoration from database
    /// Uses the fix's timestamp instead of wall clock time for proper timeout detection
    pub fn new_for_restore(fix: &Fix, is_active: bool) -> Self {
        let mut recent_fixes = VecDeque::with_capacity(10);
        recent_fixes.push_back(CompactFix::from_fix(fix, is_active));

        Self {
            recent_fixes,
            current_flight_id: None,
            current_callsign: None,
            last_timed_out_flight_id: None,
            last_timed_out_callsign: None,
            last_timed_out_at: None,
            last_update_time: fix.timestamp, // Use fix timestamp, not wall clock
            towing_info: None,
            takeoff_runway_inferred: None,
        }
    }

    /// Add a new fix to the recent history
    pub fn add_fix(&mut self, fix: &Fix, is_active: bool) {
        self.last_update_time = Utc::now();

        // Keep only last 10 fixes
        if self.recent_fixes.len() >= 10 {
            self.recent_fixes.pop_front();
        }
        self.recent_fixes
            .push_back(CompactFix::from_fix(fix, is_active));
    }

    /// Add a fix during state restoration (uses fix timestamp instead of wall clock)
    /// This is critical for timeout detection to work correctly after restart
    pub fn add_fix_for_restore(&mut self, fix: &Fix, is_active: bool) {
        // Use the fix's timestamp, not wall clock time
        // This ensures timeout detection works correctly after restart
        self.last_update_time = fix.timestamp;

        // Keep only last 10 fixes
        if self.recent_fixes.len() >= 10 {
            self.recent_fixes.pop_front();
        }
        self.recent_fixes
            .push_back(CompactFix::from_fix(fix, is_active));
    }

    /// Get the most recent fix
    pub fn last_fix(&self) -> Option<&CompactFix> {
        self.recent_fixes.back()
    }

    /// Get the last N fixes (most recent last)
    pub fn last_n_fixes(&self, n: usize) -> Vec<&CompactFix> {
        self.recent_fixes.iter().rev().take(n).rev().collect()
    }

    /// Check if we have 5 consecutive inactive fixes (for landing debounce)
    pub fn has_five_consecutive_inactive(&self) -> bool {
        if self.recent_fixes.len() < 5 {
            return false;
        }

        self.recent_fixes.iter().rev().take(5).all(|f| !f.is_active)
    }

    /// Check if the N fixes BEFORE the current one are all inactive (for takeoff detection)
    /// This skips the most recent fix (which is the active fix that triggered flight creation)
    /// and checks if the n fixes before it were all inactive (indicating aircraft was on ground)
    pub fn last_n_inactive(&self, n: usize) -> bool {
        // Need n+1 fixes: the current one (to skip) plus n previous ones to check
        if self.recent_fixes.len() < n + 1 {
            return false;
        }

        // Skip the most recent fix and check the n fixes before it
        self.recent_fixes
            .iter()
            .rev()
            .skip(1)
            .take(n)
            .all(|f| !f.is_active)
    }

    /// Calculate climb rate from recent fixes
    /// Uses first and last fix with altitude data within last 60 seconds
    pub fn calculate_climb_rate(&self) -> Option<i32> {
        if self.recent_fixes.is_empty() {
            return None;
        }

        let most_recent_timestamp = self.recent_fixes.back()?.timestamp;

        // Filter to fixes within last 60 seconds that have altitude data
        let recent_with_altitude: Vec<&CompactFix> = self
            .recent_fixes
            .iter()
            .filter(|f| {
                let age = most_recent_timestamp.signed_duration_since(f.timestamp);
                age.num_seconds() <= 60 && f.altitude_msl_ft.is_some()
            })
            .collect();

        if recent_with_altitude.len() < 2 {
            return None;
        }

        // Already sorted by timestamp (oldest first)
        let first_fix = recent_with_altitude.first()?;
        let last_fix = recent_with_altitude.last()?;

        let first_alt = first_fix.altitude_msl_ft?;
        let last_alt = last_fix.altitude_msl_ft?;

        let time_delta_seconds = (last_fix.timestamp - first_fix.timestamp).num_seconds();

        // Require at least 5 seconds between fixes to avoid noise
        if time_delta_seconds < 5 {
            return None;
        }

        // Calculate climb rate: (altitude_change_ft / time_delta_seconds) * 60 seconds/minute
        let altitude_change_ft = last_alt - first_alt;
        let climb_rate_fpm = (altitude_change_ft as f64 / time_delta_seconds as f64) * 60.0;

        Some(climb_rate_fpm.round() as i32)
    }

    /// Determine the flight phase based on current state
    /// Used to decide coalescing behavior when flight times out
    pub(crate) fn determine_flight_phase(&self) -> super::FlightPhase {
        let climb = self.calculate_climb_rate();

        // Check climb rate first (most specific indicator)
        if let Some(climb_rate) = climb {
            if climb_rate > 300 {
                return super::FlightPhase::Climbing;
            }
            if climb_rate < -300 {
                return super::FlightPhase::Descending;
            }
        }

        // If at high altitude with stable or gentle climb/descent, assume cruising
        if let Some(last) = self.last_fix()
            && let Some(alt) = last.altitude_msl_ft
            && alt > 10_000
            && climb.map(|c| c.abs() < 500).unwrap_or(true)
        {
            return super::FlightPhase::Cruising;
        }

        // Insufficient data to determine phase
        super::FlightPhase::Unknown
    }

    /// Get the timestamp of the last fix
    pub fn last_fix_timestamp(&self) -> Option<DateTime<Utc>> {
        self.recent_fixes.back().map(|f| f.timestamp)
    }

    /// Get the timestamp of the second-to-last fix (before the current one)
    /// Useful for calculating time gaps when the current fix has already been added to history
    pub fn previous_fix_timestamp(&self) -> Option<DateTime<Utc>> {
        if self.recent_fixes.len() >= 2 {
            self.recent_fixes.iter().rev().nth(1).map(|f| f.timestamp)
        } else {
            None
        }
    }

    /// Get the last fix before the current one (for gap calculations)
    pub fn previous_fix(&self) -> Option<&CompactFix> {
        if self.recent_fixes.len() >= 2 {
            self.recent_fixes.iter().rev().nth(1)
        } else {
            None
        }
    }

    /// Get the last known position (lat, lng)
    pub fn last_position(&self) -> Option<(f64, f64)> {
        self.recent_fixes.back().map(|f| (f.lat, f.lng))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn create_compact_fix(is_active: bool, seconds_offset: i64) -> CompactFix {
        let base_time = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        CompactFix {
            timestamp: base_time + chrono::Duration::seconds(seconds_offset),
            lat: 42.0,
            lng: -71.0,
            altitude_msl_ft: Some(1000),
            altitude_agl_ft: Some(500),
            ground_speed_knots: if is_active { Some(50.0) } else { Some(5.0) },
            is_active,
        }
    }

    fn create_state_with_fixes(fixes: Vec<bool>) -> AircraftState {
        let mut recent_fixes = VecDeque::with_capacity(10);
        for (i, is_active) in fixes.into_iter().enumerate() {
            recent_fixes.push_back(create_compact_fix(is_active, i as i64 * 30));
        }
        AircraftState {
            recent_fixes,
            current_flight_id: None,
            current_callsign: None,
            last_timed_out_flight_id: None,
            last_timed_out_callsign: None,
            last_timed_out_at: None,
            last_update_time: Utc::now(),
            towing_info: None,
            takeoff_runway_inferred: None,
        }
    }

    #[test]
    fn test_last_n_inactive_skips_most_recent_fix() {
        // Scenario: Aircraft was on ground (3 inactive fixes), then takes off (1 active fix)
        // The current active fix should be skipped, and we should detect takeoff from the 3 prior fixes
        let state = create_state_with_fixes(vec![false, false, false, true]);
        //                                        ^--- these 3 should be checked (all inactive)
        //                                                                   ^--- this should be skipped

        // Should detect as takeoff because the 3 fixes BEFORE the current one are all inactive
        assert!(
            state.last_n_inactive(3),
            "Should detect takeoff: 3 prior inactive fixes before current active fix"
        );
    }

    #[test]
    fn test_last_n_inactive_requires_n_plus_one_fixes() {
        // Need n+1 fixes: n to check + 1 to skip
        // With only 3 fixes (and n=3), we need 4 fixes total
        let state = create_state_with_fixes(vec![false, false, false]);

        // Should return false because we need 4 fixes (3 to check + 1 to skip)
        assert!(
            !state.last_n_inactive(3),
            "Should return false: only 3 fixes, need 4 (3 to check + 1 to skip)"
        );
    }

    #[test]
    fn test_last_n_inactive_not_takeoff_if_prior_fixes_active() {
        // Aircraft was already flying, one of the prior fixes is active
        let state = create_state_with_fixes(vec![false, true, false, true]);
        //                                        ^--- inactive
        //                                              ^--- ACTIVE (not on ground)
        //                                                    ^--- inactive
        //                                                          ^--- current (skipped)

        // Should NOT detect as takeoff because the prior fixes include an active one
        assert!(
            !state.last_n_inactive(3),
            "Should not detect takeoff: one of prior 3 fixes was active"
        );
    }

    #[test]
    fn test_last_n_inactive_with_long_ground_time() {
        // Aircraft was on ground for many fixes, then takes off
        let state =
            create_state_with_fixes(vec![false, false, false, false, false, false, false, true]);

        // Should detect as takeoff
        assert!(
            state.last_n_inactive(3),
            "Should detect takeoff after long ground time"
        );
    }

    #[test]
    fn test_last_n_inactive_empty_state() {
        let state = create_state_with_fixes(vec![]);

        assert!(
            !state.last_n_inactive(3),
            "Should return false for empty state"
        );
    }

    #[test]
    fn test_last_n_inactive_single_fix() {
        let state = create_state_with_fixes(vec![true]);

        assert!(
            !state.last_n_inactive(3),
            "Should return false with only 1 fix"
        );
    }

    #[test]
    fn test_has_five_consecutive_inactive_includes_current() {
        // Landing detection: 5 consecutive inactive fixes INCLUDING the current one
        let state = create_state_with_fixes(vec![false, false, false, false, false]);

        // Should detect landing (all 5 are inactive, including current)
        assert!(
            state.has_five_consecutive_inactive(),
            "Should detect 5 consecutive inactive fixes for landing"
        );
    }

    #[test]
    fn test_has_five_consecutive_inactive_fails_with_active() {
        // 4 inactive + 1 active at the end should not trigger landing
        let state = create_state_with_fixes(vec![false, false, false, false, true]);

        // Should NOT detect landing because current fix is active
        assert!(
            !state.has_five_consecutive_inactive(),
            "Should not detect landing: current fix is active"
        );
    }

    #[test]
    fn test_takeoff_vs_landing_detection_difference() {
        // Key test: same 5 fixes should behave differently for takeoff vs landing
        // [inactive, inactive, inactive, inactive, active]

        let state = create_state_with_fixes(vec![false, false, false, false, true]);

        // Takeoff detection (last_n_inactive): skips current active fix, checks prior 3
        // Prior 3 are [inactive, inactive, inactive] -> TRUE (takeoff detected)
        assert!(
            state.last_n_inactive(3),
            "Takeoff: should skip current active and see 3 prior inactive"
        );

        // Landing detection (has_five_consecutive_inactive): includes current fix
        // All 5 are [inactive, inactive, inactive, inactive, active] -> FALSE (not landed)
        assert!(
            !state.has_five_consecutive_inactive(),
            "Landing: should include current active and fail"
        );
    }
}
