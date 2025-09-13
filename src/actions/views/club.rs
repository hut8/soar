use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{clubs::Club, locations::Point};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClubView {
    pub id: Uuid,
    pub name: String,
    pub location_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub street1: Option<String>,
    pub street2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip_code: Option<String>,
    pub region_code: Option<String>,
    pub county_mail_code: Option<String>,
    pub country_mail_code: Option<String>,
    pub base_location: Option<Point>,
}

impl From<Club> for ClubView {
    fn from(club: Club) -> Self {
        Self {
            id: club.id,
            name: club.name,
            location_id: club.location_id,
            street1: club.street1,
            street2: club.street2,
            city: club.city,
            state: club.state,
            zip_code: club.zip_code,
            region_code: club.region_code,
            county_mail_code: club.county_mail_code,
            country_mail_code: club.country_mail_code,
            base_location: club.base_location,
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
