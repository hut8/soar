use anyhow::{Context, Result};
use async_nats::jetstream::{context::Context as JetStreamContext, stream::Stream};
use tracing::{debug, error, info, warn};

/// Publisher that sends raw Beast (ADS-B) messages to NATS JetStream for durable queuing
///
/// This allows the Beast ingestion service to be decoupled from message processing,
/// ensuring no messages are lost during restarts or deployments.
#[derive(Clone)]
pub struct JetStreamPublisher {
    jetstream: JetStreamContext,
    subject: String,
    _stream: Stream,
}

impl JetStreamPublisher {
    /// Create a new JetStream publisher
    pub fn new(jetstream: JetStreamContext, subject: String, stream: Stream) -> Self {
        Self {
            jetstream,
            subject,
            _stream: stream,
        }
    }

    /// Publish a raw Beast message to JetStream
    ///
    /// This is called by the Beast client for each message received from the Beast server
    pub async fn publish(&self, message: &[u8]) -> Result<()> {
        // Convert message to owned bytes for JetStream
        let bytes = message.to_vec();

        // Publish to JetStream with acknowledgment
        let ack = self
            .jetstream
            .publish(self.subject.clone(), bytes.into())
            .await
            .context("Failed to publish message to JetStream")?;

        // Wait for acknowledgment from JetStream
        ack.await.context("Failed to get ack from JetStream")?;

        metrics::counter!("beast.jetstream.published").increment(1);

        Ok(())
    }

    /// Publish a message with error handling and retry logic
    ///
    /// This version logs errors instead of propagating them, to prevent
    /// a single failed publish from stopping the entire ingestion process
    pub async fn publish_with_retry(&self, message: &[u8], max_retries: u32) -> Result<()> {
        const RETRY_DELAY_MS: u64 = 100;

        // Log once at the start, not on each retry
        debug!(
            "Publishing Beast message to JetStream (len={} bytes)",
            message.len()
        );

        for attempt in 1..=max_retries {
            match self.publish(message).await {
                Ok(()) => {
                    if attempt > 1 {
                        info!(
                            "Beast message published successfully after {} attempts",
                            attempt
                        );
                    }
                    return Ok(());
                }
                Err(e) => {
                    metrics::counter!("beast.jetstream.publish_error").increment(1);

                    if attempt < max_retries {
                        warn!(
                            "Failed to publish Beast message (attempt {}/{}): {} - retrying...",
                            attempt, max_retries, e
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            RETRY_DELAY_MS * attempt as u64,
                        ))
                        .await;
                    } else {
                        error!(
                            "Failed to publish Beast message after {} attempts: {}",
                            max_retries, e
                        );
                        return Err(e);
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "Failed to publish Beast message after {} retries",
            max_retries
        ))
    }

    /// Publish a message in fire-and-forget mode
    ///
    /// This version sends the message to JetStream but does not wait for acknowledgment.
    /// This maximizes throughput at the cost of not knowing if the message was persisted.
    /// Acceptable for high-volume streaming data where message loss during crashes is tolerable.
    pub async fn publish_fire_and_forget(&self, message: &[u8]) {
        let start = std::time::Instant::now();

        // Convert message to owned bytes for JetStream
        let bytes = message.to_vec();

        // Publish to JetStream - fire and forget
        // Only await the first future (sends the message), skip the second (acknowledgment)
        match self
            .jetstream
            .publish(self.subject.clone(), bytes.into())
            .await
        {
            Ok(_ack_future) => {
                // Message sent successfully, but we don't wait for the ack
                let duration_ms = start.elapsed().as_millis() as f64;
                metrics::histogram!("beast.jetstream.publish_duration_ms").record(duration_ms);
                metrics::counter!("beast.jetstream.published").increment(1);
            }
            Err(e) => {
                // Failed to even send the message
                error!("Failed to send Beast message to JetStream: {}", e);
                metrics::counter!("beast.jetstream.publish_error").increment(1);
            }
        }
    }
}
