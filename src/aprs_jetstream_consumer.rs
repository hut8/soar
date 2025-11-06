use anyhow::{Context, Result};
use async_nats::jetstream::{
    consumer::{AckPolicy, DeliverPolicy, pull::Config as PullConfig},
    context::Context as JetStreamContext,
    stream::Stream,
};
use futures_util::StreamExt;
use tracing::{error, info, warn};

/// Consumer that reads raw APRS messages from NATS JetStream durable queue
///
/// This allows the APRS processing service to consume messages from the durable queue
/// published by the soar-aprs-ingest service, ensuring no messages are lost during restarts.
#[derive(Clone)]
pub struct JetStreamConsumer {
    jetstream: JetStreamContext,
    stream_name: String,
    consumer_name: String,
    _stream: Stream,
}

impl JetStreamConsumer {
    /// Create a new JetStream consumer
    ///
    /// This will create a durable consumer if it doesn't exist, or reuse an existing one.
    /// The consumer tracks which messages have been processed, so on restart it picks up
    /// where it left off.
    pub async fn new(
        jetstream: JetStreamContext,
        stream_name: String,
        subject: String,
        consumer_name: String,
    ) -> Result<Self> {
        info!(
            "Setting up JetStream consumer '{}' for stream '{}'...",
            consumer_name, stream_name
        );

        // Get or create the stream
        let stream = jetstream
            .get_stream(&stream_name)
            .await
            .context(format!("Failed to get JetStream stream '{}'", stream_name))?;

        info!("JetStream stream '{}' found", stream_name);

        // Create or get the consumer
        // This is a durable pull consumer with explicit ACK (required for WorkQueue streams)
        // Messages are ACKed after they're successfully queued for processing
        let consumer_config = PullConfig {
            durable_name: Some(consumer_name.clone()),
            ack_policy: AckPolicy::Explicit, // Explicit ACK - required for WorkQueue retention
            deliver_policy: DeliverPolicy::All, // Start from beginning for new consumers
            filter_subject: subject,
            max_ack_pending: 1000, // Allow up to 1000 unacked messages
            ..Default::default()
        };

        match stream.get_consumer::<PullConfig>(&consumer_name).await {
            Ok(consumer) => {
                info!(
                    "JetStream consumer '{}' already exists, reusing it",
                    consumer_name
                );
                // Verify it's tracking the right subject
                drop(consumer); // We'll recreate it below
            }
            Err(_) => {
                info!("Creating new JetStream consumer '{}'...", consumer_name);
                stream
                    .create_consumer(consumer_config.clone())
                    .await
                    .context(format!(
                        "Failed to create JetStream consumer '{}'",
                        consumer_name
                    ))?;
                info!("JetStream consumer '{}' created", consumer_name);
            }
        }

        Ok(Self {
            jetstream,
            stream_name: stream_name.clone(),
            consumer_name: consumer_name.clone(),
            _stream: stream,
        })
    }

    /// Start consuming messages and process them with the provided callback
    ///
    /// This will run indefinitely, processing messages as they arrive.
    /// Messages are explicitly ACKed after successful queueing (AckPolicy::Explicit).
    /// If queueing fails, the message is NAKed and will be redelivered.
    ///
    /// The callback receives:
    /// - payload: String - the message content
    /// - msg: Arc<Message> - the JetStream message handle (not needed by callback)
    ///
    /// The callback should return Ok(()) if the message was queued successfully,
    /// or Err if queueing failed.
    pub async fn consume<F, Fut>(&self, mut process_message: F) -> Result<()>
    where
        F: FnMut(String, std::sync::Arc<async_nats::jetstream::Message>) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        info!(
            "Starting JetStream consumer '{}' for stream '{}'...",
            self.consumer_name, self.stream_name
        );

        // Get the consumer
        let consumer = self
            .jetstream
            .get_stream(&self.stream_name)
            .await
            .context("Failed to get stream")?
            .get_consumer::<PullConfig>(&self.consumer_name)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get consumer: {}", e))?;

        info!(
            "JetStream consumer ready, waiting for messages on stream '{}'",
            self.stream_name
        );

        // Create a message stream with batch size of 100
        let mut messages = consumer
            .messages()
            .await
            .context("Failed to get messages")?;

        // Track stats for logging
        let mut processed_count = 0u64;
        let start_time = std::time::Instant::now();
        let mut last_log_time = std::time::Instant::now();
        let mut last_log_count = 0u64;

        // Process messages as they arrive
        while let Some(message) = messages.next().await {
            match message {
                Ok(msg) => {
                    // Convert message payload to string
                    let payload = match String::from_utf8(msg.payload.to_vec()) {
                        Ok(s) => s,
                        Err(e) => {
                            error!("Failed to decode message payload as UTF-8: {}", e);
                            metrics::counter!("aprs.jetstream.decode_error").increment(1);
                            // ACK invalid messages so they're removed from the queue
                            if let Err(ack_err) = msg.ack().await {
                                error!("Failed to ACK invalid message: {}", ack_err);
                            }
                            continue;
                        }
                    };

                    // Wrap message in Arc for sharing with workers
                    let msg_arc = std::sync::Arc::new(msg);

                    // Process the message - ALWAYS ACK (never NAK)
                    // The callback uses blocking send which provides backpressure:
                    // - When queue is full, send_async() blocks, which prevents ACK, which stops JetStream delivery
                    // - The only error case is channel closed (shutdown), in which case we ACK to clean up
                    match process_message(payload, msg_arc.clone()).await {
                        Ok(()) | Err(_) => {
                            // Always ACK - either successfully queued or shutting down
                            // Backpressure comes from blocking send, not from NAK/redelivery
                            if let Err(e) = msg_arc.ack().await {
                                error!("Failed to ACK message: {} - will be redelivered", e);
                                metrics::counter!("aprs.jetstream.ack_error").increment(1);
                            } else {
                                processed_count += 1;
                                metrics::counter!("aprs.jetstream.consumed").increment(1);

                                // Log progress every 1000 messages
                                if processed_count.is_multiple_of(1000) {
                                    let elapsed_since_start = start_time.elapsed().as_secs_f64();
                                    let rate_since_start =
                                        processed_count as f64 / elapsed_since_start;

                                    let elapsed_since_last_log =
                                        last_log_time.elapsed().as_secs_f64();
                                    let messages_since_last_log = processed_count - last_log_count;
                                    let rate_since_last_log =
                                        messages_since_last_log as f64 / elapsed_since_last_log;

                                    info!(
                                        "Processed {} messages ({:.1} msg/s since start, {:.1} msg/s recent)",
                                        processed_count, rate_since_start, rate_since_last_log
                                    );

                                    // Update last log tracking
                                    last_log_time = std::time::Instant::now();
                                    last_log_count = processed_count;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Error receiving message from JetStream: {}", e);
                    metrics::counter!("aprs.jetstream.receive_error").increment(1);

                    // Sleep briefly to avoid tight loop on persistent errors
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }

        warn!("JetStream consumer message stream ended unexpectedly");
        Ok(())
    }
}
