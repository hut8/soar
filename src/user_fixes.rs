use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
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
