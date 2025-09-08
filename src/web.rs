use anyhow::Result;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use include_dir::{Dir, include_dir};
use mime_guess::from_path;
use serde::Deserialize;
use sqlx::postgres::PgPool;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::airports_repo::AirportsRepository;
use crate::clubs_repo::ClubsRepository;
use crate::fixes_repo::FixesRepository;
use crate::flights_repo::FlightsRepository;

// Embed web assets into the binary
static ASSETS: Dir<'_> = include_dir!("web/build");

// App state for sharing database pool
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    q: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    radius: Option<f64>,
    limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct FixQuery {
    aircraft_id: Option<String>,
    start_time: Option<String>, // ISO datetime
    end_time: Option<String>,   // ISO datetime
    limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct FlightQuery {
    aircraft_id: Option<String>,
    start_date: Option<String>, // ISO date
    in_progress: Option<bool>,  // Filter for flights in progress
    limit: Option<i64>,
}

async fn handle_static_file(uri: Uri, _state: State<AppState>) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Try to find the exact file
    if let Some(file) = ASSETS.get_file(path) {
        let mime_type = from_path(path).first_or_octet_stream();
        let mut headers = HeaderMap::new();
        headers.insert("content-type", mime_type.as_ref().parse().unwrap());
        return (StatusCode::OK, headers, file.contents()).into_response();
    }

    // For HTML requests (based on Accept header or file extension), serve index.html for SPA
    if (path.is_empty() || path.ends_with(".html") || path.ends_with('/'))
        && let Some(index_file) = ASSETS.get_file("index.html")
    {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "text/html".parse().unwrap());
        return (StatusCode::OK, headers, index_file.contents()).into_response();
    }

    // If no file found and it's not an API route, try serving index.html for SPA routing
    if !path.starts_with("api/")
        && let Some(index_file) = ASSETS.get_file("index.html")
    {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "text/html".parse().unwrap());
        return (StatusCode::OK, headers, index_file.contents()).into_response();
    }

    (StatusCode::NOT_FOUND, "Not Found").into_response()
}

async fn search_airports(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let airports_repo = AirportsRepository::new(state.pool);

    // Check if geographic search parameters are provided
    if let (Some(lat), Some(lng), Some(radius)) = (params.latitude, params.longitude, params.radius) {
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

        match airports_repo.search_nearby(lat, lng, radius, params.limit).await {
            Ok(airports) => Json(airports).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error searching airports by location: {}", e),
            )
                .into_response(),
        }
    } else if let Some(query) = params.q {
        // Text-based search
        if query.trim().is_empty() {
            return (StatusCode::BAD_REQUEST, "Query parameter 'q' cannot be empty")
                .into_response();
        }

        match airports_repo.fuzzy_search(&query, params.limit).await {
            Ok(airports) => Json(airports).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error searching airports: {}", e),
            )
                .into_response(),
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

async fn search_clubs(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let clubs_repo = ClubsRepository::new(state.pool);

    // Check if geographic search parameters are provided
    if let (Some(lat), Some(lng), Some(radius)) = (params.latitude, params.longitude, params.radius) {
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

        match clubs_repo.search_nearby_soaring(lat, lng, radius, params.limit).await {
            Ok(clubs) => Json(clubs).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error searching clubs by location: {}", e),
            )
                .into_response(),
        }
    } else if let Some(query) = params.q {
        // Text-based search
        if query.trim().is_empty() {
            return (StatusCode::BAD_REQUEST, "Query parameter 'q' cannot be empty")
                .into_response();
        }

        match clubs_repo.fuzzy_search_soaring(&query, params.limit).await {
            Ok(clubs) => Json(clubs).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error searching clubs: {}", e),
            )
                .into_response(),
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

async fn search_fixes(
    State(state): State<AppState>,
    Query(params): Query<FixQuery>,
) -> impl IntoResponse {
    use chrono::{DateTime, Utc};

    let fixes_repo = FixesRepository::new(state.pool);

    // Parse datetime strings if provided
    let start_time = if let Some(start_str) = params.start_time {
        match start_str.parse::<DateTime<Utc>>() {
            Ok(dt) => Some(dt),
            Err(_) => return (StatusCode::BAD_REQUEST, "Invalid start_time format. Use ISO 8601 format.").into_response(),
        }
    } else {
        None
    };

    let end_time = if let Some(end_str) = params.end_time {
        match end_str.parse::<DateTime<Utc>>() {
            Ok(dt) => Some(dt),
            Err(_) => return (StatusCode::BAD_REQUEST, "Invalid end_time format. Use ISO 8601 format.").into_response(),
        }
    } else {
        None
    };

    // Determine query strategy
    if let Some(aircraft_id) = params.aircraft_id {
        // Aircraft-specific query
        match fixes_repo.get_fixes_for_aircraft(&aircraft_id, params.limit).await {
            Ok(fixes) => Json(fixes).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error fetching fixes for aircraft {}: {}", aircraft_id, e),
            ).into_response(),
        }
    } else if let (Some(start), Some(end)) = (start_time, end_time) {
        // Time range query
        match fixes_repo.get_fixes_in_time_range(start, end, params.limit).await {
            Ok(fixes) => Json(fixes).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error fetching fixes in time range: {}", e),
            ).into_response(),
        }
    } else {
        // Recent fixes query
        match fixes_repo.get_recent_fixes(params.limit.unwrap_or(100)).await {
            Ok(fixes) => Json(fixes).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error fetching recent fixes: {}", e),
            ).into_response(),
        }
    }
}

async fn search_flights(
    State(state): State<AppState>,
    Query(params): Query<FlightQuery>,
) -> impl IntoResponse {
    use chrono::NaiveDate;

    let flights_repo = FlightsRepository::new(state.pool);

    // Handle in-progress filter
    if params.in_progress == Some(true) {
        match flights_repo.get_flights_in_progress().await {
            Ok(flights) => return Json(flights).into_response(),
            Err(e) => return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error fetching flights in progress: {}", e),
            ).into_response(),
        }
    }

    // Aircraft-specific query
    if let Some(aircraft_id) = params.aircraft_id {
        match flights_repo.get_flights_for_aircraft(&aircraft_id).await {
            Ok(flights) => Json(flights).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error fetching flights for aircraft {}: {}", aircraft_id, e),
            ).into_response(),
        }
    } else if let Some(date_str) = params.start_date {
        // Date-specific query
        match date_str.parse::<NaiveDate>() {
            Ok(date) => {
                match flights_repo.get_flights_for_date(date).await {
                    Ok(flights) => Json(flights).into_response(),
                    Err(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Error fetching flights for date {}: {}", date, e),
                    ).into_response(),
                }
            }
            Err(_) => (
                StatusCode::BAD_REQUEST,
                "Invalid start_date format. Use YYYY-MM-DD format.",
            ).into_response(),
        }
    } else {
        // Default: recent flights
        match flights_repo.get_flights_in_progress().await {
            Ok(flights) => {
                // Also get recently completed flights if we need more
                let limit = params.limit.unwrap_or(50) as usize;
                if flights.len() < limit {
                    // This is a simplified approach - in a real system you'd want a more sophisticated query
                    Json(flights).into_response()
                } else {
                    Json(flights).into_response()
                }
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error fetching recent flights: {}", e),
            ).into_response(),
        }
    }
}

pub async fn start_web_server(interface: String, port: u16, pool: PgPool) -> Result<()> {
    info!("Starting web server on {}:{}", interface, port);

    let app_state = AppState { pool };

    // Build the Axum application
    let app = Router::new()
        .route("/airports", get(search_airports))
        .route("/clubs", get(search_clubs))
        .route("/fixes", get(search_fixes))
        .route("/flights", get(search_flights))
        .fallback(handle_static_file)
        .with_state(app_state)
        .layer(TraceLayer::new_for_http());

    // Create the listener
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", interface, port)).await?;

    info!("Web server listening on http://{}:{}", interface, port);

    // Start the server
    axum::serve(listener, app).await?;

    Ok(())
}
