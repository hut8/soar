use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::actions::json_error;
use crate::pilots::{FlightPilot, Pilot};
use crate::pilots_repo::PilotsRepository;
use crate::web::AppState;

#[derive(Debug, Serialize)]
pub struct PilotResponse {
    pub pilot: Pilot,
}

#[derive(Debug, Serialize)]
pub struct PilotsListResponse {
    pub pilots: Vec<Pilot>,
}

#[derive(Debug, Serialize)]
pub struct FlightPilotInfo {
    pub pilot: Pilot,
    pub role: String,
    pub is_tow_pilot: bool,
    pub is_student: bool,
    pub is_instructor: bool,
}

#[derive(Debug, Serialize)]
pub struct FlightPilotsResponse {
    pub pilots: Vec<FlightPilotInfo>,
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
    let pilots_repo = PilotsRepository::new(state.pool.clone());

    match pilots_repo.get_pilot_by_id(id).await {
        Ok(Some(pilot)) => Json(PilotResponse { pilot }).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Pilot not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get pilot by ID {}: {}", id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get pilot").into_response()
        }
    }
}

/// Get all pilots for a specific club
pub async fn get_pilots_by_club(
    Path(club_id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let pilots_repo = PilotsRepository::new(state.pool.clone());

    match pilots_repo.get_pilots_by_club(club_id).await {
        Ok(pilots) => Json(PilotsListResponse { pilots }).into_response(),
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

/// Get all pilots for a specific flight
pub async fn get_pilots_for_flight(
    Path(flight_id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let pilots_repo = PilotsRepository::new(state.pool.clone());

    match pilots_repo.get_pilots_for_flight(flight_id).await {
        Ok(pilot_links) => {
            let pilots: Vec<FlightPilotInfo> = pilot_links
                .into_iter()
                .map(|(pilot, flight_pilot)| FlightPilotInfo {
                    role: flight_pilot.role().to_string(),
                    is_tow_pilot: flight_pilot.is_tow_pilot,
                    is_student: flight_pilot.is_student,
                    is_instructor: flight_pilot.is_instructor,
                    pilot,
                })
                .collect();

            Json(FlightPilotsResponse { pilots }).into_response()
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

/// Link a pilot to a flight with specific roles
pub async fn link_pilot_to_flight(
    Path(flight_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(request): Json<LinkPilotRequest>,
) -> impl IntoResponse {
    let pilots_repo = PilotsRepository::new(state.pool.clone());

    // Create the flight-pilot link
    let flight_pilot = FlightPilot::new(
        flight_id,
        request.pilot_id,
        request.is_tow_pilot,
        request.is_student,
        request.is_instructor,
    );

    match pilots_repo.link_pilot_to_flight(&flight_pilot).await {
        Ok(()) => StatusCode::CREATED.into_response(),
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
    let pilots_repo = PilotsRepository::new(state.pool.clone());

    match pilots_repo
        .unlink_pilot_from_flight(flight_id, pilot_id)
        .await
    {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "Pilot link not found").into_response(),
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
