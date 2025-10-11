use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A pilot in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pilot {
    /// Unique identifier for this pilot
    pub id: Uuid,

    /// Pilot's first name
    pub first_name: String,

    /// Pilot's last name
    pub last_name: String,

    /// Whether the pilot is licensed
    pub is_licensed: bool,

    /// Club that the pilot belongs to
    pub club_id: Option<Uuid>,

    /// Database timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Pilot {
    /// Create a new pilot
    pub fn new(
        first_name: String,
        last_name: String,
        is_licensed: bool,
        club_id: Option<Uuid>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            first_name,
            last_name,
            is_licensed,
            club_id,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the full name of the pilot
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }
}

/// Diesel model for the pilots table - used for database operations
#[derive(Debug, Clone, Queryable, Selectable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::pilots)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PilotModel {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub is_licensed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub club_id: Option<Uuid>,
}

/// Insert model for new pilots
#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::pilots)]
pub struct NewPilotModel {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub is_licensed: bool,
    pub club_id: Option<Uuid>,
}

/// Conversion from Pilot (API model) to PilotModel (database model)
impl From<Pilot> for PilotModel {
    fn from(pilot: Pilot) -> Self {
        Self {
            id: pilot.id,
            first_name: pilot.first_name,
            last_name: pilot.last_name,
            is_licensed: pilot.is_licensed,
            created_at: pilot.created_at,
            updated_at: pilot.updated_at,
            club_id: pilot.club_id,
        }
    }
}

/// Conversion from Pilot (API model) to NewPilotModel (insert model)
impl From<Pilot> for NewPilotModel {
    fn from(pilot: Pilot) -> Self {
        Self {
            id: pilot.id,
            first_name: pilot.first_name,
            last_name: pilot.last_name,
            is_licensed: pilot.is_licensed,
            club_id: pilot.club_id,
        }
    }
}

/// Conversion from PilotModel (database model) to Pilot (API model)
impl From<PilotModel> for Pilot {
    fn from(model: PilotModel) -> Self {
        Self {
            id: model.id,
            first_name: model.first_name,
            last_name: model.last_name,
            is_licensed: model.is_licensed,
            club_id: model.club_id,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

/// Linking table between flights and pilots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightPilot {
    /// Unique identifier for this flight-pilot link
    pub id: Uuid,

    /// Flight ID (foreign key to flights table)
    pub flight_id: Uuid,

    /// Pilot ID (foreign key to pilots table)
    pub pilot_id: Uuid,

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
        pilot_id: Uuid,
        is_tow_pilot: bool,
        is_student: bool,
        is_instructor: bool,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            flight_id,
            pilot_id,
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
    pub pilot_id: Uuid,
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
    pub pilot_id: Uuid,
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
            pilot_id: flight_pilot.pilot_id,
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
            pilot_id: flight_pilot.pilot_id,
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
            pilot_id: model.pilot_id,
            is_tow_pilot: model.is_tow_pilot,
            is_student: model.is_student,
            is_instructor: model.is_instructor,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
