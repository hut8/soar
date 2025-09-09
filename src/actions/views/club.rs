use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

use crate::clubs::Club;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClubView {
    pub id: Uuid,
    pub name: String,
    pub is_soaring: Option<bool>,
    pub location_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Club> for ClubView {
    fn from(club: Club) -> Self {
        Self {
            id: club.id,
            name: club.name,
            is_soaring: club.is_soaring,
            location_id: club.location_id,
            created_at: club.created_at,
            updated_at: club.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ClubWithRecentFlightsView {
    #[serde(flatten)]
    pub club: ClubView,
    pub recent_flights_count: Option<i64>,
    pub aircraft_count: Option<i64>,
    pub member_count: Option<i64>,
}
