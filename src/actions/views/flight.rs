use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::devices::AddressType;
use crate::flights::Flight;

/// Flight view for API responses with computed fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightView {
    pub id: Uuid,
    pub device_id: Option<Uuid>,
    pub device_address: String,
    pub device_address_type: AddressType,
    pub takeoff_time: Option<DateTime<Utc>>,
    pub landing_time: Option<DateTime<Utc>>,

    /// Duration of the flight in seconds (null if takeoff_time or landing_time is null)
    pub duration_seconds: Option<i64>,

    pub departure_airport: Option<String>,
    pub arrival_airport: Option<String>,
    pub tow_aircraft_id: Option<String>,
    pub tow_release_height_msl: Option<i32>,
    pub club_id: Option<Uuid>,
    pub takeoff_altitude_offset_ft: Option<i32>,
    pub landing_altitude_offset_ft: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Flight> for FlightView {
    fn from(flight: Flight) -> Self {
        // Calculate duration in seconds if both takeoff and landing times are available
        let duration_seconds = match (flight.takeoff_time, flight.landing_time) {
            (Some(takeoff), Some(landing)) => Some((landing - takeoff).num_seconds()),
            _ => None,
        };

        Self {
            id: flight.id,
            device_id: flight.device_id,
            device_address: flight.device_address,
            device_address_type: flight.device_address_type,
            takeoff_time: flight.takeoff_time,
            landing_time: flight.landing_time,
            duration_seconds,
            departure_airport: flight.departure_airport,
            arrival_airport: flight.arrival_airport,
            tow_aircraft_id: flight.tow_aircraft_id,
            tow_release_height_msl: flight.tow_release_height_msl,
            club_id: flight.club_id,
            takeoff_altitude_offset_ft: flight.takeoff_altitude_offset_ft,
            landing_altitude_offset_ft: flight.landing_altitude_offset_ft,
            created_at: flight.created_at,
            updated_at: flight.updated_at,
        }
    }
}
