use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::actions::{DataResponse, json_error};
use crate::aircraft_images::AircraftImageCollection;
use crate::aircraft_images_client::AircraftImagesClient;
use crate::aircraft_repo::AircraftRepository;
use crate::web::AppState;

/// Get aircraft images from cache or fetch from external APIs
///
/// Smart caching strategy:
/// 1. Check which sources haven't been queried yet - query them
/// 2. Check which sources were queried >1 week ago AND returned empty - re-query them
/// 3. Update timestamps for all queried sources
/// 4. Merge results from all sources
/// 5. Save updated collection to database
#[instrument(skip(state))]
pub async fn get_aircraft_images(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    info!("Fetching images for aircraft {}", id);

    let aircraft_repo = AircraftRepository::new(state.pool.clone());

    // Fetch aircraft from database
    let aircraft = match aircraft_repo.get_aircraft_model_by_id(id).await {
        Ok(Some(aircraft)) => aircraft,
        Ok(None) => {
            warn!("Aircraft not found: {}", id);
            return json_error(StatusCode::NOT_FOUND, "Aircraft not found").into_response();
        }
        Err(e) => {
            error!(aircraft_id = %id, error = %e, "Failed to fetch aircraft");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch aircraft",
            )
            .into_response();
        }
    };

    // Load existing collection or create empty one
    let mut collection = if let Some(images_json) = &aircraft.images {
        match serde_json::from_value::<AircraftImageCollection>(images_json.clone()) {
            Ok(collection) => {
                info!(
                    "Loaded cached image collection for aircraft {} ({} images, {} sources queried)",
                    id,
                    collection.images.len(),
                    collection.last_fetched.len()
                );
                collection
            }
            Err(e) => {
                warn!(
                    "Failed to deserialize cached images for aircraft {}: {} - starting fresh",
                    id, e
                );
                AircraftImageCollection::empty()
            }
        }
    } else {
        info!("No cached images for aircraft {} - starting fresh", id);
        AircraftImageCollection::empty()
    };

    // Determine which sources need querying
    let sources_to_query = collection.sources_to_query();

    if sources_to_query.is_empty() {
        info!(
            "All sources up-to-date for aircraft {} - returning cached results ({} images)",
            id,
            collection.images.len()
        );
        return Json(DataResponse { data: collection }).into_response();
    }

    info!(
        "Querying {} sources for aircraft {}: {:?}",
        sources_to_query.len(),
        id,
        sources_to_query
    );

    // Create HTTP client for fetching
    let client = AircraftImagesClient::new(reqwest::Client::new());
    let mode_s_hex = aircraft.aircraft_address_hex();

    // Track if we made any changes
    let mut collection_updated = false;

    // Query each source that needs updating
    for source in sources_to_query {
        info!("Fetching images from {:?} for aircraft {}", source, id);

        match client
            .fetch_from_source(
                source,
                mode_s_hex.as_deref(),
                aircraft.registration.as_deref(),
                5, // Request 5 images per source
            )
            .await
        {
            Ok(images) => {
                info!(
                    "Fetched {} images from {:?} for aircraft {}",
                    images.len(),
                    source,
                    id
                );
                collection.add_images_from_source(source, images);
                collection_updated = true;
            }
            Err(e) => {
                warn!(
                    "Failed to fetch images from {:?} for aircraft {}: {} - updating timestamp anyway",
                    source, id, e
                );
                // Update timestamp even on failure to prevent repeated failed attempts
                collection.update_timestamp(source);
                collection_updated = true;
            }
        }
    }

    // Save updated collection to database if we made changes
    if collection_updated {
        let images_json = match serde_json::to_value(&collection) {
            Ok(json) => json,
            Err(e) => {
                error!(aircraft_id = %id, error = %e, "Failed to serialize images for aircraft");
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to serialize images",
                )
                .into_response();
            }
        };

        if let Err(e) = aircraft_repo.update_images(id, images_json).await {
            // Log error but don't fail the request - we still have the images
            warn!("Failed to cache images for aircraft {}: {}", id, e);
        } else {
            info!("Saved updated image collection for aircraft {}", id);
        }
    }

    info!(
        "Returning {} images from {} sources for aircraft {}",
        collection.images.len(),
        collection.last_fetched.len(),
        id
    );

    Json(DataResponse { data: collection }).into_response()
}
