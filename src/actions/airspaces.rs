use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use tracing::error;

use crate::airspaces_repo::AirspacesRepository;
use crate::web::AppState;

use super::{DataResponse, json_error};

#[derive(Debug, Deserialize)]
pub struct AirspaceSearchParams {
    // Bounding box parameters (required)
    pub west: Option<f64>,
    pub south: Option<f64>,
    pub east: Option<f64>,
    pub north: Option<f64>,
    pub limit: Option<i64>,
}

/// GET /data/airspaces
/// Fetch airspaces within a bounding box
pub async fn get_airspaces(
    State(state): State<AppState>,
    Query(params): Query<AirspaceSearchParams>,
) -> impl IntoResponse {
    // Validate bounding box parameters
    let (west, south, east, north) = match (params.west, params.south, params.east, params.north) {
        (Some(w), Some(s), Some(e), Some(n)) => (w, s, e, n),
        _ => {
            return json_error(
                StatusCode::BAD_REQUEST,
                "Missing required bounding box parameters: west, south, east, north",
            )
            .into_response();
        }
    };

    // Validate coordinate ranges
    if !(-180.0..=180.0).contains(&west) || !(-180.0..=180.0).contains(&east) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Longitude values must be between -180 and 180",
        )
        .into_response();
    }

    if !(-90.0..=90.0).contains(&south) || !(-90.0..=90.0).contains(&north) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Latitude values must be between -90 and 90",
        )
        .into_response();
    }

    let repo = AirspacesRepository::new(state.pool.clone());

    // Record metrics
    metrics::counter!("api.airspaces.requests_total").increment(1);
    let start = std::time::Instant::now();

    match repo
        .get_airspaces_in_bbox(west, south, east, north, params.limit)
        .await
    {
        Ok(airspaces) => {
            let duration_ms = start.elapsed().as_millis() as f64;
            metrics::histogram!("api.airspaces.duration_ms").record(duration_ms);
            metrics::gauge!("api.airspaces.results_count").set(airspaces.len() as f64);

            // Return GeoJSON FeatureCollection wrapped in DataResponse
            let feature_collection = serde_json::json!({
                "type": "FeatureCollection",
                "features": airspaces,
            });

            Json(DataResponse {
                data: feature_collection,
            })
            .into_response()
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch airspaces");
            metrics::counter!("api.airspaces.errors_total").increment(1);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to fetch airspaces: {}", e),
            )
            .into_response()
        }
    }
}
