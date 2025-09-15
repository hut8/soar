use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    clubs::Club,
    clubs_repo::{ClubWithLocationAndDistance, ClubWithLocationAndSimilarity},
};

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
    pub home_base_airport_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_meters: Option<f64>,
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
            home_base_airport_id: club.home_base_airport_id,
            created_at: club.created_at,
            updated_at: club.updated_at,
            similarity_score: None,
            distance_meters: None,
        }
    }
}

impl From<ClubWithLocationAndDistance> for ClubView {
    fn from(club: ClubWithLocationAndDistance) -> Self {
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
            home_base_airport_id: club.home_base_airport_id,
            created_at: club.created_at,
            updated_at: club.updated_at,
            similarity_score: None,
            distance_meters: club.distance_meters,
        }
    }
}

impl From<ClubWithLocationAndSimilarity> for ClubView {
    fn from(club: ClubWithLocationAndSimilarity) -> Self {
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
            home_base_airport_id: club.home_base_airport_id,
            created_at: club.created_at,
            updated_at: club.updated_at,
            similarity_score: club.similarity_score,
            distance_meters: None,
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
