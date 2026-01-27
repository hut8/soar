use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::aircraft::{AddressType, address_type_from_str, address_type_to_str};
use crate::aircraft_types::AircraftCategory;
use crate::flights::{Flight, FlightState};

/// Helper struct for airport information when constructing FlightView
#[derive(Debug, Clone, Default)]
pub struct AirportInfo {
    pub ident: Option<String>,
    pub country: Option<String>,
}

/// Helper struct for geocoded location information when constructing FlightView
#[derive(Debug, Clone, Default)]
pub struct LocationInfo {
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
}

/// Helper struct for device information when constructing FlightView
#[derive(Debug, Clone, Default)]
pub struct AircraftInfo {
    pub aircraft_model: Option<String>,
    pub registration: Option<String>,
    pub aircraft_category: Option<AircraftCategory>,
    pub country_code: Option<String>,
}

/// Flight view for API responses with computed fields
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct FlightView {
    pub id: Uuid,
    pub aircraft_id: Option<Uuid>,
    pub device_address: String,
    #[serde(
        deserialize_with = "address_type_from_str",
        serialize_with = "address_type_to_str"
    )]
    pub device_address_type: AddressType,
    pub takeoff_time: Option<DateTime<Utc>>,
    pub landing_time: Option<DateTime<Utc>>,
    pub timed_out_at: Option<DateTime<Utc>>,

    /// Current state of the flight (active, complete, or timed_out)
    pub state: FlightState,

    /// Duration of the flight in seconds (null if takeoff_time or landing_time is null)
    #[ts(type = "number | null")]
    pub duration_seconds: Option<i64>,

    pub departure_airport: Option<String>,
    pub departure_airport_id: Option<i32>,
    pub departure_airport_country: Option<String>,
    pub arrival_airport: Option<String>,
    pub arrival_airport_id: Option<i32>,
    pub arrival_airport_country: Option<String>,

    // Geocoded location for flight start (from start_location_id)
    pub start_location_city: Option<String>,
    pub start_location_state: Option<String>,
    pub start_location_country: Option<String>,

    // Geocoded location for flight end (from end_location_id)
    pub end_location_city: Option<String>,
    pub end_location_state: Option<String>,
    pub end_location_country: Option<String>,

    pub club_id: Option<Uuid>,

    // Tow information (for glider flights)
    pub towed_by_aircraft_id: Option<Uuid>,
    pub towed_by_flight_id: Option<Uuid>,

    pub takeoff_altitude_offset_ft: Option<i32>,
    pub landing_altitude_offset_ft: Option<i32>,
    pub takeoff_runway_ident: Option<String>,
    pub landing_runway_ident: Option<String>,
    pub total_distance_meters: Option<f64>,
    pub maximum_displacement_meters: Option<f64>,
    pub runways_inferred: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Aircraft information
    pub aircraft_model: Option<String>,
    pub registration: Option<String>,
    pub aircraft_category: Option<AircraftCategory>,
    pub aircraft_country_code: Option<String>,

    // Latest altitude information (for active flights)
    pub latest_altitude_msl_feet: Option<i32>,
    pub latest_altitude_agl_feet: Option<i32>,

    // Latest fix timestamp (for active flights)
    pub latest_fix_timestamp: Option<DateTime<Utc>>,

    // Navigation to previous/next flights for the same device (chronologically by takeoff time)
    pub previous_flight_id: Option<Uuid>,
    pub next_flight_id: Option<Uuid>,

    // Flight callsign (from APRS packets)
    pub callsign: Option<String>,
}

impl FlightView {
    /// Create a FlightView from a Flight with optional airport and device info
    pub fn from_flight(
        flight: Flight,
        departure_airport: Option<AirportInfo>,
        arrival_airport: Option<AirportInfo>,
        device_info: Option<AircraftInfo>,
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
        device_info: Option<AircraftInfo>,
        start_location: Option<LocationInfo>,
        end_location: Option<LocationInfo>,
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
        let start_location = start_location.unwrap_or_default();
        let end_location = end_location.unwrap_or_default();

        Self {
            id: flight.id,
            aircraft_id: flight.aircraft_id,
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
            start_location_city: start_location.city,
            start_location_state: start_location.state,
            start_location_country: start_location.country,
            end_location_city: end_location.city,
            end_location_state: end_location.state,
            end_location_country: end_location.country,
            club_id: flight.club_id,
            towed_by_aircraft_id: flight.towed_by_aircraft_id,
            towed_by_flight_id: flight.towed_by_flight_id,
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
            aircraft_category: device_info.aircraft_category,
            aircraft_country_code: device_info.country_code,
            latest_altitude_msl_feet,
            latest_altitude_agl_feet,
            latest_fix_timestamp,
            previous_flight_id,
            next_flight_id,
            callsign: flight.callsign,
        }
    }

    /// Create a FlightView from a Flight with optional airport, device info, and altitude info
    #[allow(clippy::too_many_arguments)]
    pub fn from_flight_with_altitude(
        flight: Flight,
        departure_airport: Option<AirportInfo>,
        arrival_airport: Option<AirportInfo>,
        device_info: Option<AircraftInfo>,
        latest_altitude_msl_feet: Option<i32>,
        latest_altitude_agl_feet: Option<i32>,
        latest_fix_timestamp: Option<DateTime<Utc>>,
    ) -> Self {
        Self::from_flight_full(
            flight,
            departure_airport,
            arrival_airport,
            device_info,
            None,
            None,
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
