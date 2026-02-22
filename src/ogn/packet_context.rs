use uuid::Uuid;

/// Context containing IDs from generic packet processing
/// This is passed to specific packet processors so they don't need to duplicate receiver/message logic
#[derive(Debug, Clone)]
pub struct PacketContext {
    /// ID of the APRS message record created for this packet
    pub raw_message_id: Uuid,
    /// ID of the receiver that sent/relayed this packet
    pub receiver_id: Uuid,
    /// Timestamp when the message was received from APRS-IS
    /// This is captured at ingestion time to prevent clock skew from queue processing delays
    pub received_at: chrono::DateTime<chrono::Utc>,
}
