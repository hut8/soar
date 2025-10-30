use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A server message stored in the database
/// This represents server status messages received from APRS servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMessage {
    /// Unique identifier for this server message
    pub id: Uuid,

    /// Server software information (e.g., "aprsc 2.1.15-gc67551b")
    pub software: String,

    /// Timestamp from the server message
    pub server_timestamp: DateTime<Utc>,

    /// Timestamp when we received/processed the message
    pub received_at: DateTime<Utc>,

    /// Server name (e.g., "GLIDERN1")
    pub server_name: String,

    /// Server endpoint (e.g., "51.178.19.212:10152")
    pub server_endpoint: String,

    /// Lag between received_at and server_timestamp (in milliseconds)
    pub lag: Option<i32>,

    /// When this record was created
    pub created_at: DateTime<Utc>,

    /// When this record was last updated
    pub updated_at: DateTime<Utc>,
}

impl ServerMessage {
    /// Create a new ServerMessage from parsed server message components
    pub fn new(
        software: String,
        server_timestamp: DateTime<Utc>,
        received_at: DateTime<Utc>,
        server_name: String,
        server_endpoint: String,
    ) -> Self {
        let lag = Some((received_at - server_timestamp).num_milliseconds() as i32);
        let now = Utc::now();

        Self {
            id: Uuid::now_v7(),
            software,
            server_timestamp,
            received_at,
            server_name,
            server_endpoint,
            lag,
            created_at: now,
            updated_at: now,
        }
    }
}
