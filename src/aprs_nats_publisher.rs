use anyhow::{Context, Result};
use async_nats::Client;
use tracing::{error, warn};

/// Publisher that sends raw APRS messages to NATS for lightweight, fast queuing
///
/// This allows the APRS ingestion service to be decoupled from message processing.
/// Uses regular NATS (not JetStream) for maximum throughput with fire-and-forget semantics.
#[derive(Clone)]
pub struct NatsPublisher {
    client: Client,
    subject: String,
}

impl NatsPublisher {
    /// Create a new NATS publisher
    pub fn new(client: Client, subject: String) -> Self {
        Self { client, subject }
    }

    /// Publish a raw APRS message to NATS
    ///
    /// This is called by the APRS client for each message received from APRS-IS.
    /// Uses fire-and-forget semantics for maximum throughput.
    pub fn publish_fire_and_forget(&self, message: &str) {
        // Clone for the spawned task
        let client = self.client.clone();
        let subject = self.subject.clone();
        let message_bytes = message.as_bytes().to_vec();

        // Spawn task to publish without blocking
        tokio::spawn(async move {
            let start = std::time::Instant::now();

            // Regular NATS publish - fire and forget, no ack
            match client.publish(subject, message_bytes.into()).await {
                Ok(()) => {
                    let duration_ms = start.elapsed().as_millis() as f64;
                    metrics::histogram!("aprs.nats.publish_duration_ms").record(duration_ms);
                    metrics::counter!("aprs.nats.published_total").increment(1);

                    // Track slow publishes (>100ms is considered slow for fire-and-forget)
                    if duration_ms > 100.0 {
                        warn!("Slow NATS publish: {:.1}ms", duration_ms);
                        metrics::counter!("aprs.nats.slow_publish_total").increment(1);
                    }
                }
                Err(e) => {
                    error!("Failed to publish message to NATS: {}", e);
                    metrics::counter!("aprs.nats.publish_error_total").increment(1);
                }
            }
        });
    }

    /// Publish with acknowledgment and error propagation (for critical messages)
    pub async fn publish(&self, message: &str) -> Result<()> {
        let bytes = message.as_bytes().to_vec();

        self.client
            .publish(self.subject.clone(), bytes.into())
            .await
            .context("Failed to publish message to NATS")?;

        metrics::counter!("aprs.nats.published_total").increment(1);

        Ok(())
    }
}
