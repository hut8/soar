use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

use crate::raw_messages_repo::AprsMessage;

/// API view of a raw APRS message.
/// Converts Vec<u8> fields to strings for JSON serialization.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct RawMessageView {
    pub id: Uuid,
    pub raw_message: String,
    pub received_at: chrono::DateTime<chrono::Utc>,
    pub receiver_id: Option<Uuid>,
    pub unparsed: Option<String>,
}

impl From<AprsMessage> for RawMessageView {
    fn from(msg: AprsMessage) -> Self {
        Self {
            id: msg.id,
            raw_message: msg.raw_message_text(),
            received_at: msg.received_at,
            receiver_id: msg.receiver_id,
            unparsed: msg.unparsed,
        }
    }
}
