use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
