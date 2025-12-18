use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::actions::json_error;
use crate::actions::views::{Aircraft, AircraftView, AirportInfo, FlightView};
use crate::aircraft_repo::AircraftRepository;
use crate::airports_repo::AirportsRepository;
use crate::auth::AdminUser;
use crate::fixes::Fix;
use crate::fixes_repo::FixesRepository;
use crate::flights_repo::FlightsRepository;
use crate::web::AppState;

#[derive(Debug, Deserialize)]
pub struct FixesQuery {
    /// ISO 8601 UTC format
    pub after: Option<DateTime<Utc>>,
    /// Page number (1-indexed)
    pub page: Option<i64>,
    /// Results per page
    pub per_page: Option<i64>,
    /// Filter for active fixes only
    pub active: Option<bool>,
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
    pub fixes: Vec<crate::fixes::FixWithRawPacket>,
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
pub struct AircraftSearchQuery {
    /// Aircraft registration number (e.g., N8437D)
    pub registration: Option<String>,
    /// Aircraft address in hex format (e.g., ABCDEF)
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
    /// Optional aircraft types filter (comma-separated list, e.g., "glider,tow_tug")
    #[serde(rename = "aircraft-types")]
    pub aircraft_types: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AircraftSearchResponse {
    pub aircraft: Vec<AircraftView>,
}

#[derive(Debug, Serialize)]
pub struct AircraftFixesResponse {
    pub fixes: Vec<Fix>,
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct BulkAircraftQuery {
    /// Comma-separated list of aircraft UUIDs (max 10)
    pub ids: String,
}

#[derive(Debug, Serialize)]
pub struct BulkAircraftResponse {
    pub aircraft: HashMap<String, AircraftView>,
}

/// Get multiple aircraft by their UUIDs (max 10)
#[instrument(skip(state))]
pub async fn get_aircraft_bulk(
    Query(query): Query<BulkAircraftQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let aircraft_repo = AircraftRepository::new(state.pool);

    // Parse the comma-separated IDs
    let id_strings: Vec<&str> = query.ids.split(',').map(|s| s.trim()).collect();

    // Validate max 10 IDs
    if id_strings.len() > 10 {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Maximum 10 device IDs allowed per request",
        )
        .into_response();
    }

    // Parse UUIDs
    let mut uuids = Vec::new();
    for id_str in &id_strings {
        match Uuid::parse_str(id_str) {
            Ok(uuid) => uuids.push(uuid),
            Err(_) => {
                return json_error(
                    StatusCode::BAD_REQUEST,
                    &format!("Invalid UUID format: {}", id_str),
                )
                .into_response();
            }
        }
    }

    // Fetch aircraft
    let mut aircraft_map = HashMap::new();
    for uuid in uuids {
        if let Ok(Some(aircraft)) = aircraft_repo.get_aircraft_by_id(uuid).await {
            let aircraft_view: AircraftView = aircraft.into();
            aircraft_map.insert(uuid.to_string(), aircraft_view);
        }
    }

    Json(BulkAircraftResponse {
        aircraft: aircraft_map,
    })
    .into_response()
}

/// Get an aircraft by its UUID
pub async fn get_aircraft_by_id(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let aircraft_repo = AircraftRepository::new(state.pool);

    // First try to find the aircraft by UUID in the aircraft table
    match aircraft_repo.get_aircraft_by_id(id).await {
        Ok(Some(aircraft)) => {
            let aircraft_view: AircraftView = aircraft.into();
            Json(aircraft_view).into_response()
        }
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Aircraft not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get aircraft by ID {}: {}", id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get aircraft").into_response()
        }
    }
}

/// Get fixes for an aircraft with optional after parameter
pub async fn get_aircraft_fixes(
    Path(id): Path<Uuid>,
    Query(query): Query<FixesQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let aircraft_repo = AircraftRepository::new(state.pool.clone());
    let fixes_repo = FixesRepository::new(state.pool.clone());

    // First verify the aircraft exists
    match aircraft_repo.get_aircraft_by_id(id).await {
        Ok(Some(_aircraft)) => {
            // Aircraft exists, get fixes
            let page = query.page.unwrap_or(1).max(1);
            let per_page = query.per_page.unwrap_or(50).clamp(1, 100);
            let active_only = query.active;

            match fixes_repo
                .get_fixes_by_aircraft_paginated(id, query.after, page, per_page, active_only)
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
                        "Failed to get aircraft fixes",
                    )
                    .into_response()
                }
            }
        }
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Aircraft not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to verify device exists {}: {}", id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to verify aircraft",
            )
            .into_response()
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

    // Perform bounding box search - fetch the 10 most recent fixes per device
    // This provides enough data for trail rendering without additional API calls
    match fixes_repo
        .get_aircraft_with_fixes_in_bounding_box(
            lat_max,
            lon_min,
            lat_min,
            lon_max,
            cutoff_time,
            Some(10),
        )
        .await
    {
        Ok(aircraft_with_fixes) => {
            info!(
                "Found {} aircraft in bounding box",
                aircraft_with_fixes.len()
            );

            // Enrich with aircraft registration and model data
            let enriched =
                enrich_aircraft_with_registration_data(aircraft_with_fixes, pool.clone()).await;

            info!("Enriched {} aircraft, returning response", enriched.len());
            Json(enriched).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get devices with fixes in bounding box: {}", e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get aircraft with fixes in bounding box",
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
    let device_repo = AircraftRepository::new(pool);

    match device_repo.search_by_registration(&registration).await {
        Ok(devices) => {
            let device_views: Vec<AircraftView> = devices.into_iter().map(|d| d.into()).collect();
            Json(AircraftSearchResponse {
                aircraft: device_views,
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
                "Failed to search aircraft",
            )
            .into_response()
        }
    }
}

/// Search devices by address
/// Note: address_type_str is still accepted for backwards compatibility but ignored
async fn search_devices_by_address(
    address_str: String,
    _address_type_str: String,
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

    let device_repo = AircraftRepository::new(pool);

    // Search by address (address is unique now)
    match device_repo.search_by_address(address).await {
        Ok(Some(device)) => {
            let device_view: AircraftView = device.into();
            Json(AircraftSearchResponse {
                aircraft: vec![device_view],
            })
            .into_response()
        }
        Ok(None) => Json(AircraftSearchResponse { aircraft: vec![] }).into_response(),
        Err(e) => {
            tracing::error!("Failed to search devices by address {}: {}", address, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to search aircraft",
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
pub async fn search_aircraft(
    Query(query): Query<AircraftSearchQuery>,
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
            (None, None, None) => {
                // No search criteria - return 10 most recently heard from devices
                get_recent_aircraft_response(state.pool, query.aircraft_types).await.into_response()
            }
            _ => json_error(
                StatusCode::BAD_REQUEST,
                "Must provide either 'registration' OR both 'address' and 'address-type' parameters",
            )
            .into_response(),
        }
    }
}

/// Get 10 most recently heard from aircraft
async fn get_recent_aircraft_response(
    pool: crate::web::PgPool,
    aircraft_types: Option<String>,
) -> impl IntoResponse {
    let aircraft_repo = AircraftRepository::new(pool);

    // Parse aircraft types from comma-separated string
    let aircraft_type_filters = aircraft_types.as_ref().map(|types_str| {
        types_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<String>>()
    });

    match aircraft_repo
        .get_recent_aircraft_with_location(10, aircraft_type_filters)
        .await
    {
        Ok(aircraft_with_location) => {
            let aircraft_views: Vec<AircraftView> = aircraft_with_location
                .into_iter()
                .map(
                    |(aircraft_model, latest_lat, latest_lng, active_flight_id)| {
                        let mut aircraft_view: AircraftView = aircraft_model.into();
                        aircraft_view.latest_latitude = latest_lat;
                        aircraft_view.latest_longitude = latest_lng;
                        aircraft_view.active_flight_id = active_flight_id;
                        aircraft_view
                    },
                )
                .collect();
            Json(AircraftSearchResponse {
                aircraft: aircraft_views,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get recent aircraft: {}", e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get recent aircraft",
            )
            .into_response()
        }
    }
}

/// Helper function to enrich aircraft with registration and model data
/// Optimized with batch queries to avoid N+1 query problem
#[instrument(skip(pool, aircraft_with_fixes), fields(aircraft_count = aircraft_with_fixes.len()))]
pub(crate) async fn enrich_aircraft_with_registration_data(
    aircraft_with_fixes: Vec<(crate::aircraft::AircraftModel, Vec<crate::fixes::Fix>)>,
    pool: crate::web::PgPool,
) -> Vec<Aircraft> {
    use std::collections::HashMap;

    if aircraft_with_fixes.is_empty() {
        return Vec::new();
    }

    let aircraft_registrations_repo =
        crate::aircraft_registrations_repo::AircraftRegistrationsRepository::new(pool.clone());
    let aircraft_model_repo = crate::faa::aircraft_model_repo::AircraftModelRepository::new(pool);

    // Step 1: Collect all aircraft IDs
    let device_ids: Vec<uuid::Uuid> = aircraft_with_fixes
        .iter()
        .map(|(aircraft, _)| aircraft.id)
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
        .filter_map(|reg| reg.aircraft_id.map(|id| (id, reg)))
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
    for (aircraft_model, aircraft_fixes) in aircraft_with_fixes {
        let aircraft_registration = registration_map.get(&aircraft_model.id).cloned();

        let faa_aircraft_model = aircraft_registration.as_ref().and_then(|reg| {
            model_map
                .get(&(
                    reg.manufacturer_code.clone(),
                    reg.model_code.clone(),
                    reg.series_code.clone(),
                ))
                .cloned()
        });

        // Convert AircraftModel to AircraftView
        let mut aircraft_view = AircraftView::from_device_model(aircraft_model);
        // Add the fixes to the aircraft view
        aircraft_view.fixes = Some(aircraft_fixes);

        enriched.push(Aircraft {
            device: aircraft_view,
            aircraft_registration,
            aircraft_model_details: faa_aircraft_model,
        });
    }

    enriched
}

/// Get all devices for a club by club ID
pub async fn get_aircraft_by_club(
    Path(club_id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let device_repo = AircraftRepository::new(state.pool);

    match device_repo.search_by_club_id(club_id).await {
        Ok(devices) => {
            let device_views: Vec<AircraftView> = devices.into_iter().map(|d| d.into()).collect();
            Json(AircraftSearchResponse {
                aircraft: device_views,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get devices for club {}: {}", club_id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get aircraft for club",
            )
            .into_response()
        }
    }
}

/// Get all flights for a device by device ID with pagination
pub async fn get_aircraft_flights(
    Path(id): Path<Uuid>,
    Query(query): Query<FlightsQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool.clone());
    let airports_repo = AirportsRepository::new(state.pool.clone());

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(100).clamp(1, 100);

    match flights_repo
        .get_flights_for_device_paginated(&id, page, per_page)
        .await
    {
        Ok((flights, total_count)) => {
            let mut flight_views = Vec::new();

            for flight in flights {
                // Look up airport identifiers and country codes if airport IDs are present
                let departure_airport = if let Some(dep_id) = flight.departure_airport_id {
                    airports_repo
                        .get_airport_by_id(dep_id)
                        .await
                        .ok()
                        .flatten()
                        .map(|a| AirportInfo {
                            ident: Some(a.ident),
                            country: a.iso_country,
                        })
                } else {
                    None
                };

                let arrival_airport = if let Some(arr_id) = flight.arrival_airport_id {
                    airports_repo
                        .get_airport_by_id(arr_id)
                        .await
                        .ok()
                        .flatten()
                        .map(|a| AirportInfo {
                            ident: Some(a.ident),
                            country: a.iso_country,
                        })
                } else {
                    None
                };

                let flight_view =
                    FlightView::from_flight(flight, departure_airport, arrival_airport, None);
                flight_views.push(flight_view);
            }

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
                "Failed to get flights for aircraft",
            )
            .into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateDeviceClubRequest {
    pub club_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct UpdateDeviceClubResponse {
    pub success: bool,
    pub message: String,
}

/// Update the club assignment for a device (admin only)
pub async fn update_aircraft_club(
    AdminUser(_user): AdminUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateDeviceClubRequest>,
) -> impl IntoResponse {
    let device_repo = AircraftRepository::new(state.pool);

    match device_repo.update_club_id(id, payload.club_id).await {
        Ok(true) => Json(UpdateDeviceClubResponse {
            success: true,
            message: "Aircraft club assignment updated successfully".to_string(),
        })
        .into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "Aircraft not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to update device club assignment: {}", e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update aircraft club assignment",
            )
            .into_response()
        }
    }
}
