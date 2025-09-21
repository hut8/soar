pub mod aircraft;
pub mod auth;
pub mod clubs;
pub mod fixes;
pub mod search;
pub mod users;
pub mod views;

pub use aircraft::*;
pub use auth::*;
pub use clubs::*;
pub use fixes::*;
pub use search::*;
pub use users::*;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde_json::json;

/// Helper function to create consistent JSON error responses
pub fn json_error(status: StatusCode, message: &str) -> impl IntoResponse {
    (
        status,
        Json(json!({
            "errors": message
        })),
    )
}
