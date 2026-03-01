use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use ts_rs::TS;
use uuid::Uuid;

use crate::Fix;
use crate::aircraft::{AddressType, Aircraft};
use crate::fixes_repo::FixOrder;
use crate::geometry::spline::{GeoPoint, calculate_spline_distance, generate_spline_path};

/// Flight state enum representing the current status of a flight
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "snake_case")]
pub enum FlightState {
    /// Flight is currently active (no landing_time, no timed_out_at, last_fix_at within 10 minutes)
    Active,
    /// Flight is stale (no beacons for 10+ minutes but less than 1 hour)
    Stale,
    /// Flight completed with normal landing (has landing_time)
    Complete,
    /// Flight timed out due to no beacons for 1+ hour (has timed_out_at)
    TimedOut,
}

/// Flight phase at timeout - used to determine flight coalescing behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, DbEnum)]
#[serde(rename_all = "snake_case")]
#[db_enum(existing_type_path = "crate::schema::sql_types::TimeoutPhase")]
pub enum TimeoutPhase {
    /// Flight was climbing when it timed out
    Climbing,
    /// Flight was cruising when it timed out
    Cruising,
    /// Flight was descending when it timed out
    Descending,
    /// Flight phase could not be determined
    Unknown,
}

/// Reason a flight was classified as spurious
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, DbEnum)]
#[serde(rename_all = "snake_case")]
#[db_enum(existing_type_path = "crate::schema::sql_types::SpuriousFlightReason")]
pub enum SpuriousFlightReason {
    DurationTooShort,
    AltitudeRangeInsufficient,
    MaxAglTooLow,
    ExcessiveAltitude,
    ExcessiveSpeed,
    DisplacementTooLow,
}

/// Calculate the distance between two points using the Haversine formula
/// Returns distance in meters
pub(crate) fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_M: f64 = 6_371_000.0; // Earth's radius in meters

    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_M * c
}

/// A flight representing a complete takeoff to landing sequence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flight {
    /// Unique identifier for this flight
    pub id: Uuid,

    /// Aircraft UUID (foreign key to devices table)
    pub aircraft_id: Option<Uuid>,

    /// Aircraft address (hex ID like "39D304") - kept for compatibility
    pub device_address: String,

    /// Aircraft address type (ICAO, FLARM, OGN, etc.) - kept for compatibility
    pub device_address_type: AddressType,

    /// Takeoff time (optional - null for flights first seen airborne)
    pub takeoff_time: Option<DateTime<Utc>>,

    /// Landing time (optional - null for flights in progress)
    pub landing_time: Option<DateTime<Utc>>,

    /// Departure airport ID (foreign key to airports table)
    pub departure_airport_id: Option<i32>,

    /// Arrival airport ID (foreign key to airports table - may be same as departure for local flights)
    pub arrival_airport_id: Option<i32>,

    /// Aircraft ID of the towplane that towed this glider (if this is a glider flight)
    pub towed_by_aircraft_id: Option<Uuid>,

    /// Flight ID of the towplane flight that towed this glider (if this is a glider flight)
    pub towed_by_flight_id: Option<Uuid>,

    /// Tow release altitude in feet MSL (more precise than deprecated meters field)
    pub tow_release_altitude_msl_ft: Option<i32>,

    /// Timestamp when tow release occurred
    pub tow_release_time: Option<DateTime<Utc>>,

    /// Altitude gain during tow in feet (tow_release_altitude_msl_ft - towplane takeoff altitude MSL)
    pub tow_release_height_delta_ft: Option<i32>,

    /// Club that owns the aircraft for this flight
    pub club_id: Option<Uuid>,

    /// Altitude offset at takeoff (difference between fix altitude and true MSL altitude in feet)
    pub takeoff_altitude_offset_ft: Option<i32>,

    /// Altitude offset at landing (difference between fix altitude and true MSL altitude in feet)
    pub landing_altitude_offset_ft: Option<i32>,

    /// Runway identifier used for takeoff (e.g., "09L", "27R")
    pub takeoff_runway_ident: Option<String>,

    /// Runway identifier used for landing (e.g., "09L", "27R")
    pub landing_runway_ident: Option<String>,

    /// Total distance flown during the flight in meters
    /// Computed upon landing from consecutive fixes
    pub total_distance_meters: Option<f64>,

    /// Maximum displacement from departure airport in meters
    pub maximum_displacement_meters: Option<f64>,

    /// Whether runways were inferred from heading (true) or looked up in database (false)
    /// NULL if no runways were determined (both takeoff and landing runways are null)
    pub runways_inferred: Option<bool>,

    /// Start location ID with reverse geocoded address (foreign key to locations table)
    /// Set to airport location if takeoff from airport, or reverse geocoded detection point if airborne
    /// Uses Pelias city-level reverse geocoding for real-time performance
    pub start_location_id: Option<Uuid>,

    /// End location ID with reverse geocoded address (foreign key to locations table)
    /// Set to airport location if landing at airport, or reverse geocoded timeout point if timed out
    /// Uses Pelias city-level reverse geocoding for real-time performance
    pub end_location_id: Option<Uuid>,

    /// Timestamp when flight was timed out (no beacons for 1+ hour)
    /// Mutually exclusive with landing_time - a flight is either landed or timed out, not both
    pub timed_out_at: Option<DateTime<Utc>>,

    /// Flight phase when timeout occurred
    /// Used to determine coalescing behavior when aircraft reappears
    /// NULL if flight has not timed out or if timeout occurred before this field was added
    pub timeout_phase: Option<TimeoutPhase>,

    /// Timestamp of the last fix received for this flight
    /// Updated whenever a fix is assigned to this flight
    pub last_fix_at: DateTime<Utc>,

    /// Callsign / flight number (e.g., "KLM33K") from APRS packets
    pub callsign: Option<String>,

    /// Database timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Flight {
    /// Get the current state of the flight
    pub fn state(&self) -> FlightState {
        if self.timed_out_at.is_some() {
            FlightState::TimedOut
        } else if self.landing_time.is_some() {
            FlightState::Complete
        } else {
            // Check if flight is stale (no beacons for 10+ minutes)
            let time_since_last_fix = Utc::now().signed_duration_since(self.last_fix_at);
            if time_since_last_fix > chrono::Duration::minutes(10) {
                FlightState::Stale
            } else {
                FlightState::Active
            }
        }
    }

    /// Check if the flight is timed out
    pub fn is_timed_out(&self) -> bool {
        self.timed_out_at.is_some()
    }

    /// Create a new flight with takeoff time
    pub fn new(device_address: String, takeoff_time: Option<DateTime<Utc>>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            aircraft_id: None,
            device_address,
            device_address_type: AddressType::Unknown,
            takeoff_time,
            landing_time: None,
            departure_airport_id: None,
            arrival_airport_id: None,
            towed_by_aircraft_id: None,
            towed_by_flight_id: None,
            tow_release_altitude_msl_ft: None,
            tow_release_time: None,
            tow_release_height_delta_ft: None,
            club_id: None,
            takeoff_altitude_offset_ft: None,
            landing_altitude_offset_ft: None,
            takeoff_runway_ident: None,
            landing_runway_ident: None,
            total_distance_meters: None,
            maximum_displacement_meters: None,
            runways_inferred: None,
            start_location_id: None,
            end_location_id: None,
            timed_out_at: None,
            timeout_phase: None,
            last_fix_at: now,
            callsign: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_from_fix(fix: &Fix, device: &Aircraft, takeoff_time: Option<DateTime<Utc>>) -> Self {
        let now = fix.received_at;
        info!(
            "Creating flight for {} from fix {} with climb: {:?} alt: {:?} speed: {:?}",
            device.aircraft_address_hex().unwrap_or_default(),
            fix.id,
            fix.climb_fpm,
            fix.altitude_msl_feet,
            fix.ground_speed_knots
        );
        Self {
            id: Uuid::now_v7(),
            aircraft_id: fix.aircraft_id.into(),
            device_address: device.aircraft_address_hex().unwrap_or_default(),
            device_address_type: device.primary_address_type(),
            takeoff_time,
            landing_time: None,
            departure_airport_id: None,
            arrival_airport_id: None,
            towed_by_aircraft_id: None,
            towed_by_flight_id: None,
            tow_release_altitude_msl_ft: None,
            tow_release_time: None,
            tow_release_height_delta_ft: None,
            club_id: None, // Will be populated from device
            takeoff_altitude_offset_ft: None,
            landing_altitude_offset_ft: None,
            takeoff_runway_ident: None,
            landing_runway_ident: None,
            total_distance_meters: None,
            maximum_displacement_meters: None,
            runways_inferred: None,
            start_location_id: None,
            end_location_id: None,
            timed_out_at: None,
            timeout_phase: None,
            last_fix_at: fix.received_at,
            callsign: None,
            created_at: now,
            updated_at: now,
        }
    }
    /// Create a new flight for device already airborne (no takeoff time)
    pub fn new_airborne_from_fix(fix: &Fix, device: &Aircraft) -> Self {
        Self::new_from_fix(fix, device, None)
    }

    /// Create a new flight for device already airborne with a pre-generated UUID
    /// This is used to prevent race conditions when creating flights asynchronously
    pub fn new_airborne_from_fix_with_id(fix: &Fix, device: &Aircraft, flight_id: Uuid) -> Self {
        let now = fix.received_at;
        debug!("Creating airborne flight {} from fix: {:?}", flight_id, fix);
        Self {
            id: flight_id,
            aircraft_id: fix.aircraft_id.into(),
            device_address: device.aircraft_address_hex().unwrap_or_default(),
            device_address_type: device.primary_address_type(),
            takeoff_time: None,
            landing_time: None,
            departure_airport_id: None,
            arrival_airport_id: None,
            towed_by_aircraft_id: None,
            towed_by_flight_id: None,
            tow_release_altitude_msl_ft: None,
            tow_release_time: None,
            tow_release_height_delta_ft: None,
            club_id: None,
            takeoff_altitude_offset_ft: None,
            landing_altitude_offset_ft: None,
            takeoff_runway_ident: None,
            landing_runway_ident: None,
            total_distance_meters: None,
            maximum_displacement_meters: None,
            runways_inferred: None,
            start_location_id: None,
            end_location_id: None,
            timed_out_at: None,
            timeout_phase: None,
            last_fix_at: fix.received_at,
            callsign: fix.flight_number.clone(),
            created_at: fix.received_at,
            updated_at: now,
        }
    }

    /// Create a new flight with known takeoff time
    pub fn new_with_takeoff_from_fix(
        fix: &Fix,
        device: &Aircraft,
        takeoff_time: DateTime<Utc>,
    ) -> Self {
        Self::new_from_fix(fix, device, Some(takeoff_time))
    }

    /// Create a new flight with known takeoff time and pre-generated UUID
    /// This is used to prevent race conditions when creating flights asynchronously
    pub fn new_with_takeoff_from_fix_with_id(
        fix: &Fix,
        device: &Aircraft,
        flight_id: Uuid,
        takeoff_time: DateTime<Utc>,
    ) -> Self {
        let now = fix.received_at;
        debug!(
            "Creating flight {} with takeoff from fix: {:?}",
            flight_id, fix
        );
        Self {
            id: flight_id,
            aircraft_id: fix.aircraft_id.into(),
            device_address: device.aircraft_address_hex().unwrap_or_default(),
            device_address_type: device.primary_address_type(),
            takeoff_time: Some(takeoff_time),
            landing_time: None,
            departure_airport_id: None,
            arrival_airport_id: None,
            towed_by_aircraft_id: None,
            towed_by_flight_id: None,
            tow_release_altitude_msl_ft: None,
            tow_release_time: None,
            tow_release_height_delta_ft: None,
            club_id: None,
            takeoff_altitude_offset_ft: None,
            landing_altitude_offset_ft: None,
            takeoff_runway_ident: None,
            landing_runway_ident: None,
            total_distance_meters: None,
            maximum_displacement_meters: None,
            runways_inferred: None,
            start_location_id: None,
            end_location_id: None,
            timed_out_at: None,
            timeout_phase: None,
            last_fix_at: fix.received_at,
            callsign: fix.flight_number.clone(),
            created_at: fix.received_at,
            updated_at: now,
        }
    }

    /// Check if the flight is still in progress (no landing time)
    pub fn is_in_progress(&self) -> bool {
        self.landing_time.is_none()
    }

    #[cfg(test)]
    /// Helper for tests: create a minimal Aircraft
    fn test_aircraft() -> Aircraft {
        Aircraft {
            id: Some(Uuid::new_v4()),
            address_type: AddressType::Icao,
            address: 0xABCDEF,
            icao_address: Some(0xABCDEF),
            flarm_address: None,
            ogn_address: None,
            other_address: None,
            aircraft_model: String::new(),
            registration: None,
            competition_number: String::new(),
            tracked: true,
            identified: true,
            frequency_mhz: None,
            pilot_name: None,
            home_base_airport_ident: None,
            last_fix_at: None,
            club_id: None,
            icao_model_code: None,
            adsb_emitter_category: None,
            tracker_device_type: None,
            country_code: None,
            owner_operator: None,
            aircraft_category: None,
            engine_count: None,
            engine_type: None,
            faa_pia: None,
            faa_ladd: None,
            year: None,
            is_military: None,
            from_ogn_ddb: None,
            from_adsbx_ddb: None,
            created_at: None,
            updated_at: None,
            latitude: None,
            longitude: None,
            current_fix: None,
        }
    }

    #[cfg(test)]
    /// Helper for tests: create a minimal Fix with given flight_number
    fn test_fix(flight_number: Option<&str>) -> Fix {
        Fix {
            id: Uuid::new_v4(),
            source: "TEST".to_string(),
            latitude: 42.0,
            longitude: -122.0,
            altitude_msl_feet: Some(5000),
            altitude_agl_feet: None,
            flight_number: flight_number.map(|s| s.to_string()),
            squawk: None,
            ground_speed_knots: Some(150.0),
            track_degrees: None,
            climb_fpm: None,
            turn_rate_rot: None,
            source_metadata: None,
            flight_id: None,
            aircraft_id: Uuid::new_v4(),
            received_at: Utc::now(),
            is_active: true,
            receiver_id: None,
            raw_message_id: Uuid::new_v4(),
            altitude_agl_valid: false,
            time_gap_seconds: None,
        }
    }

    /// Get flight duration if landed, otherwise duration from takeoff to now
    /// Returns None if no takeoff time is known
    pub fn duration(&self) -> Option<chrono::Duration> {
        if let Some(takeoff_time) = self.takeoff_time {
            let end_time = self.landing_time.unwrap_or_else(Utc::now);
            Some(end_time - takeoff_time)
        } else {
            None
        }
    }

    /// Check if this flight used a tow plane
    pub fn has_tow(&self) -> bool {
        self.towed_by_aircraft_id.is_some() || self.towed_by_flight_id.is_some()
    }

    /// Calculate the total distance flown during this flight
    /// Uses centripetal Catmull-Rom spline interpolation to account for aircraft
    /// turning behavior, providing more accurate distance than straight-line segments.
    /// Returns the distance in meters, or None if there are insufficient fixes.
    ///
    /// # Arguments
    /// * `fixes_repo` - Repository for fetching fixes (only used if `cached_fixes` is None)
    /// * `cached_fixes` - Optional pre-fetched fixes to use instead of querying the database
    pub async fn total_distance(
        &self,
        fixes_repo: &crate::fixes_repo::FixesRepository,
        cached_fixes: Option<&[crate::Fix]>,
    ) -> Result<Option<f64>> {
        // Use cached fixes if provided, otherwise fetch from database
        let fixes = if let Some(cached) = cached_fixes {
            cached.to_vec()
        } else {
            let start_time = self.takeoff_time.unwrap_or(self.created_at);
            let end_time = self.landing_time.unwrap_or(self.last_fix_at);

            fixes_repo
                .get_fixes_for_aircraft_with_time_range(
                    &self.aircraft_id.unwrap_or(Uuid::nil()),
                    start_time,
                    end_time,
                    None,
                    FixOrder::Ascending,
                )
                .await?
        };

        if fixes.len() < 2 {
            return Ok(None);
        }

        // Convert fixes to GeoPoints for spline calculation
        let points: Vec<GeoPoint> = fixes
            .iter()
            .map(|fix| GeoPoint::new(fix.latitude, fix.longitude))
            .collect();

        // Use 100m sample distance for accurate distance calculation
        let distance = calculate_spline_distance(&points, 100.0);

        Ok(Some(distance))
    }

    /// Generate a human-readable aircraft identifier for display
    /// Priority: 1) Model + Registration, 2) Registration only, 3) Model only, 4) ICAO-XXYYZZ format
    fn get_aircraft_identifier(&self, device: Option<&crate::aircraft::Aircraft>) -> String {
        if let Some(device) = device {
            let has_model = !device.aircraft_model.is_empty();
            let has_registration = device.registration.as_ref().is_some_and(|r| !r.is_empty());

            match (has_model, has_registration) {
                (true, true) => {
                    // Both model and registration available: "Piper Pacer N8437D"
                    format!(
                        "{} {}",
                        device.aircraft_model,
                        device.registration.as_deref().unwrap()
                    )
                }
                (false, true) => {
                    // Only registration: "N8437D"
                    device.registration.as_ref().unwrap().clone()
                }
                (true, false) => {
                    // Only model: "Piper Pacer"
                    device.aircraft_model.clone()
                }
                (false, false) => {
                    // Neither available, fall back to ICAO-XXYYZZ format
                    let type_prefix = match device.primary_address_type() {
                        crate::aircraft::AddressType::Icao => "ICAO",
                        crate::aircraft::AddressType::Flarm => "FLARM",
                        crate::aircraft::AddressType::Ogn => "OGN",
                        crate::aircraft::AddressType::Unknown => "Unknown",
                    };
                    format!(
                        "{}-{}",
                        type_prefix,
                        device.aircraft_address_hex().unwrap_or_default()
                    )
                }
            }
        } else {
            // No device info, use the flight's device_address
            self.device_address.clone()
        }
    }

    /// Calculate the maximum displacement from the departure airport.
    /// Uses spline interpolation to check the entire flight path, not just the GPS fix points.
    /// Returns the maximum distance in meters from the departure airport, or None if no departure airport is set.
    ///
    /// # Arguments
    /// * `fixes_repo` - Repository for fetching fixes (only used if `cached_fixes` is None)
    /// * `airports_repo` - Repository for fetching airport information
    /// * `cached_fixes` - Optional pre-fetched fixes to use instead of querying the database
    pub async fn maximum_displacement(
        &self,
        fixes_repo: &crate::fixes_repo::FixesRepository,
        airports_repo: &crate::airports_repo::AirportsRepository,
        cached_fixes: Option<&[crate::Fix]>,
    ) -> Result<Option<f64>> {
        // Requires a departure airport to measure displacement from
        let airport_id = match self.departure_airport_id {
            Some(id) => id,
            None => return Ok(None),
        };
        let airport = match airports_repo.get_airport_by_id(airport_id).await? {
            Some(a) => a,
            None => return Ok(None),
        };

        let (airport_lat, airport_lon) = match (airport.latitude_deg, airport.longitude_deg) {
            (Some(lat), Some(lon)) => (
                lat.to_string().parse::<f64>().unwrap_or(0.0),
                lon.to_string().parse::<f64>().unwrap_or(0.0),
            ),
            _ => return Ok(None),
        };

        // Use cached fixes if provided, otherwise fetch from database
        let fixes = if let Some(cached) = cached_fixes {
            cached.to_vec()
        } else {
            let start_time = self.takeoff_time.unwrap_or(self.created_at);
            let end_time = self.landing_time.unwrap_or(self.last_fix_at);

            fixes_repo
                .get_fixes_for_aircraft_with_time_range(
                    &self.aircraft_id.unwrap_or(Uuid::nil()),
                    start_time,
                    end_time,
                    None,
                    FixOrder::Ascending,
                )
                .await?
        };

        if fixes.is_empty() {
            return Ok(None);
        }

        // Convert fixes to GeoPoints and generate spline-interpolated path
        let fix_points: Vec<GeoPoint> = fixes
            .iter()
            .map(|fix| GeoPoint::new(fix.latitude, fix.longitude))
            .collect();

        // Generate interpolated path with 100m spacing
        let path_points = generate_spline_path(&fix_points, 100.0);

        // Find maximum distance along the interpolated path
        let max_distance = path_points
            .iter()
            .map(|point| {
                haversine_distance(airport_lat, airport_lon, point.latitude, point.longitude)
            })
            .fold(0.0_f64, |acc, d| acc.max(d));

        Ok(Some(max_distance))
    }

    /// Generate a Google Earth compatible KML file for this flight
    /// Returns KML as a string containing the flight track with fixes
    pub async fn make_kml(
        &self,
        fixes_repo: &crate::fixes_repo::FixesRepository,
        device: Option<&crate::aircraft::Aircraft>,
    ) -> Result<String> {
        use kml::types::{
            AltitudeMode, Coord, IconStyle, LineString, LineStyle, Placemark, Point, Style,
        };
        use kml::{Kml, KmlWriter};
        use std::collections::HashMap;

        // Get all fixes for this flight based on aircraft ID and time range
        let start_time = self.takeoff_time.unwrap_or(self.created_at);
        let end_time = self.landing_time.unwrap_or(self.last_fix_at);

        let fixes = fixes_repo
            .get_fixes_for_aircraft_with_time_range(
                &self.aircraft_id.unwrap_or(Uuid::nil()),
                start_time,
                end_time,
                None,
                FixOrder::Ascending,
            )
            .await?;

        if fixes.is_empty() {
            return Ok(self.generate_empty_kml());
        }

        // Helper function to convert altitude to KML ABGR color (gradient from red to blue)
        let altitude_to_kml_color =
            |altitude_msl_feet: Option<i32>, min_alt: f64, max_alt: f64| -> String {
                if let Some(alt) = altitude_msl_feet {
                    let alt_f64 = alt as f64;
                    if max_alt > min_alt {
                        // Normalize altitude to 0-1 range
                        let normalized =
                            ((alt_f64 - min_alt) / (max_alt - min_alt)).clamp(0.0, 1.0);

                        // Interpolate from red (low) to blue (high)
                        // Red RGB: (239, 68, 68)
                        // Blue RGB: (59, 130, 246)
                        let r = (239.0 - normalized * (239.0 - 59.0)).round() as u8;
                        let g = (68.0 + normalized * (130.0 - 68.0)).round() as u8;
                        let b = (68.0 + normalized * (246.0 - 68.0)).round() as u8;

                        // KML uses ABGR format (alpha, blue, green, red)
                        return format!("ff{:02x}{:02x}{:02x}", b, g, r);
                    }
                }
                // Gray for unknown altitude
                String::from("ff888888")
            };

        // Calculate min/max altitudes for gradient
        let min_alt = fixes
            .iter()
            .filter_map(|f| f.altitude_msl_feet)
            .map(|alt| alt as f64)
            .fold(f64::INFINITY, f64::min);
        let max_alt = fixes
            .iter()
            .filter_map(|f| f.altitude_msl_feet)
            .map(|alt| alt as f64)
            .fold(f64::NEG_INFINITY, f64::max);

        let mut elements: Vec<Kml<f64>> = Vec::new();

        // Add takeoff style
        let takeoff_style = Style {
            id: Some("takeoffStyle".to_string()),
            icon: Some(IconStyle {
                id: None,
                scale: 1.2,
                heading: 0.0,
                hot_spot: None,
                icon: Default::default(),
                color: "ff00ff00".to_string(), // Green
                color_mode: Default::default(),
                attrs: HashMap::new(),
            }),
            ..Default::default()
        };
        elements.push(Kml::Style(takeoff_style));

        // Add landing style
        let landing_style = Style {
            id: Some("landingStyle".to_string()),
            icon: Some(IconStyle {
                id: None,
                scale: 1.2,
                heading: 0.0,
                hot_spot: None,
                icon: Default::default(),
                color: "ff0000ff".to_string(), // Red
                color_mode: Default::default(),
                attrs: HashMap::new(),
            }),
            ..Default::default()
        };
        elements.push(Kml::Style(landing_style));

        // Flight track as gradient LineString segments
        // Create a segment between each pair of consecutive fixes with altitude-based coloring
        for i in 0..fixes.len().saturating_sub(1) {
            let fix1 = &fixes[i];
            let fix2 = &fixes[i + 1];

            // Get color based on starting fix altitude
            let color = altitude_to_kml_color(fix1.altitude_msl_feet, min_alt, max_alt);

            // Create style for this segment
            let segment_style = Style {
                id: Some(format!("segment{}", i)),
                line: Some(LineStyle {
                    id: None,
                    color: color.clone(),
                    color_mode: Default::default(),
                    width: 3.0,
                    attrs: HashMap::new(),
                }),
                ..Default::default()
            };
            elements.push(Kml::Style(segment_style));

            // Create coordinates for this segment
            let alt1_meters = fix1
                .altitude_msl_feet
                .map(|alt| alt as f64 * 0.3048)
                .unwrap_or(0.0);
            let alt2_meters = fix2
                .altitude_msl_feet
                .map(|alt| alt as f64 * 0.3048)
                .unwrap_or(0.0);

            let coords = vec![
                Coord {
                    x: fix1.longitude,
                    y: fix1.latitude,
                    z: Some(alt1_meters),
                },
                Coord {
                    x: fix2.longitude,
                    y: fix2.latitude,
                    z: Some(alt2_meters),
                },
            ];

            // Create LineString segment
            let line_string = LineString {
                coords,
                extrude: false,
                tessellate: true,
                altitude_mode: AltitudeMode::Absolute,
                attrs: HashMap::new(),
            };

            let placemark = Placemark {
                name: Some(format!("Segment {}", i)),
                description: None,
                geometry: Some(kml::types::Geometry::LineString(line_string)),
                style_url: Some(format!("#segment{}", i)),
                attrs: HashMap::new(),
                children: vec![],
            };
            elements.push(Kml::Placemark(placemark));
        }

        // Takeoff or detected point
        if let Some(first_fix) = fixes.first() {
            let altitude_meters = first_fix
                .altitude_msl_feet
                .map(|alt| alt as f64 * 0.3048)
                .unwrap_or(0.0);

            let point = Point {
                coord: Coord {
                    x: first_fix.longitude,
                    y: first_fix.latitude,
                    z: Some(altitude_meters),
                },
                extrude: false,
                altitude_mode: AltitudeMode::Absolute,
                attrs: HashMap::new(),
            };

            // Use "Detected" if no takeoff time (flight was first seen airborne)
            let (name, description) = if self.takeoff_time.is_some() {
                (
                    "Takeoff".to_string(),
                    format!("Takeoff at {} UTC", start_time.format("%Y-%m-%d %H:%M:%S")),
                )
            } else {
                (
                    "Detected".to_string(),
                    format!(
                        "Flight first detected at {} UTC (already airborne)",
                        start_time.format("%Y-%m-%d %H:%M:%S")
                    ),
                )
            };

            let placemark = Placemark {
                name: Some(name),
                description: Some(description),
                geometry: Some(kml::types::Geometry::Point(point)),
                style_url: Some("#takeoffStyle".to_string()),
                attrs: HashMap::new(),
                children: vec![],
            };
            elements.push(Kml::Placemark(placemark));
        }

        // Landing or signal lost point (if flight is complete or timed out)
        if (self.landing_time.is_some() || self.timed_out_at.is_some())
            && let Some(last_fix) = fixes.last()
        {
            let altitude_meters = last_fix
                .altitude_msl_feet
                .map(|alt| alt as f64 * 0.3048)
                .unwrap_or(0.0);

            let point = Point {
                coord: Coord {
                    x: last_fix.longitude,
                    y: last_fix.latitude,
                    z: Some(altitude_meters),
                },
                extrude: false,
                altitude_mode: AltitudeMode::Absolute,
                attrs: HashMap::new(),
            };

            // Use "Signal Lost" if timed out, otherwise "Landing"
            let (name, description) = if self.timed_out_at.is_some() {
                (
                    "Signal Lost".to_string(),
                    format!(
                        "Last signal received at {} UTC (flight timed out after 1+ hour without updates)",
                        end_time.format("%Y-%m-%d %H:%M:%S")
                    ),
                )
            } else {
                (
                    "Landing".to_string(),
                    format!("Landing at {} UTC", end_time.format("%Y-%m-%d %H:%M:%S")),
                )
            };

            let placemark = Placemark {
                name: Some(name),
                description: Some(description),
                geometry: Some(kml::types::Geometry::Point(point)),
                style_url: Some("#landingStyle".to_string()),
                attrs: HashMap::new(),
                children: vec![],
            };
            elements.push(Kml::Placemark(placemark));
        }

        // Create the document with name and description in attrs
        let aircraft_identifier = self.get_aircraft_identifier(device);
        let flight_name = format!("Flight {}", aircraft_identifier);
        let description = format!(
            "Flight track for aircraft {} from {} to {}",
            aircraft_identifier,
            start_time.format("%Y-%m-%d %H:%M:%S UTC"),
            end_time.format("%Y-%m-%d %H:%M:%S UTC")
        );

        let mut doc_attrs = HashMap::new();
        doc_attrs.insert("name".to_string(), flight_name);
        doc_attrs.insert("description".to_string(), description);

        let document = Kml::Document {
            attrs: doc_attrs,
            elements,
        };

        // Write the KML to a string
        let mut buf = Vec::new();
        let mut writer = KmlWriter::from_writer(&mut buf);
        writer
            .write(&document)
            .map_err(|e| anyhow::anyhow!("Failed to write KML: {}", e))?;

        String::from_utf8(buf)
            .map_err(|e| anyhow::anyhow!("Failed to convert KML to string: {}", e))
    }

    /// Generate an empty KML file when no fixes are available
    fn generate_empty_kml(&self) -> String {
        let mut kml = String::new();
        kml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        kml.push_str("<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n");
        kml.push_str("  <Document>\n");
        kml.push_str(&format!(
            "    <name>Flight {} (No Track Data)</name>\n",
            self.device_address
        ));
        kml.push_str(&format!(
            "    <description>No position data available for flight {}</description>\n",
            self.device_address
        ));
        kml.push_str("  </Document>\n");
        kml.push_str("</kml>\n");
        kml
    }

    /// Generate an IGC (International Gliding Commission) file for this flight
    /// Returns IGC as a string containing the flight track with fixes
    pub async fn make_igc(
        &self,
        fixes_repo: &crate::fixes_repo::FixesRepository,
        device: Option<&crate::aircraft::Aircraft>,
    ) -> Result<String> {
        // Get all fixes for this flight based on aircraft ID and time range
        let start_time = self.takeoff_time.unwrap_or(self.created_at);
        let end_time = self.landing_time.unwrap_or(self.last_fix_at);

        let fixes = fixes_repo
            .get_fixes_for_aircraft_with_time_range(
                &self.aircraft_id.unwrap_or(Uuid::nil()),
                start_time,
                end_time,
                None,
                FixOrder::Ascending,
            )
            .await?;

        // Generate IGC content using the igc module
        Ok(crate::igc::generate_igc(self, &fixes, device))
    }
}

/// Diesel model for the flights table - used for database operations
/// IMPORTANT: Field order MUST match the database schema exactly for Queryable to work
#[derive(Debug, Clone, Queryable, Selectable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::flights)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FlightModel {
    pub id: Uuid,
    pub device_address: String,
    pub takeoff_time: Option<DateTime<Utc>>,
    pub landing_time: Option<DateTime<Utc>>,
    pub club_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub device_address_type: AddressType,
    pub aircraft_id: Option<Uuid>,
    pub takeoff_altitude_offset_ft: Option<i32>,
    pub landing_altitude_offset_ft: Option<i32>,
    pub takeoff_runway_ident: Option<String>,
    pub landing_runway_ident: Option<String>,
    pub total_distance_meters: Option<f64>,
    pub maximum_displacement_meters: Option<f64>,
    pub departure_airport_id: Option<i32>,
    pub arrival_airport_id: Option<i32>,
    pub towed_by_aircraft_id: Option<Uuid>,
    pub towed_by_flight_id: Option<Uuid>,
    pub tow_release_altitude_msl_ft: Option<i32>,
    pub tow_release_time: Option<DateTime<Utc>>,
    pub runways_inferred: Option<bool>,
    pub start_location_id: Option<Uuid>,
    pub end_location_id: Option<Uuid>,
    pub timed_out_at: Option<DateTime<Utc>>,
    pub timeout_phase: Option<TimeoutPhase>,
    pub last_fix_at: DateTime<Utc>,
    pub tow_release_height_delta_ft: Option<i32>,
    pub callsign: Option<String>,
}

/// Insert model for new flights
#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::flights)]
pub struct NewFlightModel {
    pub id: Uuid,
    pub device_address: String,
    pub takeoff_time: Option<DateTime<Utc>>,
    pub landing_time: Option<DateTime<Utc>>,
    pub club_id: Option<Uuid>,
    pub device_address_type: AddressType,
    pub aircraft_id: Option<Uuid>,
    pub takeoff_altitude_offset_ft: Option<i32>,
    pub landing_altitude_offset_ft: Option<i32>,
    pub takeoff_runway_ident: Option<String>,
    pub landing_runway_ident: Option<String>,
    pub total_distance_meters: Option<f64>,
    pub maximum_displacement_meters: Option<f64>,
    pub departure_airport_id: Option<i32>,
    pub arrival_airport_id: Option<i32>,
    pub towed_by_aircraft_id: Option<Uuid>,
    pub towed_by_flight_id: Option<Uuid>,
    pub tow_release_altitude_msl_ft: Option<i32>,
    pub tow_release_time: Option<DateTime<Utc>>,
    pub runways_inferred: Option<bool>,
    pub start_location_id: Option<Uuid>,
    pub end_location_id: Option<Uuid>,
    pub timed_out_at: Option<DateTime<Utc>>,
    pub timeout_phase: Option<TimeoutPhase>,
    pub last_fix_at: DateTime<Utc>,
    // Note: callsign and tow_release_height_delta_ft are not included in NewFlightModel
    // as they are not set during initial flight creation
}

/// Insert model for archiving spurious flights
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::spurious_flights)]
pub struct NewSpuriousFlightModel {
    pub id: Uuid,
    pub device_address: String,
    pub takeoff_time: Option<DateTime<Utc>>,
    pub landing_time: Option<DateTime<Utc>>,
    pub club_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub device_address_type: AddressType,
    pub aircraft_id: Option<Uuid>,
    pub takeoff_altitude_offset_ft: Option<i32>,
    pub landing_altitude_offset_ft: Option<i32>,
    pub takeoff_runway_ident: Option<String>,
    pub landing_runway_ident: Option<String>,
    pub total_distance_meters: Option<f64>,
    pub maximum_displacement_meters: Option<f64>,
    pub departure_airport_id: Option<i32>,
    pub arrival_airport_id: Option<i32>,
    pub towed_by_aircraft_id: Option<Uuid>,
    pub towed_by_flight_id: Option<Uuid>,
    pub tow_release_altitude_msl_ft: Option<i32>,
    pub tow_release_time: Option<DateTime<Utc>>,
    pub runways_inferred: Option<bool>,
    pub start_location_id: Option<Uuid>,
    pub end_location_id: Option<Uuid>,
    pub timed_out_at: Option<DateTime<Utc>>,
    pub timeout_phase: Option<TimeoutPhase>,
    pub last_fix_at: DateTime<Utc>,
    pub tow_release_height_delta_ft: Option<i32>,
    pub callsign: Option<String>,
    pub reasons: Vec<Option<SpuriousFlightReason>>,
    pub reason_descriptions: Vec<Option<String>>,
}

impl NewSpuriousFlightModel {
    /// Create from a FlightModel and spurious reason data
    pub fn from_flight(
        flight: FlightModel,
        reasons: Vec<SpuriousFlightReason>,
        reason_descriptions: Vec<String>,
    ) -> Self {
        Self {
            id: flight.id,
            device_address: flight.device_address,
            takeoff_time: flight.takeoff_time,
            landing_time: flight.landing_time,
            club_id: flight.club_id,
            created_at: flight.created_at,
            updated_at: flight.updated_at,
            device_address_type: flight.device_address_type,
            aircraft_id: flight.aircraft_id,
            takeoff_altitude_offset_ft: flight.takeoff_altitude_offset_ft,
            landing_altitude_offset_ft: flight.landing_altitude_offset_ft,
            takeoff_runway_ident: flight.takeoff_runway_ident,
            landing_runway_ident: flight.landing_runway_ident,
            total_distance_meters: flight.total_distance_meters,
            maximum_displacement_meters: flight.maximum_displacement_meters,
            departure_airport_id: flight.departure_airport_id,
            arrival_airport_id: flight.arrival_airport_id,
            towed_by_aircraft_id: flight.towed_by_aircraft_id,
            towed_by_flight_id: flight.towed_by_flight_id,
            tow_release_altitude_msl_ft: flight.tow_release_altitude_msl_ft,
            tow_release_time: flight.tow_release_time,
            runways_inferred: flight.runways_inferred,
            start_location_id: flight.start_location_id,
            end_location_id: flight.end_location_id,
            timed_out_at: flight.timed_out_at,
            timeout_phase: flight.timeout_phase,
            last_fix_at: flight.last_fix_at,
            tow_release_height_delta_ft: flight.tow_release_height_delta_ft,
            callsign: flight.callsign,
            reasons: reasons.into_iter().map(Some).collect(),
            reason_descriptions: reason_descriptions.into_iter().map(Some).collect(),
        }
    }
}

/// Conversion from Flight (API model) to FlightModel (database model)
impl From<Flight> for FlightModel {
    fn from(flight: Flight) -> Self {
        Self {
            id: flight.id,
            device_address: flight.device_address,
            device_address_type: flight.device_address_type,
            takeoff_time: flight.takeoff_time,
            landing_time: flight.landing_time,
            club_id: flight.club_id,
            created_at: flight.created_at,
            updated_at: flight.updated_at,
            aircraft_id: flight.aircraft_id,
            takeoff_altitude_offset_ft: flight.takeoff_altitude_offset_ft,
            landing_altitude_offset_ft: flight.landing_altitude_offset_ft,
            takeoff_runway_ident: flight.takeoff_runway_ident,
            landing_runway_ident: flight.landing_runway_ident,
            total_distance_meters: flight.total_distance_meters,
            maximum_displacement_meters: flight.maximum_displacement_meters,
            departure_airport_id: flight.departure_airport_id,
            arrival_airport_id: flight.arrival_airport_id,
            towed_by_aircraft_id: flight.towed_by_aircraft_id,
            towed_by_flight_id: flight.towed_by_flight_id,
            tow_release_altitude_msl_ft: flight.tow_release_altitude_msl_ft,
            tow_release_time: flight.tow_release_time,
            runways_inferred: flight.runways_inferred,
            start_location_id: flight.start_location_id,
            end_location_id: flight.end_location_id,
            timed_out_at: flight.timed_out_at,
            timeout_phase: flight.timeout_phase,
            last_fix_at: flight.last_fix_at,
            callsign: flight.callsign,
            tow_release_height_delta_ft: flight.tow_release_height_delta_ft,
        }
    }
}

/// Conversion from Flight (API model) to NewFlightModel (insert model)
impl From<Flight> for NewFlightModel {
    fn from(flight: Flight) -> Self {
        Self {
            id: flight.id,
            device_address: flight.device_address,
            device_address_type: flight.device_address_type,
            takeoff_time: flight.takeoff_time,
            landing_time: flight.landing_time,
            club_id: flight.club_id,
            aircraft_id: flight.aircraft_id,
            takeoff_altitude_offset_ft: flight.takeoff_altitude_offset_ft,
            landing_altitude_offset_ft: flight.landing_altitude_offset_ft,
            takeoff_runway_ident: flight.takeoff_runway_ident,
            landing_runway_ident: flight.landing_runway_ident,
            total_distance_meters: flight.total_distance_meters,
            maximum_displacement_meters: flight.maximum_displacement_meters,
            departure_airport_id: flight.departure_airport_id,
            arrival_airport_id: flight.arrival_airport_id,
            towed_by_aircraft_id: flight.towed_by_aircraft_id,
            towed_by_flight_id: flight.towed_by_flight_id,
            tow_release_altitude_msl_ft: flight.tow_release_altitude_msl_ft,
            tow_release_time: flight.tow_release_time,
            runways_inferred: flight.runways_inferred,
            start_location_id: flight.start_location_id,
            end_location_id: flight.end_location_id,
            timed_out_at: flight.timed_out_at,
            timeout_phase: flight.timeout_phase,
            last_fix_at: flight.last_fix_at,
            // Note: callsign and tow_release_height_delta_ft omitted - not set on creation
        }
    }
}

/// Conversion from FlightModel (database model) to Flight (API model)
impl From<FlightModel> for Flight {
    fn from(model: FlightModel) -> Self {
        Self {
            id: model.id,
            aircraft_id: model.aircraft_id,
            device_address: model.device_address,
            device_address_type: model.device_address_type,
            takeoff_time: model.takeoff_time,
            landing_time: model.landing_time,
            departure_airport_id: model.departure_airport_id,
            arrival_airport_id: model.arrival_airport_id,
            club_id: model.club_id,
            takeoff_altitude_offset_ft: model.takeoff_altitude_offset_ft,
            landing_altitude_offset_ft: model.landing_altitude_offset_ft,
            takeoff_runway_ident: model.takeoff_runway_ident,
            landing_runway_ident: model.landing_runway_ident,
            total_distance_meters: model.total_distance_meters,
            maximum_displacement_meters: model.maximum_displacement_meters,
            towed_by_aircraft_id: model.towed_by_aircraft_id,
            towed_by_flight_id: model.towed_by_flight_id,
            tow_release_altitude_msl_ft: model.tow_release_altitude_msl_ft,
            tow_release_time: model.tow_release_time,
            runways_inferred: model.runways_inferred,
            start_location_id: model.start_location_id,
            end_location_id: model.end_location_id,
            timed_out_at: model.timed_out_at,
            timeout_phase: model.timeout_phase,
            last_fix_at: model.last_fix_at,
            callsign: model.callsign,
            tow_release_height_delta_ft: model.tow_release_height_delta_ft,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_airborne_flight_copies_callsign_from_fix() {
        let fix = Flight::test_fix(Some("VOI5584"));
        let device = Flight::test_aircraft();
        let flight_id = Uuid::new_v4();

        let flight = Flight::new_airborne_from_fix_with_id(&fix, &device, flight_id);

        assert_eq!(flight.callsign, Some("VOI5584".to_string()));
    }

    #[test]
    fn test_takeoff_flight_copies_callsign_from_fix() {
        let fix = Flight::test_fix(Some("SAS1465"));
        let device = Flight::test_aircraft();
        let flight_id = Uuid::new_v4();

        let flight =
            Flight::new_with_takeoff_from_fix_with_id(&fix, &device, flight_id, fix.received_at);

        assert_eq!(flight.callsign, Some("SAS1465".to_string()));
    }

    #[test]
    fn test_flight_callsign_none_when_fix_has_no_callsign() {
        let fix = Flight::test_fix(None);
        let device = Flight::test_aircraft();
        let flight_id = Uuid::new_v4();

        let flight = Flight::new_airborne_from_fix_with_id(&fix, &device, flight_id);

        assert_eq!(flight.callsign, None);
    }
}
