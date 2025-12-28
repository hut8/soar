use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use tracing::error;
use uuid::Uuid;

use crate::actions::{DataListResponse, DataResponse};
use crate::auth::AuthUser;
use crate::watchlist::{AddToWatchlistRequest, UpdateWatchlistRequest};
use crate::watchlist_repo::WatchlistRepository;
use crate::web::AppState;

/// GET /data/watchlist - Get current user's watchlist
pub async fn get_watchlist(
    auth_user: AuthUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let repo = WatchlistRepository::new(state.pool);
    let user_id = auth_user.0.id;

    match repo.get_by_user(user_id).await {
        Ok(entries) => Json(DataListResponse { data: entries }).into_response(),
        Err(e) => {
            error!("Failed to get watchlist: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get watchlist").into_response()
        }
    }
}

/// POST /data/watchlist - Add aircraft to watchlist
pub async fn add_to_watchlist(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<AddToWatchlistRequest>,
) -> impl IntoResponse {
    let repo = WatchlistRepository::new(state.pool);
    let user_id = auth_user.0.id;

    match repo.add(user_id, req.aircraft_id, req.send_email).await {
        Ok(entry) => (StatusCode::CREATED, Json(DataResponse { data: entry })).into_response(),
        Err(e) => {
            error!("Failed to add to watchlist: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to add to watchlist",
            )
                .into_response()
        }
    }
}

/// PUT /data/watchlist/{aircraft_id} - Update email preference
pub async fn update_watchlist_email(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(aircraft_id): Path<Uuid>,
    Json(req): Json<UpdateWatchlistRequest>,
) -> impl IntoResponse {
    let repo = WatchlistRepository::new(state.pool);
    let user_id = auth_user.0.id;

    match repo
        .update_email_preference(user_id, aircraft_id, req.send_email)
        .await
    {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "Watchlist entry not found").into_response(),
        Err(e) => {
            error!("Failed to update watchlist: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update watchlist",
            )
                .into_response()
        }
    }
}

/// DELETE /data/watchlist/{aircraft_id} - Remove from watchlist
pub async fn remove_from_watchlist(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(aircraft_id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = WatchlistRepository::new(state.pool);
    let user_id = auth_user.0.id;

    match repo.remove(user_id, aircraft_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "Watchlist entry not found").into_response(),
        Err(e) => {
            error!("Failed to remove from watchlist: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to remove from watchlist",
            )
                .into_response()
        }
    }
}

/// DELETE /data/watchlist - Clear entire watchlist
pub async fn clear_watchlist(
    auth_user: AuthUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let repo = WatchlistRepository::new(state.pool);
    let user_id = auth_user.0.id;

    match repo.clear_user(user_id).await {
        Ok(_count) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            error!("Failed to clear watchlist: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to clear watchlist",
            )
                .into_response()
        }
    }
}
