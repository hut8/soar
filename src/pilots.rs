use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Linking table between flights and users (pilots)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightPilot {
    /// Unique identifier for this flight-pilot link
    pub id: Uuid,

    /// Flight ID (foreign key to flights table)
    pub flight_id: Uuid,

    /// User ID (foreign key to users table - the pilot)
    pub user_id: Uuid,

    /// Whether this pilot is the tow pilot
    pub is_tow_pilot: bool,

    /// Whether this pilot is a student
    pub is_student: bool,

    /// Whether this pilot is an instructor
    pub is_instructor: bool,

    /// Database timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl FlightPilot {
    /// Create a new flight-pilot link
    pub fn new(
        flight_id: Uuid,
        user_id: Uuid,
        is_tow_pilot: bool,
        is_student: bool,
        is_instructor: bool,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            flight_id,
            user_id,
            is_tow_pilot,
            is_student,
            is_instructor,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the role of this pilot in the flight
    pub fn role(&self) -> &'static str {
        if self.is_tow_pilot {
            "Tow Pilot"
        } else if self.is_instructor {
            "Instructor"
        } else if self.is_student {
            "Student"
        } else {
            "Pilot"
        }
    }
}

/// Diesel model for the flight_pilots table - used for database operations
#[derive(Debug, Clone, Queryable, Selectable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::flight_pilots)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FlightPilotModel {
    pub id: Uuid,
    pub flight_id: Uuid,
    pub user_id: Uuid,
    pub is_tow_pilot: bool,
    pub is_student: bool,
    pub is_instructor: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insert model for new flight-pilot links
#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::flight_pilots)]
pub struct NewFlightPilotModel {
    pub id: Uuid,
    pub flight_id: Uuid,
    pub user_id: Uuid,
    pub is_tow_pilot: bool,
    pub is_student: bool,
    pub is_instructor: bool,
}

/// Conversion from FlightPilot (API model) to FlightPilotModel (database model)
impl From<FlightPilot> for FlightPilotModel {
    fn from(flight_pilot: FlightPilot) -> Self {
        Self {
            id: flight_pilot.id,
            flight_id: flight_pilot.flight_id,
            user_id: flight_pilot.user_id,
            is_tow_pilot: flight_pilot.is_tow_pilot,
            is_student: flight_pilot.is_student,
            is_instructor: flight_pilot.is_instructor,
            created_at: flight_pilot.created_at,
            updated_at: flight_pilot.updated_at,
        }
    }
}

/// Conversion from FlightPilot (API model) to NewFlightPilotModel (insert model)
impl From<FlightPilot> for NewFlightPilotModel {
    fn from(flight_pilot: FlightPilot) -> Self {
        Self {
            id: flight_pilot.id,
            flight_id: flight_pilot.flight_id,
            user_id: flight_pilot.user_id,
            is_tow_pilot: flight_pilot.is_tow_pilot,
            is_student: flight_pilot.is_student,
            is_instructor: flight_pilot.is_instructor,
        }
    }
}

/// Conversion from FlightPilotModel (database model) to FlightPilot (API model)
impl From<FlightPilotModel> for FlightPilot {
    fn from(model: FlightPilotModel) -> Self {
        Self {
            id: model.id,
            flight_id: model.flight_id,
            user_id: model.user_id,
            is_tow_pilot: model.is_tow_pilot,
            is_student: model.is_student,
            is_instructor: model.is_instructor,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
