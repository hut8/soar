use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use tracing::error;
use uuid::Uuid;

use crate::clubs_repo::ClubsRepository;
use crate::web::AppState;

use super::{json_error, views::ClubView};

pub async fn get_club_by_id(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let clubs_repo = ClubsRepository::new(state.pool);

    match clubs_repo.get_by_id(id).await {
        Ok(Some(club)) => {
            let club_view = ClubView::from(club);
            Json(club_view).into_response()
        }
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Club not found").into_response(),
        Err(e) => {
            error!("Failed to get club by ID: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get club by ID").into_response()
        }
    }
}
