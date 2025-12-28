use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::actions::{DataListResponse, DataResponse, json_error};
use crate::auth::AuthUser;
use crate::users::User;
use crate::users_repo::UsersRepository;
use crate::web::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlightPilotInfo {
    pub pilot: User,
    pub role: String,
    pub is_tow_pilot: bool,
    pub is_student: bool,
    pub is_instructor: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreatePilotRequest {
    pub first_name: String,
    pub last_name: String,
    pub is_licensed: bool,
    pub is_instructor: bool,
    pub is_tow_pilot: bool,
    pub is_examiner: bool,
    pub club_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct LinkPilotRequest {
    pub pilot_id: Uuid,
    pub is_tow_pilot: bool,
    pub is_student: bool,
    pub is_instructor: bool,
}

/// Get a pilot by ID
pub async fn get_pilot_by_id(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool.clone());

    match users_repo.get_by_id(id).await {
        Ok(Some(pilot)) => Json(DataResponse { data: pilot }).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Pilot not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get pilot by ID {}: {}", id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get pilot").into_response()
        }
    }
}

/// Get all pilots for a specific club
/// Requires authentication and user must belong to the requested club
pub async fn get_pilots_by_club(
    Path(club_id): Path<Uuid>,
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> impl IntoResponse {
    // Verify that the user belongs to the requested club
    if user.club_id != Some(club_id) && !user.is_admin {
        return json_error(
            StatusCode::FORBIDDEN,
            "You must be a member of this club to view its pilots",
        )
        .into_response();
    }

    let users_repo = UsersRepository::new(state.pool.clone());

    match users_repo.get_pilots_by_club(club_id).await {
        Ok(pilots) => Json(DataListResponse { data: pilots }).into_response(),
        Err(e) => {
            tracing::error!("Failed to get pilots for club {}: {}", club_id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get pilots for club",
            )
            .into_response()
        }
    }
}

/// Create a new pilot
pub async fn create_pilot(
    State(state): State<AppState>,
    Json(request): Json<CreatePilotRequest>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool.clone());

    let pilot = User::new_pilot(
        request.first_name,
        request.last_name,
        request.is_licensed,
        request.is_instructor,
        request.is_tow_pilot,
        request.is_examiner,
        request.club_id,
    );

    match users_repo.create_pilot(pilot.clone()).await {
        Ok(_) => Json(DataResponse { data: pilot }).into_response(),
        Err(e) => {
            tracing::error!("Failed to create pilot: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create pilot").into_response()
        }
    }
}

/// Get all pilots (users) for a specific flight
pub async fn get_pilots_for_flight(
    Path(flight_id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    use crate::pilots_repo::PilotsRepository;

    let pilots_repo = PilotsRepository::new(state.pool.clone());

    match pilots_repo.get_pilots_for_flight(flight_id).await {
        Ok(user_links) => {
            let flight_pilot_infos: Vec<FlightPilotInfo> = user_links
                .into_iter()
                .map(|(user, link)| FlightPilotInfo {
                    pilot: user,
                    role: link.role().to_string(),
                    is_tow_pilot: link.is_tow_pilot,
                    is_student: link.is_student,
                    is_instructor: link.is_instructor,
                })
                .collect();

            Json(DataListResponse {
                data: flight_pilot_infos,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get pilots for flight {}: {}", flight_id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get pilots for flight",
            )
            .into_response()
        }
    }
}

/// Link a pilot (user) to a flight with specific roles
pub async fn link_pilot_to_flight(
    Path(flight_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(request): Json<LinkPilotRequest>,
) -> impl IntoResponse {
    use crate::pilots::FlightPilot;
    use crate::pilots_repo::PilotsRepository;

    let pilots_repo = PilotsRepository::new(state.pool.clone());
    let users_repo = UsersRepository::new(state.pool.clone());

    // Verify the user exists and is a pilot
    match users_repo.get_by_id(request.pilot_id).await {
        Ok(Some(user)) => {
            if !user.is_pilot() {
                return json_error(StatusCode::BAD_REQUEST, "User is not configured as a pilot")
                    .into_response();
            }
        }
        Ok(None) => {
            return json_error(StatusCode::NOT_FOUND, "Pilot not found").into_response();
        }
        Err(e) => {
            tracing::error!("Failed to verify pilot {}: {}", request.pilot_id, e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to verify pilot")
                .into_response();
        }
    }

    let flight_pilot = FlightPilot::new(
        flight_id,
        request.pilot_id,
        request.is_tow_pilot,
        request.is_student,
        request.is_instructor,
    );

    match pilots_repo.link_pilot_to_flight(&flight_pilot).await {
        Ok(_) => StatusCode::CREATED.into_response(),
        Err(e) => {
            tracing::error!(
                "Failed to link pilot {} to flight {}: {}",
                request.pilot_id,
                flight_id,
                e
            );
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to link pilot to flight",
            )
            .into_response()
        }
    }
}

/// Remove a pilot from a flight
pub async fn unlink_pilot_from_flight(
    Path((flight_id, pilot_id)): Path<(Uuid, Uuid)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    use crate::pilots_repo::PilotsRepository;

    let pilots_repo = PilotsRepository::new(state.pool.clone());

    match pilots_repo
        .unlink_pilot_from_flight(flight_id, pilot_id)
        .await
    {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => {
            json_error(StatusCode::NOT_FOUND, "Flight-pilot link not found").into_response()
        }
        Err(e) => {
            tracing::error!(
                "Failed to unlink pilot {} from flight {}: {}",
                pilot_id,
                flight_id,
                e
            );
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to unlink pilot from flight",
            )
            .into_response()
        }
    }
}

/// Soft delete a pilot by ID
pub async fn delete_pilot(
    Path(pilot_id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool.clone());

    match users_repo.soft_delete_user(pilot_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "Pilot not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to delete pilot {}: {}", pilot_id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete pilot").into_response()
        }
    }
}
