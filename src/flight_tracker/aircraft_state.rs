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
            last_update_time: Utc::now(),
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

    /// Check if last N fixes are all inactive (for takeoff detection)
    pub fn last_n_inactive(&self, n: usize) -> bool {
        if self.recent_fixes.len() < n {
            return false;
        }

        self.recent_fixes.iter().rev().take(n).all(|f| !f.is_active)
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

    /// Get the last known position (lat, lng)
    pub fn last_position(&self) -> Option<(f64, f64)> {
        self.recent_fixes.back().map(|f| (f.lat, f.lng))
    }
}
