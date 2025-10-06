use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use crate::actions::json_error;
use crate::receiver_repo::ReceiverRepository;
use crate::receivers::ReceiverModel;
use crate::web::AppState;

#[derive(Debug, Deserialize)]
pub struct ReceiverSearchQuery {
    /// Receiver callsign (partial match)
    pub callsign: Option<String>,
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
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let receiver_repo = ReceiverRepository::new(state.pool);

    match receiver_repo.get_receiver_by_id(id).await {
        Ok(Some(receiver)) => Json(receiver).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Receiver not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get receiver by ID {}: {}", id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get receiver").into_response()
        }
    }
}

/// Search receivers by callsign or bounding box
#[instrument(skip(state), fields(
    has_bbox = query.latitude_max.is_some(),
    has_callsign = query.callsign.is_some()
))]
pub async fn search_receivers(
    Query(query): Query<ReceiverSearchQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let receiver_repo = ReceiverRepository::new(state.pool);

    // Check if bounding box parameters are provided
    let has_bounding_box = query.latitude_max.is_some()
        || query.latitude_min.is_some()
        || query.longitude_max.is_some()
        || query.longitude_min.is_some();

    // Handle bounding box search
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
        json_error(
            StatusCode::BAD_REQUEST,
            "Must provide either 'callsign' OR bounding box parameters (latitude_max, latitude_min, longitude_max, longitude_min)",
        )
        .into_response()
    }
}
