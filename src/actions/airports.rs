use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use tracing::error;

use crate::airports_repo::AirportsRepository;
use crate::runways_repo::RunwaysRepository;
use crate::web::AppState;

use super::{json_error, views::AirportView};

#[derive(Debug, Deserialize)]
pub struct AirportBoundingBoxParams {
    pub nw_lat: f64, // Northwest corner latitude
    pub nw_lng: f64, // Northwest corner longitude
    pub se_lat: f64, // Southeast corner latitude
    pub se_lng: f64, // Southeast corner longitude
    pub limit: Option<i64>,
}

pub async fn get_airports_in_bounding_box(
    State(state): State<AppState>,
    Query(params): Query<AirportBoundingBoxParams>,
) -> impl IntoResponse {
    let airports_repo = AirportsRepository::new(state.pool.clone());
    let runways_repo = RunwaysRepository::new(state.pool);

    // Get airports within the bounding box
    match airports_repo
        .get_airports_in_bounding_box(
            params.nw_lat,
            params.nw_lng,
            params.se_lat,
            params.se_lng,
            params.limit,
        )
        .await
    {
        Ok(airports) => {
            // For each airport, get its runways and create the view
            let mut airport_views = Vec::new();

            for airport in airports {
                // Get runways for this airport
                let runways = match runways_repo.get_runways_by_airport_id(airport.id).await {
                    Ok(runways) => runways,
                    Err(e) => {
                        error!("Failed to get runways for airport {}: {}", airport.id, e);
                        // Continue processing other airports even if runways fail
                        Vec::new()
                    }
                };

                airport_views.push(AirportView::with_runways(airport, runways));
            }

            Json(airport_views).into_response()
        }
        Err(e) => {
            error!("Failed to get airports in bounding box: {}", e);
            json_error(
                StatusCode::BAD_REQUEST,
                &format!("Failed to get airports in bounding box: {}", e),
            )
            .into_response()
        }
    }
}
