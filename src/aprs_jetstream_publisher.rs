use anyhow::{Context, Result};
use async_nats::jetstream::{context::Context as JetStreamContext, stream::Stream};
use tracing::{debug, error, info, warn};

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

        // Log once at the start, not on each retry
        debug!(
            "Publishing message to JetStream (len={}, first 50 chars): {}",
            message.len(),
            &message.chars().take(50).collect::<String>()
        );

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

    /// Publish a message in fire-and-forget mode
    ///
    /// This version sends the message to JetStream but does not wait for acknowledgment.
    /// This maximizes throughput at the cost of not knowing if the message was persisted.
    /// Acceptable for high-volume streaming data where message loss during crashes is tolerable.
    pub async fn publish_fire_and_forget(&self, message: &str) {
        let start = std::time::Instant::now();

        // Convert message to owned bytes for JetStream
        let bytes = message.as_bytes().to_vec();
        let after_conversion = std::time::Instant::now();

        // Publish to JetStream - fire and forget
        // Only await the first future (sends the message), skip the second (acknowledgment)
        match self
            .jetstream
            .publish(self.subject.clone(), bytes.into())
            .await
        {
            Ok(_ack_future) => {
                // Message sent successfully, but we don't wait for the ack
                let after_publish = std::time::Instant::now();
                let total_duration_ms = start.elapsed().as_millis() as f64;
                let conversion_ms = after_conversion.duration_since(start).as_millis() as f64;
                let publish_call_ms =
                    after_publish.duration_since(after_conversion).as_millis() as f64;

                metrics::histogram!("aprs.jetstream.publish_duration_ms").record(total_duration_ms);
                metrics::histogram!("aprs.jetstream.publish_call_only_ms").record(publish_call_ms);
                metrics::histogram!("aprs.jetstream.conversion_ms").record(conversion_ms);
                metrics::counter!("aprs.jetstream.published").increment(1);

                // Log slow publishes with breakdown
                if total_duration_ms > 100.0 {
                    warn!(
                        "Slow publish: total={:.1}ms (conversion={:.1}ms, publish_call={:.1}ms)",
                        total_duration_ms, conversion_ms, publish_call_ms
                    );
                }
            }
            Err(e) => {
                // Failed to even send the message
                error!("Failed to send message to JetStream: {}", e);
                metrics::counter!("aprs.jetstream.publish_error").increment(1);
            }
        }
    }
}
