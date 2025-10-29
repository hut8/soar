use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::devices::AddressType;
use crate::flights::{Flight, FlightState};
use crate::ogn_aprs_aircraft::AircraftType;

/// Helper struct for airport information when constructing FlightView
#[derive(Debug, Clone, Default)]
pub struct AirportInfo {
    pub ident: Option<String>,
    pub country: Option<String>,
}

/// Helper struct for device information when constructing FlightView
#[derive(Debug, Clone, Default)]
pub struct DeviceInfo {
    pub aircraft_model: Option<String>,
    pub registration: Option<String>,
    pub aircraft_type_ogn: Option<AircraftType>,
}

/// Flight view for API responses with computed fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightView {
    pub id: Uuid,
    pub device_id: Option<Uuid>,
    pub device_address: String,
    pub device_address_type: AddressType,
    pub takeoff_time: Option<DateTime<Utc>>,
    pub landing_time: Option<DateTime<Utc>>,
    pub timed_out_at: Option<DateTime<Utc>>,

    /// Current state of the flight (active, complete, or timed_out)
    pub state: FlightState,

    /// Duration of the flight in seconds (null if takeoff_time or landing_time is null)
    pub duration_seconds: Option<i64>,

    pub departure_airport: Option<String>,
    pub departure_airport_id: Option<i32>,
    pub departure_airport_country: Option<String>,
    pub arrival_airport: Option<String>,
    pub arrival_airport_id: Option<i32>,
    pub arrival_airport_country: Option<String>,
    pub tow_release_height_msl: Option<i32>,
    pub club_id: Option<Uuid>,
    pub takeoff_altitude_offset_ft: Option<i32>,
    pub landing_altitude_offset_ft: Option<i32>,
    pub takeoff_runway_ident: Option<String>,
    pub landing_runway_ident: Option<String>,
    pub total_distance_meters: Option<f64>,
    pub maximum_displacement_meters: Option<f64>,
    pub runways_inferred: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Device information
    pub aircraft_model: Option<String>,
    pub registration: Option<String>,
    pub aircraft_type_ogn: Option<AircraftType>,

    // Latest altitude information (for active flights)
    pub latest_altitude_msl_feet: Option<i32>,
    pub latest_altitude_agl_feet: Option<i32>,

    // Latest fix timestamp (for active flights)
    pub latest_fix_timestamp: Option<DateTime<Utc>>,

    // Navigation to previous/next flights for the same device (chronologically by takeoff time)
    pub previous_flight_id: Option<Uuid>,
    pub next_flight_id: Option<Uuid>,
}

impl FlightView {
    /// Create a FlightView from a Flight with optional airport and device info
    pub fn from_flight(
        flight: Flight,
        departure_airport: Option<AirportInfo>,
        arrival_airport: Option<AirportInfo>,
        device_info: Option<DeviceInfo>,
    ) -> Self {
        Self::from_flight_with_altitude(
            flight,
            departure_airport,
            arrival_airport,
            device_info,
            None,
            None,
            None,
        )
    }

    /// Create a FlightView from a Flight with all optional fields
    #[allow(clippy::too_many_arguments)]
    pub fn from_flight_full(
        flight: Flight,
        departure_airport: Option<AirportInfo>,
        arrival_airport: Option<AirportInfo>,
        device_info: Option<DeviceInfo>,
        latest_altitude_msl_feet: Option<i32>,
        latest_altitude_agl_feet: Option<i32>,
        latest_fix_timestamp: Option<DateTime<Utc>>,
        previous_flight_id: Option<Uuid>,
        next_flight_id: Option<Uuid>,
    ) -> Self {
        // Calculate state before moving any fields
        let state = flight.state();

        // Calculate duration in seconds if both takeoff and landing times are available
        let duration_seconds = match (flight.takeoff_time, flight.landing_time) {
            (Some(takeoff), Some(landing)) => Some((landing - takeoff).num_seconds()),
            _ => None,
        };

        let departure_airport = departure_airport.unwrap_or_default();
        let arrival_airport = arrival_airport.unwrap_or_default();
        let device_info = device_info.unwrap_or_default();

        Self {
            id: flight.id,
            device_id: flight.device_id,
            device_address: flight.device_address,
            device_address_type: flight.device_address_type,
            takeoff_time: flight.takeoff_time,
            landing_time: flight.landing_time,
            timed_out_at: flight.timed_out_at,
            state,
            duration_seconds,
            departure_airport: departure_airport.ident,
            departure_airport_id: flight.departure_airport_id,
            departure_airport_country: departure_airport.country,
            arrival_airport: arrival_airport.ident,
            arrival_airport_id: flight.arrival_airport_id,
            arrival_airport_country: arrival_airport.country,
            tow_release_height_msl: flight.tow_release_height_msl,
            club_id: flight.club_id,
            takeoff_altitude_offset_ft: flight.takeoff_altitude_offset_ft,
            landing_altitude_offset_ft: flight.landing_altitude_offset_ft,
            takeoff_runway_ident: flight.takeoff_runway_ident,
            landing_runway_ident: flight.landing_runway_ident,
            total_distance_meters: flight.total_distance_meters,
            maximum_displacement_meters: flight.maximum_displacement_meters,
            runways_inferred: flight.runways_inferred,
            created_at: flight.created_at,
            updated_at: flight.updated_at,
            aircraft_model: device_info.aircraft_model,
            registration: device_info.registration,
            aircraft_type_ogn: device_info.aircraft_type_ogn,
            latest_altitude_msl_feet,
            latest_altitude_agl_feet,
            latest_fix_timestamp,
            previous_flight_id,
            next_flight_id,
        }
    }

    /// Create a FlightView from a Flight with optional airport, device info, and altitude info
    #[allow(clippy::too_many_arguments)]
    pub fn from_flight_with_altitude(
        flight: Flight,
        departure_airport: Option<AirportInfo>,
        arrival_airport: Option<AirportInfo>,
        device_info: Option<DeviceInfo>,
        latest_altitude_msl_feet: Option<i32>,
        latest_altitude_agl_feet: Option<i32>,
        latest_fix_timestamp: Option<DateTime<Utc>>,
    ) -> Self {
        Self::from_flight_full(
            flight,
            departure_airport,
            arrival_airport,
            device_info,
            latest_altitude_msl_feet,
            latest_altitude_agl_feet,
            latest_fix_timestamp,
            None,
            None,
        )
    }
}

impl From<Flight> for FlightView {
    fn from(flight: Flight) -> Self {
        Self::from_flight(flight, None, None, None)
    }
}
