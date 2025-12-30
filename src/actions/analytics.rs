use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::NaiveDate;
use serde::Deserialize;
use uuid::Uuid;

use crate::actions::{DataListResponse, DataResponse, json_error};
use crate::analytics_cache::AnalyticsCache;
use crate::analytics_repo::AnalyticsRepository;
use crate::web::AppState;

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct DateRangeParams {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct HoursParams {
    pub hours: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct OutliersParams {
    pub threshold: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct TopDevicesParams {
    pub limit: Option<i32>,
    pub period: Option<String>, // "7d", "30d", or "total"
}

#[derive(Debug, Deserialize)]
pub struct ClubAnalyticsParams {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub club_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct AirportActivityParams {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub limit: Option<i32>,
}

// ============================================================================
// Handler Functions
// ============================================================================

/// GET /data/analytics/flights/daily
/// Get daily flight analytics for a date range
pub async fn get_daily_flights(
    Query(params): Query<DateRangeParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    metrics::counter!("analytics.api.daily_flights.requests_total").increment(1);

    let end_date = params
        .end_date
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let start_date = params
        .start_date
        .unwrap_or_else(|| end_date - chrono::Duration::days(14));

    let cache = AnalyticsCache::new(AnalyticsRepository::new(state.pool.clone()));

    match cache.get_daily_flights(start_date, end_date).await {
        Ok(data) => (StatusCode::OK, Json(DataListResponse { data })).into_response(),
        Err(e) => {
            metrics::counter!("analytics.api.errors_total").increment(1);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get daily flights: {}", e),
            )
            .into_response()
        }
    }
}

/// GET /data/analytics/flights/hourly
/// Get hourly flight analytics
pub async fn get_hourly_flights(
    Query(params): Query<HoursParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    metrics::counter!("analytics.api.hourly_flights.requests_total").increment(1);

    let hours = params.hours.unwrap_or(24);

    let cache = AnalyticsCache::new(AnalyticsRepository::new(state.pool.clone()));

    match cache.get_hourly_flights(hours).await {
        Ok(data) => (StatusCode::OK, Json(DataListResponse { data })).into_response(),
        Err(e) => {
            metrics::counter!("analytics.api.errors_total").increment(1);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get hourly flights: {}", e),
            )
            .into_response()
        }
    }
}

/// GET /data/analytics/flights/duration-distribution
/// Get flight duration distribution buckets
pub async fn get_duration_distribution(State(state): State<AppState>) -> impl IntoResponse {
    metrics::counter!("analytics.api.duration_distribution.requests_total").increment(1);

    let cache = AnalyticsCache::new(AnalyticsRepository::new(state.pool.clone()));

    match cache.get_duration_distribution().await {
        Ok(data) => (StatusCode::OK, Json(DataListResponse { data })).into_response(),
        Err(e) => {
            metrics::counter!("analytics.api.errors_total").increment(1);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get duration distribution: {}", e),
            )
            .into_response()
        }
    }
}

/// GET /data/analytics/devices/outliers
/// Get device outliers (z-score > threshold)
pub async fn get_aircraft_outliers(
    Query(params): Query<OutliersParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    metrics::counter!("analytics.api.aircraft_outliers.requests_total").increment(1);

    let threshold = params.threshold.unwrap_or(3.0);

    let cache = AnalyticsCache::new(AnalyticsRepository::new(state.pool.clone()));

    match cache.get_device_outliers(threshold).await {
        Ok(data) => (StatusCode::OK, Json(DataListResponse { data })).into_response(),
        Err(e) => {
            metrics::counter!("analytics.api.errors_total").increment(1);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get device outliers: {}", e),
            )
            .into_response()
        }
    }
}

/// GET /data/analytics/devices/top
/// Get top devices by flight count
pub async fn get_top_aircraft(
    Query(params): Query<TopDevicesParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    metrics::counter!("analytics.api.top_aircraft.requests_total").increment(1);

    let limit = params.limit.unwrap_or(10);
    let period_days = match params.period.as_deref() {
        Some("7d") => 7,
        Some("30d") => 30,
        _ => 0, // total
    };

    let cache = AnalyticsCache::new(AnalyticsRepository::new(state.pool.clone()));

    match cache.get_top_aircraft(limit, period_days).await {
        Ok(data) => (StatusCode::OK, Json(DataListResponse { data })).into_response(),
        Err(e) => {
            metrics::counter!("analytics.api.errors_total").increment(1);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get top aircraft: {}", e),
            )
            .into_response()
        }
    }
}

/// GET /data/analytics/clubs/daily
/// Get club analytics for a date range
pub async fn get_club_analytics(
    Query(params): Query<ClubAnalyticsParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    metrics::counter!("analytics.api.club_analytics.requests_total").increment(1);

    let end_date = params
        .end_date
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let start_date = params
        .start_date
        .unwrap_or_else(|| end_date - chrono::Duration::days(14));

    let cache = AnalyticsCache::new(AnalyticsRepository::new(state.pool.clone()));

    match cache
        .get_club_analytics(start_date, end_date, params.club_id)
        .await
    {
        Ok(data) => (StatusCode::OK, Json(DataListResponse { data })).into_response(),
        Err(e) => {
            metrics::counter!("analytics.api.errors_total").increment(1);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get club analytics: {}", e),
            )
            .into_response()
        }
    }
}

/// GET /data/analytics/airports/activity
/// Get airport activity for a date range
pub async fn get_airport_activity(
    Query(params): Query<AirportActivityParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    metrics::counter!("analytics.api.airport_activity.requests_total").increment(1);

    let end_date = params
        .end_date
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let start_date = params
        .start_date
        .unwrap_or_else(|| end_date - chrono::Duration::days(14));
    let limit = params.limit.unwrap_or(10);

    let cache = AnalyticsCache::new(AnalyticsRepository::new(state.pool.clone()));

    match cache
        .get_airport_activity(start_date, end_date, limit)
        .await
    {
        Ok(data) => (StatusCode::OK, Json(DataListResponse { data })).into_response(),
        Err(e) => {
            metrics::counter!("analytics.api.errors_total").increment(1);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get airport activity: {}", e),
            )
            .into_response()
        }
    }
}

/// GET /data/analytics/summary
/// Get analytics summary for dashboard
pub async fn get_summary(State(state): State<AppState>) -> impl IntoResponse {
    metrics::counter!("analytics.api.summary.requests_total").increment(1);

    let cache = AnalyticsCache::new(AnalyticsRepository::new(state.pool.clone()));

    match cache.get_summary().await {
        Ok(data) => (StatusCode::OK, Json(DataResponse { data })).into_response(),
        Err(e) => {
            metrics::counter!("analytics.api.errors_total").increment(1);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get analytics summary: {}", e),
            )
            .into_response()
        }
    }
}
