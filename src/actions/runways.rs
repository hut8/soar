use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use tracing::error;

use crate::actions::views::airport::RunwayView;
use crate::runways_repo::RunwaysRepository;
use crate::web::AppState;

use super::{DataResponse, json_error};

#[derive(Debug, Deserialize)]
pub struct RunwaySearchParams {
    // Bounding box parameters (required)
    pub west: Option<f64>,
    pub south: Option<f64>,
    pub east: Option<f64>,
    pub north: Option<f64>,
    pub limit: Option<i64>,
}

/// GET /data/runways
/// Fetch runways within a bounding box
pub async fn get_runways(
    State(state): State<AppState>,
    Query(params): Query<RunwaySearchParams>,
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

    let repo = RunwaysRepository::new(state.pool.clone());

    // Record metrics
    metrics::counter!("api.runways.requests_total").increment(1);
    let start = std::time::Instant::now();

    match repo
        .get_runways_in_bbox(west, south, east, north, params.limit)
        .await
    {
        Ok(runways) => {
            let duration_ms = start.elapsed().as_millis() as f64;
            metrics::histogram!("api.runways.duration_ms").record(duration_ms);
            metrics::gauge!("api.runways.results_count").set(runways.len() as f64);

            // Convert to RunwayView which includes calculated polyline
            let runway_views: Vec<RunwayView> = runways.into_iter().map(RunwayView::from).collect();

            Json(DataResponse { data: runway_views }).into_response()
        }
        Err(e) => {
            error!("Failed to fetch runways: {}", e);
            metrics::counter!("api.runways.errors_total").increment(1);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to fetch runways: {}", e),
            )
            .into_response()
        }
    }
}
