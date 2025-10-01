use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use crate::Fix;
use crate::devices::AddressType;

/// A flight representing a complete takeoff to landing sequence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flight {
    /// Unique identifier for this flight
    pub id: Uuid,

    /// Device UUID (foreign key to devices table)
    pub device_id: Option<Uuid>,

    /// Device address (hex ID like "39D304") - kept for compatibility
    pub device_address: String,

    /// Device address type (ICAO, FLARM, OGN, etc.) - kept for compatibility
    pub device_address_type: AddressType,

    /// Takeoff time (optional - null for flights first seen airborne)
    pub takeoff_time: Option<DateTime<Utc>>,

    /// Landing time (optional - null for flights in progress)
    pub landing_time: Option<DateTime<Utc>>,

    /// Departure airport identifier
    pub departure_airport: Option<String>,

    /// Arrival airport identifier (may be same as departure for local flights)
    pub arrival_airport: Option<String>,

    /// Tow aircraft registration number (foreign key to aircraft_registrations)
    /// If present, the referenced aircraft must have is_tow_plane = true
    pub tow_aircraft_id: Option<String>,

    /// Tow release height in meters MSL (Mean Sea Level)
    pub tow_release_height_msl: Option<i32>,

    /// Club that owns the aircraft for this flight
    pub club_id: Option<Uuid>,

    /// Altitude offset at takeoff (difference between fix altitude and true MSL altitude in feet)
    pub takeoff_altitude_offset_ft: Option<i32>,

    /// Altitude offset at landing (difference between fix altitude and true MSL altitude in feet)
    pub landing_altitude_offset_ft: Option<i32>,

    /// Database timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Flight {
    /// Create a new flight with takeoff time
    pub fn new(device_address: String, takeoff_time: Option<DateTime<Utc>>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            device_id: None,
            device_address,
            device_address_type: AddressType::Unknown,
            takeoff_time,
            landing_time: None,
            departure_airport: None,
            arrival_airport: None,
            tow_aircraft_id: None,
            tow_release_height_msl: None,
            club_id: None,
            takeoff_altitude_offset_ft: None,
            landing_altitude_offset_ft: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_from_fix(fix: &Fix, takeoff_time: Option<DateTime<Utc>>) -> Self {
        let now = Utc::now();
        info!("Creating flight from fix: {:?}", fix);
        Self {
            id: Uuid::new_v4(),
            device_id: fix.device_id.into(),
            device_address: fix.device_address_hex(),
            device_address_type: fix.address_type,
            takeoff_time,
            landing_time: None,
            departure_airport: None,
            arrival_airport: None,
            tow_aircraft_id: None,
            tow_release_height_msl: None,
            club_id: fix.club_id,
            takeoff_altitude_offset_ft: None,
            landing_altitude_offset_ft: None,
            created_at: now,
            updated_at: now,
        }
    }
    /// Create a new flight for device already airborne (no takeoff time)
    pub fn new_airborne_from_fix(fix: &Fix) -> Self {
        Self::new_from_fix(fix, None)
    }

    /// Create a new flight with known takeoff time
    pub fn new_with_takeoff_from_fix(fix: &Fix, takeoff_time: DateTime<Utc>) -> Self {
        Self::new_from_fix(fix, Some(takeoff_time))
    }

    /// Check if the flight is still in progress (no landing time)
    pub fn is_in_progress(&self) -> bool {
        self.landing_time.is_none()
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
        self.tow_aircraft_id.is_some()
    }

    /// Generate a Google Earth compatible KML file for this flight
    /// Returns KML as a string containing the flight track with fixes
    pub async fn make_kml(
        &self,
        fixes_repo: &crate::fixes_repo::FixesRepository,
    ) -> Result<String> {
        // Get all fixes for this flight based on aircraft ID and time range
        let start_time = self.takeoff_time.unwrap_or(self.created_at);
        let end_time = self.landing_time.unwrap_or_else(Utc::now);

        let fixes = fixes_repo
            .get_fixes_for_aircraft_with_time_range(
                &self.device_id.unwrap_or(Uuid::nil()),
                start_time,
                end_time,
                None,
            )
            .await?;

        if fixes.is_empty() {
            return Ok(self.generate_empty_kml());
        }

        let mut kml = String::new();

        // KML header
        kml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        kml.push_str("<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n");
        kml.push_str("  <Document>\n");

        // Flight name and description
        let flight_name = format!("Flight {}", self.device_address);
        let aircraft_reg = fixes
            .first()
            .and_then(|f| f.registration.as_ref())
            .unwrap_or(&self.device_address);

        kml.push_str(&format!("    <name>{}</name>\n", flight_name));
        kml.push_str(&format!(
            "    <description>Flight track for aircraft {} from {} to {}</description>\n",
            aircraft_reg,
            start_time.format("%Y-%m-%d %H:%M:%S UTC"),
            end_time.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        // Style for flight track line
        kml.push_str("    <Style id=\"flightTrackStyle\">\n");
        kml.push_str("      <LineStyle>\n");
        kml.push_str("        <color>ff0000ff</color>\n"); // Red line
        kml.push_str("        <width>3</width>\n");
        kml.push_str("      </LineStyle>\n");
        kml.push_str("    </Style>\n");

        // Style for takeoff point
        kml.push_str("    <Style id=\"takeoffStyle\">\n");
        kml.push_str("      <IconStyle>\n");
        kml.push_str("        <color>ff00ff00</color>\n"); // Green
        kml.push_str("        <scale>1.2</scale>\n");
        kml.push_str("      </IconStyle>\n");
        kml.push_str("    </Style>\n");

        // Style for landing point
        kml.push_str("    <Style id=\"landingStyle\">\n");
        kml.push_str("      <IconStyle>\n");
        kml.push_str("        <color>ff0000ff</color>\n"); // Red
        kml.push_str("        <scale>1.2</scale>\n");
        kml.push_str("      </IconStyle>\n");
        kml.push_str("    </Style>\n");

        // Flight track as LineString
        kml.push_str("    <Placemark>\n");
        kml.push_str(&format!(
            "      <name>Flight Track - {}</name>\n",
            aircraft_reg
        ));
        kml.push_str("      <styleUrl>#flightTrackStyle</styleUrl>\n");
        kml.push_str("      <LineString>\n");
        kml.push_str("        <extrude>1</extrude>\n");
        kml.push_str("        <tessellate>1</tessellate>\n");
        kml.push_str("        <altitudeMode>absolute</altitudeMode>\n");
        kml.push_str("        <coordinates>\n");

        // Add coordinates for flight track (longitude,latitude,altitude_meters)
        for fix in &fixes {
            let altitude_meters = fix
                .altitude_feet
                .map(|alt| alt as f64 * 0.3048)
                .unwrap_or(0.0);
            kml.push_str(&format!(
                "          {},{},{}\n",
                fix.longitude, fix.latitude, altitude_meters
            ));
        }

        kml.push_str("        </coordinates>\n");
        kml.push_str("      </LineString>\n");
        kml.push_str("    </Placemark>\n");

        // Takeoff point
        if let Some(first_fix) = fixes.first() {
            kml.push_str("    <Placemark>\n");
            kml.push_str("      <name>Takeoff</name>\n");
            kml.push_str(&format!(
                "      <description>Takeoff at {} UTC</description>\n",
                start_time.format("%Y-%m-%d %H:%M:%S")
            ));
            kml.push_str("      <styleUrl>#takeoffStyle</styleUrl>\n");
            kml.push_str("      <Point>\n");
            kml.push_str("        <altitudeMode>absolute</altitudeMode>\n");
            let altitude_meters = first_fix
                .altitude_feet
                .map(|alt| alt as f64 * 0.3048)
                .unwrap_or(0.0);
            kml.push_str(&format!(
                "        <coordinates>{},{},{}</coordinates>\n",
                first_fix.longitude, first_fix.latitude, altitude_meters
            ));
            kml.push_str("      </Point>\n");
            kml.push_str("    </Placemark>\n");
        }

        // Landing point (if flight is complete)
        if self.landing_time.is_some()
            && let Some(last_fix) = fixes.last()
        {
            kml.push_str("    <Placemark>\n");
            kml.push_str("      <name>Landing</name>\n");
            kml.push_str(&format!(
                "      <description>Landing at {} UTC</description>\n",
                end_time.format("%Y-%m-%d %H:%M:%S")
            ));
            kml.push_str("      <styleUrl>#landingStyle</styleUrl>\n");
            kml.push_str("      <Point>\n");
            kml.push_str("        <altitudeMode>absolute</altitudeMode>\n");
            let altitude_meters = last_fix
                .altitude_feet
                .map(|alt| alt as f64 * 0.3048)
                .unwrap_or(0.0);
            kml.push_str(&format!(
                "        <coordinates>{},{},{}</coordinates>\n",
                last_fix.longitude, last_fix.latitude, altitude_meters
            ));
            kml.push_str("      </Point>\n");
            kml.push_str("    </Placemark>\n");
        }

        // KML footer
        kml.push_str("  </Document>\n");
        kml.push_str("</kml>\n");

        Ok(kml)
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
}

/// Diesel model for the flights table - used for database operations
#[derive(Debug, Clone, Queryable, Selectable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::flights)]
pub struct FlightModel {
    pub id: Uuid,
    pub device_address: String,
    pub takeoff_time: Option<DateTime<Utc>>,
    pub landing_time: Option<DateTime<Utc>>,
    pub departure_airport: Option<String>,
    pub arrival_airport: Option<String>,
    pub tow_aircraft_id: Option<String>,
    pub tow_release_height_msl: Option<i32>,
    pub club_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub device_address_type: AddressType,
    pub device_id: Option<Uuid>,
    pub takeoff_altitude_offset_ft: Option<i32>,
    pub landing_altitude_offset_ft: Option<i32>,
}

/// Insert model for new flights
#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::flights)]
pub struct NewFlightModel {
    pub id: Uuid,
    pub device_address: String,
    pub takeoff_time: Option<DateTime<Utc>>,
    pub landing_time: Option<DateTime<Utc>>,
    pub departure_airport: Option<String>,
    pub arrival_airport: Option<String>,
    pub tow_aircraft_id: Option<String>,
    pub tow_release_height_msl: Option<i32>,
    pub club_id: Option<Uuid>,
    pub device_address_type: AddressType,
    pub device_id: Option<Uuid>,
    pub takeoff_altitude_offset_ft: Option<i32>,
    pub landing_altitude_offset_ft: Option<i32>,
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
            departure_airport: flight.departure_airport,
            arrival_airport: flight.arrival_airport,
            tow_aircraft_id: flight.tow_aircraft_id,
            tow_release_height_msl: flight.tow_release_height_msl,
            club_id: flight.club_id,
            created_at: flight.created_at,
            updated_at: flight.updated_at,
            device_id: flight.device_id,
            takeoff_altitude_offset_ft: flight.takeoff_altitude_offset_ft,
            landing_altitude_offset_ft: flight.landing_altitude_offset_ft,
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
            departure_airport: flight.departure_airport,
            arrival_airport: flight.arrival_airport,
            tow_aircraft_id: flight.tow_aircraft_id,
            tow_release_height_msl: flight.tow_release_height_msl,
            club_id: flight.club_id,
            device_id: flight.device_id,
            takeoff_altitude_offset_ft: flight.takeoff_altitude_offset_ft,
            landing_altitude_offset_ft: flight.landing_altitude_offset_ft,
        }
    }
}

/// Conversion from FlightModel (database model) to Flight (API model)
impl From<FlightModel> for Flight {
    fn from(model: FlightModel) -> Self {
        Self {
            id: model.id,
            device_id: model.device_id,
            device_address: model.device_address,
            device_address_type: model.device_address_type,
            takeoff_time: model.takeoff_time,
            landing_time: model.landing_time,
            departure_airport: model.departure_airport,
            arrival_airport: model.arrival_airport,
            tow_aircraft_id: model.tow_aircraft_id,
            tow_release_height_msl: model.tow_release_height_msl,
            club_id: model.club_id,
            takeoff_altitude_offset_ft: model.takeoff_altitude_offset_ft,
            landing_altitude_offset_ft: model.landing_altitude_offset_ft,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
