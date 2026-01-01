use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use tracing::error;

use crate::airports_repo::AirportsRepository;
use crate::clubs_repo::ClubsRepository;
use crate::runways_repo::RunwaysRepository;
use crate::web::AppState;

use super::{DataListResponse, DataResponse, json_error, views::AirportView, views::ClubView};

#[derive(Debug, Deserialize)]
pub struct AirportSearchParams {
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub radius: Option<f64>,
    // Bounding box parameters
    pub north: Option<f64>,
    pub south: Option<f64>,
    pub east: Option<f64>,
    pub west: Option<f64>,
}

pub async fn get_airport_by_id(
    State(state): State<AppState>,
    axum::extract::Path(airport_id): axum::extract::Path<i32>,
) -> impl IntoResponse {
    let airports_repo = AirportsRepository::new(state.pool.clone());
    let runways_repo = RunwaysRepository::new(state.pool);

    // Get airport by ID
    match airports_repo.get_airport_by_id(airport_id).await {
        Ok(Some(airport)) => {
            // Get runways for this airport
            let runways = match runways_repo.get_runways_by_airport_id(airport.id).await {
                Ok(runways) => runways,
                Err(e) => {
                    error!("Failed to get runways for airport {}: {}", airport.id, e);
                    Vec::new()
                }
            };

            let airport_view = AirportView::with_runways(airport, runways);
            Json(DataResponse { data: airport_view }).into_response()
        }
        Ok(None) => json_error(
            StatusCode::NOT_FOUND,
            &format!("Airport with ID {} not found", airport_id),
        )
        .into_response(),
        Err(e) => {
            error!("Failed to get airport by ID {}: {}", airport_id, e);
            json_error(
                StatusCode::BAD_REQUEST,
                &format!("Failed to get airport by ID {}: {}", airport_id, e),
            )
            .into_response()
        }
    }
}

pub async fn search_airports(
    State(state): State<AppState>,
    Query(params): Query<AirportSearchParams>,
) -> impl IntoResponse {
    let airports_repo = AirportsRepository::new(state.pool.clone());

    // Check if bounding box parameters are provided
    if let (Some(north), Some(south), Some(east), Some(west)) =
        (params.north, params.south, params.east, params.west)
    {
        // Validate bounding box coordinates
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
        // The PostGIS query handles this case by splitting into two bounding boxes

        // Get airports within the bounding box
        let runways_repo = RunwaysRepository::new(state.pool);
        match airports_repo
            .get_airports_in_bounding_box(north, south, east, west, params.limit)
            .await
        {
            Ok(airports) => {
                // For each airport, get its runways and create the view
                let mut airport_views = Vec::new();

                for airport in airports {
                    // Get runways for this airport
                    let runways = match runways_repo.get_runways_by_airport_id(airport.id).await {
                        Ok(runways) => runways,
                        Err(e) => {
                            error!("Failed to get runways for airport {}: {}", airport.id, e);
                            // Continue processing other airports even if runways fail
                            Vec::new()
                        }
                    };

                    airport_views.push(AirportView::with_runways(airport, runways));
                }

                Json(DataListResponse {
                    data: airport_views,
                })
                .into_response()
            }
            Err(e) => {
                error!("Failed to get airports in bounding box: {}", e);
                json_error(
                    StatusCode::BAD_REQUEST,
                    &format!("Failed to get airports in bounding box: {}", e),
                )
                .into_response()
            }
        }
    }
    // Check if geographic search parameters are provided
    else if let (Some(lat), Some(lng), Some(radius)) =
        (params.latitude, params.longitude, params.radius)
    {
        // Validate radius
        if radius <= 0.0 || radius > 1000.0 {
            return json_error(
                StatusCode::BAD_REQUEST,
                "Radius must be between 0 and 1000 kilometers",
            )
            .into_response();
        }

        // Validate latitude
        if !(-90.0..=90.0).contains(&lat) {
            return json_error(
                StatusCode::BAD_REQUEST,
                "Latitude must be between -90 and 90 degrees",
            )
            .into_response();
        }

        // Validate longitude
        if !(-180.0..=180.0).contains(&lng) {
            return json_error(
                StatusCode::BAD_REQUEST,
                "Longitude must be between -180 and 180 degrees",
            )
            .into_response();
        }

        match airports_repo
            .search_nearby(lat, lng, radius, params.limit)
            .await
        {
            Ok(airports) => Json(DataListResponse { data: airports }).into_response(),
            Err(e) => {
                error!("Failed to search nearby airports: {}", e);
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to search nearby airports",
                )
                .into_response()
            }
        }
    } else if let Some(query) = params.q {
        // Text-based search
        if query.trim().is_empty() {
            return json_error(
                StatusCode::BAD_REQUEST,
                "Query parameter 'q' cannot be empty",
            )
            .into_response();
        }

        match airports_repo.fuzzy_search(&query, params.limit).await {
            Ok(airports) => Json(DataListResponse { data: airports }).into_response(),
            Err(e) => {
                error!("Failed to search airports: {}", e);
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to search airports",
                )
                .into_response()
            }
        }
    } else if params.latitude.is_some() || params.longitude.is_some() || params.radius.is_some() {
        // Some geographic parameters provided but not all
        json_error(
            StatusCode::BAD_REQUEST,
            "Geographic search requires all three parameters: latitude, longitude, and radius",
        )
        .into_response()
    } else {
        // No search parameters provided
        json_error(
            StatusCode::BAD_REQUEST,
            "Either 'q' for text search, 'latitude', 'longitude', and 'radius' for geographic search, or 'nw_lat', 'nw_lng', 'se_lat', and 'se_lng' for bounding box search must be provided",
        ).into_response()
    }
}

/// Get clubs based at a specific airport
pub async fn get_clubs_by_airport(
    State(state): State<AppState>,
    axum::extract::Path(airport_id): axum::extract::Path<i32>,
) -> impl IntoResponse {
    let clubs_repo = ClubsRepository::new(state.pool);

    match clubs_repo.get_clubs_by_airport(airport_id).await {
        Ok(clubs) => {
            let club_views: Vec<ClubView> = clubs.into_iter().map(|club| club.into()).collect();
            Json(DataListResponse { data: club_views }).into_response()
        }
        Err(e) => {
            error!("Failed to get clubs for airport {}: {}", airport_id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get clubs for airport",
            )
            .into_response()
        }
    }
}
