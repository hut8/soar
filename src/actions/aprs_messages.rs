use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::raw_messages_repo::{AprsMessage, RawMessagesRepository};
use crate::web::AppState;

use super::json_error;

/// Request body for bulk APRS message retrieval
#[derive(Debug, Deserialize)]
pub struct AprsMessageBulkRequest {
    pub ids: Vec<Uuid>,
}

/// Response for single APRS message
#[derive(Debug, Serialize)]
pub struct AprsMessageResponse {
    pub message: AprsMessage,
}

/// Response for bulk APRS messages
#[derive(Debug, Serialize)]
pub struct AprsMessageBulkResponse {
    pub messages: Vec<AprsMessage>,
}

/// Get a single APRS message by ID
/// GET /data/aprs-messages/{id}
pub async fn get_aprs_message(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let messages_repo = RawMessagesRepository::new(state.pool.clone());

    match messages_repo.get_by_id(id).await {
        Ok(Some(message)) => Json(AprsMessageResponse { message }).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "APRS message not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get APRS message {}: {}", id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get APRS message",
            )
            .into_response()
        }
    }
}

/// Get multiple APRS messages by their IDs
/// POST /data/aprs-messages/ (with body containing list of IDs)
pub async fn get_aprs_messages_bulk(
    State(state): State<AppState>,
    Json(request): Json<AprsMessageBulkRequest>,
) -> impl IntoResponse {
    let messages_repo = RawMessagesRepository::new(state.pool.clone());

    // Validate that we have at least one ID
    if request.ids.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "At least one ID must be provided")
            .into_response();
    }

    match messages_repo.get_by_ids(request.ids).await {
        Ok(messages) => Json(AprsMessageBulkResponse { messages }).into_response(),
        Err(e) => {
            tracing::error!("Failed to get APRS messages: {}", e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get APRS messages",
            )
            .into_response()
        }
    }
}
