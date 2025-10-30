use anyhow::{Context, Result};
use async_nats::jetstream::{context::Context as JetStreamContext, stream::Stream};
use tracing::{error, info, warn};

/// Publisher that sends raw APRS messages to NATS JetStream for durable queuing
///
/// This allows the APRS ingestion service to be decoupled from message processing,
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

    /// Publish a raw APRS message to JetStream
    ///
    /// This is called by the APRS client for each message received from APRS-IS
    #[tracing::instrument(skip(self, message), fields(message_len = message.len()))]
    pub async fn publish(&self, message: &str) -> Result<()> {
        // Convert message to owned bytes for JetStream
        let bytes = message.as_bytes().to_vec();

        // Publish to JetStream with acknowledgment
        let ack = self
            .jetstream
            .publish(self.subject.clone(), bytes.into())
            .await
            .context("Failed to publish message to JetStream")?;

        // Wait for acknowledgment from JetStream
        ack.await.context("Failed to get ack from JetStream")?;

        metrics::counter!("aprs.jetstream.published").increment(1);

        Ok(())
    }

    /// Publish a message with error handling and retry logic
    ///
    /// This version logs errors instead of propagating them, to prevent
    /// a single failed publish from stopping the entire ingestion process
    pub async fn publish_with_retry(&self, message: &str) {
        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAY_MS: u64 = 100;

        for attempt in 1..=MAX_RETRIES {
            match self.publish(message).await {
                Ok(()) => {
                    if attempt > 1 {
                        info!("Message published successfully after {} attempts", attempt);
                    }
                    return;
                }
                Err(e) => {
                    metrics::counter!("aprs.jetstream.publish_error").increment(1);

                    if attempt < MAX_RETRIES {
                        warn!(
                            "Failed to publish message (attempt {}/{}): {} - retrying...",
                            attempt, MAX_RETRIES, e
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            RETRY_DELAY_MS * attempt as u64,
                        ))
                        .await;
                    } else {
                        error!(
                            "Failed to publish message after {} attempts: {}",
                            MAX_RETRIES, e
                        );
                    }
                }
            }
        }
    }
}
