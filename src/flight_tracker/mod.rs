mod aircraft_state;
mod aircraft_tracker;
pub mod altitude;
mod flight_lifecycle;
mod geometry;
mod location;
mod runway;
mod state_transitions;
mod towing;
pub(crate) mod utils;

// Re-export should_be_active for use in fix_processor
pub use aircraft_state::{AircraftState, CompactFix};
pub use state_transitions::should_be_active;

use crate::Fix;
use crate::aircraft_repo::AircraftRepository;
use crate::airports_repo::AirportsRepository;
use crate::elevation::ElevationDB;
use crate::fixes_repo::FixesRepository;
use crate::flights_repo::FlightsRepository;
use crate::locations_repo::LocationsRepository;
use crate::runways_repo::RunwaysRepository;
use anyhow::Result;
use dashmap::DashMap;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::Instrument;
use tracing::{debug, error, info, trace, warn};
use uuid::Uuid;

/// Represents the flight state when timeout occurs
/// Used to determine coalescing strategy when aircraft reappears
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum FlightPhase {
    /// Aircraft is climbing (climb_fpm > 300)
    Climbing,
    /// Aircraft is cruising (altitude > 10,000 ft, |climb_fpm| < 500)
    Cruising,
    /// Aircraft is descending (climb_fpm < -300)
    Descending,
    /// Aircraft state is unknown (insufficient data)
    Unknown,
}

/// Unified aircraft state map using DashMap for concurrent per-aircraft locking
/// Tracks all aircraft with fixes in the last 18 hours
pub(crate) type AircraftStatesMap = Arc<DashMap<Uuid, AircraftState>>;

/// Type alias for device locks map: aircraft_id -> Arc<Mutex<()>>
/// Still needed for serializing operations on the same aircraft
pub(crate) type AircraftLocksMap = Arc<DashMap<Uuid, Arc<Mutex<()>>>>;

/// Context for flight processing operations
/// Contains all repositories and state needed for flight lifecycle management
pub(crate) struct FlightProcessorContext<'a> {
    pub flights_repo: &'a FlightsRepository,
    pub aircraft_repo: &'a AircraftRepository,
    pub airports_repo: &'a AirportsRepository,
    pub locations_repo: &'a LocationsRepository,
    pub runways_repo: &'a RunwaysRepository,
    pub fixes_repo: &'a FixesRepository,
    pub elevation_db: &'a ElevationDB,
    pub aircraft_states: &'a AircraftStatesMap,
    pub pool: diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>,
}

/// Simple flight tracker - just tracks which device is currently on which flight
pub struct FlightTracker {
    pool: Pool<ConnectionManager<PgConnection>>,
    flights_repo: FlightsRepository,
    device_repo: AircraftRepository,
    airports_repo: AirportsRepository,
    runways_repo: RunwaysRepository,
    fixes_repo: FixesRepository,
    locations_repo: LocationsRepository,
    elevation_db: ElevationDB,
    // Unified aircraft state map: all aircraft seen in last 18 hours
    // DashMap provides concurrent per-key locking so one aircraft update doesn't block another
    aircraft_states: AircraftStatesMap,
    // Per-device mutexes to ensure sequential processing per device
    device_locks: AircraftLocksMap,
}

impl Clone for FlightTracker {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            flights_repo: self.flights_repo.clone(),
            device_repo: self.device_repo.clone(),
            airports_repo: self.airports_repo.clone(),
            runways_repo: self.runways_repo.clone(),
            fixes_repo: self.fixes_repo.clone(),
            locations_repo: self.locations_repo.clone(),
            elevation_db: self.elevation_db.clone(),
            aircraft_states: Arc::clone(&self.aircraft_states),
            device_locks: Arc::clone(&self.device_locks),
        }
    }
}

impl FlightTracker {
    pub fn new(pool: &Pool<ConnectionManager<PgConnection>>) -> Self {
        let elevation_db = ElevationDB::new().expect("Failed to initialize ElevationDB");
        Self {
            pool: pool.clone(),
            flights_repo: FlightsRepository::new(pool.clone()),
            device_repo: AircraftRepository::new(pool.clone()),
            airports_repo: AirportsRepository::new(pool.clone()),
            runways_repo: RunwaysRepository::new(pool.clone()),
            fixes_repo: FixesRepository::new(pool.clone()),
            locations_repo: LocationsRepository::new(pool.clone()),
            elevation_db,
            aircraft_states: Arc::new(DashMap::new()),
            device_locks: Arc::new(DashMap::new()),
        }
    }

    /// Get a context reference for flight processing operations
    /// This provides a convenient way to pass all necessary dependencies to flight lifecycle functions
    fn context(&self) -> FlightProcessorContext<'_> {
        FlightProcessorContext {
            flights_repo: &self.flights_repo,
            aircraft_repo: &self.device_repo,
            airports_repo: &self.airports_repo,
            locations_repo: &self.locations_repo,
            runways_repo: &self.runways_repo,
            fixes_repo: &self.fixes_repo,
            elevation_db: &self.elevation_db,
            aircraft_states: &self.aircraft_states,
            pool: self.pool.clone(),
        }
    }

    /// Initialize the flight tracker on startup:
    /// 1. Timeout old incomplete flights (where last_fix_at is older than timeout_duration)
    /// 2. Restore aircraft states from database for all active flights
    ///
    /// This restores the in-memory state by loading recent fixes for each active flight,
    /// which is critical for:
    /// - Takeoff detection (needs last 3 fixes)
    /// - Landing detection (needs last 5 fixes)
    /// - Flight coalescing (needs flight phase from recent fixes)
    /// - Tow release detection (needs recent fixes for climb rate)
    ///
    /// Returns (timed_out_count, restored_states_count)
    pub async fn initialize_from_database(
        &self,
        timeout_duration: chrono::Duration,
    ) -> Result<(usize, usize)> {
        info!(
            "Timing out incomplete flights older than {} hours...",
            timeout_duration.num_hours()
        );
        let timed_out_count = self
            .flights_repo
            .timeout_old_incomplete_flights(timeout_duration)
            .await?;

        if timed_out_count > 0 {
            info!(
                "Timed out {} old incomplete flights on startup",
                timed_out_count
            );
        } else {
            info!("No old incomplete flights to timeout");
        }

        // Restore aircraft states from active flights
        info!("Restoring aircraft states from database...");
        let active_flights = self
            .flights_repo
            .get_active_flights_for_tracker(timeout_duration)
            .await?;

        info!("Found {} active flights to restore", active_flights.len());

        let mut restored_count: usize = 0;
        for flight in active_flights {
            // Skip flights without aircraft_id (shouldn't happen but be defensive)
            let aircraft_id = match flight.aircraft_id {
                Some(id) => id,
                None => {
                    warn!(
                        "Flight {} has no aircraft_id, skipping state restore",
                        flight.id
                    );
                    continue;
                }
            };

            // Get last 10 fixes for this flight
            let start_time = chrono::Utc::now() - chrono::Duration::hours(24);
            let fixes = self
                .fixes_repo
                .get_fixes_for_flight(flight.id, Some(10), start_time, None)
                .await
                .unwrap_or_default();

            if fixes.is_empty() {
                continue;
            }

            // Create aircraft state with the oldest fix first
            let first_fix = &fixes[fixes.len() - 1];
            let is_active = state_transitions::should_be_active(first_fix);
            let mut state = aircraft_state::AircraftState::new(first_fix, is_active);
            state.current_flight_id = Some(flight.id);

            // Add remaining fixes in chronological order (oldest to newest)
            for fix in fixes.iter().rev().skip(1) {
                let is_active = state_transitions::should_be_active(fix);
                state.add_fix(fix, is_active);
            }

            // Insert into map
            self.aircraft_states.insert(aircraft_id, state);
            restored_count += 1;
        }

        info!(
            "Restored state for {} aircraft with active flights",
            restored_count
        );

        // Update metrics
        metrics::counter!("flight_tracker.startup.aircraft_states_restored_total")
            .increment(restored_count as u64);
        utils::update_flight_tracker_metrics(&self.aircraft_states);

        Ok((timed_out_count, restored_count))
    }

    /// Get a reference to the elevation database
    pub fn elevation_db(&self) -> &ElevationDB {
        &self.elevation_db
    }

    /// Start a background task to periodically check for timed-out flights
    pub fn start_timeout_checker(&self, check_interval_secs: u64) {
        let tracker = self.clone();
        tokio::spawn(
            async move {
                let mut interval =
                    tokio::time::interval(std::time::Duration::from_secs(check_interval_secs));
                // Skip the first tick (immediate execution)
                interval.tick().await;

                loop {
                    interval.tick().await;
                    tracker.check_and_timeout_stale_flights().await;
                }
            }
            .instrument(tracing::info_span!("flight_timeout_checker")),
        );
        info!(
            "Started flight timeout checker (every {} seconds)",
            check_interval_secs
        );
    }

    /// Start a background task to periodically clean up stale aircraft states (older than 18 hours)
    pub fn start_state_cleanup(&self, check_interval_secs: u64) {
        let tracker = self.clone();
        tokio::spawn(
            async move {
                let mut interval =
                    tokio::time::interval(std::time::Duration::from_secs(check_interval_secs));
                // Skip the first tick (immediate execution)
                interval.tick().await;

                loop {
                    interval.tick().await;
                    tracker.cleanup_stale_aircraft_states().await;
                }
            }
            .instrument(tracing::info_span!("aircraft_state_cleanup")),
        );
        info!(
            "Started aircraft state cleanup (every {} seconds)",
            check_interval_secs
        );
    }

    /// Clean up aircraft states that haven't been updated in 18+ hours
    async fn cleanup_stale_aircraft_states(&self) {
        let cleanup_threshold = chrono::Duration::hours(18);
        let now = chrono::Utc::now();

        let mut removed_count = 0;
        self.aircraft_states.retain(|aircraft_id, state| {
            let elapsed = now.signed_duration_since(state.last_update_time);
            if elapsed > cleanup_threshold {
                debug!(
                    "Removing stale aircraft state for {} (last update {} hours ago)",
                    aircraft_id,
                    elapsed.num_hours()
                );
                removed_count += 1;
                false // Remove this entry
            } else {
                true // Keep this entry
            }
        });

        if removed_count > 0 {
            info!("Cleaned up {} stale aircraft states", removed_count);
            metrics::counter!("flight_tracker.state_cleanup.removed_total")
                .increment(removed_count);
        }

        // Update metrics after cleanup
        utils::update_flight_tracker_metrics(&self.aircraft_states);
    }

    /// Calculate altitude AGL and update the fix in the database asynchronously
    #[tracing::instrument(skip(self, fix, fixes_repo), fields(fix_id = %fix_id))]
    pub async fn calculate_and_update_agl_async(
        &self,
        fix_id: uuid::Uuid,
        fix: &Fix,
        fixes_repo: FixesRepository,
    ) {
        altitude::calculate_and_update_agl_async(&self.elevation_db, fix_id, fix, fixes_repo).await;
    }

    /// Clean up the device lock for a specific device
    /// This should be called when a flight completes or times out
    pub async fn cleanup_device_lock(&self, aircraft_id: Uuid) {
        if self.device_locks.remove(&aircraft_id).is_some() {
            trace!("Cleaned up device lock for device {}", aircraft_id);
        }
    }

    /// Check all aircraft with active flights and timeout any that haven't received fixes for 1+ hours
    #[tracing::instrument(skip(self))]
    pub async fn check_and_timeout_stale_flights(&self) {
        let timeout_threshold = chrono::Duration::hours(1);
        let now = chrono::Utc::now();

        // Collect flights that need to be timed out
        let flights_to_timeout: Vec<(Uuid, Uuid)> = self
            .aircraft_states
            .iter()
            .filter_map(|entry| {
                let aircraft_id = *entry.key();
                let state = entry.value();

                // Only check aircraft with active flights
                let flight_id = state.current_flight_id?;

                let elapsed = now.signed_duration_since(state.last_update_time);
                if elapsed > timeout_threshold {
                    info!(
                        "Flight {} for device {} is stale (last update {} seconds ago)",
                        flight_id,
                        aircraft_id,
                        elapsed.num_seconds()
                    );
                    return Some((flight_id, aircraft_id));
                }
                None
            })
            .collect();

        // Update continuous metrics before processing timeouts
        crate::flight_tracker::utils::update_flight_tracker_metrics(&self.aircraft_states);

        let timeout_count = flights_to_timeout.len();

        // Timeout each stale flight
        for (flight_id, aircraft_id) in flights_to_timeout {
            // Double-check that the flight still exists before timing it out
            let should_timeout = self
                .aircraft_states
                .get(&aircraft_id)
                .and_then(|state| state.current_flight_id)
                .map(|fid| fid == flight_id)
                .unwrap_or(false);

            if !should_timeout {
                // Flight was already removed (probably landed), skip timeout
                continue;
            }

            // Create context for timeout processing
            let ctx = self.context();

            if let Err(e) = flight_lifecycle::timeout_flight(&ctx, flight_id, aircraft_id).await {
                error!(
                    "Failed to timeout flight {} for device {}: {}",
                    flight_id, aircraft_id, e
                );
            } else {
                // Clean up the device lock after successful timeout
                self.cleanup_device_lock(aircraft_id).await;
                // Increment timeout counter
                metrics::counter!("flight_tracker_timeouts_detected_total").increment(1);
            }
        }

        if timeout_count > 0 {
            info!("Timed out {} flights", timeout_count);
        }
    }

    /// Process a fix, insert it into the database, and return it with updated flight_id
    /// This method holds the per-device lock through the entire process including DB insertion
    pub async fn process_and_insert_fix(
        &self,
        fix: Fix,
        fixes_repo: &FixesRepository,
    ) -> Option<Fix> {
        // Get or create the per-device lock (DashMap provides automatic concurrent access)
        let device_lock = self
            .device_locks
            .entry(fix.aircraft_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();

        // Acquire the per-device lock - ensures sequential processing
        let _guard = device_lock.lock().await;

        // Check for duplicate fixes (within 1 second)
        let is_duplicate = if let Some(state) = self.aircraft_states.get(&fix.aircraft_id) {
            if let Some(last_fix_time) = state.last_fix_timestamp() {
                let time_diff = fix.timestamp.signed_duration_since(last_fix_time);
                time_diff.num_seconds().abs() < 1
            } else {
                false
            }
        } else {
            false
        };

        if is_duplicate {
            trace!(
                "Discarding duplicate fix for aircraft {} (less than 1 second from previous)",
                fix.aircraft_id
            );
            return None;
        }

        trace!(
            "Processing fix for aircraft {} at {:.6}, {:.6} (speed: {:?} knots)",
            fix.aircraft_id, fix.latitude, fix.longitude, fix.ground_speed_knots
        );

        // Process state transition
        let state_transition_start = std::time::Instant::now();
        let updated_fix =
            match state_transitions::process_state_transition(&self.context(), fix).await {
                Ok(updated_fix) => updated_fix,
                Err(e) => {
                    error!("Failed to process state transition: {}", e);
                    return None;
                }
            };
        metrics::histogram!("aprs.aircraft.state_transition_ms")
            .record(state_transition_start.elapsed().as_micros() as f64 / 1000.0);

        // Insert the fix into the database WHILE STILL HOLDING THE LOCK
        let fix_insert_start = std::time::Instant::now();
        let result = match fixes_repo.insert(&updated_fix).await {
            Ok(_) => {
                trace!(
                    "Successfully saved fix to database for device {}",
                    updated_fix.aircraft_id
                );
                Some(updated_fix)
            }
            Err(e) => {
                error!(
                    "Failed to save fix: aircraft={}, flight_id={:?}, speed={:?}kts, alt_msl={:?}ft, error={}",
                    updated_fix.aircraft_id,
                    updated_fix.flight_id,
                    updated_fix.ground_speed_knots,
                    updated_fix.altitude_msl_feet,
                    e
                );
                None
            }
        };
        metrics::histogram!("aprs.aircraft.fix_db_insert_ms")
            .record(fix_insert_start.elapsed().as_micros() as f64 / 1000.0);

        // Increment counter for stats logging
        if result.is_some() {
            metrics::counter!("flight_tracker.fixes_processed_total").increment(1);
        }

        result
    }

    /// Get a flight by its ID
    pub async fn get_flight_by_id(
        &self,
        flight_id: Uuid,
    ) -> Result<Option<crate::flights::Flight>> {
        self.flights_repo.get_flight_by_id(flight_id).await
    }

    /// Update the callsign for a flight
    pub async fn update_flight_callsign(
        &self,
        flight_id: Uuid,
        callsign: Option<String>,
    ) -> Result<bool> {
        self.flights_repo.update_callsign(flight_id, callsign).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, TimeZone, Utc};

    // Helper function to create a test fix
    fn create_test_fix(timestamp: DateTime<Utc>, altitude_msl: Option<i32>) -> Fix {
        Fix {
            id: Uuid::new_v4(),
            source: "TEST".to_string(),
            aprs_type: "position".to_string(),
            via: vec![],
            timestamp,
            latitude: 42.0,
            longitude: -122.0,
            altitude_msl_feet: altitude_msl,
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
            received_at: timestamp,
            is_active: true,
            receiver_id: Uuid::new_v4(),
            raw_message_id: Uuid::new_v4(),
            altitude_agl_valid: false,
            time_gap_seconds: None,
        }
    }

    #[test]
    fn test_calculate_climb_rate_ascending() {
        // Create fixes: 1000 ft at T+0s, 1600 ft at T+60s
        // Expected: +600 FPM
        let base_time = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();

        let fix1 = create_test_fix(base_time, Some(1000));
        let fix2 = create_test_fix(base_time + chrono::Duration::seconds(60), Some(1600));

        let mut state = aircraft_state::AircraftState::new(&fix1, true);
        state.add_fix(&fix2, true);

        let result = state.calculate_climb_rate();
        assert_eq!(result, Some(600));
    }

    #[test]
    fn test_calculate_climb_rate_descending() {
        // Create fixes: 5000 ft at T+0s, 4000 ft at T+60s
        // Expected: -1000 FPM
        let base_time = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();

        let fix1 = create_test_fix(base_time, Some(5000));
        let fix2 = create_test_fix(base_time + chrono::Duration::seconds(60), Some(4000));

        let mut state = aircraft_state::AircraftState::new(&fix1, true);
        state.add_fix(&fix2, true);

        let result = state.calculate_climb_rate();
        assert_eq!(result, Some(-1000));
    }

    #[test]
    fn test_calculate_climb_rate_insufficient_data() {
        // Only 1 fix with altitude - should return None
        let base_time = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();

        let fix1 = create_test_fix(base_time, Some(1000));
        let fix2 = create_test_fix(base_time + chrono::Duration::seconds(60), None);

        let mut state = aircraft_state::AircraftState::new(&fix1, true);
        state.add_fix(&fix2, true);

        let result = state.calculate_climb_rate();
        assert_eq!(result, None);
    }

    #[test]
    fn test_calculate_climb_rate_time_too_short() {
        // Fixes only 2 seconds apart - should return None
        let base_time = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();

        let fix1 = create_test_fix(base_time, Some(1000));
        let fix2 = create_test_fix(base_time + chrono::Duration::seconds(2), Some(1020));

        let mut state = aircraft_state::AircraftState::new(&fix1, true);
        state.add_fix(&fix2, true);

        let result = state.calculate_climb_rate();
        assert_eq!(result, None);
    }
}
