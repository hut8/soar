use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFix {
    pub id: Uuid,
    pub user_id: Uuid,
    pub latitude: f64,
    pub longitude: f64,
    pub heading: Option<f64>,
    pub raw: Option<JsonValue>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserFixRequest {
    pub latitude: f64,
    pub longitude: f64,
    pub heading: Option<f64>,
    pub raw: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateUserFixResponse {
    pub id: Uuid,
    pub latitude: f64,
    pub longitude: f64,
    pub heading: Option<f64>,
    pub timestamp: DateTime<Utc>,
}

impl From<UserFix> for CreateUserFixResponse {
    fn from(fix: UserFix) -> Self {
        Self {
            id: fix.id,
            latitude: fix.latitude,
            longitude: fix.longitude,
            heading: fix.heading,
            timestamp: fix.timestamp,
        }
    }
}

/// A user who was present at an airport on a given day
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct AirportUserPresence {
    pub user_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub is_licensed: bool,
    pub is_instructor: bool,
    pub is_tow_pilot: bool,
    pub is_examiner: bool,
    pub last_seen_at: DateTime<Utc>,
}
