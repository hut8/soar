mod altitude;
mod flight_lifecycle;
mod geometry;
mod location;
mod runway;
mod state_transitions;

pub use altitude::calculate_and_update_agl_async;

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
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, trace};
use uuid::Uuid;

/// Type alias for active flights map: device_id -> (flight_id, last_fix_timestamp, last_update_time)
pub(crate) type ActiveFlightsMap = Arc<RwLock<HashMap<Uuid, (Uuid, DateTime<Utc>, DateTime<Utc>)>>>;

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
    device_locks: Arc<RwLock<HashMap<Uuid, Arc<Mutex<()>>>>>,
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
        }
    }

    /// Create a new FlightTracker with state persistence enabled
    /// Note: State persistence has been removed for simplicity
    pub fn with_state_persistence(
        pool: &Pool<ConnectionManager<PgConnection>>,
        _state_path: std::path::PathBuf,
    ) -> Self {
        // Just create a normal FlightTracker - state persistence removed
        Self::new(pool)
    }

    /// Load state from disk - now a no-op
    pub async fn load_state(&self) -> Result<()> {
        // State persistence removed
        Ok(())
    }

    /// Start periodic state saving - now a no-op
    pub fn start_periodic_state_saving(&self, _interval_secs: u64) {
        // State persistence removed
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
            let active_flights = self.active_flights.read().await;
            active_flights
                .iter()
                .filter_map(
                    |(device_id, (flight_id, _last_fix_time, last_update_time))| {
                        let elapsed = now.signed_duration_since(*last_update_time);
                        if elapsed > timeout_threshold {
                            info!(
                                "Flight {} for device {} is stale (last update {} seconds ago)",
                                flight_id,
                                device_id,
                                elapsed.num_seconds()
                            );
                            return Some((*flight_id, *device_id));
                        }
                        None
                    },
                )
                .collect()
        };

        // Timeout each stale flight
        for (flight_id, device_id) in flights_to_timeout {
            // Double-check that the flight still exists in the map before timing it out
            // (it may have already landed and been removed)
            let should_timeout = {
                let active_flights = self.active_flights.read().await;
                active_flights
                    .get(&device_id)
                    .map(|(fid, _, _)| *fid == flight_id)
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
            }
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
            if let Some((_flight_id, last_fix_time, _last_update)) =
                active_flights.get(&fix.device_id)
            {
                let time_diff = fix.timestamp.signed_duration_since(*last_fix_time);
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
                    "Failed to save fix to database for fix: {:?}\ncause:{:?}",
                    updated_fix, e
                );
                None
            }
        }
    }
}
