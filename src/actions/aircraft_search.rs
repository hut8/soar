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

use crate::actions::views::{
    Aircraft, AircraftCluster, AircraftOrCluster, AircraftSearchResponse, AircraftView,
    AirportInfo, ClusterBounds, FlightView,
};
use crate::actions::{
    DataListResponse, DataResponse, PaginatedDataResponse, PaginationMetadata, json_error,
};
use crate::aircraft_repo::AircraftRepository;
use crate::airports_repo::AirportsRepository;
use crate::auth::AdminUser;
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
    pub south: Option<f64>,
    pub north: Option<f64>,
    pub west: Option<f64>,
    pub east: Option<f64>,
    /// Optional cutoff time for fixes (ISO 8601 format)
    pub after: Option<DateTime<Utc>>,
    /// Optional aircraft types filter (comma-separated list, e.g., "glider,tow_tug")
    #[serde(rename = "aircraft-types")]
    pub aircraft_types: Option<String>,
    /// Optional limit for number of aircraft returned (for bounding box searches)
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct BulkAircraftQuery {
    /// Comma-separated list of aircraft UUIDs (max 100)
    pub ids: String,
}

/// Get multiple aircraft by their UUIDs (max 100)
#[instrument(skip(state))]
pub async fn get_aircraft_bulk(
    Query(query): Query<BulkAircraftQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let aircraft_repo = AircraftRepository::new(state.pool);

    // Parse the comma-separated IDs
    let id_strings: Vec<&str> = query.ids.split(',').map(|s| s.trim()).collect();

    // Validate max 100 IDs
    if id_strings.len() > 100 {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Maximum 100 device IDs allowed per request",
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

    Json(DataResponse { data: aircraft_map }).into_response()
}

/// Get an aircraft by its UUID
pub async fn get_aircraft_by_id(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    use crate::aircraft::AircraftModel;
    use crate::schema::aircraft;
    use diesel::prelude::*;

    let mut conn = match state.pool.get() {
        Ok(conn) => conn,
        Err(e) => {
            tracing::error!("Failed to get database connection: {}", e);
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database connection failed",
            )
            .into_response();
        }
    };

    // Query aircraft table directly to preserve current_fix field
    match aircraft::table
        .filter(aircraft::id.eq(id))
        .select(AircraftModel::as_select())
        .first(&mut conn)
        .optional()
    {
        Ok(Some(aircraft_model)) => {
            // Convert AircraftModel directly to AircraftView to preserve current_fix
            let aircraft_view: AircraftView = aircraft_model.into();
            Json(DataResponse {
                data: aircraft_view,
            })
            .into_response()
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
                    Json(PaginatedDataResponse {
                        data: fixes,
                        metadata: PaginationMetadata {
                            page,
                            total_pages,
                            total_count,
                        },
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

/// Search aircraft by bounding box with enriched aircraft data
async fn search_aircraft_by_bbox(
    north: f64,
    south: f64,
    east: f64,
    west: f64,
    after: Option<DateTime<Utc>>,
    limit: Option<i64>,
    pool: crate::web::PgPool,
) -> impl IntoResponse {
    // Validate limit (hard cap at 1000)
    if let Some(lim) = limit
        && lim > 1000
    {
        return json_error(StatusCode::BAD_REQUEST, "Limit cannot exceed 1000 aircraft")
            .into_response();
    }

    // Validate coordinates
    if !(-90.0..=90.0).contains(&north) || !(-90.0..=90.0).contains(&south) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Latitude must be between -90 and 90 degrees",
        )
        .into_response();
    }

    if !(-180.0..=180.0).contains(&east) || !(-180.0..=180.0).contains(&west) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Longitude must be between -180 and 180 degrees",
        )
        .into_response();
    }

    if south >= north {
        return json_error(StatusCode::BAD_REQUEST, "south must be less than north")
            .into_response();
    }

    // Note: west can be >= east when crossing the International Date Line
    // The PostGIS query in fixes_repo.rs handles this case by splitting into two bounding boxes

    // Set default cutoff time to 15 minutes ago if not provided
    let cutoff_time = after.unwrap_or_else(|| Utc::now() - Duration::minutes(15));

    info!(
        "Performing bounding box search with cutoff_time: {}",
        cutoff_time
    );

    let fixes_repo = FixesRepository::new(pool.clone());
    let aircraft_repo = AircraftRepository::new(pool.clone());

    // First, get the total count of aircraft in the bounding box
    let total_count = match fixes_repo
        .count_aircraft_in_bounding_box(north, west, south, east, cutoff_time)
        .await
    {
        Ok(count) => count,
        Err(e) => {
            tracing::error!("Failed to count aircraft in bounding box: {}", e);
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to count aircraft in bounding box",
            )
            .into_response();
        }
    };

    info!("Total aircraft in bounding box: {}", total_count);

    // Use clustering if total count exceeds 50 aircraft
    if total_count > 50 {
        info!("Total count exceeds 50, using clustering");

        let grid_size = 0.1; // 0.1 degrees (~11km)

        match fixes_repo
            .get_clustered_aircraft_in_bounding_box(
                north,
                west,
                south,
                east,
                cutoff_time,
                grid_size,
            )
            .await
        {
            Ok(clusters) => {
                info!("Generated {} clusters", clusters.len());

                let items: Vec<AircraftOrCluster> = clusters
                    .into_iter()
                    .map(|cluster| {
                        let cluster_id = format!(
                            "cluster_{}_{}",
                            (cluster.grid_lat * 1000.0) as i64,
                            (cluster.grid_lng * 1000.0) as i64
                        );

                        AircraftOrCluster::Cluster {
                            data: AircraftCluster {
                                id: cluster_id,
                                latitude: cluster.centroid_lat,
                                longitude: cluster.centroid_lng,
                                count: cluster.aircraft_count,
                                bounds: ClusterBounds {
                                    north: cluster.max_lat,
                                    south: cluster.min_lat,
                                    east: cluster.max_lng,
                                    west: cluster.min_lng,
                                },
                            },
                        }
                    })
                    .collect();

                Json(AircraftSearchResponse {
                    items,
                    total: total_count,
                    clustered: true,
                })
                .into_response()
            }
            Err(e) => {
                tracing::error!("Failed to get clustered aircraft: {}", e);
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to get clustered aircraft",
                )
                .into_response()
            }
        }
    } else {
        // Perform bounding box search - get aircraft with all fields from database
        // Pass limit to database query for efficient filtering
        match aircraft_repo
            .find_aircraft_in_bounding_box(north, west, south, east, cutoff_time, limit)
            .await
        {
            Ok(aircraft_models) => {
                info!("Found {} aircraft in bounding box", aircraft_models.len());
                info!("Converting {} aircraft to views", aircraft_models.len());

                // Convert AircraftModel to AircraftView (this preserves all fields including current_fix)
                let aircraft_views: Vec<AircraftView> = aircraft_models
                    .into_iter()
                    .map(|model| model.into())
                    .collect();

                // Wrap in Aircraft struct (which just contains AircraftView)
                let aircraft_list: Vec<Aircraft> = aircraft_views
                    .into_iter()
                    .map(|view| Aircraft { device: view })
                    .collect();

                // Wrap aircraft in AircraftOrCluster enum
                let items: Vec<AircraftOrCluster> = aircraft_list
                    .into_iter()
                    .map(|aircraft| AircraftOrCluster::Aircraft {
                        data: Box::new(aircraft),
                    })
                    .collect();

                Json(AircraftSearchResponse {
                    items,
                    total: total_count,
                    clustered: false,
                })
                .into_response()
            }
            Err(e) => {
                tracing::error!("Failed to get aircraft in bounding box: {}", e);
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to get aircraft in bounding box",
                )
                .into_response()
            }
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
            Json(DataListResponse { data: device_views }).into_response()
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
            Json(DataListResponse {
                data: vec![device_view],
            })
            .into_response()
        }
        Ok(None) => Json(DataListResponse::<AircraftView> { data: vec![] }).into_response(),
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
    has_bbox = query.north.is_some(),
    has_registration = query.registration.is_some(),
    has_address = query.address.is_some()
))]
pub async fn search_aircraft(
    Query(query): Query<AircraftSearchQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Check if bounding box parameters are provided
    let has_bounding_box = query.north.is_some()
        || query.south.is_some()
        || query.east.is_some()
        || query.west.is_some();

    // Route to appropriate handler
    if has_bounding_box {
        // Validate all four bounding box parameters are provided
        match (
            query.north,
            query.south,
            query.east,
            query.west,
        ) {
            (Some(north), Some(south), Some(east), Some(west)) => {
                search_aircraft_by_bbox(north, south, east, west, query.after, query.limit, state.pool).await.into_response()
            }
            _ => json_error(
                StatusCode::BAD_REQUEST,
                "When using bounding box search, all four parameters must be provided: north, south, east, west",
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
        .get_recent_aircraft(10, aircraft_type_filters)
        .await
    {
        Ok(aircraft) => {
            let aircraft_views: Vec<AircraftView> =
                aircraft.into_iter().map(|a| a.into()).collect();
            Json(DataListResponse {
                data: aircraft_views,
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

/// Get all devices for a club by club ID
pub async fn get_aircraft_by_club(
    Path(club_id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let device_repo = AircraftRepository::new(state.pool);

    match device_repo.search_by_club_id(club_id).await {
        Ok(devices) => {
            let device_views: Vec<AircraftView> = devices.into_iter().map(|d| d.into()).collect();
            Json(DataListResponse { data: device_views }).into_response()
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
            Json(PaginatedDataResponse {
                data: flight_views,
                metadata: PaginationMetadata {
                    page,
                    total_pages,
                    total_count,
                },
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
#[serde(rename_all = "camelCase")]
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
        Ok(true) => Json(DataResponse {
            data: UpdateDeviceClubResponse {
                success: true,
                message: "Aircraft club assignment updated successfully".to_string(),
            },
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
