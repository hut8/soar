use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use tracing::error;
use uuid::Uuid;

use crate::auth::{AdminUser, AuthUser};
use crate::email::EmailService;
use crate::users::{CreatePilotRequest, SendInvitationRequest, UpdateUserRequest, User};
use crate::users_repo::UsersRepository;
use crate::web::AppState;

use super::{json_error, views::UserView};

#[derive(Debug, Deserialize)]
pub struct SetClubRequest {
    pub club_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UserQueryParams {
    pub limit: Option<i64>,
}

pub async fn get_all_users(
    _admin_user: AdminUser,
    State(state): State<AppState>,
    Query(params): Query<UserQueryParams>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    match users_repo.get_all(params.limit).await {
        Ok(users) => {
            let user_views: Vec<UserView> = users.into_iter().map(UserView::from).collect();
            Json(user_views).into_response()
        }
        Err(e) => {
            error!("Failed to get all users: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get users").into_response()
        }
    }
}

pub async fn get_user_by_id(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    // Check if user is admin or requesting their own info
    if !auth_user.0.is_admin && auth_user.0.id != user_id {
        return (StatusCode::FORBIDDEN, "Insufficient permissions").into_response();
    }

    match users_repo.get_by_id(user_id).await {
        Ok(Some(user)) => Json(UserView::from(user)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "User not found").into_response(),
        Err(e) => {
            error!("Failed to get user: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get user").into_response()
        }
    }
}

pub async fn update_user_by_id(
    _admin_user: AdminUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdateUserRequest>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    match users_repo.update_user(user_id, &payload).await {
        Ok(Some(user)) => Json(UserView::from(user)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "User not found").into_response(),
        Err(e) => {
            error!("Failed to update user: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to update user").into_response()
        }
    }
}

pub async fn delete_user_by_id(
    _admin_user: AdminUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    match users_repo.soft_delete_user(user_id).await {
        Ok(true) => (StatusCode::NO_CONTENT, "").into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "User not found").into_response(),
        Err(e) => {
            error!("Failed to delete user: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete user").into_response()
        }
    }
}

pub async fn get_users_by_club(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(club_id): Path<Uuid>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    // Check if user is admin or belongs to the same club
    if !auth_user.0.is_admin && auth_user.0.club_id != Some(club_id) {
        return (StatusCode::FORBIDDEN, "Insufficient permissions").into_response();
    }

    match users_repo.get_by_club_id(club_id).await {
        Ok(users) => {
            let user_views: Vec<UserView> = users.into_iter().map(UserView::from).collect();
            Json(user_views).into_response()
        }
        Err(e) => {
            error!("Failed to get users by club: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get users by club",
            )
                .into_response()
        }
    }
}

pub async fn set_user_club(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<SetClubRequest>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    // Users can only set their own club membership
    let user_id = auth_user.0.id;

    // Create update request with just the club_id
    let update_request = UpdateUserRequest {
        first_name: None,
        last_name: None,
        email: None,
        is_admin: None,
        club_id: Some(payload.club_id),
        email_verified: None,
    };

    match users_repo.update_user(user_id, &update_request).await {
        Ok(Some(user)) => Json(UserView::from(user)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "User not found").into_response(),
        Err(e) => {
            error!("Failed to set user club: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to set user club").into_response()
        }
    }
}

/// Create a pilot (user without email/password)
/// This is used when adding a pilot to the club roster who doesn't need login access yet
pub async fn create_pilot(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreatePilotRequest>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    // Verify that the user is admin or belongs to the same club
    if !auth_user.0.is_admin && auth_user.0.club_id != payload.club_id {
        return json_error(
            StatusCode::FORBIDDEN,
            "You must be a member of this club to add pilots",
        )
        .into_response();
    }

    let pilot = User::new_pilot(
        payload.first_name,
        payload.last_name,
        payload.is_licensed,
        payload.is_instructor,
        payload.is_tow_pilot,
        payload.is_examiner,
        payload.club_id,
    );

    match users_repo.create_pilot(pilot.clone()).await {
        Ok(_) => Json(UserView::from(pilot)).into_response(),
        Err(e) => {
            error!("Failed to create pilot: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create pilot").into_response()
        }
    }
}

/// Get pilots for a specific club (convenience endpoint that filters to only pilots)
pub async fn get_pilots_by_club(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(club_id): Path<Uuid>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    // Verify that the user is admin or belongs to the requested club
    if !auth_user.0.is_admin && auth_user.0.club_id != Some(club_id) {
        return json_error(
            StatusCode::FORBIDDEN,
            "You must be a member of this club to view its pilots",
        )
        .into_response();
    }

    match users_repo.get_pilots_by_club(club_id).await {
        Ok(pilots) => {
            let pilot_views: Vec<UserView> = pilots.into_iter().map(UserView::from).collect();
            Json(pilot_views).into_response()
        }
        Err(e) => {
            error!("Failed to get pilots for club {}: {}", club_id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get pilots for club",
            )
            .into_response()
        }
    }
}

/// Send email invitation to an existing pilot without email
/// This allows admins to add a pilot to the roster first, then send them an invitation later
pub async fn send_pilot_invitation(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<SendInvitationRequest>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    // Get the pilot
    let pilot = match users_repo.get_by_id(user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Pilot not found").into_response(),
        Err(e) => {
            error!("Failed to get pilot: {}", e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get pilot")
                .into_response();
        }
    };

    // Verify that the user is admin or belongs to the same club as the pilot
    if !auth_user.0.is_admin && auth_user.0.club_id != pilot.club_id {
        return json_error(
            StatusCode::FORBIDDEN,
            "You must be a member of this club to send invitations",
        )
        .into_response();
    }

    // Check if pilot already has an email
    if pilot.email.is_some() {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Pilot already has an email address",
        )
        .into_response();
    }

    // Set email and generate verification token
    let token = match users_repo
        .set_email_and_generate_token(user_id, &payload.email)
        .await
    {
        Ok(token) => token,
        Err(e) => {
            error!("Failed to set email and generate token: {}", e);
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to generate invitation",
            )
            .into_response();
        }
    };

    // Send invitation email
    match EmailService::new() {
        Ok(email_service) => {
            if let Err(e) = email_service
                .send_pilot_invitation_email(&payload.email, &pilot.full_name(), &token)
                .await
            {
                error!("Failed to send pilot invitation email: {}", e);
                sentry::capture_message(
                    &format!(
                        "Failed to send pilot invitation email to {}: {}",
                        payload.email, e
                    ),
                    sentry::Level::Error,
                );
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to send invitation email",
                )
                .into_response();
            }
        }
        Err(e) => {
            error!("Email service initialization failed: {}", e);
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Email service not configured",
            )
            .into_response();
        }
    }

    Json(serde_json::json!({
        "message": "Invitation email sent successfully"
    }))
    .into_response()
}
