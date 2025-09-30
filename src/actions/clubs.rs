use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use tracing::error;
use uuid::Uuid;

use crate::clubs_repo::ClubsRepository;
use crate::web::AppState;

use super::{json_error, views::ClubView};

#[derive(Debug, Deserialize)]
pub struct ClubSearchParams {
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub radius: Option<f64>,
}

pub async fn get_club_by_id(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let clubs_repo = ClubsRepository::new(state.pool);

    match clubs_repo.get_by_id(id).await {
        Ok(Some(club)) => Json(ClubView::from(club)).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Club not found").into_response(),
        Err(e) => {
            error!("Failed to get club by ID: {}", e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get club by ID",
            )
            .into_response()
        }
    }
}

pub async fn search_clubs(
    State(state): State<AppState>,
    Query(params): Query<ClubSearchParams>,
) -> impl IntoResponse {
    let clubs_repo = ClubsRepository::new(state.pool);

    // Check if geographic search parameters are provided
    if let (Some(lat), Some(lng)) = (params.latitude, params.longitude) {
        let radius = params.radius.unwrap_or(50.0); // Default to 50km radius if not specified

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
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to search nearby clubs",
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

        match clubs_repo.fuzzy_search_soaring(&query, params.limit).await {
            Ok(clubs) => {
                let club_views: Vec<ClubView> = clubs.into_iter().map(ClubView::from).collect();
                Json(club_views).into_response()
            }
            Err(e) => {
                error!("Failed to search clubs: {}", e);
                json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to search clubs")
                    .into_response()
            }
        }
    } else if params.latitude.is_some() || params.longitude.is_some() || params.radius.is_some() {
        // Some geographic parameters provided but not latitude and longitude
        json_error(
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
                json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get clubs").into_response()
            }
        }
    }
}
