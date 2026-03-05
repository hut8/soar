use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::receivers::ReceiverModel;

/// API view of a receiver with extracted latitude and longitude
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ReceiverView {
    pub id: Uuid,
    pub callsign: String,
    pub description: Option<String>,
    pub contact: Option<String>,
    pub email: Option<String>,
    pub ogn_db_country: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub street_address: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
    pub country: Option<String>,
    pub postal_code: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub latest_packet_at: Option<chrono::DateTime<chrono::Utc>>,
    pub from_ogn_db: bool,
}

impl From<ReceiverModel> for ReceiverView {
    fn from(r: ReceiverModel) -> Self {
        Self {
            id: r.id,
            callsign: r.callsign,
            description: r.description,
            contact: r.contact,
            email: r.email,
            ogn_db_country: r.ogn_db_country,
            latitude: r.latitude,
            longitude: r.longitude,
            street_address: r.street_address,
            city: r.city,
            region: r.region,
            country: r.country,
            postal_code: r.postal_code,
            created_at: r.created_at,
            updated_at: r.updated_at,
            latest_packet_at: r.latest_packet_at,
            from_ogn_db: r.from_ogn_db,
        }
    }
}
