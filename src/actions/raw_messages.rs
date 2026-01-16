use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use uuid::Uuid;

use crate::actions::{DataResponse, json_error};
use crate::raw_messages_repo::RawMessagesRepository;
use crate::web::AppState;

/// Get a raw message by ID
/// Returns the raw message with proper encoding based on source type:
/// - APRS: UTF-8 text
/// - ADS-B/Beast: hex-encoded binary
pub async fn get_raw_message(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let repo = RawMessagesRepository::new(state.pool.clone());

    match repo.get_message_response_by_id(id).await {
        Ok(Some(message)) => Json(DataResponse { data: message }).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Raw message not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get raw message {}: {}", id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get raw message",
            )
            .into_response()
        }
    }
}
