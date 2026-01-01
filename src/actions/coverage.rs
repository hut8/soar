use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::actions::json_error;
use crate::coverage::CoverageHexFeature;
use crate::coverage_cache::CoverageCache;
use crate::coverage_repo::CoverageRepository;
use crate::web::AppState;

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CoverageQueryParams {
    /// H3 resolution (3, 4, 5, 6, 7, or 8)
    pub resolution: Option<i16>,

    /// Bounding box: western longitude
    pub west: f64,

    /// Bounding box: eastern longitude
    pub east: f64,

    /// Bounding box: southern latitude
    pub south: f64,

    /// Bounding box: northern latitude
    pub north: f64,

    /// Start date for coverage (YYYY-MM-DD)
    pub start_date: Option<NaiveDate>,

    /// End date for coverage (YYYY-MM-DD)
    pub end_date: Option<NaiveDate>,

    /// Filter by receiver ID
    pub receiver_id: Option<Uuid>,

    /// Minimum altitude MSL (feet)
    pub min_altitude: Option<i32>,

    /// Maximum altitude MSL (feet)
    pub max_altitude: Option<i32>,

    /// Maximum number of hexes to return
    pub limit: Option<i64>,
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct CoverageGeoJsonResponse {
    #[serde(rename = "type")]
    pub feature_collection_type: String, // Always "FeatureCollection"
    pub features: Vec<CoverageHexFeature>,
}

// ============================================================================
// Handler Functions
// ============================================================================

/// GET /data/coverage/hexes
/// Get receiver coverage as GeoJSON FeatureCollection of H3 hexagons
pub async fn get_coverage_hexes(
    Query(params): Query<CoverageQueryParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    metrics::counter!("coverage.api.hexes.requests_total").increment(1);

    // Validate and set defaults
    let resolution = params.resolution.unwrap_or(7);
    if ![3, 4, 5, 6, 7, 8].contains(&resolution) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Resolution must be 3, 4, 5, 6, 7, or 8",
        )
        .into_response();
    }

    let end_date = params
        .end_date
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let start_date = params
        .start_date
        .unwrap_or_else(|| end_date - chrono::Duration::days(30));

    let limit = params.limit.unwrap_or(5000).min(10000);

    // Validate bounding box
    if params.north <= params.south || params.west >= params.east {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Invalid bounding box: north must be > south and west must be < east",
        )
        .into_response();
    }

    let cache = CoverageCache::new(CoverageRepository::new(state.pool.clone()));

    match cache
        .get_coverage_geojson(
            resolution,
            start_date,
            end_date,
            params.west,
            params.south,
            params.east,
            params.north,
            params.receiver_id,
            params.min_altitude,
            params.max_altitude,
            limit,
        )
        .await
    {
        Ok(features) => {
            metrics::counter!("coverage.api.hexes.success_total").increment(1);
            metrics::histogram!("coverage.api.hexes.count").record(features.len() as f64);

            (
                StatusCode::OK,
                Json(CoverageGeoJsonResponse {
                    feature_collection_type: "FeatureCollection".to_string(),
                    features,
                }),
            )
                .into_response()
        }
        Err(e) => {
            metrics::counter!("coverage.api.errors_total").increment(1);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get coverage hexes: {}", e),
            )
            .into_response()
        }
    }
}
