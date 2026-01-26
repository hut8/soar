use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::actions::{DataResponse, json_error};
use crate::raw_messages_repo::RawMessagesRepository;
use crate::web::AppState;

/// Query parameters for raw message lookup
#[derive(Debug, Deserialize)]
pub struct RawMessageQuery {
    /// Timestamp hint to optimize partition lookup (raw_messages is partitioned by received_at)
    /// If provided, the query will filter to a narrow time range around this timestamp
    pub timestamp: Option<DateTime<Utc>>,
}

/// Get a raw message by ID
/// Returns the raw message with proper encoding based on source type:
/// - APRS: UTF-8 text
/// - ADS-B/Beast: hex-encoded binary
///
/// Query parameters:
/// - timestamp: Optional timestamp hint to optimize partition lookup
pub async fn get_raw_message(
    Path(id): Path<Uuid>,
    Query(query): Query<RawMessageQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let repo = RawMessagesRepository::new(state.pool.clone());

    let result = match query.timestamp {
        Some(ts) => repo.get_message_response_by_id_with_hint(id, ts).await,
        None => repo.get_message_response_by_id(id).await,
    };

    match result {
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
