use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// API view of a receiver with extracted latitude and longitude
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiverView {
    pub id: Uuid,
    pub callsign: String,
    pub description: Option<String>,
    pub contact: Option<String>,
    pub email: Option<String>,
    pub country: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub latest_packet_at: Option<chrono::DateTime<chrono::Utc>>,
    pub from_ogn_db: bool,
}
