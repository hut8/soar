use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::actions::json_error;
use crate::fixes::Fix;
use crate::fixes_repo::FixesRepository;
use crate::flights::Flight;
use crate::flights_repo::FlightsRepository;
use crate::web::AppState;

#[derive(Debug, Deserialize)]
pub struct FlightsQueryParams {
    pub club_id: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct FlightResponse {
    pub flight: Flight,
}

#[derive(Debug, Serialize)]
pub struct FlightFixesResponse {
    pub fixes: Vec<Fix>,
    pub count: usize,
}

/// Get a flight by its UUID
pub async fn get_flight_by_id(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool);

    match flights_repo.get_flight_by_id(id).await {
        Ok(Some(flight)) => Json(FlightResponse { flight }).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Flight not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get flight by ID {}: {}", id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get flight").into_response()
        }
    }
}

/// Get KML file for a flight
pub async fn get_flight_kml(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool.clone());
    let fixes_repo = FixesRepository::new(state.pool.clone());

    // First get the flight
    let flight = match flights_repo.get_flight_by_id(id).await {
        Ok(Some(flight)) => flight,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Flight not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get flight by ID {}: {}", id, e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get flight")
                .into_response();
        }
    };

    // Generate KML
    match flight.make_kml(&fixes_repo).await {
        Ok(kml_content) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                "content-type",
                "application/vnd.google-earth.kml+xml".parse().unwrap(),
            );

            // Generate filename based on flight info
            let filename = generate_kml_filename(&flight);
            headers.insert(
                "content-disposition",
                format!("attachment; filename=\"{}\"", filename)
                    .parse()
                    .unwrap(),
            );

            (StatusCode::OK, headers, kml_content).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to generate KML for flight {}: {}", id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to generate KML").into_response()
        }
    }
}

/// Get fixes for a flight by flight ID
pub async fn get_flight_fixes(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool.clone());
    let fixes_repo = FixesRepository::new(state.pool.clone());

    // First verify the flight exists
    let flight = match flights_repo.get_flight_by_id(id).await {
        Ok(Some(flight)) => flight,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Flight not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get flight by ID {}: {}", id, e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get flight")
                .into_response();
        }
    };

    // Get fixes for the flight based on device address and time range
    let start_time = flight.takeoff_time.unwrap_or(flight.created_at);
    let end_time = flight.landing_time.unwrap_or_else(chrono::Utc::now);

    match fixes_repo
        .get_fixes_for_aircraft_with_time_range(
            &flight.device_id.unwrap_or(Uuid::nil()),
            start_time,
            end_time,
            None, // No limit
        )
        .await
    {
        Ok(fixes) => {
            let count = fixes.len();
            Json(FlightFixesResponse { fixes, count }).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get fixes for flight {}: {}", id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get flight fixes",
            )
            .into_response()
        }
    }
}

/// Generate an appropriate filename for the KML download
fn generate_kml_filename(flight: &Flight) -> String {
    let base_name = if let Some(takeoff_time) = flight.takeoff_time {
        format!(
            "flight-{}-{}",
            flight.device_address,
            takeoff_time.format("%Y%m%d-%H%M")
        )
    } else {
        format!("flight-{}-{}", flight.device_address, flight.id)
    };

    format!("{}.kml", base_name)
}

/// Search flights by club ID, or return recent flights in progress
pub async fn search_flights(
    State(state): State<AppState>,
    Query(params): Query<FlightsQueryParams>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool);

    if let Some(_club_id) = params.club_id {
        // Club-based flight search would require joining with aircraft_registrations
        // For now, just return flights in progress
        match flights_repo.get_flights_in_progress().await {
            Ok(flights) => Json(flights).into_response(),
            Err(e) => {
                tracing::error!("Failed to get flights by club ID: {}", e);
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to get flights by club ID",
                )
                .into_response()
            }
        }
    } else {
        match flights_repo.get_flights_in_progress().await {
            Ok(flights) => Json(flights).into_response(),
            Err(e) => {
                tracing::error!("Failed to get recent flights: {}", e);
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to get recent flights",
                )
                .into_response()
            }
        }
    }
}

/// Get recent completed flights
pub async fn get_completed_flights(
    State(state): State<AppState>,
    Query(params): Query<FlightsQueryParams>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool);
    let limit = params.limit.unwrap_or(100);

    match flights_repo.get_completed_flights(limit).await {
        Ok(flights) => Json(flights).into_response(),
        Err(e) => {
            tracing::error!("Failed to get completed flights: {}", e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get completed flights",
            )
            .into_response()
        }
    }
}
