use axum::{extract::Query, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use tracing::warn;
use ts_rs::TS;

use super::{DataResponse, json_error};
use crate::geocoding::Geocoder;

#[derive(Debug, Deserialize)]
pub struct ReverseGeocodeParams {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ReverseGeocodeResponse {
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub display_name: String,
}

/// Handler for GET /data/geocode/reverse
///
/// Reverse geocodes coordinates to a place name using Pelias.
pub async fn reverse_geocode(Query(params): Query<ReverseGeocodeParams>) -> impl IntoResponse {
    // Validate coordinate ranges
    if !(-90.0..=90.0).contains(&params.lat) || !(-180.0..=180.0).contains(&params.lon) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Invalid coordinates: lat must be -90..90, lon must be -180..180",
        )
        .into_response();
    }

    let geocoder = match Geocoder::new_realtime_flight_tracking() {
        Ok(g) => g,
        Err(e) => {
            warn!("Pelias not configured for reverse geocoding: {}", e);
            return json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Geocoding service unavailable",
            )
            .into_response();
        }
    };

    match geocoder.reverse_geocode(params.lat, params.lon).await {
        Ok(result) => {
            let response = ReverseGeocodeResponse {
                city: result.city,
                state: result.state,
                country: result.country,
                display_name: result.display_name,
            };
            (StatusCode::OK, axum::Json(DataResponse { data: response })).into_response()
        }
        Err(e) => {
            warn!(
                "Reverse geocoding failed for ({}, {}): {}",
                params.lat, params.lon, e
            );
            json_error(
                StatusCode::NOT_FOUND,
                "No location found for the given coordinates",
            )
            .into_response()
        }
    }
}
