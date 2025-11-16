use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Daily flight analytics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightAnalyticsDaily {
    pub date: NaiveDate,
    pub flight_count: i32,
    pub total_duration_seconds: i64,
    pub avg_duration_seconds: i32,
    pub total_distance_meters: i64,
    pub tow_flight_count: i32,
    pub cross_country_count: i32,
    pub updated_at: DateTime<Utc>,
}

/// Flight duration bucket for histogram analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightDurationBucket {
    pub bucket_name: String,
    pub bucket_order: i32,
    pub min_minutes: i32,
    pub max_minutes: Option<i32>,
    pub flight_count: i32,
    pub updated_at: DateTime<Utc>,
}

/// Hourly flight analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightAnalyticsHourly {
    pub hour: DateTime<Utc>,
    pub flight_count: i32,
    pub active_devices: i32,
    pub active_clubs: i32,
    pub updated_at: DateTime<Utc>,
}

/// Device analytics with anomaly detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAnalytics {
    pub device_id: Uuid,
    pub registration: Option<String>,
    pub aircraft_model: Option<String>,
    pub flight_count_total: i32,
    pub flight_count_30d: i32,
    pub flight_count_7d: i32,
    pub last_flight_at: Option<DateTime<Utc>>,
    pub avg_flight_duration_seconds: i32,
    pub total_distance_meters: i64,
    pub z_score_30d: Option<f64>,
    pub updated_at: DateTime<Utc>,
}

/// Club analytics daily summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClubAnalyticsDaily {
    pub club_id: Uuid,
    pub date: NaiveDate,
    pub club_name: Option<String>,
    pub flight_count: i32,
    pub active_devices: i32,
    pub total_airtime_seconds: i64,
    pub tow_count: i32,
    pub updated_at: DateTime<Utc>,
}

/// Airport activity analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirportAnalyticsDaily {
    pub airport_id: i32,
    pub date: NaiveDate,
    pub airport_ident: Option<String>,
    pub airport_name: Option<String>,
    pub departure_count: i32,
    pub arrival_count: i32,
    pub updated_at: DateTime<Utc>,
}

/// Data quality metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQualityMetricsDaily {
    pub metric_date: NaiveDate,
    pub total_fixes: i64,
    pub fixes_with_gaps_60s: i32,
    pub fixes_with_gaps_300s: i32,
    pub unparsed_aprs_messages: i32,
    pub flights_timed_out: i32,
    pub avg_fixes_per_flight: f64,
    pub quality_score: f64,
    pub updated_at: DateTime<Utc>,
}

/// Summary statistics for dashboard overview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSummary {
    pub flights_today: i32,
    pub flights_7d: i32,
    pub flights_30d: i32,
    pub active_devices_7d: i32,
    pub outlier_devices_count: i32,
    pub data_quality_score: Option<f64>,
}

/// Device outlier for anomaly detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceOutlier {
    pub device_id: Uuid,
    pub registration: Option<String>,
    pub aircraft_model: Option<String>,
    pub flight_count_30d: i32,
    pub z_score: f64,
}

/// Top device by flight count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopDevice {
    pub device_id: Uuid,
    pub registration: Option<String>,
    pub aircraft_model: Option<String>,
    pub flight_count: i32,
    pub total_distance_meters: i64,
}

/// Airport activity summary (aggregated)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirportActivity {
    pub airport_id: i32,
    pub ident: Option<String>,
    pub name: Option<String>,
    pub departure_count: i64,
    pub arrival_count: i64,
}
