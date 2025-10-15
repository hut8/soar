mod aircraft_tracker;
mod altitude;
mod flight_lifecycle;
mod geometry;
mod location;
mod runway;
mod state_transitions;
mod towing;
mod utils;

pub use altitude::calculate_and_update_agl_async;

use crate::Fix;
use crate::airports_repo::AirportsRepository;
use crate::elevation::ElevationDB;
use crate::fixes_repo::FixesRepository;
use crate::flights_repo::FlightsRepository;
use crate::locations_repo::LocationsRepository;
use crate::runways_repo::RunwaysRepository;
use aircraft_tracker::AircraftTracker;
use anyhow::Result;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use metrics::{gauge, histogram};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, trace, warn};
use uuid::Uuid;

pub struct FlightTracker {
    flights_repo: FlightsRepository,
    airports_repo: AirportsRepository,
    runways_repo: RunwaysRepository,
    fixes_repo: FixesRepository,
    locations_repo: LocationsRepository,
    elevation_db: ElevationDB,
    aircraft_trackers: Arc<RwLock<HashMap<Uuid, AircraftTracker>>>,
    // Per-device mutexes to ensure sequential processing per device
    device_locks: Arc<RwLock<HashMap<Uuid, Arc<Mutex<()>>>>>,
    state_file_path: Option<std::path::PathBuf>,
    fix_counter: Arc<AtomicU64>,
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
            aircraft_trackers: Arc::clone(&self.aircraft_trackers),
            device_locks: Arc::clone(&self.device_locks),
            state_file_path: self.state_file_path.clone(),
            fix_counter: Arc::clone(&self.fix_counter),
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
            aircraft_trackers: Arc::new(RwLock::new(HashMap::new())),
            device_locks: Arc::new(RwLock::new(HashMap::new())),
            state_file_path: None,
            fix_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Create a new FlightTracker with state persistence enabled
    pub fn with_state_persistence(
        pool: &Pool<ConnectionManager<PgConnection>>,
        state_path: std::path::PathBuf,
    ) -> Self {
        let elevation_db = ElevationDB::new().expect("Failed to initialize ElevationDB");
        Self {
            flights_repo: FlightsRepository::new(pool.clone()),
            airports_repo: AirportsRepository::new(pool.clone()),
            runways_repo: RunwaysRepository::new(pool.clone()),
            fixes_repo: FixesRepository::new(pool.clone()),
            locations_repo: LocationsRepository::new(pool.clone()),
            elevation_db,
            aircraft_trackers: Arc::new(RwLock::new(HashMap::new())),
            device_locks: Arc::new(RwLock::new(HashMap::new())),
            state_file_path: Some(state_path),
            fix_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Save the current state to disk atomically
    pub async fn save_state(&self) -> Result<()> {
        let start = Instant::now();

        if let Some(state_path) = &self.state_file_path {
            // Get a read lock on the trackers
            let trackers = self.aircraft_trackers.read().await;

            // Serialize to JSON
            let json = serde_json::to_string_pretty(&*trackers)?;

            // Write atomically by writing to a temporary file first, then renaming
            let temp_path = state_path.with_extension("tmp");
            tokio::fs::write(&temp_path, json).await?;
            tokio::fs::rename(&temp_path, state_path).await?;

            debug!("Saved flight tracker state to {:?}", state_path);

            // Record metrics for state persistence
            histogram!("flight_tracker_save_duration_seconds")
                .record(start.elapsed().as_secs_f64());
            gauge!("flight_tracker_active_devices").set(trackers.len() as f64);
        }
        Ok(())
    }

    /// Load state from disk if it exists and is less than 24 hours old
    pub async fn load_state(&self) -> Result<()> {
        if let Some(state_path) = &self.state_file_path
            && state_path.exists()
        {
            // Check if the file is older than 24 hours
            let metadata = tokio::fs::metadata(state_path).await?;
            if let Ok(modified) = metadata.modified() {
                let age = std::time::SystemTime::now()
                    .duration_since(modified)
                    .unwrap_or(std::time::Duration::from_secs(0));

                if age > std::time::Duration::from_secs(24 * 60 * 60) {
                    info!("Flight state file is older than 24 hours, deleting it");
                    tokio::fs::remove_file(state_path).await?;
                    return Ok(());
                }
            }

            // Load and deserialize the state
            let json = tokio::fs::read_to_string(state_path).await?;
            let trackers: HashMap<Uuid, AircraftTracker> = serde_json::from_str(&json)?;

            // Filter out stale devices (inactive for more than 2 hours)
            let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(2);
            let original_count = trackers.len();
            let filtered_trackers: HashMap<Uuid, AircraftTracker> = trackers
                .into_iter()
                .filter(|(_, tracker)| tracker.last_update >= cutoff_time)
                .collect();

            let filtered_count = original_count - filtered_trackers.len();
            if filtered_count > 0 {
                info!(
                    "Filtered out {} stale devices from loaded state",
                    filtered_count
                );
            }

            // Replace the current trackers with the filtered loaded state
            let mut current_trackers = self.aircraft_trackers.write().await;
            *current_trackers = filtered_trackers;

            info!(
                "Loaded flight tracker state from {:?} ({} aircraft)",
                state_path,
                current_trackers.len()
            );
        }
        Ok(())
    }

    /// Start a background task to periodically save state
    pub fn start_periodic_state_saving(&self, interval_secs: u64) {
        if self.state_file_path.is_some() {
            let tracker = self.clone();
            tokio::spawn(async move {
                let mut interval =
                    tokio::time::interval(std::time::Duration::from_secs(interval_secs));
                loop {
                    interval.tick().await;
                    if let Err(e) = tracker.save_state().await {
                        warn!("Failed to save flight tracker state: {}", e);
                    }
                }
            });
            info!(
                "Started periodic state saving (every {} seconds)",
                interval_secs
            );
        }
    }

    /// Start a background task to periodically check for timed-out flights
    pub fn start_timeout_checker(&self, check_interval_secs: u64) {
        let tracker = self.clone();
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(check_interval_secs));
            // Skip the first tick (immediate execution)
            interval.tick().await;

            loop {
                interval.tick().await;
                tracker.check_and_timeout_stale_flights().await;
            }
        });
        info!(
            "Started flight timeout checker (every {} seconds)",
            check_interval_secs
        );
    }

    /// Calculate altitude AGL and update the fix in the database asynchronously
    /// This method is designed to be called in a background task after the fix is inserted
    pub async fn calculate_and_update_agl_async(
        &self,
        fix_id: uuid::Uuid,
        fix: &Fix,
        fixes_repo: FixesRepository,
    ) {
        altitude::calculate_and_update_agl_async(&self.elevation_db, fix_id, fix, fixes_repo).await;
    }

    /// Check all active flights and timeout any that haven't received beacons for 5+ minutes
    pub async fn check_and_timeout_stale_flights(&self) {
        let timeout_threshold = chrono::Duration::minutes(5);
        let now = chrono::Utc::now();

        // Collect flights that need to be timed out
        let flights_to_timeout: Vec<(Uuid, Uuid)> = {
            let trackers = self.aircraft_trackers.read().await;
            trackers
                .iter()
                .filter_map(|(device_id, tracker)| {
                    // Check if this tracker has an active flight
                    if let Some(flight_id) = tracker.current_flight_id {
                        // Check if last update was more than 5 minutes ago
                        let elapsed = now.signed_duration_since(tracker.last_update);
                        if elapsed > timeout_threshold {
                            info!(
                                "Flight {} for device {} is stale (last update {} seconds ago)",
                                flight_id,
                                device_id,
                                elapsed.num_seconds()
                            );
                            return Some((flight_id, *device_id));
                        }
                    }
                    None
                })
                .collect()
        };

        // Timeout each stale flight
        for (flight_id, device_id) in flights_to_timeout {
            if let Err(e) = flight_lifecycle::timeout_flight(
                &self.flights_repo,
                &self.aircraft_trackers,
                flight_id,
                device_id,
            )
            .await
            {
                error!(
                    "Failed to timeout flight {} for device {}: {}",
                    flight_id, device_id, e
                );
            }
        }
    }

    /// Process a fix and return it with updated flight_id
    /// This replaces the old FixHandler::process_fix method
    pub async fn process_fix(&self, fix: Fix) -> Option<Fix> {
        // Get or create the per-device lock
        // This ensures fixes for the same device are processed sequentially
        let device_lock = {
            // First try to get an existing lock
            {
                let locks_read = self.device_locks.read().await;
                if let Some(lock) = locks_read.get(&fix.device_id) {
                    Arc::clone(lock)
                } else {
                    // Lock doesn't exist, need to create it
                    drop(locks_read);
                    let mut locks_write = self.device_locks.write().await;
                    // Double-check it wasn't created while we waited for write lock
                    locks_write
                        .entry(fix.device_id)
                        .or_insert_with(|| Arc::new(Mutex::new(())))
                        .clone()
                }
            }
        };

        // Acquire the per-device lock - this ensures sequential processing
        let _guard = device_lock.lock().await;

        // Check for duplicate fixes first (within 1 second)
        let is_duplicate = {
            let lock_start = Instant::now();
            let trackers_read = self.aircraft_trackers.try_read();
            match trackers_read {
                Ok(trackers) => {
                    histogram!("flight_tracker_read_lock_duration_seconds")
                        .record(lock_start.elapsed().as_secs_f64());
                    if let Some(tracker) = trackers.get(&fix.device_id) {
                        tracker.is_duplicate_fix(&fix)
                    } else {
                        false // New aircraft, not a duplicate
                    }
                }
                Err(_) => {
                    // Could not get read lock, process anyway
                    metrics::counter!("flight_tracker_read_lock_contention_total").increment(1);
                    false
                }
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

        // Process state transition and return updated fix
        let fix_device_address = fix.device_address; // Store for error logging
        match state_transitions::process_state_transition(
            &self.flights_repo,
            &self.airports_repo,
            &self.locations_repo,
            &self.runways_repo,
            &self.fixes_repo,
            &self.elevation_db,
            &self.aircraft_trackers,
            fix,
        )
        .await
        {
            Ok(updated_fix) => {
                // Deterministically clean up old trackers every 1000 fixes
                let count = self.fix_counter.fetch_add(1, Ordering::Relaxed);
                if count.is_multiple_of(1000) {
                    let trackers = Arc::clone(&self.aircraft_trackers);
                    tokio::spawn(async move {
                        let mut trackers_write = trackers.write().await;
                        utils::cleanup_old_trackers(&mut trackers_write).await;
                    });
                }
                Some(updated_fix)
            }
            Err(e) => {
                error!(
                    "Failed to process state transition for aircraft {}: {}",
                    fix_device_address, e
                );
                None
            }
        }
        // Lock is automatically released when _guard goes out of scope
    }
}
