use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::device_repo::DeviceRepository;
use crate::fixes_repo::FixesRepository;
use crate::actions::json_error;
use crate::devices::Device;
use crate::fixes::Fix;
use crate::web::AppState;

#[derive(Debug, Deserialize)]
pub struct FixesQuery {
    /// YYYYMMDDHHMMSS UTC format
    pub after: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeviceResponse {
    pub device: Device,
}

#[derive(Debug, Serialize)]
pub struct DeviceFixesResponse {
    pub fixes: Vec<Fix>,
    pub count: usize,
}

/// Get a device by its UUID
pub async fn get_device_by_id(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let device_repo = DeviceRepository::new(state.pool);

    // First try to find the device by UUID in the devices table
    match device_repo.get_device_by_uuid(id).await {
        Ok(Some(device)) => {
            Json(DeviceResponse { device }).into_response()
        }
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Device not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get device by ID {}: {}", id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get device").into_response()
        }
    }
}

/// Get fixes for a device with optional after parameter
pub async fn get_device_fixes(
    Path(id): Path<Uuid>,
    Query(query): Query<FixesQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let device_repo = DeviceRepository::new(state.pool.clone());
    let fixes_repo = FixesRepository::new(state.pool.clone());

    // First verify the device exists
    match device_repo.get_device_by_uuid(id).await {
        Ok(Some(_device)) => {
            // Device exists, get fixes
            let after_datetime = if let Some(after_str) = query.after {
                match parse_datetime_string(&after_str) {
                    Ok(dt) => Some(dt),
                    Err(_) => {
                        return json_error(
                            StatusCode::BAD_REQUEST,
                            "Invalid 'after' parameter format. Expected YYYYMMDDHHMMSS",
                        ).into_response();
                    }
                }
            } else {
                None
            };

            match fixes_repo.get_fixes_by_device(id, after_datetime, 1000).await {
                Ok(fixes) => {
                    let count = fixes.len();
                    Json(DeviceFixesResponse { fixes, count }).into_response()
                }
                Err(e) => {
                    tracing::error!("Failed to get fixes for device {}: {}", id, e);
                    json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get device fixes").into_response()
                }
            }
        }
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Device not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to verify device exists {}: {}", id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to verify device").into_response()
        }
    }
}

/// Parse YYYYMMDDHHMMSS format to DateTime<Utc>
fn parse_datetime_string(datetime_str: &str) -> Result<DateTime<Utc>, &'static str> {
    if datetime_str.len() != 14 {
        return Err("Invalid length: expected 14 characters");
    }

    let naive_datetime = NaiveDateTime::parse_from_str(datetime_str, "%Y%m%d%H%M%S")
        .map_err(|_| "Invalid datetime format")?;
    Ok(DateTime::from_naive_utc_and_offset(naive_datetime, Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_parse_datetime_string() {
        // Test valid format
        let result = parse_datetime_string("20231225143000").unwrap();
        let expected = Utc.with_ymd_and_hms(2023, 12, 25, 14, 30, 0).unwrap();
        assert_eq!(result, expected);

        // Test invalid length
        assert!(parse_datetime_string("2023122514300").is_err());
        assert!(parse_datetime_string("202312251430000").is_err());

        // Test invalid format
        assert!(parse_datetime_string("20231325143000").is_err()); // Invalid month
        assert!(parse_datetime_string("20231225253000").is_err()); // Invalid hour
    }
}