use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::actions::json_error;
use crate::actions::views::{DeviceView, FlightView};
use crate::aircraft_registrations_repo::AircraftRegistrationsRepository;
use crate::device_repo::DeviceRepository;
use crate::devices::AddressType;
use crate::faa::aircraft_model_repo::AircraftModelRepository;
use crate::fixes::Fix;
use crate::fixes_repo::FixesRepository;
use crate::flights_repo::FlightsRepository;
use crate::web::AppState;

#[derive(Debug, Deserialize)]
pub struct FixesQuery {
    /// YYYYMMDDHHMMSS UTC format
    pub after: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeviceSearchQuery {
    /// Aircraft registration number (e.g., N8437D)
    pub registration: Option<String>,
    /// Device address in hex format (e.g., ABCDEF)
    pub address: Option<String>,
    /// Address type: I (ICAO), O (OGN), F (FLARM)
    #[serde(rename = "address-type")]
    pub address_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeviceSearchResponse {
    pub devices: Vec<DeviceView>,
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
            let device_view: DeviceView = device.into();
            Json(device_view).into_response()
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
                        )
                        .into_response();
                    }
                }
            } else {
                None
            };

            match fixes_repo
                .get_fixes_by_device(id, after_datetime, 1000)
                .await
            {
                Ok(fixes) => {
                    let count = fixes.len();
                    Json(DeviceFixesResponse { fixes, count }).into_response()
                }
                Err(e) => {
                    tracing::error!("Failed to get fixes for device {}: {}", id, e);
                    json_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to get device fixes",
                    )
                    .into_response()
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

/// Search devices by registration or address+type
pub async fn search_devices(
    Query(query): Query<DeviceSearchQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let device_repo = DeviceRepository::new(state.pool);

    // Validate query parameters - must have either registration OR (address + address-type)
    match (&query.registration, &query.address, &query.address_type) {
        (Some(registration), None, None) => {
            // Search by registration
            match device_repo.search_by_registration(registration).await {
                Ok(devices) => {
                    let device_views: Vec<DeviceView> =
                        devices.into_iter().map(|d| d.into()).collect();
                    Json(DeviceSearchResponse {
                        devices: device_views,
                    })
                    .into_response()
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to search devices by registration {}: {}",
                        registration,
                        e
                    );
                    json_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to search devices",
                    )
                    .into_response()
                }
            }
        }
        (None, Some(address_str), Some(address_type_str)) => {
            // Parse address from hex string
            let address = match u32::from_str_radix(address_str, 16) {
                Ok(addr) => addr,
                Err(_) => {
                    return json_error(
                        StatusCode::BAD_REQUEST,
                        "Invalid address format. Expected hexadecimal string",
                    )
                    .into_response();
                }
            };

            // Parse address type
            let address_type = match address_type_str.to_uppercase().as_str() {
                "I" => AddressType::Icao,
                "O" => AddressType::Ogn,
                "F" => AddressType::Flarm,
                _ => {
                    return json_error(
                        StatusCode::BAD_REQUEST,
                        "Invalid address-type. Must be I (ICAO), O (OGN), or F (FLARM)",
                    )
                    .into_response();
                }
            };

            // Search by address and type
            match device_repo
                .search_by_address_and_type(address, address_type)
                .await
            {
                Ok(Some(device)) => {
                    let device_view: DeviceView = device.into();
                    Json(DeviceSearchResponse {
                        devices: vec![device_view],
                    })
                    .into_response()
                }
                Ok(None) => Json(DeviceSearchResponse { devices: vec![] }).into_response(),
                Err(e) => {
                    tracing::error!(
                        "Failed to search devices by address {} and type {}: {}",
                        address,
                        address_type_str,
                        e
                    );
                    json_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to search devices",
                    )
                    .into_response()
                }
            }
        }
        _ => {
            // Invalid parameter combination
            json_error(
                StatusCode::BAD_REQUEST,
                "Must provide either 'registration' OR both 'address' and 'address-type' parameters",
            ).into_response()
        }
    }
}

/// Get aircraft registration for a device by device ID
pub async fn get_device_aircraft_registration(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let aircraft_repo = AircraftRegistrationsRepository::new(state.pool.clone());
    let device_repo = DeviceRepository::new(state.pool.clone());

    // First try to query aircraft_registrations table for a record with the given device_id
    match aircraft_repo
        .get_aircraft_registration_by_device_id(id)
        .await
    {
        Ok(Some(aircraft_registration)) => {
            return Json(aircraft_registration).into_response();
        }
        Ok(None) => {
            // Fallback: try to find device and then look up by registration number
            tracing::debug!(
                "No aircraft registration found by device_id {}, trying registration lookup",
                id
            );
        }
        Err(e) => {
            tracing::error!(
                "Failed to get aircraft registration by device_id {}: {}",
                id,
                e
            );
        }
    }

    // Fallback: Get device and look up aircraft by registration number
    match device_repo.get_device_by_uuid(id).await {
        Ok(Some(device)) => {
            // Try to find aircraft registration by registration number
            match aircraft_repo
                .get_aircraft_registration_model_by_n_number(&device.registration)
                .await
            {
                Ok(Some(aircraft_model)) => Json(aircraft_model).into_response(),
                Ok(None) => (StatusCode::NOT_FOUND).into_response(),
                Err(e) => {
                    tracing::error!(
                        "Failed to get aircraft registration for device {} by n-number {}: {}",
                        id,
                        device.registration,
                        e
                    );
                    json_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to get aircraft registration",
                    )
                    .into_response()
                }
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND).into_response(),
        Err(e) => {
            tracing::error!("Failed to get device by ID {}: {}", id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get device").into_response()
        }
    }
}

/// Get aircraft model for a device by device ID
/// This joins aircraft_registrations to aircraft_models using manufacturer_code, model_code, and series_code
pub async fn get_device_aircraft_model(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let aircraft_repo = AircraftRegistrationsRepository::new(state.pool.clone());
    let aircraft_model_repo = AircraftModelRepository::new(state.pool.clone());
    let device_repo = DeviceRepository::new(state.pool.clone());

    // First try to get the aircraft registration for this device by device_id
    let aircraft_registration = match aircraft_repo
        .get_aircraft_registration_by_device_id(id)
        .await
    {
        Ok(Some(registration)) => registration,
        Ok(None) => {
            // Fallback: try to find device and then look up by registration number
            tracing::debug!(
                "No aircraft registration found by device_id {}, trying registration lookup",
                id
            );

            match device_repo.get_device_by_uuid(id).await {
                Ok(Some(device)) => {
                    match aircraft_repo
                        .get_aircraft_registration_model_by_n_number(&device.registration)
                        .await
                    {
                        Ok(Some(aircraft_model)) => aircraft_model,
                        Ok(None) => {
                            return (StatusCode::NOT_FOUND).into_response();
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to get aircraft registration for device {} by n-number {}: {}",
                                id,
                                device.registration,
                                e
                            );
                            return json_error(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to get aircraft registration",
                            )
                            .into_response();
                        }
                    }
                }
                Ok(None) => {
                    return (StatusCode::NOT_FOUND).into_response();
                }
                Err(e) => {
                    tracing::error!("Failed to get device by ID {}: {}", id, e);
                    return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get device")
                        .into_response();
                }
            }
        }
        Err(e) => {
            tracing::error!(
                "Failed to get aircraft registration for device {}: {}",
                id,
                e
            );
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get aircraft registration",
            )
            .into_response();
        }
    };

    // Now get the aircraft model using the codes from the registration
    match aircraft_model_repo
        .get_aircraft_model_by_key(
            &aircraft_registration.manufacturer_code,
            &aircraft_registration.model_code,
            &aircraft_registration.series_code,
        )
        .await
    {
        Ok(Some(aircraft_model)) => Json(aircraft_model).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND).into_response(),
        Err(e) => {
            tracing::error!(
                "Failed to get aircraft model for device {} with codes {}-{}-{}: {}",
                id,
                aircraft_registration.manufacturer_code,
                aircraft_registration.model_code,
                aircraft_registration.series_code,
                e
            );
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get aircraft model",
            )
            .into_response()
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

/// Get all devices for a club by club ID
pub async fn get_devices_by_club(
    Path(club_id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let device_repo = DeviceRepository::new(state.pool);

    match device_repo.search_by_club_id(club_id).await {
        Ok(devices) => {
            let device_views: Vec<DeviceView> = devices.into_iter().map(|d| d.into()).collect();
            Json(DeviceSearchResponse {
                devices: device_views,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get devices for club {}: {}", club_id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get devices for club",
            )
            .into_response()
        }
    }
}

/// Get all flights for a device by device ID
pub async fn get_device_flights(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool);

    match flights_repo.get_flights_for_device(&id).await {
        Ok(flights) => {
            let flight_views: Vec<FlightView> = flights.into_iter().map(|f| f.into()).collect();
            Json(flight_views).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get flights for device {}: {}", id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get flights for device",
            )
            .into_response()
        }
    }
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
