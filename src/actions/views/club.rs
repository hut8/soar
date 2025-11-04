use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    clubs::Club,
    clubs_repo::{ClubWithLocationAndDistance, ClubWithLocationAndSimilarity},
    faa::aircraft_model_repo::AircraftModelRecord,
    locations::{Location, Point},
};

/// Helper function to convert Option<BigDecimal> to Option<f64>
fn bigdecimal_to_f64(value: Option<BigDecimal>) -> Option<f64> {
    value.and_then(|v| v.to_f64())
}

/// Helper function to create a Location object from individual address fields
/// Returns None if location_id is None or all address fields are empty
#[allow(clippy::too_many_arguments)]
fn create_location_from_fields(
    location_id: Option<Uuid>,
    street1: Option<String>,
    street2: Option<String>,
    city: Option<String>,
    state: Option<String>,
    zip_code: Option<String>,
    region_code: Option<String>,
    country_mail_code: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
) -> Option<Location> {
    location_id.map(|id| Location {
        id,
        street1,
        street2,
        city,
        state,
        zip_code,
        region_code,
        country_mail_code,
        geolocation: if let (Some(lat), Some(lng)) = (latitude, longitude) {
            Some(Point::new(lat, lng))
        } else {
            None
        },
        created_at,
        updated_at,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AircraftModelView {
    pub manufacturer_code: String,
    pub model_code: String,
    pub series_code: String,
    pub manufacturer_name: String,
    pub model_name: String,
    pub aircraft_type: Option<String>,
    pub engine_type: Option<String>,
    pub aircraft_category: Option<String>,
    pub builder_certification: Option<String>,
    pub number_of_engines: Option<i16>,
    pub number_of_seats: Option<i16>,
    pub weight_class: Option<String>,
    pub cruising_speed: Option<i16>,
    pub type_certificate_data_sheet: Option<String>,
    pub type_certificate_data_holder: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<AircraftModelRecord> for AircraftModelView {
    fn from(model: AircraftModelRecord) -> Self {
        Self {
            manufacturer_code: model.manufacturer_code,
            model_code: model.model_code,
            series_code: model.series_code,
            manufacturer_name: model.manufacturer_name,
            model_name: model.model_name,
            aircraft_type: model.aircraft_type,
            engine_type: model.engine_type,
            aircraft_category: model.aircraft_category,
            builder_certification: model.builder_certification,
            number_of_engines: model.number_of_engines,
            number_of_seats: model.number_of_seats,
            weight_class: model.weight_class,
            cruising_speed: model.cruising_speed,
            type_certificate_data_sheet: model.type_certificate_data_sheet,
            type_certificate_data_holder: model.type_certificate_data_holder,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClubView {
    pub id: Uuid,
    pub name: String,
    pub home_base_airport_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home_base_airport_ident: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_meters: Option<f64>,
}

impl From<Club> for ClubView {
    fn from(club: Club) -> Self {
        let location = create_location_from_fields(
            club.location_id,
            club.street1,
            club.street2,
            club.city,
            club.state,
            club.zip_code,
            club.region_code,
            club.country_mail_code,
            club.base_location.as_ref().map(|p| p.latitude),
            club.base_location.as_ref().map(|p| p.longitude),
            club.created_at,
            club.updated_at,
        );

        Self {
            id: club.id,
            name: club.name,
            home_base_airport_id: club.home_base_airport_id,
            home_base_airport_ident: club.home_base_airport_ident,
            location,
            created_at: club.created_at,
            updated_at: club.updated_at,
            similarity_score: None,
            distance_meters: None,
        }
    }
}

impl From<ClubWithLocationAndDistance> for ClubView {
    fn from(club: ClubWithLocationAndDistance) -> Self {
        let location = create_location_from_fields(
            club.location_id,
            club.street1,
            club.street2,
            club.city,
            club.state,
            club.zip_code,
            club.region_code,
            club.country_mail_code,
            bigdecimal_to_f64(club.latitude),
            bigdecimal_to_f64(club.longitude),
            club.created_at,
            club.updated_at,
        );

        Self {
            id: club.id,
            name: club.name,
            home_base_airport_id: club.home_base_airport_id,
            home_base_airport_ident: club.home_base_airport_ident,
            location,
            created_at: club.created_at,
            updated_at: club.updated_at,
            similarity_score: None,
            distance_meters: club.distance_meters,
        }
    }
}

impl From<ClubWithLocationAndSimilarity> for ClubView {
    fn from(club: ClubWithLocationAndSimilarity) -> Self {
        let location = create_location_from_fields(
            club.location_id,
            club.street1,
            club.street2,
            club.city,
            club.state,
            club.zip_code,
            club.region_code,
            club.country_mail_code,
            bigdecimal_to_f64(club.latitude),
            bigdecimal_to_f64(club.longitude),
            club.created_at,
            club.updated_at,
        );

        Self {
            id: club.id,
            name: club.name,
            home_base_airport_id: club.home_base_airport_id,
            home_base_airport_ident: club.home_base_airport_ident,
            location,
            created_at: club.created_at,
            updated_at: club.updated_at,
            similarity_score: club.similarity_score.map(|s| s as f64),
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
