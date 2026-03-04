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
use crate::users::UpdateUserRequest;
use crate::users_repo::UsersRepository;
use crate::web::AppState;

use super::views::ClubJoinRequestView;
use super::{DataListResponse, json_error};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateJoinRequestBody {
    pub message: Option<String>,
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
        return json_error(StatusCode::BAD_REQUEST, "You are already a member of this club")
            .into_response();
    }

    let repo = ClubJoinRequestsRepository::new(state.pool.clone());

    // Check for an existing pending request
    match repo
        .get_pending_by_user_and_club(user.id, club_id)
        .await
    {
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
        Ok(request) => (
            StatusCode::CREATED,
            Json(ClubJoinRequestView::from(request)),
        )
            .into_response(),
        Err(e) => {
            error!(error = %e, "Failed to create join request");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create join request")
                .into_response()
        }
    }
}

/// Get all pending join requests for a club (club members and admins only)
pub async fn get_join_requests(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(club_id): Path<Uuid>,
) -> impl IntoResponse {
    let user = &auth_user.0;

    if !user.is_admin && user.club_id != Some(club_id) {
        return json_error(
            StatusCode::FORBIDDEN,
            "You must be a member of this club to view join requests",
        )
        .into_response();
    }

    let repo = ClubJoinRequestsRepository::new(state.pool.clone());

    match repo.get_pending_by_club(club_id).await {
        Ok(requests) => {
            let views: Vec<ClubJoinRequestView> =
                requests.into_iter().map(ClubJoinRequestView::from).collect();
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

/// Approve a join request (club members and admins only)
pub async fn approve_join_request(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path((club_id, request_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let user = &auth_user.0;

    if !user.is_admin && user.club_id != Some(club_id) {
        return json_error(
            StatusCode::FORBIDDEN,
            "You must be a member of this club to approve join requests",
        )
        .into_response();
    }

    let repo = ClubJoinRequestsRepository::new(state.pool.clone());

    // Verify the request belongs to this club
    let request = match repo.get_by_id(request_id).await {
        Ok(Some(r)) if r.club_id == club_id => r,
        Ok(Some(_)) => {
            return json_error(StatusCode::NOT_FOUND, "Join request not found").into_response();
        }
        Ok(None) => {
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

    // Approve the request
    let approved = match repo.approve(request_id, user.id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return json_error(
                StatusCode::CONFLICT,
                "Request is no longer pending",
            )
            .into_response();
        }
        Err(e) => {
            error!(error = %e, "Failed to approve join request");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to approve join request",
            )
            .into_response();
        }
    };

    // Update the user's club_id
    let users_repo = UsersRepository::new(state.pool.clone());
    let update_request = UpdateUserRequest {
        first_name: None,
        last_name: None,
        email: None,
        is_admin: None,
        club_id: Some(request.club_id),
        email_verified: None,
    };

    if let Err(e) = users_repo.update_user(request.user_id, &update_request).await {
        error!(error = %e, "Failed to update user club after approval");
        // Still return the approved request even if club update failed
        // (admin can manually fix this)
    }

    Json(ClubJoinRequestView::from(approved)).into_response()
}

/// Reject a join request (club members and admins only)
pub async fn reject_join_request(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path((club_id, request_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let user = &auth_user.0;

    if !user.is_admin && user.club_id != Some(club_id) {
        return json_error(
            StatusCode::FORBIDDEN,
            "You must be a member of this club to reject join requests",
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
