pub mod aircraft;
pub mod airports;
pub mod airspaces;
pub mod analytics;
pub mod aprs_messages;
pub mod auth;
pub mod clubs;
pub mod coverage;
pub mod devices;
pub mod fixes;
pub mod flights;
pub mod pilots;
pub mod receivers;
pub mod user_fixes;
pub mod user_settings;
pub mod users;
pub mod views;
pub mod watchlist;

pub use aircraft::*;
pub use airports::*;
pub use airspaces::*;
pub use analytics::*;
pub use aprs_messages::*;
pub use auth::*;
pub use clubs::*;
pub use devices::*;
pub use fixes::*;
pub use flights::*;
pub use receivers::*;
pub use user_fixes::*;
pub use user_settings::*;
pub use users::*;
pub use watchlist::*;

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
