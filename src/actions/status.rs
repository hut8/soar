//! Status endpoint for build information and uptime
//!
//! Provides runtime information about the server including:
//! - Git commit and version from build time
//! - Binary modification time
//! - Server uptime

use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use std::sync::OnceLock;
use std::time::Instant;

use super::DataResponse;

/// Server start time - initialized on first status request
static SERVER_START_TIME: OnceLock<Instant> = OnceLock::new();

/// Initialize the server start time (call this when the server starts)
pub fn init_server_start_time() {
    SERVER_START_TIME.get_or_init(Instant::now);
}

/// Status response with build and runtime information
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusInfo {
    /// Git version from `git describe --tags --always --dirty`
    pub version: &'static str,
    /// Git commit SHA
    pub git_commit: &'static str,
    /// Build timestamp (ISO 8601)
    pub build_timestamp: &'static str,
    /// Target triple (e.g., x86_64-unknown-linux-gnu)
    pub target: &'static str,
    /// Binary modification time (ISO 8601), if available
    pub binary_modified: Option<String>,
    /// Binary path
    pub binary_path: Option<String>,
    /// Server uptime in seconds
    pub uptime_seconds: u64,
    /// Human-readable uptime
    pub uptime_human: String,
}

/// Format seconds into a human-readable duration string
fn format_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, secs)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

/// Get binary modification time as ISO 8601 string
fn get_binary_modified_time() -> Option<String> {
    let exe_path = std::env::current_exe().ok()?;
    let metadata = std::fs::metadata(&exe_path).ok()?;
    let modified = metadata.modified().ok()?;

    // Convert to chrono DateTime
    let duration = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
    let datetime =
        chrono::DateTime::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
            .map(|dt| dt.to_rfc3339())?;

    Some(datetime)
}

/// Get binary path as string
fn get_binary_path() -> Option<String> {
    std::env::current_exe()
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}

/// Handler for GET /data/status
///
/// Returns build information and server uptime
#[tracing::instrument]
pub async fn get_status() -> impl IntoResponse {
    // Get uptime
    let start_time = SERVER_START_TIME.get_or_init(Instant::now);
    let uptime_seconds = start_time.elapsed().as_secs();

    let status = StatusInfo {
        version: env!("VERGEN_GIT_DESCRIBE"),
        git_commit: env!("VERGEN_GIT_SHA"),
        build_timestamp: env!("VERGEN_BUILD_TIMESTAMP"),
        target: env!("VERGEN_CARGO_TARGET_TRIPLE"),
        binary_modified: get_binary_modified_time(),
        binary_path: get_binary_path(),
        uptime_seconds,
        uptime_human: format_duration(uptime_seconds),
    };

    (StatusCode::OK, Json(DataResponse { data: status }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(45), "45s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(125), "2m 5s");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(3725), "1h 2m 5s");
    }

    #[test]
    fn test_format_duration_days() {
        assert_eq!(format_duration(90125), "1d 1h 2m 5s");
    }
}
