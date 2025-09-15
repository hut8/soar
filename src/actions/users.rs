use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use tracing::error;
use uuid::Uuid;

use crate::auth::{AdminUser, AuthUser};
use crate::users::UpdateUserRequest;
use crate::users_repo::UsersRepository;
use crate::web::AppState;

use super::views::UserView;

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

    match users_repo.delete_user(user_id).await {
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
