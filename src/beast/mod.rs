pub mod adsb_to_fix;
pub mod client;
pub mod cpr_decoder;
pub mod decoder;

pub use adsb_to_fix::adsb_message_to_fix;
pub use client::{BeastClient, BeastClientConfig};
pub use cpr_decoder::{CprDecoder, DecodedPosition};
pub use decoder::{DecodedBeastMessage, decode_beast_frame};

use anyhow::Result;

/// Trait for Beast message publishers
/// Allows the Beast client to work with different publisher implementations
#[async_trait::async_trait]
pub trait BeastPublisher: Clone + Send + Sync + 'static {
    /// Publish a message with fire-and-forget semantics
    async fn publish_fire_and_forget(&self, message: &[u8]);

    /// Publish a message with retry logic (for graceful shutdown)
    async fn publish_with_retry(&self, message: &[u8], max_retries: u32) -> Result<()>;
}
