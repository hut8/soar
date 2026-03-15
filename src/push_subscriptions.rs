use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// API view of a push subscription
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct PushSubscriptionView {
    pub id: Uuid,
    pub notify_takeoff: bool,
    pub notify_landing: bool,
    pub created_at: DateTime<Utc>,
}

/// Request to create/update a push subscription
#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct PushSubscriptionRequest {
    pub endpoint: String,
    pub p256dh_key: String,
    pub auth_key: String,
    #[serde(default = "default_true")]
    pub notify_takeoff: bool,
    #[serde(default = "default_true")]
    pub notify_landing: bool,
}

/// Response containing the VAPID public key
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct VapidPublicKeyResponse {
    pub public_key: String,
}

fn default_true() -> bool {
    true
}

/// Notification payload sent to the browser via Web Push
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlightNotificationPayload {
    pub event_type: String,
    pub flight_id: Uuid,
    pub club_id: Uuid,
    pub club_name: String,
    pub aircraft_registration: Option<String>,
    pub aircraft_model: String,
    pub airport_ident: Option<String>,
    pub timestamp: String,
}
