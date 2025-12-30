use anyhow::{Context, Result};
use async_nats::Client;
use tracing::{error, warn};

use crate::beast::BeastPublisher;

/// Publisher that sends raw Beast messages to NATS for lightweight, fast queuing
///
/// This allows the Beast ingestion service to be decoupled from message processing.
/// Uses regular NATS (not JetStream) for maximum throughput with fire-and-forget semantics.
#[derive(Clone)]
pub struct NatsPublisher {
    client: Client,
    subject: String,
}

impl NatsPublisher {
    /// Create a new NATS publisher for Beast messages
    pub fn new(client: Client, subject: String) -> Self {
        Self { client, subject }
    }

    /// Publish a raw Beast message to NATS
    ///
    /// This is called by the Beast client for each message received from the Beast server.
    /// Uses fire-and-forget semantics for maximum throughput.
    pub fn publish_fire_and_forget(&self, message: &[u8]) {
        // Clone for the spawned task
        let client = self.client.clone();
        let subject = self.subject.clone();
        let message_bytes = message.to_vec();

        // Spawn task to publish without blocking
        tokio::spawn(async move {
            let start = std::time::Instant::now();

            // Regular NATS publish - fire and forget, no ack
            match client.publish(subject, message_bytes.into()).await {
                Ok(()) => {
                    let duration_ms = start.elapsed().as_millis() as f64;
                    metrics::histogram!("beast.nats.publish_duration_ms").record(duration_ms);
                    metrics::counter!("beast.nats.published_total").increment(1);

                    // Log slow publishes
                    if duration_ms > 100.0 {
                        warn!("Slow NATS publish: {:.1}ms", duration_ms);
                    }
                }
                Err(e) => {
                    error!("Failed to publish Beast message to NATS: {}", e);
                    metrics::counter!("beast.nats.publish_error_total").increment(1);
                }
            }
        });
    }

    /// Publish with acknowledgment and error propagation (for critical messages)
    pub async fn publish(&self, message: &[u8]) -> Result<()> {
        self.client
            .publish(self.subject.clone(), message.to_vec().into())
            .await
            .context("Failed to publish Beast message to NATS")?;

        metrics::counter!("beast.nats.published_total").increment(1);

        Ok(())
    }

    /// Publish a message with retry logic (for graceful shutdown flushing)
    ///
    /// Retries up to `max_retries` times with exponential backoff on failure
    pub async fn publish_with_retry(&self, message: &[u8], max_retries: u32) -> Result<()> {
        let mut retries = 0;

        loop {
            match self.publish(message).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    retries += 1;
                    if retries >= max_retries {
                        return Err(e)
                            .context(format!("Failed to publish after {} retries", max_retries));
                    }

                    let delay_ms = 100 * (2_u64.pow(retries - 1)); // Exponential backoff: 100ms, 200ms, 400ms
                    warn!(
                        "Failed to publish Beast message (retry {}/{}): {} - retrying in {}ms",
                        retries, max_retries, e, delay_ms
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }
}

// Implement BeastPublisher trait for NATS publisher
#[async_trait::async_trait]
impl BeastPublisher for NatsPublisher {
    async fn publish_fire_and_forget(&self, message: &[u8]) {
        self.publish_fire_and_forget(message)
    }

    async fn publish_with_retry(&self, message: &[u8], max_retries: u32) -> Result<()> {
        self.publish_with_retry(message, max_retries).await
    }
}
