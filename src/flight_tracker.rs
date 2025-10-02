use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, trace, warn};
use uuid::Uuid;

use crate::Fix;
use crate::airports_repo::AirportsRepository;
use crate::devices::AddressType;
use crate::elevation::ElevationDB;
use crate::fixes_repo::FixesRepository;
use crate::flights::Flight;
use crate::flights_repo::FlightsRepository;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};

/// Helper function to format device address with type for logging
fn format_device_address_with_type(device_address: &str, address_type: AddressType) -> String {
    match address_type {
        AddressType::Flarm => format!("FLARM-{}", device_address),
        AddressType::Ogn => format!("OGN-{}", device_address),
        AddressType::Icao => format!("ICAO-{}", device_address),
        AddressType::Unknown => device_address.to_string(),
    }
}

/// Simplified aircraft state - either idle or active
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AircraftState {
    Idle,   // Stationary or moving slowly (< 10 knots)
    Active, // Moving fast (>= 10 knots) on ground or airborne
}

/// Aircraft tracker with simplified state management
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AircraftTracker {
    state: AircraftState,
    current_flight_id: Option<Uuid>,
    last_update: DateTime<Utc>,
    last_position: Option<(f64, f64)>, // (lat, lon) for calculating speed
    last_position_time: Option<DateTime<Utc>>,
    last_fix_timestamp: Option<DateTime<Utc>>, // Track last processed fix to avoid duplicates
}

impl AircraftTracker {
    fn new(initial_state: AircraftState) -> Self {
        Self {
            state: initial_state,
            current_flight_id: None,
            last_update: Utc::now(),
            last_position: None,
            last_position_time: None,
            last_fix_timestamp: None,
        }
    }

    /// Determine if aircraft should be considered active based on fix
    fn should_be_active(&self, fix: &Fix) -> bool {
        // Check ground speed first
        if let Some(ground_speed_knots) = fix.ground_speed_knots {
            return ground_speed_knots >= 10.0;
        }

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

                return speed_knots >= 10.0;
            }
        }

        // Default to current state if we can't determine speed
        self.state == AircraftState::Active
    }

    fn update_position(&mut self, fix: &Fix) {
        self.last_position = Some((fix.latitude, fix.longitude));
        self.last_position_time = Some(fix.timestamp);
        self.last_fix_timestamp = Some(fix.timestamp);
        self.last_update = Utc::now();
    }

    /// Check if this fix is a duplicate (within 1 second of the last processed fix)
    fn is_duplicate_fix(&self, fix: &Fix) -> bool {
        if let Some(last_timestamp) = self.last_fix_timestamp {
            let time_diff = fix.timestamp.signed_duration_since(last_timestamp);
            time_diff.num_seconds().abs() < 1
        } else {
            false
        }
    }
}

/// Calculate distance between two points using Haversine formula
fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_M: f64 = 6_371_000.0;

    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_M * c
}

impl FlightTracker {
    /// Calculate altitude offset in feet between reported altitude and true MSL elevation
    /// Returns the difference (reported_altitude_ft - true_elevation_ft)
    /// Returns None if elevation lookup fails or fix has no altitude
    async fn calculate_altitude_offset_ft(&self, fix: &Fix) -> Option<i32> {
        // Get reported altitude from fix (in feet)
        let reported_altitude_ft = fix.altitude_feet?;

        let lat = fix.latitude;
        let lon = fix.longitude;

        // Run blocking elevation lookup in a separate thread
        let elevation_result = self.elevation_db.elevation_egm2008(lat, lon).await.ok()?;

        // Get true elevation at this location (in meters)
        match elevation_result {
            Some(elevation_m) => {
                // Convert elevation from meters to feet (1 meter = 3.28084 feet)
                let elevation_ft = elevation_m * 3.28084;
                // Calculate offset
                let offset = reported_altitude_ft as f64 - elevation_ft;

                info!(
                    "Altitude offset calculation: indicated={} ft, known_elevation={:.1} ft, offset={:.0} ft at ({:.6}, {:.6})",
                    reported_altitude_ft, elevation_ft, offset, lat, lon
                );

                Some(offset.round() as i32)
            }
            None => {
                // No elevation data available (e.g., ocean)
                debug!(
                    "No elevation data available for location ({}, {})",
                    fix.latitude, fix.longitude
                );
                None
            }
        }
    }
}

pub struct FlightTracker {
    flights_repo: FlightsRepository,
    airports_repo: AirportsRepository,
    fixes_repo: FixesRepository,
    elevation_db: ElevationDB,
    aircraft_trackers: Arc<RwLock<HashMap<uuid::Uuid, AircraftTracker>>>,
    state_file_path: Option<std::path::PathBuf>,
}

impl Clone for FlightTracker {
    fn clone(&self) -> Self {
        Self {
            flights_repo: self.flights_repo.clone(),
            airports_repo: self.airports_repo.clone(),
            fixes_repo: self.fixes_repo.clone(),
            elevation_db: self.elevation_db.clone(),
            aircraft_trackers: Arc::clone(&self.aircraft_trackers),
            state_file_path: self.state_file_path.clone(),
        }
    }
}

impl FlightTracker {
    pub fn new(pool: &Pool<ConnectionManager<PgConnection>>) -> Self {
        let elevation_db = ElevationDB::new().expect("Failed to initialize ElevationDB");
        Self {
            flights_repo: FlightsRepository::new(pool.clone()),
            airports_repo: AirportsRepository::new(pool.clone()),
            fixes_repo: FixesRepository::new(pool.clone()),
            elevation_db,
            aircraft_trackers: Arc::new(RwLock::new(HashMap::new())),
            state_file_path: None,
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
            fixes_repo: FixesRepository::new(pool.clone()),
            elevation_db,
            aircraft_trackers: Arc::new(RwLock::new(HashMap::new())),
            state_file_path: Some(state_path),
        }
    }

    /// Find nearest airport within 2km of given coordinates
    async fn find_nearby_airport(&self, latitude: f64, longitude: f64) -> Option<String> {
        match self
            .airports_repo
            .find_nearest_airports(latitude, longitude, 2000.0, 1) // 2km radius, 1 result
            .await
        {
            Ok(airports) if !airports.is_empty() => Some(airports[0].0.ident.clone()),
            _ => None,
        }
    }

    /// Create a new flight for aircraft already airborne (no takeoff data)
    async fn create_airborne_flight(&self, fix: &Fix) -> Result<Uuid> {
        info!("Creating airborne flight from fix: {:?}", fix);
        let mut flight = Flight::new_airborne_from_fix(fix);
        flight.device_address_type = fix.address_type;
        // No departure airport since we don't know where they took off from

        let flight_id = flight.id;

        match self.flights_repo.create_flight(flight).await {
            Ok(_) => {
                info!(
                    "Created airborne flight {} for aircraft {} (first seen at {:.6}, {:.6})",
                    flight_id,
                    format_device_address_with_type(
                        fix.device_address_hex().as_ref(),
                        fix.address_type
                    ),
                    fix.latitude,
                    fix.longitude
                );

                // Update existing fixes for this device to associate them with the new flight
                // Use a time range from 10 minutes ago to now to catch recent fixes
                let lookback_time = fix.timestamp - chrono::Duration::minutes(10);
                if let Err(e) = self
                    .fixes_repo
                    .update_flight_id_by_device_and_time(
                        fix.device_id,
                        flight_id,
                        lookback_time,
                        None, // No end time - update all fixes from lookback_time onward
                    )
                    .await
                {
                    warn!(
                        "Failed to update existing fixes with flight_id {} for aircraft {}: {}",
                        flight_id, fix.device_id, e
                    );
                }

                Ok(flight_id)
            }
            Err(e) => {
                error!(
                    "Failed to create airborne flight for aircraft {}: {}",
                    fix.device_id, e
                );
                Err(e)
            }
        }
    }

    /// Create a new flight for takeoff
    async fn create_flight(&self, fix: &Fix) -> Result<Uuid> {
        let departure_airport = self.find_nearby_airport(fix.latitude, fix.longitude).await;

        let mut flight = Flight::new_with_takeoff_from_fix(fix, fix.timestamp);
        flight.device_address_type = fix.address_type;
        flight.departure_airport = departure_airport.clone();

        // Calculate takeoff altitude offset (difference between reported altitude and true elevation)
        flight.takeoff_altitude_offset_ft = self.calculate_altitude_offset_ft(fix).await;

        let flight_id = flight.id;

        match self.flights_repo.create_flight(flight).await {
            Ok(_) => {
                info!(
                    "Created flight {} for aircraft {} (takeoff at {:.6}, {:.6}{})",
                    flight_id,
                    fix.device_id,
                    fix.latitude,
                    fix.longitude,
                    if departure_airport.is_some() {
                        format!(" from {}", departure_airport.as_ref().unwrap())
                    } else {
                        String::new()
                    }
                );

                // Update existing fixes for this device to associate them with the new flight
                // Use a time range from 10 minutes ago to now to catch recent fixes
                let lookback_time = fix.timestamp - chrono::Duration::minutes(10);
                if let Err(e) = self
                    .fixes_repo
                    .update_flight_id_by_device_and_time(
                        fix.device_id,
                        flight_id,
                        lookback_time,
                        None, // No end time - update all fixes from lookback_time onward
                    )
                    .await
                {
                    warn!(
                        "Failed to update existing fixes with flight_id {} for aircraft {}: {}",
                        flight_id, fix.device_id, e
                    );
                }

                Ok(flight_id)
            }
            Err(e) => {
                error!(
                    "Failed to create flight for aircraft {}: {}",
                    fix.device_id, e
                );
                Err(e)
            }
        }
    }

    /// Update flight with landing information
    async fn complete_flight(&self, flight_id: Uuid, fix: &Fix) -> Result<()> {
        let arrival_airport = self.find_nearby_airport(fix.latitude, fix.longitude).await;

        // Calculate landing altitude offset (difference between reported altitude and true elevation)
        let landing_altitude_offset_ft = self.calculate_altitude_offset_ft(fix).await;

        match self
            .flights_repo
            .update_flight_landing(
                flight_id,
                fix.timestamp,
                arrival_airport.clone(),
                landing_altitude_offset_ft,
            )
            .await
        {
            Ok(_) => {
                info!(
                    "Completed flight {} with landing at {:.6}, {:.6}{}",
                    flight_id,
                    fix.latitude,
                    fix.longitude,
                    if arrival_airport.is_some() {
                        format!(" at {}", arrival_airport.as_ref().unwrap())
                    } else {
                        String::new()
                    }
                );
                Ok(())
            }
            Err(e) => {
                error!("Failed to update flight {} with landing: {}", flight_id, e);
                Err(e)
            }
        }
    }

    /// Process state transition for an aircraft and return updated fix with flight_id
    async fn process_state_transition(&self, mut fix: Fix) -> Result<Fix> {
        // Determine the new state first
        let should_be_active = {
            let trackers = self.aircraft_trackers.read().await;
            match trackers.get(&fix.device_id) {
                Some(tracker) => tracker.should_be_active(&fix),
                None => {
                    // New aircraft - determine initial state
                    let ground_speed = fix.ground_speed_knots.unwrap_or(0.0);
                    ground_speed >= 10.0
                }
            }
        };

        let new_state = if should_be_active {
            AircraftState::Active
        } else {
            AircraftState::Idle
        };

        // Check if this is an existing aircraft and get the old state
        let (is_existing, old_state, current_flight_id) = {
            let trackers = self.aircraft_trackers.read().await;
            match trackers.get(&fix.device_id) {
                Some(tracker) => (true, tracker.state.clone(), tracker.current_flight_id),
                None => (false, AircraftState::Idle, None), // Default values for new aircraft
            }
        };

        if is_existing {
            // Handle existing aircraft
            match (old_state, &new_state) {
                (AircraftState::Idle, AircraftState::Active) => {
                    // Takeoff detected
                    debug!(
                        "Takeoff detected for aircraft {}",
                        format_device_address_with_type(
                            fix.device_address_hex().as_ref(),
                            fix.address_type
                        )
                    );
                    match self.create_flight(&fix).await {
                        Ok(flight_id) => {
                            fix.flight_id = Some(flight_id);
                            let mut trackers = self.aircraft_trackers.write().await;
                            if let Some(tracker) = trackers.get_mut(&fix.device_id) {
                                tracker.update_position(&fix);
                                tracker.current_flight_id = Some(flight_id);
                                tracker.state = new_state;
                            }
                        }
                        Err(e) => {
                            warn!("Failed to create flight for takeoff: {}", e);
                            // Still update position and state even if flight creation failed
                            let mut trackers = self.aircraft_trackers.write().await;
                            if let Some(tracker) = trackers.get_mut(&fix.device_id) {
                                tracker.update_position(&fix);
                                tracker.state = new_state;
                            }
                        }
                    }
                }
                (AircraftState::Active, AircraftState::Idle) => {
                    // Landing detected
                    debug!(
                        "Landing detected for aircraft {}",
                        format_device_address_with_type(
                            fix.device_address_hex().as_ref(),
                            fix.address_type
                        )
                    );
                    if let Some(flight_id) = current_flight_id {
                        // Keep the flight_id on the fix since it was part of this flight
                        fix.flight_id = Some(flight_id);
                        if let Err(e) = self.complete_flight(flight_id, &fix).await {
                            warn!("Failed to complete flight for landing: {}", e);
                        }
                    }

                    let mut trackers = self.aircraft_trackers.write().await;
                    if let Some(tracker) = trackers.get_mut(&fix.device_id) {
                        tracker.update_position(&fix);
                        tracker.current_flight_id = None;
                        tracker.state = new_state;
                    }
                }
                _ => {
                    // No state change, just update position
                    // If there's an ongoing flight, keep its flight_id
                    if let Some(flight_id) = current_flight_id {
                        fix.flight_id = Some(flight_id);
                    }
                    let mut trackers = self.aircraft_trackers.write().await;
                    if let Some(tracker) = trackers.get_mut(&fix.device_id) {
                        tracker.update_position(&fix);
                        tracker.state = new_state;
                    }
                }
            }
        } else {
            // New aircraft - only create a flight if we detect a true takeoff (idle→active)
            // For aircraft first seen in Active state, just track them without creating a flight
            // A flight will be created later when they transition from active→idle→active (true takeoff)
            let mut new_tracker = AircraftTracker::new(new_state.clone());
            new_tracker.update_position(&fix);

            if new_state == AircraftState::Active {
                debug!(
                    "New in-flight aircraft detected: {}",
                    format_device_address_with_type(
                        fix.device_address_hex().as_ref(),
                        fix.address_type
                    )
                );
                // Create a flight for aircraft already airborne, but without takeoff data
                match self.create_airborne_flight(&fix).await {
                    Ok(flight_id) => {
                        fix.flight_id = Some(flight_id);
                        new_tracker.current_flight_id = Some(flight_id);
                        info!(
                            "Created airborne flight {} for aircraft {} (no takeoff data)",
                            flight_id,
                            format_device_address_with_type(
                                fix.device_address_hex().as_ref(),
                                fix.address_type
                            )
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to create airborne flight for {}: {}",
                            fix.device_id, e
                        );
                    }
                }
            }

            let mut trackers = self.aircraft_trackers.write().await;
            trackers.insert(fix.device_id, new_tracker);
            info!(
                "Started tracking aircraft {} in {:?} state",
                fix.device_id, new_state
            );
        }

        Ok(fix)
    }

    /// Clean up old trackers for aircraft that haven't been seen recently
    async fn cleanup_old_trackers(&self) {
        let mut trackers = self.aircraft_trackers.write().await;
        let cutoff_time = Utc::now() - chrono::Duration::hours(24);

        let old_count = trackers.len();
        trackers.retain(|device_address, tracker| {
            if tracker.last_update < cutoff_time {
                debug!("Removing stale tracker for aircraft {}", device_address);
                false
            } else {
                true
            }
        });

        let removed_count = old_count - trackers.len();
        if removed_count > 0 {
            info!("Cleaned up {} stale aircraft trackers", removed_count);
        }
    }

    /// Save the current state to disk atomically
    pub async fn save_state(&self) -> Result<()> {
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

            // Replace the current trackers with the loaded state
            let mut current_trackers = self.aircraft_trackers.write().await;
            *current_trackers = trackers;

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
}

impl FlightTracker {
    /// Process a fix and return it with updated flight_id
    /// This replaces the old FixHandler::process_fix method
    pub async fn process_fix(&self, fix: Fix) -> Option<Fix> {
        // Check for duplicate fixes first (within 1 second)
        let is_duplicate = {
            let trackers_read = self.aircraft_trackers.try_read();
            match trackers_read {
                Ok(trackers) => {
                    if let Some(tracker) = trackers.get(&fix.device_id) {
                        tracker.is_duplicate_fix(&fix)
                    } else {
                        false // New aircraft, not a duplicate
                    }
                }
                Err(_) => false, // Could not get read lock, process anyway
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
        match self.process_state_transition(fix).await {
            Ok(updated_fix) => {
                // Periodically clean up old trackers (roughly every 1000 fixes)
                if rand::random::<u16>().is_multiple_of(1000) {
                    let processor = self.clone();
                    tokio::spawn(async move {
                        processor.cleanup_old_trackers().await;
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devices::AddressType;
    use chrono::Utc;

    #[test]
    fn test_haversine_distance() {
        // Test distance between two known points
        let lat1 = 40.7128; // New York
        let lon1 = -74.0060;
        let lat2 = 40.7489; // Times Square
        let lon2 = -73.9857;

        let distance = haversine_distance(lat1, lon1, lat2, lon2);
        // Should be approximately 4.5km
        assert!(distance > 4000.0 && distance < 5000.0);
    }

    #[test]
    fn test_aircraft_state_transitions() {
        let tracker = AircraftTracker::new(AircraftState::Idle);
        assert_eq!(tracker.state, AircraftState::Idle);

        // Create a fix with high ground speed
        let mut fix = Fix {
            id: uuid::Uuid::new_v4(),
            raw_packet: "TEST-1>APRS,TCPXX*:!4000.00N/07400.00W>000/000/A=001000".to_string(),
            source: "TEST".to_string(),
            destination: "APRS".to_string(),
            via: vec![],
            timestamp: Utc::now(),
            received_at: Utc::now(),
            lag: None,
            latitude: 40.0,
            longitude: -74.0,
            altitude_feet: Some(1000),
            device_address: 0x123456,
            address_type: AddressType::Icao,
            aircraft_type_ogn: None,
            flight_id: None,
            flight_number: None,
            emitter_category: None,
            registration: None,
            model: None,
            squawk: None,
            ground_speed_knots: Some(50.0), // 50 knots - should be active
            track_degrees: None,
            climb_fpm: None,
            turn_rate_rot: None,
            snr_db: None,
            bit_errors_corrected: None,
            freq_offset_khz: None,
            club_id: None,
            unparsed_data: None,
            device_id: uuid::Uuid::new_v4(),
            is_active: true, // 50 knots is active
        };

        assert!(tracker.should_be_active(&fix));

        // Test with low speed
        fix.ground_speed_knots = Some(5.0); // 5 knots - should be idle
        assert!(!tracker.should_be_active(&fix));
    }
}
