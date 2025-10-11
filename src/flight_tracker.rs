use anyhow::Result;
use chrono::{DateTime, Utc};
use metrics::histogram;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, trace, warn};
use uuid::Uuid;

use crate::Fix;
use crate::airports_repo::AirportsRepository;
use crate::devices::AddressType;
use crate::elevation::ElevationDB;
use crate::fixes_repo::FixesRepository;
use crate::flights::Flight;
use crate::flights_repo::FlightsRepository;
use crate::runways_repo::RunwaysRepository;
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
    Idle,   // Stationary or moving slowly (< 20 knots)
    Active, // Moving fast (>= 20 knots) on ground or airborne
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
    towed_by_device_id: Option<Uuid>, // For gliders: device_id of towplane (if being towed)
    tow_released: bool,               // For gliders: whether tow release has been detected/recorded
    takeoff_runway_inferred: Option<bool>, // Track whether takeoff runway was inferred (for determining runways_inferred field)
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
            towed_by_device_id: None,
            tow_released: false,
            takeoff_runway_inferred: None,
        }
    }

    /// Determine if aircraft should be considered active based on fix
    fn should_be_active(&self, fix: &Fix) -> bool {
        // Check ground speed first
        let speed_indicates_active = if let Some(ground_speed_knots) = fix.ground_speed_knots {
            ground_speed_knots >= 20.0
        } else {
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

                    speed_knots >= 20.0
                } else {
                    // Can't calculate speed, use current state
                    self.state == AircraftState::Active
                }
            } else {
                // No previous position, use current state
                self.state == AircraftState::Active
            }
        };

        // If speed indicates active, aircraft is active
        if speed_indicates_active {
            return true;
        }

        // Speed indicates not active (potential landing)
        // But don't register landing if AGL altitude is >= 250 feet
        // Only land if altitude is unavailable OR altitude is < 250 feet AGL
        if let Some(altitude_agl) = fix.altitude_agl
            && altitude_agl >= 250
        {
            // Still too high to land - remain active
            return true;
        }

        // Either altitude is unavailable, or altitude is < 250 feet
        // Speed is low, so aircraft should be idle (landing)
        false
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

/// Calculate the angular difference between two headings in degrees
/// Returns the smallest angle between the two headings (0-180 degrees)
fn angular_difference(angle1: f64, angle2: f64) -> f64 {
    let diff = (angle1 - angle2).abs() % 360.0;
    if diff > 180.0 { 360.0 - diff } else { diff }
}

impl FlightTracker {
    /// Calculate altitude offset in feet between reported altitude and true MSL elevation
    /// Returns the difference (reported_altitude_ft - true_elevation_ft)
    /// Returns None if elevation lookup fails or fix has no altitude
    #[instrument(skip(self), fields(
        aircraft = %format_device_address_with_type(&fix.device_address_hex(), fix.address_type)
    ))]
    async fn calculate_altitude_offset_ft(&self, fix: &Fix) -> Option<i32> {
        // Get reported altitude from fix (in feet)
        let reported_altitude_ft = fix.altitude_msl_feet?;

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

                // Log warning if offset is too large (> 250 feet)
                if offset.abs() > 250.0 {
                    warn!(
                        "Large altitude offset detected: {:.0} ft (indicated={} ft, known_elevation={:.1} ft) at ({:.6}, {:.6})",
                        offset, reported_altitude_ft, elevation_ft, lat, lon
                    );
                }

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

    async fn calculate_altitude_agl(&self, fix: &Fix) -> Option<i32> {
        // Get reported altitude from fix (in feet)
        let reported_altitude_ft = fix.altitude_msl_feet?;

        let lat = fix.latitude;
        let lon = fix.longitude;

        // Run blocking elevation lookup in a separate thread
        let elevation_result = self.elevation_db.elevation_egm2008(lat, lon).await.ok()?;

        // Get true elevation at this location (in meters)
        match elevation_result {
            Some(elevation_m) => {
                // Convert elevation from meters to feet (1 meter = 3.28084 feet)
                let elevation_ft = elevation_m * 3.28084;
                // Calculate AGL (Above Ground Level)
                let agl = reported_altitude_ft as f64 - elevation_ft;

                Some(agl.round() as i32)
            }
            None => {
                // No elevation data available (e.g., ocean)
                None
            }
        }
    }

    /// Calculate altitude AGL and update the fix in the database asynchronously
    /// This method is designed to be called in a background task after the fix is inserted
    pub async fn calculate_and_update_agl_async(
        &self,
        fix_id: uuid::Uuid,
        fix: &Fix,
        fixes_repo: crate::fixes_repo::FixesRepository,
    ) {
        match self.calculate_altitude_agl(fix).await {
            Some(agl) => {
                if let Err(e) = fixes_repo.update_altitude_agl(fix_id, agl).await {
                    debug!("Failed to update altitude_agl for fix {}: {}", fix_id, e);
                }
            }
            None => {
                trace!(
                    "No altitude or elevation data for fix {}, altitude_agl remains NULL",
                    fix_id
                );
            }
        }
    }
}

pub struct FlightTracker {
    flights_repo: FlightsRepository,
    airports_repo: AirportsRepository,
    runways_repo: RunwaysRepository,
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
            runways_repo: self.runways_repo.clone(),
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
            runways_repo: RunwaysRepository::new(pool.clone()),
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
            runways_repo: RunwaysRepository::new(pool.clone()),
            fixes_repo: FixesRepository::new(pool.clone()),
            elevation_db,
            aircraft_trackers: Arc::new(RwLock::new(HashMap::new())),
            state_file_path: Some(state_path),
        }
    }

    /// Find nearest airport within 2km of given coordinates
    /// Returns the airport ID (not the identifier string)
    async fn find_nearby_airport(&self, latitude: f64, longitude: f64) -> Option<i32> {
        match self
            .airports_repo
            .find_nearest_airports(latitude, longitude, 2000.0, 1) // 2km radius, 1 result
            .await
        {
            Ok(airports) if !airports.is_empty() => Some(airports[0].0.id),
            _ => None,
        }
    }

    /// Convert heading in degrees to runway identifier
    /// e.g., 230° -> "23", 47° -> "05", 354° -> "35"
    fn heading_to_runway_identifier(heading: f64) -> String {
        // Round to nearest 10 degrees and divide by 10
        let runway_number = ((heading / 10.0).round() as i32) % 36;
        // Handle 360° -> 36 -> 0
        let runway_number = if runway_number == 0 {
            36
        } else {
            runway_number
        };
        // Format with leading zero
        format!("{:02}", runway_number)
    }

    /// Check if an aircraft type uses runways
    fn uses_runways(aircraft_type: &crate::ogn_aprs_aircraft::AircraftType) -> bool {
        use crate::ogn_aprs_aircraft::AircraftType;
        match aircraft_type {
            // Aircraft that use runways
            AircraftType::Glider => true,
            AircraftType::TowTug => true,
            AircraftType::RecipEngine => true,
            AircraftType::JetTurboprop => true,
            AircraftType::DropPlane => true,
            // Aircraft that don't use runways
            AircraftType::Paraglider => false,
            AircraftType::HangGlider => false,
            AircraftType::HelicopterGyro => false,
            AircraftType::Balloon => false,
            AircraftType::Airship => false,
            AircraftType::SkydiverParachute => false, // The parachute itself, not the plane
            AircraftType::Uav => false,
            AircraftType::StaticObstacle => false,
            AircraftType::Reserved => false,
            AircraftType::Unknown => true, // Default to true for unknown types
        }
    }

    /// Determine runway identifier based on aircraft course during takeoff/landing
    /// First tries to match against nearby runways with coordinates from the database.
    /// If no runways found or no good match, infers runway from aircraft heading.
    /// Loads fixes from 20 seconds before to 20 seconds after the event time
    ///
    /// Returns a tuple of (runway_identifier, was_inferred)
    /// - runway_identifier: e.g., "14" or "32"
    /// - was_inferred: true if inferred from heading, false if looked up in database
    ///
    /// If airport_ref is provided, only searches for runways at that specific airport
    async fn determine_runway_identifier(
        &self,
        device_id: &Uuid,
        event_time: DateTime<Utc>,
        latitude: f64,
        longitude: f64,
        airport_ref: Option<i32>,
    ) -> Option<(String, bool)> {
        // Get fixes from 20 seconds before to 20 seconds after the event
        let start_time = event_time - chrono::Duration::seconds(20);
        let end_time = event_time + chrono::Duration::seconds(20);

        let fixes = match self
            .fixes_repo
            .get_fixes_for_aircraft_with_time_range(device_id, start_time, end_time, None)
            .await
        {
            Ok(f) if !f.is_empty() => {
                debug!(
                    "Found {} fixes for device {} between {} and {}",
                    f.len(),
                    device_id,
                    start_time,
                    end_time
                );
                f
            }
            Ok(_) => {
                debug!(
                    "No fixes found for device {} between {} and {}",
                    device_id, start_time, end_time
                );
                return None;
            }
            Err(e) => {
                warn!(
                    "Error loading fixes for device {} during runway detection: {}",
                    device_id, e
                );
                return None;
            }
        };

        // Check if this aircraft type uses runways
        // Get aircraft type from the first fix (all fixes should have the same type for a device)
        if let Some(first_fix) = fixes.first()
            && let Some(aircraft_type) = first_fix.aircraft_type_ogn
            && !Self::uses_runways(&aircraft_type)
        {
            debug!(
                "Aircraft type {:?} does not use runways, skipping runway detection for device {}",
                aircraft_type, device_id
            );
            return None;
        }

        // Calculate average course from fixes that have track_degrees
        let courses: Vec<f32> = fixes.iter().filter_map(|fix| fix.track_degrees).collect();

        if courses.is_empty() {
            debug!(
                "No track_degrees data in fixes for device {}, cannot determine runway",
                device_id
            );
            return None;
        }

        let avg_course = courses.iter().sum::<f32>() as f64 / courses.len() as f64;
        debug!(
            "Calculated average course {:.1}° from {} fixes for device {}",
            avg_course,
            courses.len(),
            device_id
        );

        // Try to find nearby runways (within 2km)
        // If we have an airport_ref, use it to filter runways to that airport only
        let nearby_runways = match self
            .runways_repo
            .find_nearest_runway_endpoints(latitude, longitude, 2000.0, 10, airport_ref)
            .await
        {
            Ok(runways) if !runways.is_empty() => {
                debug!(
                    "Found {} nearby runway endpoints for device {} at ({}, {})",
                    runways.len(),
                    device_id,
                    latitude,
                    longitude
                );
                Some(runways)
            }
            Ok(_) => {
                debug!(
                    "No nearby runways found for device {} at ({}, {}), will infer from heading",
                    device_id, latitude, longitude
                );
                None
            }
            Err(e) => {
                warn!(
                    "Error finding nearby runways for device {}: {}, will infer from heading",
                    device_id, e
                );
                None
            }
        };

        // If we have nearby runways, try to match against them
        if let Some(runways) = nearby_runways {
            let mut best_match: Option<(String, f64)> = None;

            for (runway, _, endpoint_type) in runways {
                // Determine which end to check based on which is closer
                let (ident, heading) = match endpoint_type.as_str() {
                    "low_end" => {
                        // Aircraft is near low end, check both ends to see which direction they're traveling
                        if let (Some(le_heading), Some(he_heading)) =
                            (runway.le_heading_degt, runway.he_heading_degt)
                        {
                            // Calculate angular difference for both directions
                            let le_diff = angular_difference(avg_course, le_heading);
                            let he_diff = angular_difference(avg_course, he_heading);

                            if le_diff < he_diff {
                                (runway.le_ident.clone(), le_heading)
                            } else {
                                (runway.he_ident.clone(), he_heading)
                            }
                        } else {
                            continue;
                        }
                    }
                    "high_end" => {
                        // Aircraft is near high end, check both ends to see which direction they're traveling
                        if let (Some(le_heading), Some(he_heading)) =
                            (runway.le_heading_degt, runway.he_heading_degt)
                        {
                            let le_diff = angular_difference(avg_course, le_heading);
                            let he_diff = angular_difference(avg_course, he_heading);

                            if he_diff < le_diff {
                                (runway.he_ident.clone(), he_heading)
                            } else {
                                (runway.le_ident.clone(), le_heading)
                            }
                        } else {
                            continue;
                        }
                    }
                    _ => continue,
                };

                if let Some(ident_str) = ident {
                    let heading_diff = angular_difference(avg_course, heading);

                    // Update best match if this is closer (or first match)
                    match best_match {
                        None => best_match = Some((ident_str, heading_diff)),
                        Some((_, current_diff)) if heading_diff < current_diff => {
                            best_match = Some((ident_str, heading_diff));
                        }
                        _ => {}
                    }
                }
            }

            // If we found a good match (within 30 degrees), use it
            if let Some((ident, diff)) = best_match {
                if diff < 30.0 {
                    debug!(
                        "Matched runway {} from database for device {} (heading diff: {:.1}°)",
                        ident, device_id, diff
                    );
                    return Some((ident, false)); // false = from database, not inferred
                } else {
                    debug!(
                        "Best runway match {} has large heading diff ({:.1}°), will infer from heading instead",
                        ident, diff
                    );
                }
            }
        }

        // Fallback: Infer runway from heading
        let inferred_runway = Self::heading_to_runway_identifier(avg_course);
        debug!(
            "Inferred runway {} from heading {:.1}° for device {}",
            inferred_runway, avg_course, device_id
        );
        Some((inferred_runway, true)) // true = inferred from heading
    }

    /// Detect if a glider taking off is being towed by a nearby towplane
    /// Returns the towplane's (device_id, flight_id, current_altitude) if found
    async fn detect_towing_at_takeoff(
        &self,
        glider_device_id: &Uuid,
        glider_fix: &Fix,
    ) -> Option<(Uuid, Uuid, i32)> {
        // Only check for towing if this is a glider
        use crate::ogn_aprs_aircraft::AircraftType;
        if glider_fix.aircraft_type_ogn != Some(AircraftType::Glider) {
            return None;
        }

        // Get all currently active aircraft trackers
        let active_flights = {
            let trackers = self.aircraft_trackers.read().await;
            trackers
                .iter()
                .filter_map(|(device_id, tracker)| {
                    // Skip ourselves and aircraft without active flights
                    if device_id == glider_device_id || tracker.current_flight_id.is_none() {
                        return None;
                    }
                    // Only consider aircraft that are active (flying)
                    if tracker.state != AircraftState::Active {
                        return None;
                    }
                    Some((*device_id, tracker.current_flight_id.unwrap()))
                })
                .collect::<Vec<_>>()
        };

        if active_flights.is_empty() {
            return None;
        }

        // Get recent fixes for potential towplanes (within last 30 seconds)
        let time_window_start = glider_fix.timestamp - chrono::Duration::seconds(30);

        for (towplane_device_id, towplane_flight_id) in active_flights {
            // Get the most recent fix for this potential towplane
            match self
                .fixes_repo
                .get_latest_fix_for_device(towplane_device_id, time_window_start)
                .await
            {
                Ok(Some(towplane_fix)) => {
                    // Check if aircraft type suggests it's a towplane
                    let is_likely_towplane = match towplane_fix.aircraft_type_ogn {
                        Some(AircraftType::TowTug) => true,
                        Some(AircraftType::RecipEngine) => true,
                        Some(AircraftType::JetTurboprop) => false,
                        Some(AircraftType::Glider) => false, // Gliders don't tow gliders
                        _ => true,                           // Unknown types could be towplanes
                    };

                    if !is_likely_towplane {
                        continue;
                    }

                    // Calculate distance between glider and potential towplane
                    let distance_meters = haversine_distance(
                        glider_fix.latitude,
                        glider_fix.longitude,
                        towplane_fix.latitude,
                        towplane_fix.longitude,
                    );

                    // Check if they're close enough to be towing (within 200 meters / ~650 feet)
                    if distance_meters <= 200.0 {
                        // Check altitude difference (should be similar, within 200 feet)
                        if let (Some(glider_alt), Some(towplane_alt)) =
                            (glider_fix.altitude_msl_feet, towplane_fix.altitude_msl_feet)
                        {
                            let altitude_diff = (glider_alt - towplane_alt).abs();
                            if altitude_diff <= 200 {
                                info!(
                                    "Detected towing: glider {} is being towed by towplane {} (distance: {:.0}m, alt diff: {}ft)",
                                    glider_device_id,
                                    towplane_device_id,
                                    distance_meters,
                                    altitude_diff
                                );
                                return Some((
                                    towplane_device_id,
                                    towplane_flight_id,
                                    towplane_alt,
                                ));
                            }
                        }
                    }
                }
                Ok(None) => continue,
                Err(e) => {
                    debug!(
                        "Failed to get latest fix for potential towplane {}: {}",
                        towplane_device_id, e
                    );
                    continue;
                }
            }
        }

        None
    }

    /// Check if a glider flight has been released from tow
    /// This is called periodically during active flight to detect separation
    async fn check_tow_release(
        &self,
        glider_device_id: &Uuid,
        _glider_flight_id: &Uuid,
        glider_fix: &Fix,
        towplane_device_id: &Uuid,
    ) -> bool {
        // Get recent fix for towplane (within last 10 seconds)
        let time_window = glider_fix.timestamp - chrono::Duration::seconds(10);

        match self
            .fixes_repo
            .get_latest_fix_for_device(*towplane_device_id, time_window)
            .await
        {
            Ok(Some(towplane_fix)) => {
                // Calculate horizontal distance
                let distance_meters = haversine_distance(
                    glider_fix.latitude,
                    glider_fix.longitude,
                    towplane_fix.latitude,
                    towplane_fix.longitude,
                );

                // Calculate 3D distance if we have altitudes
                let separation_feet = if let (Some(glider_alt), Some(towplane_alt)) =
                    (glider_fix.altitude_msl_feet, towplane_fix.altitude_msl_feet)
                {
                    let horizontal_feet = distance_meters * 3.28084; // meters to feet
                    let vertical_feet = (glider_alt - towplane_alt).abs() as f64;
                    (horizontal_feet.powi(2) + vertical_feet.powi(2)).sqrt()
                } else {
                    distance_meters * 3.28084 // Just horizontal distance in feet
                };

                // Release detected if separation > 500 feet
                if separation_feet > 500.0 {
                    info!(
                        "Tow release detected for glider {}: separated {:.0} feet from towplane {}",
                        glider_device_id, separation_feet, towplane_device_id
                    );
                    return true;
                }

                // Also check for diverging headings (one or both turned significantly)
                if let (Some(glider_track), Some(towplane_track)) =
                    (glider_fix.track_degrees, towplane_fix.track_degrees)
                {
                    let heading_diff =
                        angular_difference(glider_track as f64, towplane_track as f64);
                    if heading_diff > 45.0 && distance_meters > 100.0 {
                        info!(
                            "Tow release detected for glider {}: diverged {:.0}° from towplane {} (distance: {:.0}m)",
                            glider_device_id, heading_diff, towplane_device_id, distance_meters
                        );
                        return true;
                    }
                }

                false
            }
            Ok(None) => {
                // No recent fix from towplane - might have landed or lost signal
                // Consider this a release if we haven't seen them for 30+ seconds
                warn!(
                    "Lost contact with towplane {} for glider {} - assuming release",
                    towplane_device_id, glider_device_id
                );
                true
            }
            Err(e) => {
                debug!(
                    "Error checking tow release for glider {}: {}",
                    glider_device_id, e
                );
                false
            }
        }
    }

    /// Record tow release in the database
    async fn record_tow_release(&self, glider_flight_id: &Uuid, release_fix: &Fix) -> Result<()> {
        if let Some(altitude_ft) = release_fix.altitude_msl_feet {
            info!(
                "Recording tow release for flight {} at {}ft MSL",
                glider_flight_id, altitude_ft
            );

            self.flights_repo
                .update_tow_release(*glider_flight_id, altitude_ft, release_fix.timestamp)
                .await?;
        }
        Ok(())
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
        let departure_airport_id = self.find_nearby_airport(fix.latitude, fix.longitude).await;

        // Determine takeoff runway and whether it was inferred
        // Pass the departure airport to optimize runway search
        let takeoff_runway_info = self
            .determine_runway_identifier(
                &fix.device_id,
                fix.timestamp,
                fix.latitude,
                fix.longitude,
                departure_airport_id,
            )
            .await;

        let (takeoff_runway, takeoff_was_inferred) = match takeoff_runway_info {
            Some((runway, was_inferred)) => (Some(runway), Some(was_inferred)),
            None => (None, None),
        };

        let mut flight = Flight::new_with_takeoff_from_fix(fix, fix.timestamp);
        flight.device_address_type = fix.address_type;
        flight.departure_airport_id = departure_airport_id;
        flight.takeoff_runway_ident = takeoff_runway.clone();

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
                    if departure_airport_id.is_some() {
                        format!(" from airport ID {}", departure_airport_id.unwrap())
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

                // Store takeoff runway source in tracker for later use when landing
                {
                    let mut trackers = self.aircraft_trackers.write().await;
                    if let Some(tracker) = trackers.get_mut(&fix.device_id) {
                        tracker.takeoff_runway_inferred = takeoff_was_inferred;
                    }
                }

                // Check if this is a glider being towed
                if let Some((towplane_device_id, towplane_flight_id, _)) =
                    self.detect_towing_at_takeoff(&fix.device_id, fix).await
                {
                    // Update the flight with towing information
                    if let Err(e) = self
                        .flights_repo
                        .update_towing_info(flight_id, towplane_device_id, towplane_flight_id)
                        .await
                    {
                        warn!(
                            "Failed to update towing info for flight {}: {}",
                            flight_id, e
                        );
                    } else {
                        // Update tracker to remember we're being towed
                        let mut trackers = self.aircraft_trackers.write().await;
                        if let Some(tracker) = trackers.get_mut(&fix.device_id) {
                            tracker.towed_by_device_id = Some(towplane_device_id);
                            tracker.tow_released = false;
                        }
                    }
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
        let arrival_airport_id = self.find_nearby_airport(fix.latitude, fix.longitude).await;

        // Determine landing runway and whether it was inferred
        // Pass the arrival airport to optimize runway search
        let landing_runway_info = self
            .determine_runway_identifier(
                &fix.device_id,
                fix.timestamp,
                fix.latitude,
                fix.longitude,
                arrival_airport_id,
            )
            .await;

        let (landing_runway, landing_was_inferred) = match landing_runway_info {
            Some((runway, was_inferred)) => (Some(runway), Some(was_inferred)),
            None => (None, None),
        };

        // Get the takeoff runway source from the tracker
        let takeoff_runway_inferred = {
            let trackers = self.aircraft_trackers.read().await;
            trackers
                .get(&fix.device_id)
                .and_then(|tracker| tracker.takeoff_runway_inferred)
        };

        // Calculate landing altitude offset (difference between reported altitude and true elevation)
        let landing_altitude_offset_ft = self.calculate_altitude_offset_ft(fix).await;

        // Determine runways_inferred based on takeoff and landing runway sources
        // Logic:
        // - NULL if both takeoff and landing runways are null
        // - true if both were inferred from heading
        // - false if both were looked up in database
        // - NULL if mixed sources (one inferred, one from database, or one is unknown)
        let runways_inferred = match (takeoff_runway_inferred, landing_was_inferred) {
            (Some(true), Some(true)) => Some(true),    // Both inferred
            (Some(false), Some(false)) => Some(false), // Both from database
            _ => None,                                 // Mixed, unknown, or one/both are null
        };

        // Fetch the flight to compute distance metrics
        let flight = match self.flights_repo.get_flight_by_id(flight_id).await? {
            Some(f) => f,
            None => {
                error!("Flight {} not found when completing", flight_id);
                return Err(anyhow::anyhow!("Flight not found"));
            }
        };

        // Check if this is a spurious flight (too short or no altitude variation)
        if let Some(takeoff_time) = flight.takeoff_time {
            let duration_seconds = (fix.timestamp - takeoff_time).num_seconds();

            // Get all fixes for this flight to check altitude range
            let flight_fixes = self
                .fixes_repo
                .get_fixes_for_flight(flight_id, None)
                .await?;

            let altitude_range = if !flight_fixes.is_empty() {
                let altitudes: Vec<i32> = flight_fixes
                    .iter()
                    .filter_map(|f| f.altitude_msl_feet)
                    .collect();

                if altitudes.is_empty() {
                    None
                } else {
                    let max_alt = altitudes.iter().max().unwrap();
                    let min_alt = altitudes.iter().min().unwrap();
                    Some(max_alt - min_alt)
                }
            } else {
                None
            };

            // Check max AGL altitude if elevation data is available
            let max_agl_altitude = if !flight_fixes.is_empty() {
                flight_fixes.iter().filter_map(|f| f.altitude_agl).max()
            } else {
                None
            };

            // Check if flight is spurious:
            // - Duration < 60 seconds OR
            // - Altitude range < 50 feet OR
            // - If elevation data is available, max AGL < 100 feet
            let is_spurious = duration_seconds < 60
                || altitude_range.map(|range| range < 50).unwrap_or(false)
                || max_agl_altitude.map(|agl| agl < 100).unwrap_or(false);

            if is_spurious {
                warn!(
                    "Detected spurious flight {}: duration={}s, altitude_range={:?}ft, max_agl={:?}ft. Deleting flight. Fix was {:?}",
                    flight_id, duration_seconds, altitude_range, max_agl_altitude, fix
                );

                // Clear flight_id from all associated fixes
                match self.fixes_repo.clear_flight_id(flight_id).await {
                    Ok(count) => {
                        info!("Cleared flight_id from {} fixes", count);
                    }
                    Err(e) => {
                        error!("Failed to clear flight_id from fixes: {}", e);
                    }
                }

                // Delete the flight
                match self.flights_repo.delete_flight(flight_id).await {
                    Ok(true) => {
                        info!("Deleted spurious flight {}", flight_id);
                        return Ok(());
                    }
                    Ok(false) => {
                        warn!("Flight {} was already deleted", flight_id);
                        return Ok(());
                    }
                    Err(e) => {
                        error!("Failed to delete spurious flight {}: {}", flight_id, e);
                        return Err(e);
                    }
                }
            }
        }

        // Calculate total distance flown
        let total_distance_meters = match flight.total_distance(&self.fixes_repo).await {
            Ok(dist) => dist,
            Err(e) => {
                warn!(
                    "Failed to calculate total distance for flight {}: {}",
                    flight_id, e
                );
                None
            }
        };

        // Calculate maximum displacement (only for local flights)
        let maximum_displacement_meters = match flight
            .maximum_displacement(&self.fixes_repo, &self.airports_repo)
            .await
        {
            Ok(disp) => disp,
            Err(e) => {
                warn!(
                    "Failed to calculate maximum displacement for flight {}: {}",
                    flight_id, e
                );
                None
            }
        };

        match self
            .flights_repo
            .update_flight_landing(
                flight_id,
                fix.timestamp,
                arrival_airport_id,
                landing_altitude_offset_ft,
                landing_runway.clone(),
                total_distance_meters,
                maximum_displacement_meters,
                runways_inferred,
            )
            .await
        {
            Ok(_) => {
                info!(
                    "Completed flight {} with landing at {:.6}, {:.6}{}",
                    flight_id,
                    fix.latitude,
                    fix.longitude,
                    if arrival_airport_id.is_some() {
                        format!(" at airport ID {}", arrival_airport_id.unwrap())
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
                    ground_speed >= 20.0
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
                    // Takeoff detected - update state FIRST to prevent race condition
                    debug!(
                        "Takeoff detected for aircraft {}",
                        format_device_address_with_type(
                            fix.device_address_hex().as_ref(),
                            fix.address_type
                        )
                    );

                    // Check altitude offset to validate this is a real takeoff
                    // If offset > 250 ft, the altitude data is likely unreliable
                    let altitude_offset = self.calculate_altitude_offset_ft(&fix).await;
                    let skip_flight_creation = if let Some(offset) = altitude_offset {
                        if offset.abs() > 250 {
                            warn!(
                                "Skipping flight creation for aircraft {} due to large altitude offset ({} ft) - altitude data may be unreliable",
                                format_device_address_with_type(
                                    fix.device_address_hex().as_ref(),
                                    fix.address_type
                                ),
                                offset
                            );
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    // Update tracker state immediately to prevent duplicate flight creation
                    let mut trackers = self.aircraft_trackers.write().await;
                    if let Some(tracker) = trackers.get_mut(&fix.device_id) {
                        tracker.update_position(&fix);
                        tracker.state = new_state.clone();
                    }
                    drop(trackers); // Release lock immediately

                    // Only create flight if altitude data is reliable
                    if !skip_flight_creation {
                        // Create flight in background (similar to landing)
                        let tracker_clone = self.clone();
                        let takeoff_fix = fix.clone();
                        tokio::spawn(async move {
                            match tracker_clone.create_flight(&takeoff_fix).await {
                                Ok(flight_id) => {
                                    // Update tracker with the flight_id
                                    let mut trackers =
                                        tracker_clone.aircraft_trackers.write().await;
                                    if let Some(tracker) = trackers.get_mut(&takeoff_fix.device_id)
                                    {
                                        tracker.current_flight_id = Some(flight_id);
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to create flight for takeoff: {}", e);
                                }
                            }
                        });
                    }

                    // Set flight_id on the fix (it will be set later by the background task)
                    // For now, we don't have it yet, so leave it as None
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

                    // Update tracker state FIRST to prevent race condition
                    let mut trackers = self.aircraft_trackers.write().await;
                    if let Some(tracker) = trackers.get_mut(&fix.device_id) {
                        tracker.update_position(&fix);
                        tracker.current_flight_id = None;
                        tracker.state = new_state;
                        // Reset towing state for next flight
                        tracker.towed_by_device_id = None;
                        tracker.tow_released = false;
                    }
                    drop(trackers); // Release lock immediately

                    // Complete flight in background if there was an active flight
                    if let Some(flight_id) = current_flight_id {
                        // Keep the flight_id on the fix since it was part of this flight
                        fix.flight_id = Some(flight_id);

                        // Spawn background task to complete flight (don't await)
                        let tracker = self.clone();
                        let landing_fix = fix.clone();
                        tokio::spawn(async move {
                            if let Err(e) = tracker.complete_flight(flight_id, &landing_fix).await {
                                warn!("Failed to complete flight for landing: {}", e);
                            }
                        });
                    }
                }
                _ => {
                    // No state change, just update position
                    // If there's an ongoing flight, keep its flight_id
                    if let Some(flight_id) = current_flight_id {
                        fix.flight_id = Some(flight_id);

                        // Check if this is a glider being towed that hasn't been released yet
                        let (is_being_towed, towplane_id, already_released) = {
                            let trackers = self.aircraft_trackers.read().await;
                            if let Some(tracker) = trackers.get(&fix.device_id) {
                                (
                                    tracker.towed_by_device_id.is_some(),
                                    tracker.towed_by_device_id,
                                    tracker.tow_released,
                                )
                            } else {
                                (false, None, false)
                            }
                        };

                        if is_being_towed
                            && !already_released
                            && let Some(towplane_device_id) = towplane_id
                        {
                            // Check for tow release
                            if self
                                .check_tow_release(
                                    &fix.device_id,
                                    &flight_id,
                                    &fix,
                                    &towplane_device_id,
                                )
                                .await
                            {
                                // Record the release
                                if let Err(e) = self.record_tow_release(&flight_id, &fix).await {
                                    warn!(
                                        "Failed to record tow release for flight {}: {}",
                                        flight_id, e
                                    );
                                } else {
                                    // Mark as released in tracker
                                    let mut trackers = self.aircraft_trackers.write().await;
                                    if let Some(tracker) = trackers.get_mut(&fix.device_id) {
                                        tracker.tow_released = true;
                                    }
                                }
                            }
                        }
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

            // Insert tracker FIRST to prevent race condition with duplicate flight creation
            let mut trackers = self.aircraft_trackers.write().await;
            trackers.insert(fix.device_id, new_tracker);
            info!(
                "Started tracking aircraft {} in {:?} state",
                fix.device_id, new_state
            );
            drop(trackers); // Release lock immediately

            if new_state == AircraftState::Active {
                debug!(
                    "New in-flight aircraft detected: {}",
                    format_device_address_with_type(
                        fix.device_address_hex().as_ref(),
                        fix.address_type
                    )
                );
                // Create flight in background to avoid blocking and prevent race condition
                let tracker_clone = self.clone();
                let airborne_fix = fix.clone();
                tokio::spawn(async move {
                    match tracker_clone.create_airborne_flight(&airborne_fix).await {
                        Ok(flight_id) => {
                            // Update tracker with the flight_id
                            let mut trackers = tracker_clone.aircraft_trackers.write().await;
                            if let Some(tracker) = trackers.get_mut(&airborne_fix.device_id) {
                                tracker.current_flight_id = Some(flight_id);
                            }
                            info!(
                                "Created airborne flight {} for aircraft {} (no takeoff data)",
                                flight_id,
                                format_device_address_with_type(
                                    airborne_fix.device_address_hex().as_ref(),
                                    airborne_fix.address_type
                                )
                            );
                        }
                        Err(e) => {
                            warn!(
                                "Failed to create airborne flight for {}: {}",
                                airborne_fix.device_id, e
                            );
                        }
                    }
                });
                // Note: flight_id will be set on subsequent fixes for this aircraft
            }
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

            // Record metric for state persistence duration
            histogram!("flight_tracker_save_duration_seconds")
                .record(start.elapsed().as_secs_f64());
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
            altitude_msl_feet: Some(1000),
            altitude_agl: None,
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
            gnss_horizontal_resolution: None,
            gnss_vertical_resolution: None,
            unparsed_data: None,
            device_id: uuid::Uuid::new_v4(),
            is_active: true, // 50 knots is active
            receiver_id: None,
            aprs_message_id: None,
        };

        assert!(tracker.should_be_active(&fix));

        // Test with low speed
        fix.ground_speed_knots = Some(15.0); // 15 knots - should be idle (below 20 knot threshold)
        assert!(!tracker.should_be_active(&fix));
    }
}
