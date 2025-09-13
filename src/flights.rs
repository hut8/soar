use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use diesel::prelude::*;

/// A flight representing a complete takeoff to landing sequence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flight {
    /// Unique identifier for this flight
    pub id: Uuid,

    /// Aircraft identifier (hex ID like "39D304")
    pub aircraft_id: String,

    /// Takeoff time (required)
    pub takeoff_time: DateTime<Utc>,

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

    /// Database timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Flight {
    /// Create a new flight with takeoff time
    pub fn new(aircraft_id: String, takeoff_time: DateTime<Utc>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            aircraft_id,
            takeoff_time,
            landing_time: None,
            departure_airport: None,
            arrival_airport: None,
            tow_aircraft_id: None,
            tow_release_height_msl: None,
            club_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if the flight is still in progress (no landing time)
    pub fn is_in_progress(&self) -> bool {
        self.landing_time.is_none()
    }

    /// Get flight duration if landed, otherwise duration from takeoff to now
    pub fn duration(&self) -> chrono::Duration {
        let end_time = self.landing_time.unwrap_or_else(Utc::now);
        end_time - self.takeoff_time
    }

    /// Check if this flight used a tow plane
    pub fn has_tow(&self) -> bool {
        self.tow_aircraft_id.is_some()
    }
}

/// Diesel model for the flights table - used for database operations
#[derive(Debug, Clone, Queryable, Selectable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::flights)]
pub struct FlightModel {
    pub id: Uuid,
    pub aircraft_id: String,
    pub takeoff_time: DateTime<Utc>,
    pub landing_time: Option<DateTime<Utc>>,
    pub departure_airport: Option<String>,
    pub arrival_airport: Option<String>,
    pub tow_aircraft_id: Option<String>,
    pub tow_release_height_msl: Option<i32>,
    pub club_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insert model for new flights
#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::flights)]
pub struct NewFlightModel {
    pub id: Uuid,
    pub aircraft_id: String,
    pub takeoff_time: DateTime<Utc>,
    pub landing_time: Option<DateTime<Utc>>,
    pub departure_airport: Option<String>,
    pub arrival_airport: Option<String>,
    pub tow_aircraft_id: Option<String>,
    pub tow_release_height_msl: Option<i32>,
    pub club_id: Option<Uuid>,
}

/// Conversion from Flight (API model) to FlightModel (database model)
impl From<Flight> for FlightModel {
    fn from(flight: Flight) -> Self {
        Self {
            id: flight.id,
            aircraft_id: flight.aircraft_id,
            takeoff_time: flight.takeoff_time,
            landing_time: flight.landing_time,
            departure_airport: flight.departure_airport,
            arrival_airport: flight.arrival_airport,
            tow_aircraft_id: flight.tow_aircraft_id,
            tow_release_height_msl: flight.tow_release_height_msl,
            club_id: flight.club_id,
            created_at: flight.created_at,
            updated_at: flight.updated_at,
        }
    }
}

/// Conversion from Flight (API model) to NewFlightModel (insert model)
impl From<Flight> for NewFlightModel {
    fn from(flight: Flight) -> Self {
        Self {
            id: flight.id,
            aircraft_id: flight.aircraft_id,
            takeoff_time: flight.takeoff_time,
            landing_time: flight.landing_time,
            departure_airport: flight.departure_airport,
            arrival_airport: flight.arrival_airport,
            tow_aircraft_id: flight.tow_aircraft_id,
            tow_release_height_msl: flight.tow_release_height_msl,
            club_id: flight.club_id,
        }
    }
}

/// Conversion from FlightModel (database model) to Flight (API model)
impl From<FlightModel> for Flight {
    fn from(model: FlightModel) -> Self {
        Self {
            id: model.id,
            aircraft_id: model.aircraft_id,
            takeoff_time: model.takeoff_time,
            landing_time: model.landing_time,
            departure_airport: model.departure_airport,
            arrival_airport: model.arrival_airport,
            tow_aircraft_id: model.tow_aircraft_id,
            tow_release_height_msl: model.tow_release_height_msl,
            club_id: model.club_id,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
