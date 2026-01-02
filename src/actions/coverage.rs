use axum::{
    extract::{Path, Query, State},
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
use crate::fixes::Fix;
use crate::fixes_repo::FixesRepository;
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
    // Note: west can be > east when crossing the International Date Line
    if params.north <= params.south {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Invalid bounding box: north must be > south",
        )
        .into_response();
    }

    if params.west == params.east {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Invalid bounding box: west and east cannot be identical",
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

/// Query parameters for hex fixes endpoint
#[derive(Debug, Deserialize)]
pub struct HexFixesQueryParams {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub receiver_id: Option<Uuid>,
    pub min_altitude: Option<i32>,
    pub max_altitude: Option<i32>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Response for fixes within an H3 hexagon
#[derive(Debug, Serialize)]
pub struct FixesInHexResponse {
    pub data: Vec<Fix>,
    pub total: i64,
    #[serde(rename = "h3Index")]
    pub h3_index: String,
    pub resolution: i16,
}

/// GET /data/coverage/hexes/{h3_index}/fixes
/// Get individual position fixes within an H3 hexagon
pub async fn get_hex_fixes(
    Path(h3_index_str): Path<String>,
    Query(params): Query<HexFixesQueryParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    metrics::counter!("coverage.api.hex_fixes.requests_total").increment(1);
    let start_time = std::time::Instant::now();

    // Parse and validate H3 index
    let h3_index_u64 = match u64::from_str_radix(&h3_index_str, 16) {
        Ok(idx) => idx,
        Err(_) => {
            return json_error(StatusCode::BAD_REQUEST, "Invalid H3 index format").into_response();
        }
    };

    // Validate it's a valid H3 index and get resolution
    let cell = match h3o::CellIndex::try_from(h3_index_u64) {
        Ok(c) => c,
        Err(_) => {
            return json_error(StatusCode::BAD_REQUEST, "Invalid H3 index").into_response();
        }
    };

    let resolution = cell.resolution() as i16;
    let h3_index_i64 = h3_index_u64 as i64;

    // Apply date defaults (same as main coverage endpoint)
    let end_date = params
        .end_date
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let start_date = params
        .start_date
        .unwrap_or_else(|| end_date - chrono::Duration::days(30));

    // Validate and cap limit
    let limit = params.limit.unwrap_or(100).clamp(1, 1000);
    let offset = params.offset.unwrap_or(0).max(0);

    // Query fixes
    let fixes_repo = FixesRepository::new(state.pool.clone());
    match fixes_repo
        .get_fixes_in_h3_cell(
            h3_index_i64,
            start_date,
            end_date,
            params.receiver_id,
            params.min_altitude,
            params.max_altitude,
            limit,
            offset,
        )
        .await
    {
        Ok((fixes, total)) => {
            let duration_ms = start_time.elapsed().as_millis() as f64;

            metrics::counter!("coverage.api.hex_fixes.success_total").increment(1);
            metrics::histogram!("coverage.api.hex_fixes.count").record(fixes.len() as f64);
            metrics::histogram!("coverage.api.hex_fixes.query_ms").record(duration_ms);

            (
                StatusCode::OK,
                Json(FixesInHexResponse {
                    data: fixes,
                    total,
                    h3_index: h3_index_str,
                    resolution,
                }),
            )
                .into_response()
        }
        Err(e) => {
            metrics::counter!("coverage.api.hex_fixes.errors_total").increment(1);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get hex fixes: {}", e),
            )
            .into_response()
        }
    }
}
