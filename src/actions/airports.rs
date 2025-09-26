use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use tracing::error;

use crate::airports_repo::AirportsRepository;
use crate::runways_repo::RunwaysRepository;
use crate::web::AppState;

use super::{json_error, views::AirportView};


pub async fn get_airport_by_id(
    State(state): State<AppState>,
    axum::extract::Path(airport_id): axum::extract::Path<i32>,
) -> impl IntoResponse {
    let airports_repo = AirportsRepository::new(state.pool.clone());
    let runways_repo = RunwaysRepository::new(state.pool);

    // Get airport by ID
    match airports_repo.get_airport_by_id(airport_id).await {
        Ok(Some(airport)) => {
            // Get runways for this airport
            let runways = match runways_repo.get_runways_by_airport_id(airport.id).await {
                Ok(runways) => runways,
                Err(e) => {
                    error!("Failed to get runways for airport {}: {}", airport.id, e);
                    Vec::new()
                }
            };

            let airport_view = AirportView::with_runways(airport, runways);
            Json(airport_view).into_response()
        }
        Ok(None) => json_error(
            StatusCode::NOT_FOUND,
            &format!("Airport with ID {} not found", airport_id),
        )
        .into_response(),
        Err(e) => {
            error!("Failed to get airport by ID {}: {}", airport_id, e);
            json_error(
                StatusCode::BAD_REQUEST,
                &format!("Failed to get airport by ID {}: {}", airport_id, e),
            )
            .into_response()
        }
    }
}
