use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use tracing::error;
use uuid::Uuid;

use crate::aircraft_registrations_repo::AircraftRegistrationsRepository;
use crate::web::AppState;

use super::views::{AircraftView, club::AircraftModelView};

pub async fn get_aircraft_by_club(
    State(state): State<AppState>,
    Path(club_id): Path<Uuid>,
) -> impl IntoResponse {
    let aircraft_repo = AircraftRegistrationsRepository::new(state.pool);

    match aircraft_repo.get_aircraft_with_models_by_club_id(club_id).await {
        Ok(aircraft_with_models) => {
            let aircraft_views: Vec<AircraftView> = aircraft_with_models
                .into_iter()
                .map(|(aircraft, model)| {
                    let mut view = AircraftView::from(aircraft);
                    view.club_id = Some(club_id); // Set the club_id in the view
                    view.model = model.map(AircraftModelView::from); // Add model data if available
                    view
                })
                .collect();
            Json(aircraft_views).into_response()
        }
        Err(e) => {
            error!("Failed to get aircraft by club: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get aircraft by club",
            )
                .into_response()
        }
    }
}
