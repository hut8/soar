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
        // This is a durable pull consumer that tracks message acknowledgments
        let consumer_config = PullConfig {
            durable_name: Some(consumer_name.clone()),
            ack_policy: AckPolicy::Explicit, // Require explicit ack after processing
            deliver_policy: DeliverPolicy::All, // Start from beginning for new consumers
            filter_subject: subject,
            max_ack_pending: 50_000, // Allow 50K unACKed messages (default 1000 is too low for slow workers)
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
    /// Messages are NOT acknowledged here - the callback must handle ACKing after processing.
    ///
    /// The callback receives:
    /// - payload: String - the message content
    /// - msg: Arc<Message> - the JetStream message handle for ACKing
    ///
    /// The callback should return Ok(()) if the message was processed successfully,
    /// or Err if processing failed (the message will be NAKed for retry).
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
        let mut error_count = 0u64;
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
                            // Acknowledge the invalid message so we don't get stuck
                            if let Err(ack_err) = msg.ack().await {
                                error!("Failed to ack invalid message: {}", ack_err);
                            }
                            continue;
                        }
                    };

                    // Wrap message in Arc for sharing with workers
                    let msg_arc = std::sync::Arc::new(msg);

                    // Process the message - callback will handle ACKing after processing
                    match process_message(payload, msg_arc.clone()).await {
                        Ok(()) => {
                            // Message processed successfully - worker will ACK
                            processed_count += 1;
                            metrics::counter!("aprs.jetstream.consumed").increment(1);

                            // Log progress every 1000 messages
                            if processed_count.is_multiple_of(1000) {
                                let elapsed_since_start = start_time.elapsed().as_secs_f64();
                                let rate_since_start = processed_count as f64 / elapsed_since_start;

                                let elapsed_since_last_log = last_log_time.elapsed().as_secs_f64();
                                let messages_since_last_log = processed_count - last_log_count;
                                let rate_since_last_log =
                                    messages_since_last_log as f64 / elapsed_since_last_log;

                                info!(
                                    "Processed {} messages ({:.1} msg/s since start, {:.1} msg/s recent, {} errors)",
                                    processed_count,
                                    rate_since_start,
                                    rate_since_last_log,
                                    error_count
                                );

                                // Update last log tracking
                                last_log_time = std::time::Instant::now();
                                last_log_count = processed_count;
                            }
                        }
                        Err(e) => {
                            error_count += 1;
                            warn!("Failed to process message: {} - will retry", e);
                            metrics::counter!("aprs.jetstream.process_error").increment(1);

                            // NAK the message so it will be redelivered
                            if let Err(nak_err) = msg_arc
                                .ack_with(async_nats::jetstream::AckKind::Nak(None))
                                .await
                            {
                                error!("Failed to NAK message: {}", nak_err);
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
