use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use tracing::error;

use crate::auth::AuthUser;
use crate::user_fixes::{CreateUserFixRequest, CreateUserFixResponse};
use crate::user_fixes_repo::UserFixesRepository;
use crate::web::AppState;

use super::json_error;

/// POST /data/user-fix - Record a user's location
pub async fn create_user_fix(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Json(request): Json<CreateUserFixRequest>,
) -> impl IntoResponse {
    let user_id = auth_user.0.id;
    let repo = UserFixesRepository::new(state.pool);

    // Validate coordinates
    if request.latitude < -90.0 || request.latitude > 90.0 {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Latitude must be between -90 and 90",
        )
        .into_response();
    }
    if request.longitude < -180.0 || request.longitude > 180.0 {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Longitude must be between -180 and 180",
        )
        .into_response();
    }

    // Validate heading if provided
    if let Some(heading) = request.heading
        && !(0.0..=360.0).contains(&heading)
    {
        return json_error(StatusCode::BAD_REQUEST, "Heading must be between 0 and 360")
            .into_response();
    }

    match repo
        .create(
            user_id,
            request.latitude,
            request.longitude,
            request.heading,
            request.raw.clone(),
        )
        .await
    {
        Ok(fix) => {
            let response: CreateUserFixResponse = fix.into();
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            error!("Failed to create user fix: {}", e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to record location",
            )
            .into_response()
        }
    }
}
