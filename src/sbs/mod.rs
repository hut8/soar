pub mod client;
pub mod decoder;
pub mod sbs_to_fix;

pub use client::{SbsClient, SbsClientConfig};
pub use decoder::{DecodedSbsMessage, SbsMessageType, decode_sbs_message, message_to_json};
pub use sbs_to_fix::{
    extract_altitude, extract_flight_number, extract_icao_address, extract_position,
    extract_velocity, message_can_produce_fix, sbs_message_to_fix,
};

use anyhow::Result;

/// Trait for SBS message publishers
/// Allows the SBS client to work with both JetStream and plain NATS publishers
#[async_trait::async_trait]
pub trait SbsPublisher: Clone + Send + Sync + 'static {
    /// Publish a message with fire-and-forget semantics
    async fn publish_fire_and_forget(&self, message: &[u8]);

    /// Publish a message with retry logic (for graceful shutdown)
    async fn publish_with_retry(&self, message: &[u8], max_retries: u32) -> Result<()>;
}
