use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use uuid::Uuid;

use crate::actions::json_error;
use crate::receiver_repo::ReceiverRepository;
use crate::receivers::ReceiverModel;
use crate::web::AppState;

#[derive(Debug, Deserialize)]
pub struct ReceiverSearchQuery {
    /// General text search across callsign, description, country, contact, and email
    pub query: Option<String>,
    /// Receiver callsign (partial match)
    pub callsign: Option<String>,
    /// Location-based search parameters
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    /// Radius in miles (default 100)
    pub radius_miles: Option<f64>,
    /// Bounding box search parameters
    pub latitude_min: Option<f64>,
    pub latitude_max: Option<f64>,
    pub longitude_min: Option<f64>,
    pub longitude_max: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ReceiverSearchResponse {
    pub receivers: Vec<ReceiverModel>,
}

/// Get a receiver by its ID
pub async fn get_receiver_by_id(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let receiver_repo = ReceiverRepository::new(state.pool);

    match receiver_repo.get_receiver_view_by_id(id).await {
        Ok(Some(receiver)) => Json(receiver).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Receiver not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get receiver by ID {}: {}", id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get receiver").into_response()
        }
    }
}

/// Search receivers by query, location, callsign, or bounding box
#[instrument(skip(state), fields(
    has_query = query.query.is_some(),
    has_location = query.latitude.is_some() && query.longitude.is_some(),
    has_bbox = query.latitude_max.is_some(),
    has_callsign = query.callsign.is_some()
))]
pub async fn search_receivers(
    Query(query): Query<ReceiverSearchQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let receiver_repo = ReceiverRepository::new(state.pool);

    // Priority 1: General text query search
    if let Some(search_query) = query.query {
        info!("Performing query search: {}", search_query);
        match receiver_repo.search_by_query(&search_query).await {
            Ok(receivers) => {
                info!("Found {} receivers matching query", receivers.len());
                return Json(ReceiverSearchResponse { receivers }).into_response();
            }
            Err(e) => {
                tracing::error!(
                    "Failed to search receivers by query {}: {}",
                    search_query,
                    e
                );
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to search receivers",
                )
                .into_response();
            }
        }
    }

    // Priority 2: Location-based search with radius
    if let (Some(lat), Some(lon)) = (query.latitude, query.longitude) {
        let radius = query.radius_miles.unwrap_or(100.0);

        // Validate coordinates
        if !(-90.0..=90.0).contains(&lat) {
            return json_error(
                StatusCode::BAD_REQUEST,
                "Latitude must be between -90 and 90 degrees",
            )
            .into_response();
        }

        if !(-180.0..=180.0).contains(&lon) {
            return json_error(
                StatusCode::BAD_REQUEST,
                "Longitude must be between -180 and 180 degrees",
            )
            .into_response();
        }

        info!(
            "Performing location search: lat={}, lon={}, radius={} miles",
            lat, lon, radius
        );

        match receiver_repo
            .get_receivers_within_radius(lat, lon, radius)
            .await
        {
            Ok(receivers) => {
                info!("Found {} receivers within radius", receivers.len());
                return Json(ReceiverSearchResponse { receivers }).into_response();
            }
            Err(e) => {
                tracing::error!("Failed to search receivers by location: {}", e);
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to search receivers by location",
                )
                .into_response();
            }
        }
    }

    // Priority 3: Bounding box search
    let has_bounding_box = query.latitude_max.is_some()
        || query.latitude_min.is_some()
        || query.longitude_max.is_some()
        || query.longitude_min.is_some();

    if has_bounding_box {
        match (
            query.latitude_max,
            query.latitude_min,
            query.longitude_max,
            query.longitude_min,
        ) {
            (Some(lat_max), Some(lat_min), Some(lon_max), Some(lon_min)) => {
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

                info!(
                    "Performing bounding box search for receivers: lat=[{}, {}], lon=[{}, {}]",
                    lat_min, lat_max, lon_min, lon_max
                );

                // Perform bounding box search
                match receiver_repo
                    .get_receivers_in_bounding_box(lat_max, lon_min, lat_min, lon_max)
                    .await
                {
                    Ok(receivers) => {
                        info!("Found {} receivers in bounding box", receivers.len());
                        Json(ReceiverSearchResponse { receivers }).into_response()
                    }
                    Err(e) => {
                        tracing::error!("Failed to get receivers in bounding box: {}", e);
                        json_error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Failed to get receivers in bounding box",
                        )
                        .into_response()
                    }
                }
            }
            _ => json_error(
                StatusCode::BAD_REQUEST,
                "When using bounding box search, all four parameters must be provided: latitude_max, latitude_min, longitude_max, longitude_min",
            )
            .into_response(),
        }
    } else if let Some(callsign) = query.callsign {
        // Search by callsign
        match receiver_repo.search_by_callsign(&callsign).await {
            Ok(receivers) => {
                let receiver_models: Vec<ReceiverModel> =
                    receivers.into_iter().map(|r| r.into()).collect();
                Json(ReceiverSearchResponse {
                    receivers: receiver_models,
                })
                .into_response()
            }
            Err(e) => {
                tracing::error!("Failed to search receivers by callsign {}: {}", callsign, e);
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to search receivers",
                )
                .into_response()
            }
        }
    } else {
        // No search parameters provided - return recently updated receivers
        info!("No search parameters provided, returning recently updated receivers");
        match receiver_repo.get_recently_updated_receivers(10).await {
            Ok(receivers) => {
                info!("Returning {} recently updated receivers", receivers.len());
                let receiver_models: Vec<ReceiverModel> =
                    receivers.into_iter().map(|r| r.into()).collect();
                Json(ReceiverSearchResponse {
                    receivers: receiver_models,
                })
                .into_response()
            }
            Err(e) => {
                tracing::error!("Failed to get recently updated receivers: {}", e);
                json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get receivers")
                    .into_response()
            }
        }
    }
}

/// Get fixes received by a specific receiver (last 24 hours only)
pub async fn get_receiver_fixes(
    Path(id): Path<Uuid>,
    Query(params): Query<ReceiverFixesQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    use crate::fixes_repo::FixesRepository;

    let receiver_repo = ReceiverRepository::new(state.pool.clone());
    let fixes_repo = FixesRepository::new(state.pool.clone());

    // First verify the receiver exists
    match receiver_repo.get_receiver_by_id(id).await {
        Ok(Some(_)) => {}
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Receiver not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get receiver {}: {}", id, e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get receiver")
                .into_response();
        }
    };

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(100).clamp(1, 100);

    // Get fixes where receiver_id = id (last 24 hours only)
    match fixes_repo
        .get_fixes_by_receiver_id_paginated(id, page, per_page)
        .await
    {
        Ok((fixes, total_count)) => {
            let total_pages = ((total_count as f64) / (per_page as f64)).ceil() as i64;
            Json(ReceiverFixesResponse {
                fixes,
                page,
                total_pages,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get fixes for receiver {}: {}", id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get receiver fixes",
            )
            .into_response()
        }
    }
}

/// Get statuses for a specific receiver
pub async fn get_receiver_statuses(
    Path(id): Path<Uuid>,
    Query(params): Query<ReceiverStatusesQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    use crate::receiver_status_repo::ReceiverStatusRepository;

    let receiver_repo = ReceiverRepository::new(state.pool.clone());
    let status_repo = ReceiverStatusRepository::new(state.pool.clone());

    // First verify the receiver exists
    match receiver_repo.get_receiver_by_id(id).await {
        Ok(Some(_)) => {}
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Receiver not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get receiver {}: {}", id, e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get receiver")
                .into_response();
        }
    };

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(100).clamp(1, 100);

    // Get statuses for this receiver
    match status_repo
        .get_statuses_by_receiver_paginated(id, page, per_page)
        .await
    {
        Ok((statuses, total_count)) => {
            let total_pages = ((total_count as f64) / (per_page as f64)).ceil() as i64;
            Json(ReceiverStatusesResponse {
                statuses,
                page,
                total_pages,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get statuses for receiver {}: {}", id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get receiver statuses",
            )
            .into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ReceiverFixesQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ReceiverFixesResponse {
    pub fixes: Vec<crate::fixes::Fix>,
    pub page: i64,
    pub total_pages: i64,
}

#[derive(Debug, Deserialize)]
pub struct ReceiverStatusesQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ReceiverStatusesResponse {
    pub statuses: Vec<crate::receiver_statuses::ReceiverStatus>,
    pub page: i64,
    pub total_pages: i64,
}

/// Get raw messages for a specific receiver (last 24 hours only)
pub async fn get_receiver_raw_messages(
    Path(id): Path<Uuid>,
    Query(params): Query<ReceiverRawMessagesQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    use crate::aprs_messages_repo::AprsMessagesRepository;

    let receiver_repo = ReceiverRepository::new(state.pool.clone());
    let messages_repo = AprsMessagesRepository::new(state.pool.clone());

    // First verify the receiver exists
    match receiver_repo.get_receiver_by_id(id).await {
        Ok(Some(_)) => {}
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Receiver not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get receiver {}: {}", id, e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get receiver")
                .into_response();
        }
    };

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(100).clamp(1, 100);

    // Get raw messages for this receiver (last 24 hours only)
    match messages_repo
        .get_messages_by_receiver_paginated(id, page, per_page)
        .await
    {
        Ok((messages, total_count)) => {
            let total_pages = ((total_count as f64) / (per_page as f64)).ceil() as i64;
            Json(ReceiverRawMessagesResponse {
                messages,
                page,
                total_pages,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get raw messages for receiver {}: {}", id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get receiver raw messages",
            )
            .into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ReceiverStatisticsQuery {
    /// Number of days to include in statistics (defaults to all time if not specified)
    pub days: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ReceiverStatisticsResponse {
    pub average_update_interval_seconds: Option<f64>,
    pub total_status_count: i64,
    pub days_included: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ReceiverRawMessagesQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ReceiverRawMessagesResponse {
    pub messages: Vec<crate::aprs_messages_repo::AprsMessage>,
    pub page: i64,
    pub total_pages: i64,
}

#[derive(Debug, Serialize)]
pub struct AprsTypeCount {
    pub aprs_type: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct DeviceFixCount {
    pub device_id: uuid::Uuid,
    pub count: i64,
}

/// Get statistics for a specific receiver
pub async fn get_receiver_statistics(
    Path(id): Path<Uuid>,
    Query(params): Query<ReceiverStatisticsQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    use crate::receiver_status_repo::ReceiverStatusRepository;

    let receiver_repo = ReceiverRepository::new(state.pool.clone());
    let status_repo = ReceiverStatusRepository::new(state.pool.clone());

    // First verify the receiver exists
    match receiver_repo.get_receiver_by_id(id).await {
        Ok(Some(_)) => {}
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Receiver not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get receiver {}: {}", id, e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get receiver")
                .into_response();
        }
    };

    // Determine time range for statistics
    let (start_time, end_time) = if let Some(days) = params.days {
        let end = chrono::Utc::now();
        let start = end - chrono::Duration::days(days);
        (Some(start), Some(end))
    } else {
        (None, None)
    };

    // Get average update interval
    let avg_interval_seconds = match status_repo
        .get_average_update_interval(id, start_time, end_time)
        .await
    {
        Ok(interval) => interval,
        Err(e) => {
            tracing::error!(
                "Failed to get average update interval for receiver {}: {}",
                id,
                e
            );
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to calculate statistics",
            )
            .into_response();
        }
    };

    // Get total status count
    let total_count = match status_repo.count_for_receiver(id).await {
        Ok(count) => count,
        Err(e) => {
            tracing::error!("Failed to count statuses for receiver {}: {}", id, e);
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to calculate statistics",
            )
            .into_response();
        }
    };

    Json(ReceiverStatisticsResponse {
        average_update_interval_seconds: avg_interval_seconds,
        total_status_count: total_count,
        days_included: params.days,
    })
    .into_response()
}

/// Get aggregate statistics for a specific receiver (fix counts by type and device, last 24 hours only)
pub async fn get_receiver_aggregate_stats(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    use crate::fixes_repo::FixesRepository;

    let receiver_repo = ReceiverRepository::new(state.pool.clone());
    let fixes_repo = FixesRepository::new(state.pool.clone());

    // First verify the receiver exists
    match receiver_repo.get_receiver_by_id(id).await {
        Ok(Some(_)) => {}
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Receiver not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get receiver {}: {}", id, e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get receiver")
                .into_response();
        }
    };

    // Get fix counts grouped by APRS type (last 24 hours only)
    let fix_counts = match fixes_repo
        .get_fix_counts_by_aprs_type_for_receiver(id)
        .await
    {
        Ok(counts) => counts,
        Err(e) => {
            tracing::error!(
                "Failed to get fix counts by APRS type for receiver {}: {}",
                id,
                e
            );
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get fix counts by APRS type",
            )
            .into_response();
        }
    };

    // Get fix counts grouped by device (last 24 hours only)
    let device_fix_counts = match fixes_repo.get_fix_counts_by_device_for_receiver(id).await {
        Ok(counts) => counts,
        Err(e) => {
            tracing::error!(
                "Failed to get fix counts by device for receiver {}: {}",
                id,
                e
            );
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get fix counts by device",
            )
            .into_response();
        }
    };

    Json(ReceiverAggregateStatsResponse {
        fix_counts_by_aprs_type: fix_counts,
        fix_counts_by_device: device_fix_counts,
    })
    .into_response()
}

#[derive(Debug, Serialize)]
pub struct ReceiverAggregateStatsResponse {
    pub fix_counts_by_aprs_type: Vec<AprsTypeCount>,
    pub fix_counts_by_device: Vec<DeviceFixCount>,
}
