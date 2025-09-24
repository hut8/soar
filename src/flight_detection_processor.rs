use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, trace, warn};
use uuid::Uuid;

use crate::airports_repo::AirportsRepository;
use crate::devices::AddressType;
use crate::flights::Flight;
use crate::flights_repo::FlightsRepository;
use crate::{Fix, FixHandler};
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};

/// Simplified aircraft state - either idle or active
#[derive(Debug, Clone, PartialEq)]
pub enum AircraftState {
    Idle,   // Stationary or moving slowly (< 10 knots)
    Active, // Moving fast (>= 10 knots) on ground or airborne
}

/// Aircraft tracker with simplified state management
#[derive(Debug, Clone)]
struct AircraftTracker {
    state: AircraftState,
    current_flight_id: Option<Uuid>,
    last_update: DateTime<Utc>,
    last_position: Option<(f64, f64)>, // (lat, lon) for calculating speed
    last_position_time: Option<DateTime<Utc>>,
}

impl AircraftTracker {
    fn new(initial_state: AircraftState) -> Self {
        Self {
            state: initial_state,
            current_flight_id: None,
            last_update: Utc::now(),
            last_position: None,
            last_position_time: None,
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
        self.last_update = Utc::now();
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

pub struct FlightDetectionProcessor {
    flights_repo: FlightsRepository,
    airports_repo: AirportsRepository,
    aircraft_trackers: Arc<RwLock<HashMap<String, AircraftTracker>>>,
}

impl Clone for FlightDetectionProcessor {
    fn clone(&self) -> Self {
        Self {
            flights_repo: self.flights_repo.clone(),
            airports_repo: self.airports_repo.clone(),
            aircraft_trackers: Arc::clone(&self.aircraft_trackers),
        }
    }
}

impl FlightDetectionProcessor {
    pub fn new(pool: &Pool<ConnectionManager<PgConnection>>) -> Self {
        Self {
            flights_repo: FlightsRepository::new(pool.clone()),
            airports_repo: AirportsRepository::new(pool.clone()),
            aircraft_trackers: Arc::new(RwLock::new(HashMap::new())),
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

    /// Create a new flight for takeoff
    async fn create_flight(&self, device_address: &str, fix: &Fix) -> Result<Uuid> {
        let departure_airport = self.find_nearby_airport(fix.latitude, fix.longitude).await;

        let mut flight = Flight::new(device_address.to_string(), fix.timestamp);
        flight.device_address_type = fix.address_type.unwrap_or(AddressType::Unknown);
        flight.departure_airport = departure_airport.clone();

        let flight_id = flight.id;

        match self.flights_repo.create_flight(flight).await {
            Ok(_) => {
                info!(
                    "Created flight {} for aircraft {} (takeoff at {:.6}, {:.6}{})",
                    flight_id,
                    device_address,
                    fix.latitude,
                    fix.longitude,
                    if departure_airport.is_some() {
                        format!(" from {}", departure_airport.as_ref().unwrap())
                    } else {
                        String::new()
                    }
                );
                Ok(flight_id)
            }
            Err(e) => {
                error!(
                    "Failed to create flight for aircraft {}: {}",
                    device_address, e
                );
                Err(e)
            }
        }
    }

    /// Update flight with landing information
    async fn complete_flight(&self, flight_id: Uuid, fix: &Fix) -> Result<()> {
        let arrival_airport = self.find_nearby_airport(fix.latitude, fix.longitude).await;

        match self
            .flights_repo
            .update_flight_landing(flight_id, fix.timestamp, arrival_airport.clone())
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

    /// Process state transition for an aircraft
    async fn process_state_transition(&self, device_address: &str, fix: &Fix) -> Result<()> {
        // Determine the new state first
        let should_be_active = {
            let trackers = self.aircraft_trackers.read().await;
            match trackers.get(device_address) {
                Some(tracker) => tracker.should_be_active(fix),
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
            match trackers.get(device_address) {
                Some(tracker) => (true, tracker.state.clone(), tracker.current_flight_id),
                None => (false, AircraftState::Idle, None), // Default values for new aircraft
            }
        };

        if is_existing {
            // Handle existing aircraft
            match (old_state, &new_state) {
                (AircraftState::Idle, AircraftState::Active) => {
                    // Takeoff detected
                    debug!("Takeoff detected for aircraft {}", device_address);
                    match self.create_flight(device_address, fix).await {
                        Ok(flight_id) => {
                            let mut trackers = self.aircraft_trackers.write().await;
                            if let Some(tracker) = trackers.get_mut(device_address) {
                                tracker.update_position(fix);
                                tracker.current_flight_id = Some(flight_id);
                                tracker.state = new_state;
                            }
                        }
                        Err(e) => {
                            warn!("Failed to create flight for takeoff: {}", e);
                            // Still update position and state even if flight creation failed
                            let mut trackers = self.aircraft_trackers.write().await;
                            if let Some(tracker) = trackers.get_mut(device_address) {
                                tracker.update_position(fix);
                                tracker.state = new_state;
                            }
                        }
                    }
                }
                (AircraftState::Active, AircraftState::Idle) => {
                    // Landing detected
                    debug!("Landing detected for aircraft {}", device_address);
                    if let Some(flight_id) = current_flight_id
                        && let Err(e) = self.complete_flight(flight_id, fix).await
                    {
                        warn!("Failed to complete flight for landing: {}", e);
                    }

                    let mut trackers = self.aircraft_trackers.write().await;
                    if let Some(tracker) = trackers.get_mut(device_address) {
                        tracker.update_position(fix);
                        tracker.current_flight_id = None;
                        tracker.state = new_state;
                    }
                }
                _ => {
                    // No state change, just update position
                    let mut trackers = self.aircraft_trackers.write().await;
                    if let Some(tracker) = trackers.get_mut(device_address) {
                        tracker.update_position(fix);
                        tracker.state = new_state;
                    }
                }
            }
        } else {
            // New aircraft
            let mut new_tracker = AircraftTracker::new(new_state.clone());
            new_tracker.update_position(fix);

            // If aircraft is initially active (in-flight), create flight with no departure details
            if new_state == AircraftState::Active {
                debug!("New in-flight aircraft detected: {}", device_address);
                match self.create_flight(device_address, fix).await {
                    Ok(flight_id) => {
                        new_tracker.current_flight_id = Some(flight_id);
                    }
                    Err(e) => {
                        warn!(
                            "Failed to create in-flight tracking for aircraft {}: {}",
                            device_address, e
                        );
                    }
                }
            }

            let mut trackers = self.aircraft_trackers.write().await;
            trackers.insert(device_address.to_string(), new_tracker);
            info!(
                "Started tracking aircraft {} in {:?} state",
                device_address, new_state
            );
        }

        Ok(())
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
}

impl FixHandler for FlightDetectionProcessor {
    fn process_fix(&self, fix: Fix, _raw_message: &str) {
        let device_address = fix.device_address_hex();

        trace!(
            "Processing fix for aircraft {} at {:.6}, {:.6} (speed: {:?} knots)",
            device_address, fix.latitude, fix.longitude, fix.ground_speed_knots
        );

        // Clone self for async processing
        let processor = self.clone();
        let device_address_owned = device_address.clone();

        tokio::spawn(async move {
            if let Err(e) = processor
                .process_state_transition(&device_address_owned, &fix)
                .await
            {
                error!(
                    "Failed to process state transition for aircraft {}: {}",
                    device_address_owned, e
                );
            }
        });

        // Periodically clean up old trackers (roughly every 1000 fixes)
        if rand::random::<u16>() % 1000 == 0 {
            let processor = self.clone();
            tokio::spawn(async move {
                processor.cleanup_old_trackers().await;
            });
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
            source: "TEST".to_string(),
            destination: "APRS".to_string(),
            via: vec![],
            timestamp: Utc::now(),
            received_at: Utc::now(),
            lag: None,
            latitude: 40.0,
            longitude: -74.0,
            altitude_feet: Some(1000),
            device_address: Some(0x123456),
            address_type: Some(AddressType::Icao),
            aircraft_type: None,
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
            club_name: None,
            unparsed_data: None,
        };

        assert!(tracker.should_be_active(&fix));

        // Test with low speed
        fix.ground_speed_knots = Some(5.0); // 5 knots - should be idle
        assert!(!tracker.should_be_active(&fix));
    }
}
