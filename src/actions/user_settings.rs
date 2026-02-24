use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde_json::Value as JsonValue;
use tracing::error;

use crate::auth::AuthUser;
use crate::users_repo::UsersRepository;
use crate::web::AppState;

/// GET /api/user/settings - Get current user's settings
pub async fn get_user_settings(
    auth_user: AuthUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    // Get the current user's ID from the authenticated user
    let user_id = auth_user.0.id;

    match users_repo.get_by_id(user_id).await {
        Ok(Some(user)) => Json(user.settings).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "User not found").into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get user settings");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get user settings",
            )
                .into_response()
        }
    }
}

/// PUT /api/user/settings - Update current user's settings
pub async fn update_user_settings(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Json(settings): Json<JsonValue>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    // Get the current user's ID from the authenticated user
    let user_id = auth_user.0.id;

    match users_repo.update_user_settings(user_id, settings).await {
        Ok(true) => (StatusCode::NO_CONTENT, "").into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "User not found").into_response(),
        Err(e) => {
            error!(error = %e, "Failed to update user settings");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update user settings",
            )
                .into_response()
        }
    }
}
