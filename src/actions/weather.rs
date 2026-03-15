use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use tracing::warn;
use ts_rs::TS;

use super::{DataResponse, json_error};
use crate::web::AppState;

#[derive(Debug, Deserialize)]
pub struct WeatherParams {
    pub lat: f64,
    pub lon: f64,
}

/// Open-Meteo API response (subset of fields we care about)
#[derive(Debug, Deserialize)]
struct OpenMeteoResponse {
    current: OpenMeteoCurrent,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoCurrent {
    surface_pressure: f64,
    pressure_msl: f64,
    temperature_2m: f64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct WeatherResponse {
    /// Surface pressure in hPa
    pub surface_pressure: f64,
    /// Mean sea level pressure in hPa
    pub pressure_msl: f64,
    /// Temperature at 2m above ground in Celsius
    pub temperature_2m: f64,
}

/// Handler for GET /data/weather
///
/// Fetches current weather conditions from Open-Meteo for the given coordinates.
/// Used primarily for setting the altimeter (QNH).
pub async fn get_weather(
    State(state): State<AppState>,
    Query(params): Query<WeatherParams>,
) -> impl IntoResponse {
    if !(-90.0..=90.0).contains(&params.lat) || !(-180.0..=180.0).contains(&params.lon) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Invalid coordinates: lat must be -90..90, lon must be -180..180",
        )
        .into_response();
    }

    let response = match state
        .http_client
        .get("https://api.open-meteo.com/v1/forecast")
        .query(&[
            ("latitude", params.lat.to_string()),
            ("longitude", params.lon.to_string()),
            (
                "current",
                "surface_pressure,pressure_msl,temperature_2m".to_string(),
            ),
            ("wind_speed_unit", "kn".to_string()),
            ("temperature_unit", "celsius".to_string()),
        ])
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("Weather API request failed: {}", e);
            return json_error(StatusCode::BAD_GATEWAY, "Weather service unavailable")
                .into_response();
        }
    };

    if !response.status().is_success() {
        warn!("Weather API returned status {}", response.status());
        return json_error(StatusCode::BAD_GATEWAY, "Weather service returned an error")
            .into_response();
    }

    match response.json::<OpenMeteoResponse>().await {
        Ok(data) => {
            let weather = WeatherResponse {
                surface_pressure: data.current.surface_pressure,
                pressure_msl: data.current.pressure_msl,
                temperature_2m: data.current.temperature_2m,
            };
            (StatusCode::OK, axum::Json(DataResponse { data: weather })).into_response()
        }
        Err(e) => {
            warn!("Failed to parse weather response: {}", e);
            json_error(StatusCode::BAD_GATEWAY, "Failed to parse weather data").into_response()
        }
    }
}
