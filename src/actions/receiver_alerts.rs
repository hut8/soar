use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use tracing::error;
use uuid::Uuid;

use crate::actions::{DataListResponse, DataResponse, json_error};
use crate::auth::AuthUser;
use crate::receiver_alerts::UpsertReceiverAlertRequest;
use crate::receiver_alerts_repo::ReceiverAlertsRepository;
use crate::receiver_repo::ReceiverRepository;
use crate::web::AppState;

/// GET /data/receivers/{id}/alerts - Get current user's alert for this receiver
pub async fn get_receiver_alert(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(receiver_id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = ReceiverAlertsRepository::new(state.pool);
    let user_id = auth_user.0.id;

    match repo.get_by_user_and_receiver(user_id, receiver_id).await {
        Ok(Some(alert)) => Json(DataResponse { data: alert }).into_response(),
        Ok(None) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get receiver alert");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get receiver alert",
            )
            .into_response()
        }
    }
}

/// PUT /data/receivers/{id}/alerts - Create or update alert subscription
pub async fn upsert_receiver_alert(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(receiver_id): Path<Uuid>,
    Json(req): Json<UpsertReceiverAlertRequest>,
) -> impl IntoResponse {
    let receiver_repo = ReceiverRepository::new(state.pool.clone());
    let repo = ReceiverAlertsRepository::new(state.pool);
    let user_id = auth_user.0.id;

    // Verify receiver exists
    match receiver_repo.get_receiver_by_id(receiver_id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return json_error(StatusCode::NOT_FOUND, "Receiver not found").into_response();
        }
        Err(e) => {
            error!(error = %e, "Failed to verify receiver existence");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to verify receiver",
            )
            .into_response();
        }
    }

    // Validate thresholds
    if req.down_after_minutes < 5 {
        return json_error(
            StatusCode::BAD_REQUEST,
            "down_after_minutes must be at least 5",
        )
        .into_response();
    }
    if req.base_cooldown_minutes < 5 {
        return json_error(
            StatusCode::BAD_REQUEST,
            "base_cooldown_minutes must be at least 5",
        )
        .into_response();
    }
    if req.cpu_threshold <= 0.0 || req.cpu_threshold > 1.0 {
        return json_error(
            StatusCode::BAD_REQUEST,
            "cpu_threshold must be between 0.0 and 1.0",
        )
        .into_response();
    }
    if req.temperature_threshold_c <= 0.0 || req.temperature_threshold_c > 200.0 {
        return json_error(
            StatusCode::BAD_REQUEST,
            "temperature_threshold_c must be between 0 and 200",
        )
        .into_response();
    }

    match repo.upsert(user_id, receiver_id, &req).await {
        Ok(alert) => Json(DataResponse { data: alert }).into_response(),
        Err(e) => {
            error!(error = %e, "Failed to upsert receiver alert");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to save receiver alert",
            )
            .into_response()
        }
    }
}

/// DELETE /data/receivers/{id}/alerts - Remove alert subscription
pub async fn delete_receiver_alert(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(receiver_id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = ReceiverAlertsRepository::new(state.pool);
    let user_id = auth_user.0.id;

    match repo.delete(user_id, receiver_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => {
            json_error(StatusCode::NOT_FOUND, "Alert subscription not found").into_response()
        }
        Err(e) => {
            error!(error = %e, "Failed to delete receiver alert");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to delete receiver alert",
            )
            .into_response()
        }
    }
}

/// GET /data/user/receiver-alerts - List all receiver alert subscriptions for current user
pub async fn list_user_receiver_alerts(
    auth_user: AuthUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let repo = ReceiverAlertsRepository::new(state.pool);
    let user_id = auth_user.0.id;

    match repo.get_by_user(user_id).await {
        Ok(alerts) => Json(DataListResponse { data: alerts }).into_response(),
        Err(e) => {
            error!(error = %e, "Failed to list receiver alerts");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list receiver alerts",
            )
            .into_response()
        }
    }
}
