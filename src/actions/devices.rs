use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use uuid::Uuid;

use crate::actions::json_error;
use crate::actions::views::{DeviceView, FlightView};
use crate::device_repo::DeviceRepository;
use crate::devices::AddressType;
use crate::fixes::Fix;
use crate::fixes_repo::FixesRepository;
use crate::flights_repo::FlightsRepository;
use crate::web::AppState;

#[derive(Debug, Deserialize)]
pub struct FixesQuery {
    /// YYYYMMDDHHMMSS UTC format
    pub after: Option<String>,
    /// Page number (1-indexed)
    pub page: Option<i64>,
    /// Results per page
    pub per_page: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct FlightsQuery {
    /// Page number (1-indexed)
    pub page: Option<i64>,
    /// Results per page
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedFixesResponse {
    pub fixes: Vec<Fix>,
    pub page: i64,
    pub total_pages: i64,
}

#[derive(Debug, Serialize)]
pub struct PaginatedFlightsResponse {
    pub flights: Vec<FlightView>,
    pub page: i64,
    pub total_pages: i64,
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
    /// Bounding box search parameters
    pub latitude_min: Option<f64>,
    pub latitude_max: Option<f64>,
    pub longitude_min: Option<f64>,
    pub longitude_max: Option<f64>,
    /// Optional cutoff time for fixes (ISO 8601 format)
    pub after: Option<DateTime<Utc>>,
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

            let page = query.page.unwrap_or(1).max(1);
            let per_page = query.per_page.unwrap_or(50).clamp(1, 100);

            match fixes_repo
                .get_fixes_by_device_paginated(id, after_datetime, page, per_page)
                .await
            {
                Ok((fixes, total_count)) => {
                    let total_pages = ((total_count as f64) / (per_page as f64)).ceil() as i64;
                    Json(PaginatedFixesResponse {
                        fixes,
                        page,
                        total_pages,
                    })
                    .into_response()
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

/// Search devices by bounding box with enriched aircraft data
async fn search_devices_by_bbox(
    lat_max: f64,
    lat_min: f64,
    lon_max: f64,
    lon_min: f64,
    after: Option<DateTime<Utc>>,
    pool: crate::web::PgPool,
) -> impl IntoResponse {
    // Validate coordinates
    if !(-90.0..=90.0).contains(&lat_max) || !(-90.0..=90.0).contains(&lat_min) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Latitude must be between -90 and 90 degrees",
        )
        .into_response();
    }

    if !(-180.0..=180.0).contains(&lon_max) || !(-180.0..=180.0).contains(&lon_min) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Longitude must be between -180 and 180 degrees",
        )
        .into_response();
    }

    if lat_min >= lat_max {
        return json_error(
            StatusCode::BAD_REQUEST,
            "latitude_min must be less than latitude_max",
        )
        .into_response();
    }

    if lon_min >= lon_max {
        return json_error(
            StatusCode::BAD_REQUEST,
            "longitude_min must be less than longitude_max",
        )
        .into_response();
    }

    // Set default cutoff time to 24 hours ago if not provided
    let cutoff_time = after.unwrap_or_else(|| Utc::now() - Duration::hours(24));

    info!(
        "Performing bounding box search with cutoff_time: {}",
        cutoff_time
    );

    let fixes_repo = FixesRepository::new(pool.clone());

    // Perform bounding box search
    match fixes_repo
        .get_devices_with_fixes_in_bounding_box(
            lat_max,
            lon_min,
            lat_min,
            lon_max,
            cutoff_time,
            None,
        )
        .await
    {
        Ok(devices_with_fixes) => {
            info!("Found {} devices in bounding box", devices_with_fixes.len());

            // Enrich with aircraft registration and model data
            let enriched =
                enrich_devices_with_aircraft_data(devices_with_fixes, pool.clone()).await;

            info!("Enriched {} devices, returning response", enriched.len());
            Json(enriched).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get devices with fixes in bounding box: {}", e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get devices with fixes in bounding box",
            )
            .into_response()
        }
    }
}

/// Search devices by aircraft registration number
async fn search_devices_by_registration(
    registration: String,
    pool: crate::web::PgPool,
) -> impl IntoResponse {
    let device_repo = DeviceRepository::new(pool);

    match device_repo.search_by_registration(&registration).await {
        Ok(devices) => {
            let device_views: Vec<DeviceView> = devices.into_iter().map(|d| d.into()).collect();
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

/// Search devices by address and address type
async fn search_devices_by_address(
    address_str: String,
    address_type_str: String,
    pool: crate::web::PgPool,
) -> impl IntoResponse {
    // Parse address from hex string
    let address = match u32::from_str_radix(&address_str, 16) {
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

    let device_repo = DeviceRepository::new(pool);

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

/// Search devices by registration, address+type, or bounding box
#[instrument(skip(state), fields(
    has_bbox = query.latitude_max.is_some(),
    has_registration = query.registration.is_some(),
    has_address = query.address.is_some()
))]
pub async fn search_devices(
    Query(query): Query<DeviceSearchQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Check if bounding box parameters are provided
    let has_bounding_box = query.latitude_max.is_some()
        || query.latitude_min.is_some()
        || query.longitude_max.is_some()
        || query.longitude_min.is_some();

    // Route to appropriate handler
    if has_bounding_box {
        // Validate all four bounding box parameters are provided
        match (
            query.latitude_max,
            query.latitude_min,
            query.longitude_max,
            query.longitude_min,
        ) {
            (Some(lat_max), Some(lat_min), Some(lon_max), Some(lon_min)) => {
                search_devices_by_bbox(lat_max, lat_min, lon_max, lon_min, query.after, state.pool).await.into_response()
            }
            _ => json_error(
                StatusCode::BAD_REQUEST,
                "When using bounding box search, all four parameters must be provided: latitude_max, latitude_min, longitude_max, longitude_min",
            )
            .into_response(),
        }
    } else {
        // Route based on registration or address+type parameters
        match (&query.registration, &query.address, &query.address_type) {
            (Some(registration), None, None) => {
                search_devices_by_registration(registration.clone(), state.pool).await.into_response()
            }
            (None, Some(address_str), Some(address_type_str)) => {
                search_devices_by_address(address_str.clone(), address_type_str.clone(), state.pool).await.into_response()
            }
            _ => json_error(
                StatusCode::BAD_REQUEST,
                "Must provide either 'registration' OR both 'address' and 'address-type' parameters",
            )
            .into_response(),
        }
    }
}

/// Helper function to enrich devices with aircraft registration and model data
/// Optimized with batch queries to avoid N+1 query problem
#[instrument(skip(pool, devices_with_fixes), fields(device_count = devices_with_fixes.len()))]
pub(crate) async fn enrich_devices_with_aircraft_data(
    devices_with_fixes: Vec<(crate::devices::DeviceModel, Vec<crate::fixes::Fix>)>,
    pool: crate::web::PgPool,
) -> Vec<crate::live_fixes::DeviceWithFixes> {
    use std::collections::HashMap;

    if devices_with_fixes.is_empty() {
        return Vec::new();
    }

    let aircraft_registrations_repo =
        crate::aircraft_registrations_repo::AircraftRegistrationsRepository::new(pool.clone());
    let aircraft_model_repo = crate::faa::aircraft_model_repo::AircraftModelRepository::new(pool);

    // Step 1: Collect all device IDs
    let device_ids: Vec<uuid::Uuid> = devices_with_fixes
        .iter()
        .map(|(device, _)| device.id)
        .collect();

    // Step 2: Batch fetch all aircraft registrations
    info!(
        "Fetching aircraft registrations for {} devices",
        device_ids.len()
    );
    let registrations = aircraft_registrations_repo
        .get_aircraft_registrations_by_device_ids(&device_ids)
        .await
        .unwrap_or_default();
    info!("Fetched {} aircraft registrations", registrations.len());

    // Step 3: Build HashMap for O(1) lookup: device_id -> registration
    // Filter out registrations without device_id
    let registration_map: HashMap<
        uuid::Uuid,
        crate::aircraft_registrations::AircraftRegistrationModel,
    > = registrations
        .into_iter()
        .filter_map(|reg| reg.device_id.map(|id| (id, reg)))
        .collect();

    // Step 4: Collect all unique (manufacturer, model, series) keys
    let model_keys: Vec<(String, String, String)> = registration_map
        .values()
        .map(|reg| {
            (
                reg.manufacturer_code.clone(),
                reg.model_code.clone(),
                reg.series_code.clone(),
            )
        })
        .collect();

    // Step 5: Batch fetch all aircraft models
    info!("Fetching aircraft models for {} keys", model_keys.len());
    let models = aircraft_model_repo
        .get_aircraft_models_by_keys(&model_keys)
        .await
        .unwrap_or_default();
    info!("Fetched {} aircraft models", models.len());

    // Step 6: Build HashMap for O(1) lookup: (manufacturer, model, series) -> aircraft_model
    let model_map: HashMap<(String, String, String), crate::faa::aircraft_models::AircraftModel> =
        models
            .into_iter()
            .map(|model| {
                (
                    (
                        model.manufacturer_code.clone(),
                        model.model_code.clone(),
                        model.series_code.clone(),
                    ),
                    model,
                )
            })
            .collect();

    // Step 7: Build enriched results
    info!("Building enriched results");
    let mut enriched = Vec::new();
    for (device_model, device_fixes) in devices_with_fixes {
        let aircraft_registration = registration_map.get(&device_model.id).cloned();

        let aircraft_model = aircraft_registration.as_ref().and_then(|reg| {
            model_map
                .get(&(
                    reg.manufacturer_code.clone(),
                    reg.model_code.clone(),
                    reg.series_code.clone(),
                ))
                .cloned()
        });

        enriched.push(crate::live_fixes::DeviceWithFixes {
            device: device_model,
            aircraft_registration,
            aircraft_model,
            recent_fixes: device_fixes,
        });
    }

    enriched
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

/// Get all flights for a device by device ID with pagination
pub async fn get_device_flights(
    Path(id): Path<Uuid>,
    Query(query): Query<FlightsQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool);

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(100).clamp(1, 100);

    match flights_repo
        .get_flights_for_device_paginated(&id, page, per_page)
        .await
    {
        Ok((flights, total_count)) => {
            let flight_views: Vec<FlightView> = flights.into_iter().map(|f| f.into()).collect();
            let total_pages = ((total_count as f64) / (per_page as f64)).ceil() as i64;
            Json(PaginatedFlightsResponse {
                flights: flight_views,
                page,
                total_pages,
            })
            .into_response()
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
