use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use tracing::error;
use uuid::Uuid;

use crate::aircraft_registrations_repo::AircraftRegistrationsRepository;
use crate::clubs_repo::ClubsRepository;
use crate::web::AppState;

use super::{
    json_error,
    views::{ClubView, club::AircraftModelView},
};

pub async fn get_club_by_id(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let clubs_repo = ClubsRepository::new(state.pool.clone());
    let aircraft_repo = AircraftRegistrationsRepository::new(state.pool);

    match clubs_repo.get_by_id(id).await {
        Ok(Some(club)) => {
            // Get aircraft models for this club
            let aircraft_models = match aircraft_repo.get_aircraft_models_by_club_id(id).await {
                Ok(models) => models.into_iter().map(AircraftModelView::from).collect(),
                Err(e) => {
                    error!("Failed to get aircraft models for club {}: {}", id, e);
                    Vec::new() // Return empty vec if aircraft models query fails
                }
            };

            let mut club_view = ClubView::from(club);
            club_view.models = aircraft_models;
            Json(club_view).into_response()
        }
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Club not found").into_response(),
        Err(e) => {
            error!("Failed to get club by ID: {}", e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get club by ID",
            )
            .into_response()
        }
    }
}
