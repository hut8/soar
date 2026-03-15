use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::flights::FlightState;

/// Request body for POST /data/tracker
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackerFixRequest {
    pub latitude: f64,
    pub longitude: f64,
    pub heading: Option<f64>,
    /// GPS altitude MSL in meters
    pub altitude_meters: Option<f64>,
    /// Ground speed in m/s
    pub speed_mps: Option<f64>,
    /// GPS accuracy in meters
    pub accuracy_meters: Option<f64>,
    /// Barometric pressure in hPa
    pub pressure_hpa: Option<f64>,
    /// Accelerometer X axis (g-force)
    pub accel_x: Option<f64>,
    /// Accelerometer Y axis (g-force)
    pub accel_y: Option<f64>,
    /// Accelerometer Z axis (g-force)
    pub accel_z: Option<f64>,
    /// Derived vertical speed in m/s
    pub vertical_speed_mps: Option<f64>,
}

/// Response from POST /data/tracker
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct TrackerFixResponse {
    pub fix_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub matched_aircraft: Option<TrackerAircraftInfo>,
    pub flight: Option<TrackerFlightInfo>,
    pub nearby_aircraft: Vec<NearbyAircraftInfo>,
    pub ground_elevation_meters: Option<f64>,
    pub altitude_agl_feet: Option<i32>,
}

/// Matched aircraft information
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct TrackerAircraftInfo {
    pub id: Uuid,
    pub registration: Option<String>,
    pub aircraft_model: String,
    pub competition_number: String,
    pub distance_meters: f64,
}

/// Active flight information for the matched aircraft
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct TrackerFlightInfo {
    pub id: Uuid,
    pub state: FlightState,
    pub takeoff_time: Option<DateTime<Utc>>,
    pub departure_airport: Option<String>,
    pub duration_seconds: Option<i64>,
    pub towed_by_registration: Option<String>,
    pub towed_by_aircraft_model: Option<String>,
    pub tow_release_time: Option<DateTime<Utc>>,
    pub tow_release_altitude_msl_ft: Option<i32>,
}

/// Nearby aircraft information
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct NearbyAircraftInfo {
    pub id: Uuid,
    pub registration: Option<String>,
    pub aircraft_model: String,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_feet: Option<f64>,
    pub ground_speed_knots: Option<f64>,
    pub distance_meters: f64,
    pub last_fix_at: Option<DateTime<Utc>>,
    pub climb_fpm: Option<f64>,
    pub track_degrees: Option<f64>,
}
