use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use tracing::error;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::club_join_requests::{NewClubJoinRequest, STATUS_PENDING};
use crate::club_join_requests_repo::ClubJoinRequestsRepository;
use crate::notifications;
use crate::users_repo::UsersRepository;
use crate::web::AppState;

use super::views::{ClubJoinRequestView, UserView};
use super::{DataListResponse, json_error};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateJoinRequestBody {
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetClubAdminBody {
    pub is_club_admin: bool,
}

/// Create a join request for the current user to join a club
pub async fn create_join_request(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(club_id): Path<Uuid>,
    Json(payload): Json<CreateJoinRequestBody>,
) -> impl IntoResponse {
    let user = &auth_user.0;

    // User can't request to join a club they're already in
    if user.club_id == Some(club_id) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "You are already a member of this club",
        )
        .into_response();
    }

    let repo = ClubJoinRequestsRepository::new(state.pool.clone());

    // Check for an existing pending request
    match repo.get_pending_by_user_and_club(user.id, club_id).await {
        Ok(Some(_)) => {
            return json_error(
                StatusCode::CONFLICT,
                "You already have a pending join request for this club",
            )
            .into_response();
        }
        Err(e) => {
            error!(error = %e, "Failed to check existing join request");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to check existing requests",
            )
            .into_response();
        }
        Ok(None) => {}
    }

    let new_request = NewClubJoinRequest {
        user_id: user.id,
        club_id,
        status: STATUS_PENDING.to_string(),
        message: payload.message,
    };

    match repo.create(new_request).await {
        Ok(request) => {
            // Send notification to club admins in the background
            let pool = state.pool.clone();
            let requester_name = user.full_name();
            tokio::spawn(async move {
                if let Err(e) =
                    notifications::send_join_request_notification(&pool, club_id, &requester_name)
                        .await
                {
                    error!(error = %e, "Failed to send join request notification");
                }
            });

            (
                StatusCode::CREATED,
                Json(ClubJoinRequestView::from(request)),
            )
                .into_response()
        }
        Err(e) => {
            error!(error = %e, "Failed to create join request");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create join request",
            )
            .into_response()
        }
    }
}

/// Get all pending join requests for a club (club admins and system admins only)
pub async fn get_join_requests(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(club_id): Path<Uuid>,
) -> impl IntoResponse {
    let user = &auth_user.0;

    let is_club_admin = user.is_club_admin && user.club_id == Some(club_id);
    if !user.is_admin && !is_club_admin {
        return json_error(
            StatusCode::FORBIDDEN,
            "You must be a club admin to view join requests",
        )
        .into_response();
    }

    let repo = ClubJoinRequestsRepository::new(state.pool.clone());

    match repo.get_pending_by_club(club_id).await {
        Ok(requests) => {
            let views: Vec<ClubJoinRequestView> = requests
                .into_iter()
                .map(ClubJoinRequestView::from)
                .collect();
            Json(DataListResponse { data: views }).into_response()
        }
        Err(e) => {
            error!(error = %e, "Failed to get join requests");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get join requests",
            )
            .into_response()
        }
    }
}

/// Get the current user's pending join request for a club
pub async fn get_my_join_request(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(club_id): Path<Uuid>,
) -> impl IntoResponse {
    let user = &auth_user.0;
    let repo = ClubJoinRequestsRepository::new(state.pool.clone());

    match repo.get_pending_by_user_and_club(user.id, club_id).await {
        Ok(Some(request)) => Json(ClubJoinRequestView::from(request)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "No pending request found").into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get user join request");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get join request",
            )
            .into_response()
        }
    }
}

/// Approve a join request (club admins and system admins only)
pub async fn approve_join_request(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path((club_id, request_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let user = &auth_user.0;

    // Only system admins or club admins of this club can approve
    let is_club_admin = user.is_club_admin && user.club_id == Some(club_id);
    if !user.is_admin && !is_club_admin {
        return json_error(
            StatusCode::FORBIDDEN,
            "You must be a club admin to approve join requests",
        )
        .into_response();
    }

    let repo = ClubJoinRequestsRepository::new(state.pool.clone());

    // Verify the request belongs to this club
    match repo.get_by_id(request_id).await {
        Ok(Some(r)) if r.club_id == club_id => {}
        Ok(Some(_) | None) => {
            return json_error(StatusCode::NOT_FOUND, "Join request not found").into_response();
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch join request");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch join request",
            )
            .into_response();
        }
    };

    // Approve the request and update user's club_id atomically
    // The transaction also checks for conflicting club membership (TOCTOU-safe)
    match repo.approve_and_set_club(request_id, user.id).await {
        Ok(Some(r)) => Json(ClubJoinRequestView::from(r)).into_response(),
        Ok(None) => {
            json_error(StatusCode::CONFLICT, "Request is no longer pending").into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("already a member of another club") {
                return json_error(
                    StatusCode::CONFLICT,
                    "User is already a member of another club",
                )
                .into_response();
            }
            error!(error = %e, "Failed to approve join request");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to approve join request",
            )
            .into_response()
        }
    }
}

/// Reject a join request (club admins and system admins only)
pub async fn reject_join_request(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path((club_id, request_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let user = &auth_user.0;

    // Only system admins or club admins of this club can reject
    let is_club_admin = user.is_club_admin && user.club_id == Some(club_id);
    if !user.is_admin && !is_club_admin {
        return json_error(
            StatusCode::FORBIDDEN,
            "You must be a club admin to reject join requests",
        )
        .into_response();
    }

    let repo = ClubJoinRequestsRepository::new(state.pool.clone());

    // Verify the request belongs to this club
    match repo.get_by_id(request_id).await {
        Ok(Some(r)) if r.club_id == club_id => {}
        Ok(_) => {
            return json_error(StatusCode::NOT_FOUND, "Join request not found").into_response();
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch join request");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch join request",
            )
            .into_response();
        }
    }

    match repo.reject(request_id, user.id).await {
        Ok(Some(request)) => Json(ClubJoinRequestView::from(request)).into_response(),
        Ok(None) => {
            json_error(StatusCode::CONFLICT, "Request is no longer pending").into_response()
        }
        Err(e) => {
            error!(error = %e, "Failed to reject join request");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to reject join request",
            )
            .into_response()
        }
    }
}

/// Cancel a pending join request (the requesting user or admin only)
pub async fn cancel_join_request(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path((club_id, request_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let user = &auth_user.0;
    let repo = ClubJoinRequestsRepository::new(state.pool.clone());

    // Verify the request exists and belongs to this club
    let request = match repo.get_by_id(request_id).await {
        Ok(Some(r)) if r.club_id == club_id => r,
        Ok(_) => {
            return json_error(StatusCode::NOT_FOUND, "Join request not found").into_response();
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch join request");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch join request",
            )
            .into_response();
        }
    };

    // Only the requesting user or a system admin can cancel
    if request.user_id != user.id && !user.is_admin {
        return json_error(StatusCode::FORBIDDEN, "You cannot cancel this request").into_response();
    }

    match repo.cancel(request_id).await {
        Ok(true) => (StatusCode::NO_CONTENT, "").into_response(),
        Ok(false) => {
            json_error(StatusCode::CONFLICT, "Request is no longer pending").into_response()
        }
        Err(e) => {
            error!(error = %e, "Failed to cancel join request");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to cancel join request",
            )
            .into_response()
        }
    }
}

/// Set or remove club admin status for a club member.
/// Only club admins of the same club or system admins can do this.
pub async fn set_club_admin(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path((club_id, user_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<SetClubAdminBody>,
) -> impl IntoResponse {
    let user = &auth_user.0;

    // Only system admins or club admins of this club can change admin status
    let is_club_admin = user.is_club_admin && user.club_id == Some(club_id);
    if !user.is_admin && !is_club_admin {
        return json_error(
            StatusCode::FORBIDDEN,
            "You must be a club admin to change admin status",
        )
        .into_response();
    }

    let users_repo = UsersRepository::new(state.pool.clone());

    // Verify the target user is a member of this club
    let target_user = match users_repo.get_by_id(user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return json_error(StatusCode::NOT_FOUND, "User not found").into_response();
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch user");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch user")
                .into_response();
        }
    };

    if target_user.club_id != Some(club_id) {
        return json_error(StatusCode::BAD_REQUEST, "User is not a member of this club")
            .into_response();
    }

    let update_request = crate::users::UpdateUserRequest {
        first_name: None,
        last_name: None,
        email: None,
        is_admin: None,
        is_club_admin: Some(payload.is_club_admin),
        club_id: None,
        email_verified: None,
    };

    match users_repo.update_user(user_id, &update_request).await {
        Ok(Some(updated)) => Json(UserView::from(updated)).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "User not found").into_response(),
        Err(e) => {
            error!(error = %e, "Failed to update club admin status");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update club admin status",
            )
            .into_response()
        }
    }
}
