use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::devices::AddressType;
use crate::flights::Flight;
use crate::ogn_aprs_aircraft::AircraftType;

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
    pub departure_airport_id: Option<i32>,
    pub arrival_airport: Option<String>,
    pub arrival_airport_id: Option<i32>,
    pub tow_aircraft_id: Option<String>,
    pub tow_release_height_msl: Option<i32>,
    pub club_id: Option<Uuid>,
    pub takeoff_altitude_offset_ft: Option<i32>,
    pub landing_altitude_offset_ft: Option<i32>,
    pub takeoff_runway_ident: Option<String>,
    pub landing_runway_ident: Option<String>,
    pub total_distance_meters: Option<f64>,
    pub maximum_displacement_meters: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Device information
    pub aircraft_model: Option<String>,
    pub registration: Option<String>,
    pub aircraft_type_ogn: Option<AircraftType>,
}

impl FlightView {
    /// Create a FlightView from a Flight with optional airport identifiers and device info
    pub fn from_flight(
        flight: Flight,
        departure_airport_ident: Option<String>,
        arrival_airport_ident: Option<String>,
        aircraft_model: Option<String>,
        registration: Option<String>,
        aircraft_type_ogn: Option<AircraftType>,
    ) -> Self {
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
            departure_airport: departure_airport_ident,
            departure_airport_id: flight.departure_airport_id,
            arrival_airport: arrival_airport_ident,
            arrival_airport_id: flight.arrival_airport_id,
            tow_aircraft_id: flight.tow_aircraft_id,
            tow_release_height_msl: flight.tow_release_height_msl,
            club_id: flight.club_id,
            takeoff_altitude_offset_ft: flight.takeoff_altitude_offset_ft,
            landing_altitude_offset_ft: flight.landing_altitude_offset_ft,
            takeoff_runway_ident: flight.takeoff_runway_ident,
            landing_runway_ident: flight.landing_runway_ident,
            total_distance_meters: flight.total_distance_meters,
            maximum_displacement_meters: flight.maximum_displacement_meters,
            created_at: flight.created_at,
            updated_at: flight.updated_at,
            aircraft_model,
            registration,
            aircraft_type_ogn,
        }
    }
}

impl From<Flight> for FlightView {
    fn from(flight: Flight) -> Self {
        Self::from_flight(flight, None, None, None, None, None)
    }
}
