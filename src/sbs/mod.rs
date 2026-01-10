pub mod client;
pub mod parser;

pub use client::{SbsClient, SbsClientConfig};
pub use parser::{SbsMessage, SbsMessageType, parse_sbs_message};

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
