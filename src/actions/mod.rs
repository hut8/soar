pub mod aircraft;
pub mod airports;
pub mod auth;
pub mod clubs;
pub mod devices;
pub mod fixes;
pub mod flights;
pub mod receivers;
pub mod users;
pub mod views;

pub use aircraft::*;
pub use airports::*;
pub use auth::*;
pub use clubs::*;
pub use devices::*;
pub use fixes::*;
pub use flights::*;
pub use receivers::*;
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
