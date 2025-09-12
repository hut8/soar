use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use tracing::error;

use crate::airports_repo::AirportsRepository;
use crate::clubs_repo::ClubsRepository;
use crate::fixes_repo::FixesRepository;
use crate::flights_repo::FlightsRepository;
use crate::web::AppState;

use super::views::ClubView;

#[derive(Debug, Deserialize)]
pub struct SearchQueryParams {
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub radius: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct FixesQueryParams {
    pub device_id: Option<u32>,
    pub flight_id: Option<sqlx::types::Uuid>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct FlightsQueryParams {
    pub device_id: Option<u32>,
    pub club_id: Option<sqlx::types::Uuid>,
    pub limit: Option<i64>,
}

pub async fn search_airports(
    State(state): State<AppState>,
    Query(params): Query<SearchQueryParams>,
) -> impl IntoResponse {
    let airports_repo = AirportsRepository::new(state.diesel_pool);

    // Check if geographic search parameters are provided
    if let (Some(lat), Some(lng), Some(radius)) = (params.latitude, params.longitude, params.radius)
    {
        // Validate radius
        if radius <= 0.0 || radius > 1000.0 {
            return (
                StatusCode::BAD_REQUEST,
                "Radius must be between 0 and 1000 kilometers",
            )
                .into_response();
        }

        // Validate latitude
        if !(-90.0..=90.0).contains(&lat) {
            return (
                StatusCode::BAD_REQUEST,
                "Latitude must be between -90 and 90 degrees",
            )
                .into_response();
        }

        // Validate longitude
        if !(-180.0..=180.0).contains(&lng) {
            return (
                StatusCode::BAD_REQUEST,
                "Longitude must be between -180 and 180 degrees",
            )
                .into_response();
        }

        match airports_repo
            .search_nearby(lat, lng, radius, params.limit)
            .await
        {
            Ok(airports) => Json(airports).into_response(),
            Err(e) => {
                error!("Failed to search nearby airports: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to search nearby airports",
                )
                    .into_response()
            }
        }
    } else if let Some(query) = params.q {
        // Text-based search
        if query.trim().is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                "Query parameter 'q' cannot be empty",
            )
                .into_response();
        }

        match airports_repo.fuzzy_search(&query, params.limit).await {
            Ok(airports) => Json(airports).into_response(),
            Err(e) => {
                error!("Failed to search airports: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to search airports",
                )
                    .into_response()
            }
        }
    } else if params.latitude.is_some() || params.longitude.is_some() || params.radius.is_some() {
        // Some geographic parameters provided but not all
        (
            StatusCode::BAD_REQUEST,
            "Geographic search requires all three parameters: latitude, longitude, and radius",
        )
            .into_response()
    } else {
        // No search parameters provided
        (
            StatusCode::BAD_REQUEST,
            "Either 'q' for text search or 'latitude', 'longitude', and 'radius' for geographic search must be provided",
        )
            .into_response()
    }
}

pub async fn search_clubs(
    State(state): State<AppState>,
    Query(params): Query<SearchQueryParams>,
) -> impl IntoResponse {
    let clubs_repo = ClubsRepository::new(state.pool);

    // Check if geographic search parameters are provided
    if let (Some(lat), Some(lng)) = (params.latitude, params.longitude) {
        let radius = params.radius.unwrap_or(50.0); // Default to 50km radius if not specified

        // Validate radius
        if radius <= 0.0 || radius > 1000.0 {
            return (
                StatusCode::BAD_REQUEST,
                "Radius must be between 0 and 1000 kilometers",
            )
                .into_response();
        }

        // Validate latitude
        if !(-90.0..=90.0).contains(&lat) {
            return (
                StatusCode::BAD_REQUEST,
                "Latitude must be between -90 and 90 degrees",
            )
                .into_response();
        }

        // Validate longitude
        if !(-180.0..=180.0).contains(&lng) {
            return (
                StatusCode::BAD_REQUEST,
                "Longitude must be between -180 and 180 degrees",
            )
                .into_response();
        }

        match clubs_repo
            .search_nearby_soaring(lat, lng, radius, params.limit)
            .await
        {
            Ok(clubs) => {
                let club_views: Vec<ClubView> = clubs.into_iter().map(ClubView::from).collect();
                Json(club_views).into_response()
            }
            Err(e) => {
                error!("Failed to search nearby clubs: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to search nearby clubs",
                )
                    .into_response()
            }
        }
    } else if let Some(query) = params.q {
        // Text-based search
        if query.trim().is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                "Query parameter 'q' cannot be empty",
            )
                .into_response();
        }

        match clubs_repo.fuzzy_search_soaring(&query, params.limit).await {
            Ok(clubs) => {
                let club_views: Vec<ClubView> = clubs.into_iter().map(ClubView::from).collect();
                Json(club_views).into_response()
            }
            Err(e) => {
                error!("Failed to search clubs: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to search clubs").into_response()
            }
        }
    } else if params.latitude.is_some() || params.longitude.is_some() || params.radius.is_some() {
        // Some geographic parameters provided but not latitude and longitude
        (
            StatusCode::BAD_REQUEST,
            "Geographic search requires at least latitude and longitude parameters",
        )
            .into_response()
    } else {
        // No search parameters provided - return all clubs
        match clubs_repo.get_all().await {
            Ok(clubs) => {
                let club_views: Vec<ClubView> = clubs.into_iter().map(ClubView::from).collect();
                Json(club_views).into_response()
            }
            Err(e) => {
                error!("Failed to get clubs: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get clubs").into_response()
            }
        }
    }
}

pub async fn search_fixes(
    State(state): State<AppState>,
    Query(params): Query<FixesQueryParams>,
) -> impl IntoResponse {
    let fixes_repo = FixesRepository::new(state.pool);

    if let Some(device_id) = params.device_id {
        match fixes_repo
            .get_fixes_for_aircraft(&device_id.to_string(), Some(params.limit.unwrap_or(1000)))
            .await
        {
            Ok(fixes) => Json(fixes).into_response(),
            Err(e) => {
                error!("Failed to get fixes by device ID: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to get fixes by device ID",
                )
                    .into_response()
            }
        }
    } else if let Some(_flight_id) = params.flight_id {
        // Flight-based fix search would require looking up the flight first
        // For now, just return recent fixes
        match fixes_repo
            .get_recent_fixes(params.limit.unwrap_or(1000))
            .await
        {
            Ok(fixes) => Json(fixes).into_response(),
            Err(e) => {
                error!("Failed to get fixes by flight ID: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to get fixes by flight ID",
                )
                    .into_response()
            }
        }
    } else {
        match fixes_repo
            .get_recent_fixes(params.limit.unwrap_or(100))
            .await
        {
            Ok(fixes) => Json(fixes).into_response(),
            Err(e) => {
                error!("Failed to get recent fixes: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to get recent fixes",
                )
                    .into_response()
            }
        }
    }
}

pub async fn search_flights(
    State(state): State<AppState>,
    Query(params): Query<FlightsQueryParams>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool);

    if let Some(device_id) = params.device_id {
        match flights_repo
            .get_flights_for_aircraft(&device_id.to_string())
            .await
        {
            Ok(flights) => Json(flights).into_response(),
            Err(e) => {
                error!("Failed to get flights by device ID: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to get flights by device ID",
                )
                    .into_response()
            }
        }
    } else if let Some(_club_id) = params.club_id {
        // Club-based flight search would require joining with aircraft_registrations
        // For now, just return flights in progress
        match flights_repo.get_flights_in_progress().await {
            Ok(flights) => Json(flights).into_response(),
            Err(e) => {
                error!("Failed to get flights by club ID: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to get flights by club ID",
                )
                    .into_response()
            }
        }
    } else {
        match flights_repo.get_flights_in_progress().await {
            Ok(flights) => Json(flights).into_response(),
            Err(e) => {
                error!("Failed to get recent flights: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to get recent flights",
                )
                    .into_response()
            }
        }
    }
}
