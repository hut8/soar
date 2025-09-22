use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

use crate::database_fix_processor::DatabaseFixProcessor;
use crate::flights::Flight;
use crate::flights_repo::FlightsRepository;
use crate::{Fix, FixProcessor};
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};

/// Circular buffer to store recent fixes for flight state analysis
#[derive(Debug, Clone)]
struct FixHistory {
    fixes: Vec<Fix>,
    max_size: usize,
}

impl FixHistory {
    fn new(max_size: usize) -> Self {
        Self {
            fixes: Vec::with_capacity(max_size),
            max_size,
        }
    }

    fn push(&mut self, fix: Fix) {
        self.fixes.push(fix);
        if self.fixes.len() > self.max_size {
            self.fixes.remove(0);
        }
    }

    fn len(&self) -> usize {
        self.fixes.len()
    }

    fn iter(&'_ self) -> std::slice::Iter<'_, Fix> {
        self.fixes.iter()
    }
}

/// Flight state for tracking aircraft transitions
#[derive(Debug, Clone, PartialEq)]
enum FlightState {
    Ground,
    TakingOff,
    Airborne,
    Landing,
}

/// Aircraft tracking information
#[derive(Debug, Clone)]
struct AircraftTracker {
    flight_state: FlightState,
    fix_history: FixHistory,
    current_flight_id: Option<uuid::Uuid>,
    last_update: DateTime<Utc>,
}

impl AircraftTracker {
    fn new() -> Self {
        Self {
            flight_state: FlightState::Ground, // Start as ground since existing APRS records are either ground or already airborne
            fix_history: FixHistory::new(10),  // Store last 10 fixes for analysis
            current_flight_id: None,
            last_update: Utc::now(),
        }
    }
}

/// Flight detection processor that extends the database fix processor with flight tracking
pub struct FlightDetectionProcessor {
    db_processor: DatabaseFixProcessor,
    flights_repo: FlightsRepository,
    aircraft_trackers: HashMap<String, AircraftTracker>,
    diesel_pool: Pool<ConnectionManager<PgConnection>>,

    // Configuration thresholds
    takeoff_speed_threshold: f32, // Minimum speed to consider takeoff (knots)
    takeoff_altitude_gain_threshold: i32, // Minimum altitude gain for takeoff (feet)
    landing_speed_threshold: f32, // Maximum speed to consider landed (knots)
    ground_altitude_variance: i32, // Maximum altitude variance on ground (feet)
    min_fixes_for_state_change: usize, // Minimum consecutive fixes needed to change state
}

impl FlightDetectionProcessor {
    pub fn new(diesel_pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self {
            db_processor: DatabaseFixProcessor::new(diesel_pool.clone()),
            flights_repo: FlightsRepository::new(diesel_pool.clone()),
            aircraft_trackers: HashMap::new(),
            diesel_pool,

            // Default thresholds - can be made configurable later
            takeoff_speed_threshold: 35.0,        // 35 knots
            takeoff_altitude_gain_threshold: 200, // 200 feet
            landing_speed_threshold: 15.0,        // 15 knots
            ground_altitude_variance: 50,         // 50 feet
            min_fixes_for_state_change: 3,        // 3 consecutive fixes
        }
    }

    /// Analyze fix history to determine if aircraft is on ground
    fn is_on_ground(&self, history: &FixHistory) -> bool {
        if history.len() < 2 {
            return true; // Assume ground if insufficient data
        }

        let fixes: Vec<_> = history.iter().collect();

        // Check speed criteria - all recent fixes should be low speed
        let low_speed = fixes
            .iter()
            .rev()
            .take(3)
            .all(|fix| fix.ground_speed_knots.unwrap_or(0.0) <= self.landing_speed_threshold);

        // Check altitude variance - should be relatively stable on ground
        if let Some(altitudes) = fixes
            .iter()
            .rev()
            .take(5)
            .map(|f| f.altitude_feet)
            .collect::<Option<Vec<_>>>()
            && altitudes.len() >= 2
        {
            let min_alt = *altitudes.iter().min().unwrap();
            let max_alt = *altitudes.iter().max().unwrap();
            let stable_altitude = (max_alt - min_alt) <= self.ground_altitude_variance;

            return low_speed && stable_altitude;
        }

        // Fallback to just speed check
        low_speed
    }

    /// Analyze fix history to detect takeoff pattern
    fn is_taking_off(&self, history: &FixHistory) -> bool {
        if history.len() < self.min_fixes_for_state_change {
            return false;
        }

        let fixes: Vec<_> = history.iter().collect();

        // Check for speed increase pattern
        let recent_fixes = fixes
            .iter()
            .rev()
            .take(self.min_fixes_for_state_change)
            .collect::<Vec<_>>();
        let speed_increasing = recent_fixes.windows(2).all(|window| {
            let current_speed = window[0].ground_speed_knots.unwrap_or(0.0);
            let prev_speed = window[1].ground_speed_knots.unwrap_or(0.0);
            current_speed > prev_speed && current_speed >= self.takeoff_speed_threshold
        });

        // Check for altitude gain pattern
        if let Some(altitudes) = recent_fixes
            .iter()
            .map(|f| f.altitude_feet)
            .collect::<Option<Vec<_>>>()
            && altitudes.len() >= 2
        {
            let altitude_gain = altitudes.first().unwrap() - altitudes.last().unwrap();
            let significant_climb = altitude_gain >= self.takeoff_altitude_gain_threshold;

            return speed_increasing && significant_climb;
        }

        // Fallback to just speed pattern
        speed_increasing
    }

    /// Analyze fix history to detect landing pattern
    fn is_landing(&self, history: &FixHistory) -> bool {
        if history.len() < self.min_fixes_for_state_change {
            return false;
        }

        let fixes: Vec<_> = history.iter().collect();

        // Check for speed decrease and altitude loss pattern
        let recent_fixes = fixes
            .iter()
            .rev()
            .take(self.min_fixes_for_state_change)
            .collect::<Vec<_>>();

        let speed_decreasing = recent_fixes.windows(2).all(|window| {
            let current_speed = window[0].ground_speed_knots.unwrap_or(0.0);
            let prev_speed = window[1].ground_speed_knots.unwrap_or(0.0);
            current_speed <= prev_speed
        });

        let final_speed_low = recent_fixes
            .first()
            .and_then(|f| f.ground_speed_knots)
            .map(|s| s <= self.landing_speed_threshold)
            .unwrap_or(false);

        // Check for altitude decrease pattern
        if let Some(altitudes) = recent_fixes
            .iter()
            .map(|f| f.altitude_feet)
            .collect::<Option<Vec<_>>>()
            && altitudes.len() >= 2
        {
            let altitude_change = altitudes.first().unwrap() - altitudes.last().unwrap();
            let descending = altitude_change <= -100; // At least 100ft descent

            return speed_decreasing && final_speed_low && descending;
        }

        // Fallback to speed pattern
        speed_decreasing && final_speed_low
    }

    /// Process flight state transitions for an aircraft
    async fn process_flight_state_transition(&mut self, device_address: &str, fix: &Fix) {
        let current_state = self
            .aircraft_trackers
            .get(device_address)
            .unwrap()
            .flight_state
            .clone();
        let should_takeoff = {
            let tracker = self.aircraft_trackers.get(device_address).unwrap();
            self.is_taking_off(&tracker.fix_history)
        };

        debug!(
            "Processing flight state for aircraft {}: current state {:?}",
            device_address, current_state
        );

        match current_state {
            FlightState::Ground => {
                if should_takeoff {
                    info!("Detected takeoff for aircraft {}", device_address);

                    // Create new flight record
                    let flight = Flight::new(device_address.to_string(), fix.timestamp);
                    let flight_id = flight.id;

                    // Save to database
                    match self.flights_repo.insert_flight(&flight).await {
                        Ok(_) => {
                            let tracker = self.aircraft_trackers.get_mut(device_address).unwrap();
                            tracker.flight_state = FlightState::TakingOff;
                            tracker.current_flight_id = Some(flight_id);
                            info!(
                                "Created flight record {} for aircraft {}",
                                flight_id, device_address
                            );
                        }
                        Err(e) => {
                            error!(
                                "Failed to create flight record for aircraft {}: {}",
                                device_address, e
                            );
                        }
                    }
                }
            }

            FlightState::TakingOff => {
                // Transition to airborne after sustained flight
                let should_become_airborne = {
                    let tracker = self.aircraft_trackers.get(device_address).unwrap();
                    tracker.fix_history.len() >= 5
                        && tracker.fix_history.iter().rev().take(3).all(|f| {
                            f.ground_speed_knots.unwrap_or(0.0) >= self.takeoff_speed_threshold
                                && f.altitude_feet.unwrap_or(0) > 300 // Above 300 feet
                        })
                };

                if should_become_airborne {
                    let tracker = self.aircraft_trackers.get_mut(device_address).unwrap();
                    tracker.flight_state = FlightState::Airborne;
                    debug!("Aircraft {} transitioned to airborne", device_address);
                }
            }

            FlightState::Airborne => {
                let should_land = {
                    let tracker = self.aircraft_trackers.get(device_address).unwrap();
                    self.is_landing(&tracker.fix_history)
                };

                if should_land {
                    let tracker = self.aircraft_trackers.get_mut(device_address).unwrap();
                    tracker.flight_state = FlightState::Landing;
                    debug!("Aircraft {} appears to be landing", device_address);
                }
            }

            FlightState::Landing => {
                let should_complete_landing = {
                    let tracker = self.aircraft_trackers.get(device_address).unwrap();
                    self.is_on_ground(&tracker.fix_history)
                };

                if should_complete_landing {
                    info!("Detected landing for aircraft {}", device_address);

                    // Get the flight ID before borrowing mutably
                    let flight_id = self
                        .aircraft_trackers
                        .get(device_address)
                        .unwrap()
                        .current_flight_id;

                    // Update flight record with landing time
                    if let Some(flight_id) = flight_id {
                        match self
                            .flights_repo
                            .update_landing_time(flight_id, fix.timestamp)
                            .await
                        {
                            Ok(true) => {
                                let tracker =
                                    self.aircraft_trackers.get_mut(device_address).unwrap();
                                tracker.flight_state = FlightState::Ground;
                                tracker.current_flight_id = None;
                                info!(
                                    "Updated flight record {} with landing time for aircraft {}",
                                    flight_id, device_address
                                );
                            }
                            Ok(false) => {
                                warn!(
                                    "Flight record {} not found when updating landing time for aircraft {}",
                                    flight_id, device_address
                                );
                                let tracker =
                                    self.aircraft_trackers.get_mut(device_address).unwrap();
                                tracker.flight_state = FlightState::Ground;
                                tracker.current_flight_id = None;
                            }
                            Err(e) => {
                                error!(
                                    "Failed to update landing time for flight {}: {}",
                                    flight_id, e
                                );
                            }
                        }
                    } else {
                        warn!(
                            "Aircraft {} landed but no current flight ID",
                            device_address
                        );
                        let tracker = self.aircraft_trackers.get_mut(device_address).unwrap();
                        tracker.flight_state = FlightState::Ground;
                    }
                }
            }
        }
    }

    /// Clean up old aircraft trackers to prevent memory leaks
    fn cleanup_old_trackers(&mut self) {
        let cutoff = Utc::now() - Duration::hours(6); // Remove trackers older than 6 hours
        self.aircraft_trackers.retain(|device_address, tracker| {
            let should_retain = tracker.last_update > cutoff;
            if !should_retain {
                debug!("Removing old tracker for aircraft {}", device_address);
            }
            should_retain
        });
    }
}

impl FixProcessor for FlightDetectionProcessor {
    fn process_fix(&self, fix: Fix, raw_message: &str) {
        // First, delegate to the database processor
        self.db_processor.process_fix(fix.clone(), raw_message);

        // Only process fixes that have device address
        if fix.device_address.is_some() {
            // Clone self for async processing
            let mut processor = self.clone();
            let device_address_hex = fix.device_address_hex();
            let fix_clone = fix.clone();

            tokio::spawn(async move {
                // Get or create aircraft tracker
                if !processor
                    .aircraft_trackers
                    .contains_key(&device_address_hex)
                {
                    processor
                        .aircraft_trackers
                        .insert(device_address_hex.clone(), AircraftTracker::new());
                }

                // Update tracker with new fix
                {
                    let tracker = processor
                        .aircraft_trackers
                        .get_mut(&device_address_hex)
                        .unwrap();
                    tracker.fix_history.push(fix_clone.clone());
                    tracker.last_update = Utc::now();
                }

                // Process flight state transitions
                processor
                    .process_flight_state_transition(&device_address_hex, &fix_clone)
                    .await;

                // Periodic cleanup (run roughly every 256th fix processing)
                // Use a simple hash-based check to avoid frequent cleanup
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                device_address_hex.hash(&mut hasher);
                if hasher.finish().is_multiple_of(256) {
                    processor.cleanup_old_trackers();
                }
            });
        }
    }
}

// Manual Clone implementation due to HashMap<String, AircraftTracker>
impl Clone for FlightDetectionProcessor {
    fn clone(&self) -> Self {
        Self {
            db_processor: DatabaseFixProcessor::new(self.diesel_pool.clone()),
            flights_repo: FlightsRepository::new(self.diesel_pool.clone()),
            aircraft_trackers: self.aircraft_trackers.clone(),
            diesel_pool: self.diesel_pool.clone(),
            takeoff_speed_threshold: self.takeoff_speed_threshold,
            takeoff_altitude_gain_threshold: self.takeoff_altitude_gain_threshold,
            landing_speed_threshold: self.landing_speed_threshold,
            ground_altitude_variance: self.ground_altitude_variance,
            min_fixes_for_state_change: self.min_fixes_for_state_change,
        }
    }
}
