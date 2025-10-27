mod aircraft_tracker;
pub mod altitude;
mod flight_lifecycle;
mod geometry;
mod location;
mod runway;
mod state_transitions;
mod towing;
pub(crate) mod utils;

use crate::Fix;
use crate::airports_repo::AirportsRepository;
use crate::elevation::ElevationDB;
use crate::fixes_repo::FixesRepository;
use crate::flights_repo::FlightsRepository;
use crate::locations_repo::LocationsRepository;
use crate::runways_repo::RunwaysRepository;
use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::Instrument;
use tracing::{error, info, trace};
use uuid::Uuid;

/// Tracks the current flight state for a device
#[derive(Debug, Clone)]
pub(crate) struct CurrentFlightState {
    /// The flight ID for the current active flight
    pub flight_id: Uuid,
    /// Timestamp of the last fix received for this flight
    pub last_fix_timestamp: DateTime<Utc>,
    /// Wall-clock time when we last updated this flight state
    pub last_update_time: DateTime<Utc>,
    /// History of the last 5 fixes' is_active status (most recent last)
    /// Used to detect takeoff (inactive -> active transition) and landing debounce
    pub recent_fix_history: VecDeque<bool>,
}

impl CurrentFlightState {
    /// Create a new CurrentFlightState with initial fix activity
    pub fn new(flight_id: Uuid, fix_timestamp: DateTime<Utc>, is_active: bool) -> Self {
        let mut history = VecDeque::with_capacity(5);
        history.push_back(is_active);
        Self {
            flight_id,
            last_fix_timestamp: fix_timestamp,
            last_update_time: Utc::now(),
            recent_fix_history: history,
        }
    }

    /// Update state with a new fix
    pub fn update(&mut self, fix_timestamp: DateTime<Utc>, is_active: bool) {
        self.last_fix_timestamp = fix_timestamp;
        self.last_update_time = Utc::now();

        // Keep only last 5 fixes
        if self.recent_fix_history.len() >= 5 {
            self.recent_fix_history.pop_front();
        }
        self.recent_fix_history.push_back(is_active);
    }

    /// Check if we have 5 consecutive inactive fixes (for landing debounce)
    pub fn has_five_consecutive_inactive(&self) -> bool {
        self.recent_fix_history.len() >= 5 && self.recent_fix_history.iter().all(|&active| !active)
    }
}

/// Type alias for active flights map: device_id -> CurrentFlightState
pub(crate) type ActiveFlightsMap = Arc<RwLock<HashMap<Uuid, CurrentFlightState>>>;

/// Type alias for device locks map: device_id -> Arc<Mutex<()>>
pub(crate) type DeviceLocksMap = Arc<RwLock<HashMap<Uuid, Arc<Mutex<()>>>>>;

/// Type alias for aircraft trackers map: device_id -> AircraftTracker
pub(crate) type AircraftTrackersMap = Arc<RwLock<HashMap<Uuid, aircraft_tracker::AircraftTracker>>>;

/// Simple flight tracker - just tracks which device is currently on which flight
pub struct FlightTracker {
    flights_repo: FlightsRepository,
    airports_repo: AirportsRepository,
    runways_repo: RunwaysRepository,
    fixes_repo: FixesRepository,
    locations_repo: LocationsRepository,
    elevation_db: ElevationDB,
    // Simple map: device_id -> (flight_id, last_fix_timestamp, last_update_time)
    active_flights: ActiveFlightsMap,
    // Per-device mutexes to ensure sequential processing per device
    device_locks: DeviceLocksMap,
    // Aircraft trackers for towplane towing detection
    aircraft_trackers: AircraftTrackersMap,
}

impl Clone for FlightTracker {
    fn clone(&self) -> Self {
        Self {
            flights_repo: self.flights_repo.clone(),
            airports_repo: self.airports_repo.clone(),
            runways_repo: self.runways_repo.clone(),
            fixes_repo: self.fixes_repo.clone(),
            locations_repo: self.locations_repo.clone(),
            elevation_db: self.elevation_db.clone(),
            active_flights: Arc::clone(&self.active_flights),
            device_locks: Arc::clone(&self.device_locks),
            aircraft_trackers: Arc::clone(&self.aircraft_trackers),
        }
    }
}

impl FlightTracker {
    pub fn new(pool: &Pool<ConnectionManager<PgConnection>>) -> Self {
        let elevation_db = ElevationDB::new().expect("Failed to initialize ElevationDB");
        Self {
            flights_repo: FlightsRepository::new(pool.clone()),
            airports_repo: AirportsRepository::new(pool.clone()),
            runways_repo: RunwaysRepository::new(pool.clone()),
            fixes_repo: FixesRepository::new(pool.clone()),
            locations_repo: LocationsRepository::new(pool.clone()),
            elevation_db,
            active_flights: Arc::new(RwLock::new(HashMap::new())),
            device_locks: Arc::new(RwLock::new(HashMap::new())),
            aircraft_trackers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize the flight tracker on startup:
    /// 1. Timeout old incomplete flights (where last_fix_at is older than timeout_duration)
    /// 2. Load active flights from the database into the in-memory tracker
    ///
    /// Returns (number of flights timed out, number of flights loaded)
    pub async fn initialize_from_database(
        &self,
        timeout_duration: chrono::Duration,
    ) -> Result<(usize, usize)> {
        // Phase 1: Timeout old incomplete flights
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

        // Phase 2: Load active flights into the tracker
        info!(
            "Loading active flights from the last {} hours into tracker...",
            timeout_duration.num_hours()
        );
        let active_flights_from_db = self
            .flights_repo
            .get_active_flights_for_tracker(timeout_duration)
            .await?;

        let loaded_count = active_flights_from_db.len();

        // Populate the active_flights map
        if !active_flights_from_db.is_empty() {
            let mut active_flights = self.active_flights.write().await;

            for flight in active_flights_from_db {
                // We need the device_id to populate the map
                if let Some(device_id) = flight.device_id {
                    // Create a CurrentFlightState from the database flight
                    // Initialize with empty history - it will be populated as new fixes arrive
                    let mut history = VecDeque::with_capacity(5);
                    // Assume the flight was active (since it's in our active flights list)
                    history.push_back(true);

                    let state = CurrentFlightState {
                        flight_id: flight.id,
                        last_fix_timestamp: flight.last_fix_at,
                        last_update_time: Utc::now(),
                        recent_fix_history: history,
                    };

                    active_flights.insert(device_id, state);
                    trace!(
                        "Loaded flight {} for device {} into tracker (last fix: {})",
                        flight.id, device_id, flight.last_fix_at
                    );
                }
            }

            info!(
                "Loaded {} active flights into tracker from database",
                loaded_count
            );
        } else {
            info!("No active flights to load into tracker");
        }

        // Update metrics
        {
            let active_flights = self.active_flights.read().await;
            utils::update_flight_tracker_metrics(&active_flights);
        }

        Ok((timed_out_count, loaded_count))
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
    pub async fn cleanup_device_lock(&self, device_id: Uuid) {
        let mut locks = self.device_locks.write().await;
        if locks.remove(&device_id).is_some() {
            trace!("Cleaned up device lock for device {}", device_id);
        }
    }

    /// Check all active flights and timeout any that haven't received beacons for 8+ hours
    #[tracing::instrument(skip(self))]
    pub async fn check_and_timeout_stale_flights(&self) {
        let timeout_threshold = chrono::Duration::hours(8);
        let now = chrono::Utc::now();

        // Collect flights that need to be timed out
        let flights_to_timeout: Vec<(Uuid, Uuid)> = {
            let active_flights = self.active_flights.read().await;
            active_flights
                .iter()
                .filter_map(|(device_id, state)| {
                    let elapsed = now.signed_duration_since(state.last_update_time);
                    if elapsed > timeout_threshold {
                        info!(
                            "Flight {} for device {} is stale (last update {} seconds ago)",
                            state.flight_id,
                            device_id,
                            elapsed.num_seconds()
                        );
                        return Some((state.flight_id, *device_id));
                    }
                    None
                })
                .collect()
        };

        // Update continuous metrics before processing timeouts
        {
            let active_flights = self.active_flights.read().await;
            crate::flight_tracker::utils::update_flight_tracker_metrics(&active_flights);
        }

        let timeout_count = flights_to_timeout.len();

        // Timeout each stale flight
        for (flight_id, device_id) in flights_to_timeout {
            // Double-check that the flight still exists in the map before timing it out
            // (it may have already landed and been removed)
            let should_timeout = {
                let active_flights = self.active_flights.read().await;
                active_flights
                    .get(&device_id)
                    .map(|state| state.flight_id == flight_id)
                    .unwrap_or(false)
            };

            if !should_timeout {
                // Flight was already removed (probably landed), skip timeout
                continue;
            }

            if let Err(e) = flight_lifecycle::timeout_flight(
                &self.flights_repo,
                &self.active_flights,
                flight_id,
                device_id,
            )
            .await
            {
                error!(
                    "Failed to timeout flight {} for device {}: {}",
                    flight_id, device_id, e
                );
            } else {
                // Clean up the device lock after successful timeout
                self.cleanup_device_lock(device_id).await;
                // Increment timeout counter
                metrics::counter!("flight_tracker_timeouts_detected").increment(1);
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
        // Get or create the per-device lock
        let device_lock = {
            let locks_read = self.device_locks.read().await;
            if let Some(lock) = locks_read.get(&fix.device_id) {
                Arc::clone(lock)
            } else {
                drop(locks_read);
                let mut locks_write = self.device_locks.write().await;
                locks_write
                    .entry(fix.device_id)
                    .or_insert_with(|| Arc::new(Mutex::new(())))
                    .clone()
            }
        };

        // Acquire the per-device lock - ensures sequential processing
        let _guard = device_lock.lock().await;

        // Check for duplicate fixes (within 1 second)
        let is_duplicate = {
            let active_flights = self.active_flights.read().await;
            if let Some(state) = active_flights.get(&fix.device_id) {
                let time_diff = fix
                    .timestamp
                    .signed_duration_since(state.last_fix_timestamp);
                time_diff.num_seconds().abs() < 1
            } else {
                false
            }
        };

        if is_duplicate {
            trace!(
                "Discarding duplicate fix for aircraft {} (less than 1 second from previous)",
                fix.device_id
            );
            return None;
        }

        trace!(
            "Processing fix for aircraft {} at {:.6}, {:.6} (speed: {:?} knots)",
            fix.device_id, fix.latitude, fix.longitude, fix.ground_speed_knots
        );

        // Process state transition
        let updated_fix = match state_transitions::process_state_transition(
            &self.flights_repo,
            &self.airports_repo,
            &self.locations_repo,
            &self.runways_repo,
            &self.fixes_repo,
            &self.elevation_db,
            &self.active_flights,
            &self.device_locks,
            &self.aircraft_trackers,
            fix,
        )
        .await
        {
            Ok(updated_fix) => updated_fix,
            Err(e) => {
                error!("Failed to process state transition: {}", e);
                return None;
            }
        };

        // Insert the fix into the database WHILE STILL HOLDING THE LOCK
        match fixes_repo.insert(&updated_fix).await {
            Ok(_) => {
                trace!(
                    "Successfully saved fix to database for aircraft {}",
                    updated_fix.device_address_hex()
                );
                Some(updated_fix)
            }
            Err(e) => {
                error!(
                    "Failed to save fix: device={}, flight_id={:?}, speed={:?}kts, alt_msl={:?}ft, aircraft_type={:?}, error={}",
                    updated_fix.device_id,
                    updated_fix.flight_id,
                    updated_fix.ground_speed_knots,
                    updated_fix.altitude_msl_feet,
                    updated_fix.aircraft_type_ogn,
                    e
                );
                None
            }
        }
    }

    /// Get a flight by its ID
    pub async fn get_flight_by_id(
        &self,
        flight_id: Uuid,
    ) -> Result<Option<crate::flights::Flight>> {
        self.flights_repo.get_flight_by_id(flight_id).await
    }
}
