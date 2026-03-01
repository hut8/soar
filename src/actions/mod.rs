pub mod aircraft;
pub mod aircraft_images;
pub mod aircraft_search;
pub mod airports;
pub mod airspaces;
pub mod analytics;
pub mod auth;
pub mod club_tow_fees;
pub mod clubs;
pub mod coverage;
pub mod data_streams;
pub mod fixes;
pub mod flights;
pub mod geocoding;
pub mod geofences;
pub mod payments;
pub mod pilots;
pub mod raw_messages;
pub mod receivers;
pub mod status;
pub mod stripe_connect;
pub mod user_fixes;
pub mod user_settings;
pub mod users;
pub mod views;
pub mod watchlist;

pub use aircraft::*;
pub use aircraft_images::*;
pub use aircraft_search::*;
pub use airports::*;
pub use airspaces::*;
pub use analytics::*;
pub use auth::*;
pub use clubs::*;
pub use fixes::*;
pub use flights::*;
pub use receivers::*;
pub use status::*;
pub use user_fixes::*;
pub use user_settings::*;
pub use users::*;
pub use watchlist::*;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Serialize;
use serde_json::json;

/// Standard wrapper for single resource responses
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataResponse<T> {
    pub data: T,
}

/// Standard wrapper for list responses
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataListResponse<T> {
    pub data: Vec<T>,
}

/// Standard wrapper for list responses with total count
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataListResponseWithTotal<T> {
    pub data: Vec<T>,
    pub total: i64,
}

/// Pagination metadata (nested in paginated responses)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationMetadata {
    pub page: i64,
    pub total_pages: i64,
    pub total_count: i64,
}

/// Standard wrapper for paginated list responses
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedDataResponse<T> {
    pub data: Vec<T>,
    pub metadata: PaginationMetadata,
}

/// Helper function to create consistent JSON error responses
pub fn json_error(status: StatusCode, message: &str) -> impl IntoResponse {
    (
        status,
        Json(json!({
            "errors": message
        })),
    )
}
