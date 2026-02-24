//! Geofence API handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use tracing::error;
use uuid::Uuid;

use crate::actions::{DataListResponse, DataResponse, json_error};
use crate::auth::AuthUser;
use crate::geofence::{
    CreateGeofenceRequest, GeofenceDetailResponse, GeofenceExitEventsResponse,
    GeofenceListResponse, GeofenceWithCounts, LinkAircraftRequest, SubscribeToGeofenceRequest,
    UpdateGeofenceRequest,
};
use crate::geofence_repo::GeofenceRepository;
use crate::web::AppState;

/// Query parameters for listing geofences
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListGeofencesQuery {
    pub club_id: Option<Uuid>,
}

/// Query parameters for exit events
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitEventsQuery {
    pub limit: Option<i64>,
}

// ==================== Geofence CRUD ====================

/// GET /data/geofences - List user's geofences
pub async fn list_geofences(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<ListGeofencesQuery>,
) -> impl IntoResponse {
    let repo = GeofenceRepository::new(state.pool);
    let user = &auth_user.0;

    // Use club_id from query or fall back to user's club
    let club_id = query.club_id.or(user.club_id);

    match repo.get_for_user(user.id, club_id).await {
        Ok(results) => {
            let geofences: Vec<GeofenceWithCounts> = results
                .into_iter()
                .map(
                    |(geofence, aircraft_count, subscriber_count)| GeofenceWithCounts {
                        geofence,
                        aircraft_count,
                        subscriber_count,
                    },
                )
                .collect();
            Json(GeofenceListResponse { geofences }).into_response()
        }
        Err(e) => {
            error!(error = %e, "Failed to list geofences");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list geofences",
            )
            .into_response()
        }
    }
}

/// POST /data/geofences - Create a new geofence
pub async fn create_geofence(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateGeofenceRequest>,
) -> impl IntoResponse {
    // Validate request
    if let Err(msg) = req.validate() {
        return json_error(StatusCode::BAD_REQUEST, &msg).into_response();
    }

    // If club_id provided, verify user belongs to that club
    let user = &auth_user.0;
    if let Some(club_id) = req.club_id
        && !user.is_admin
        && user.club_id != Some(club_id)
    {
        return json_error(
            StatusCode::FORBIDDEN,
            "Cannot create geofence for a club you don't belong to",
        )
        .into_response();
    }

    let repo = GeofenceRepository::new(state.pool);

    match repo.create(user.id, req).await {
        Ok(geofence) => (
            StatusCode::CREATED,
            Json(GeofenceDetailResponse {
                geofence,
                aircraft_count: 0,
                subscriber_count: 0,
            }),
        )
            .into_response(),
        Err(e) => {
            error!(error = %e, "Failed to create geofence");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create geofence",
            )
            .into_response()
        }
    }
}

/// GET /data/geofences/{id} - Get geofence details
pub async fn get_geofence(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(geofence_id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = GeofenceRepository::new(state.pool);
    let user = &auth_user.0;

    match repo.get_by_id(geofence_id).await {
        Ok(Some(geofence)) => {
            // Check permission: user owns it, or user is in the same club, or user is admin
            if geofence.owner_user_id != user.id
                && !user.is_admin
                && (geofence.club_id.is_none() || user.club_id != geofence.club_id)
            {
                return json_error(StatusCode::FORBIDDEN, "Access denied").into_response();
            }

            // Get counts
            let aircraft_count = repo
                .get_aircraft(geofence_id)
                .await
                .map(|v| v.len() as i64)
                .unwrap_or(0);
            let subscriber_count = repo
                .get_subscribers(geofence_id)
                .await
                .map(|v| v.len() as i64)
                .unwrap_or(0);

            Json(GeofenceDetailResponse {
                geofence,
                aircraft_count,
                subscriber_count,
            })
            .into_response()
        }
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Geofence not found").into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get geofence");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get geofence").into_response()
        }
    }
}

/// PUT /data/geofences/{id} - Update a geofence
pub async fn update_geofence(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(geofence_id): Path<Uuid>,
    Json(req): Json<UpdateGeofenceRequest>,
) -> impl IntoResponse {
    // Validate request
    if let Err(msg) = req.validate() {
        return json_error(StatusCode::BAD_REQUEST, &msg).into_response();
    }

    let repo = GeofenceRepository::new(state.pool);
    let user = &auth_user.0;

    // First get the geofence to check permission
    match repo.get_by_id(geofence_id).await {
        Ok(Some(existing)) => {
            // Check permission: must own it or be admin
            if existing.owner_user_id != user.id && !user.is_admin {
                return json_error(
                    StatusCode::FORBIDDEN,
                    "Only the owner can update this geofence",
                )
                .into_response();
            }
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Geofence not found").into_response(),
        Err(e) => {
            error!(geofence_id = %geofence_id, error = %e, "Failed to get geofence for update");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update geofence",
            )
            .into_response();
        }
    }

    match repo.update(geofence_id, req).await {
        Ok(Some(geofence)) => {
            let aircraft_count = repo
                .get_aircraft(geofence_id)
                .await
                .map(|v| v.len() as i64)
                .unwrap_or(0);
            let subscriber_count = repo
                .get_subscribers(geofence_id)
                .await
                .map(|v| v.len() as i64)
                .unwrap_or(0);

            Json(GeofenceDetailResponse {
                geofence,
                aircraft_count,
                subscriber_count,
            })
            .into_response()
        }
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Geofence not found").into_response(),
        Err(e) => {
            error!(geofence_id = %geofence_id, error = %e, "Failed to update geofence");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update geofence",
            )
            .into_response()
        }
    }
}

/// DELETE /data/geofences/{id} - Delete a geofence (soft delete)
pub async fn delete_geofence(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(geofence_id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = GeofenceRepository::new(state.pool);
    let user = &auth_user.0;

    // First get the geofence to check permission
    match repo.get_by_id(geofence_id).await {
        Ok(Some(existing)) => {
            // Check permission: must own it or be admin
            if existing.owner_user_id != user.id && !user.is_admin {
                return json_error(
                    StatusCode::FORBIDDEN,
                    "Only the owner can delete this geofence",
                )
                .into_response();
            }
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Geofence not found").into_response(),
        Err(e) => {
            error!(geofence_id = %geofence_id, error = %e, "Failed to get geofence for delete");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to delete geofence",
            )
            .into_response();
        }
    }

    match repo.delete(geofence_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "Geofence not found").into_response(),
        Err(e) => {
            error!(geofence_id = %geofence_id, error = %e, "Failed to delete geofence");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to delete geofence",
            )
            .into_response()
        }
    }
}

// ==================== Aircraft Links ====================

/// GET /data/geofences/{id}/aircraft - Get aircraft linked to a geofence
pub async fn get_geofence_aircraft(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(geofence_id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = GeofenceRepository::new(state.pool);
    let user = &auth_user.0;

    // Check permission to view geofence
    match repo.get_by_id(geofence_id).await {
        Ok(Some(geofence)) => {
            if geofence.owner_user_id != user.id
                && !user.is_admin
                && (geofence.club_id.is_none() || user.club_id != geofence.club_id)
            {
                return json_error(StatusCode::FORBIDDEN, "Access denied").into_response();
            }
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Geofence not found").into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get geofence");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get aircraft")
                .into_response();
        }
    }

    match repo.get_aircraft(geofence_id).await {
        Ok(aircraft_ids) => Json(DataListResponse { data: aircraft_ids }).into_response(),
        Err(e) => {
            error!(geofence_id = %geofence_id, error = %e, "Failed to get geofence aircraft");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get aircraft").into_response()
        }
    }
}

/// POST /data/geofences/{id}/aircraft - Link aircraft to a geofence
pub async fn add_geofence_aircraft(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(geofence_id): Path<Uuid>,
    Json(req): Json<LinkAircraftRequest>,
) -> impl IntoResponse {
    let repo = GeofenceRepository::new(state.pool);
    let user = &auth_user.0;

    // Check permission to modify geofence
    match repo.get_by_id(geofence_id).await {
        Ok(Some(geofence)) => {
            // Must own it, be in the same club, or be admin
            if geofence.owner_user_id != user.id
                && !user.is_admin
                && (geofence.club_id.is_none() || user.club_id != geofence.club_id)
            {
                return json_error(StatusCode::FORBIDDEN, "Access denied").into_response();
            }
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Geofence not found").into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get geofence");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to add aircraft")
                .into_response();
        }
    }

    // TODO: Optionally verify user has access to the aircraft being added

    match repo.add_aircraft(geofence_id, req.aircraft_id).await {
        Ok(link) => (StatusCode::CREATED, Json(DataResponse { data: link })).into_response(),
        Err(e) => {
            error!(geofence_id = %geofence_id, error = %e, "Failed to add aircraft to geofence");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to add aircraft").into_response()
        }
    }
}

/// DELETE /data/geofences/{geofence_id}/aircraft/{aircraft_id} - Remove aircraft from geofence
pub async fn remove_geofence_aircraft(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path((geofence_id, aircraft_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let repo = GeofenceRepository::new(state.pool);
    let user = &auth_user.0;

    // Check permission to modify geofence
    match repo.get_by_id(geofence_id).await {
        Ok(Some(geofence)) => {
            if geofence.owner_user_id != user.id
                && !user.is_admin
                && (geofence.club_id.is_none() || user.club_id != geofence.club_id)
            {
                return json_error(StatusCode::FORBIDDEN, "Access denied").into_response();
            }
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Geofence not found").into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get geofence");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to remove aircraft",
            )
            .into_response();
        }
    }

    match repo.remove_aircraft(geofence_id, aircraft_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => {
            json_error(StatusCode::NOT_FOUND, "Aircraft not linked to geofence").into_response()
        }
        Err(e) => {
            error!(geofence_id = %geofence_id, aircraft_id = %aircraft_id, error = %e, "Failed to remove aircraft from geofence");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to remove aircraft",
            )
            .into_response()
        }
    }
}

// ==================== Subscribers ====================

/// GET /data/geofences/{id}/subscribers - Get subscribers for a geofence
pub async fn get_geofence_subscribers(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(geofence_id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = GeofenceRepository::new(state.pool);
    let user = &auth_user.0;

    // Check permission to view geofence
    match repo.get_by_id(geofence_id).await {
        Ok(Some(geofence)) => {
            if geofence.owner_user_id != user.id
                && !user.is_admin
                && (geofence.club_id.is_none() || user.club_id != geofence.club_id)
            {
                return json_error(StatusCode::FORBIDDEN, "Access denied").into_response();
            }
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Geofence not found").into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get geofence");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get subscribers",
            )
            .into_response();
        }
    }

    match repo.get_subscribers(geofence_id).await {
        Ok(subscribers) => Json(DataListResponse { data: subscribers }).into_response(),
        Err(e) => {
            error!(geofence_id = %geofence_id, error = %e, "Failed to get geofence subscribers");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get subscribers",
            )
            .into_response()
        }
    }
}

/// POST /data/geofences/{id}/subscribers - Subscribe to a geofence
pub async fn subscribe_to_geofence(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(geofence_id): Path<Uuid>,
    Json(req): Json<SubscribeToGeofenceRequest>,
) -> impl IntoResponse {
    let repo = GeofenceRepository::new(state.pool);
    let user = &auth_user.0;

    // Check that geofence exists and user can access it
    match repo.get_by_id(geofence_id).await {
        Ok(Some(geofence)) => {
            if geofence.owner_user_id != user.id
                && !user.is_admin
                && (geofence.club_id.is_none() || user.club_id != geofence.club_id)
            {
                return json_error(StatusCode::FORBIDDEN, "Access denied").into_response();
            }
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Geofence not found").into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get geofence");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to subscribe")
                .into_response();
        }
    }

    match repo
        .add_subscriber(geofence_id, user.id, req.send_email)
        .await
    {
        Ok(subscriber) => {
            (StatusCode::CREATED, Json(DataResponse { data: subscriber })).into_response()
        }
        Err(e) => {
            error!(geofence_id = %geofence_id, error = %e, "Failed to subscribe to geofence");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to subscribe").into_response()
        }
    }
}

/// DELETE /data/geofences/{geofence_id}/subscribers/{user_id} - Unsubscribe from geofence
pub async fn unsubscribe_from_geofence(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path((geofence_id, user_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let repo = GeofenceRepository::new(state.pool);
    let user = &auth_user.0;

    // Users can unsubscribe themselves; owners/admins can unsubscribe anyone
    match repo.get_by_id(geofence_id).await {
        Ok(Some(geofence)) => {
            let is_owner = geofence.owner_user_id == user.id;
            let is_self = user_id == user.id;
            let is_club_member = geofence.club_id.is_some() && user.club_id == geofence.club_id;

            if !is_self && !is_owner && !user.is_admin && !is_club_member {
                return json_error(StatusCode::FORBIDDEN, "Access denied").into_response();
            }
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Geofence not found").into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get geofence");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to unsubscribe")
                .into_response();
        }
    }

    match repo.remove_subscriber(geofence_id, user_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "Subscription not found").into_response(),
        Err(e) => {
            error!(geofence_id = %geofence_id, error = %e, "Failed to unsubscribe from geofence");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to unsubscribe").into_response()
        }
    }
}

// ==================== Exit Events ====================

/// GET /data/geofences/{id}/events - Get exit events for a geofence
pub async fn get_geofence_events(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(geofence_id): Path<Uuid>,
    Query(query): Query<ExitEventsQuery>,
) -> impl IntoResponse {
    let repo = GeofenceRepository::new(state.pool);
    let user = &auth_user.0;

    // Check permission to view geofence
    match repo.get_by_id(geofence_id).await {
        Ok(Some(geofence)) => {
            if geofence.owner_user_id != user.id
                && !user.is_admin
                && (geofence.club_id.is_none() || user.club_id != geofence.club_id)
            {
                return json_error(StatusCode::FORBIDDEN, "Access denied").into_response();
            }
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Geofence not found").into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get geofence");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get events")
                .into_response();
        }
    }

    match repo
        .get_exit_events_for_geofence(geofence_id, query.limit)
        .await
    {
        Ok(events) => Json(GeofenceExitEventsResponse { events }).into_response(),
        Err(e) => {
            error!(geofence_id = %geofence_id, error = %e, "Failed to get geofence events");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get events").into_response()
        }
    }
}

/// GET /data/flights/{id}/geofence-events - Get geofence events for a flight
pub async fn get_flight_geofence_events(
    _auth_user: AuthUser,
    State(state): State<AppState>,
    Path(flight_id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = GeofenceRepository::new(state.pool);

    // Note: We don't check geofence permission here because flight events are
    // associated with the flight, and anyone who can view the flight can see
    // the associated geofence events.

    match repo.get_exit_events_for_flight(flight_id).await {
        Ok(events) => Json(GeofenceExitEventsResponse { events }).into_response(),
        Err(e) => {
            error!(flight_id = %flight_id, error = %e, "Failed to get flight geofence events");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get events").into_response()
        }
    }
}
