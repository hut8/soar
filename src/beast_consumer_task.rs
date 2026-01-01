use anyhow::{Context as _, Result};
use async_nats::Client;
use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::beast::decoder::{decode_beast_frame, message_to_json};
use crate::raw_messages_repo::{NewBeastMessage, RawMessagesRepository};

/// Task that consumes Beast messages from NATS and stores them in the database
///
/// This task:
/// 1. Reads binary messages from NATS (8-byte timestamp + Beast frame)
/// 2. Decodes the timestamp and frame
/// 3. Stores the message in the raw_messages table via RawMessagesRepository
/// 4. Provides backpressure by blocking when database writes are slow
pub struct BeastConsumerTask {
    nats_client: Client,
    subject: String,
    repository: RawMessagesRepository,
    receiver_id: Uuid,
    batch_size: usize,
}

impl BeastConsumerTask {
    pub fn new(
        nats_client: Client,
        subject: String,
        repository: RawMessagesRepository,
        receiver_id: Uuid,
    ) -> Self {
        Self {
            nats_client,
            subject,
            repository,
            receiver_id,
            batch_size: 100, // Process messages in batches
        }
    }

    /// Start consuming and storing Beast messages
    ///
    /// This will run indefinitely, consuming messages from NATS and storing them
    /// in the database. Messages are processed in batches for efficiency.
    pub async fn run(&self) -> Result<()> {
        info!(
            "Starting Beast consumer task for receiver {}",
            self.receiver_id
        );

        // Subscribe to NATS subject
        let mut subscriber = self
            .nats_client
            .subscribe(self.subject.clone())
            .await
            .context("Failed to subscribe to NATS subject")?;

        info!("Subscribed to NATS subject '{}'", self.subject);

        // Create bounded channel for batch processing
        let (batch_tx, batch_rx) = flume::bounded::<NewBeastMessage>(self.batch_size * 2);

        // Spawn batch writer task
        let repository = self.repository.clone();
        let batch_writer_handle = tokio::spawn(async move {
            Self::batch_writer_task(batch_rx, repository).await;
        });

        // Start consuming messages from NATS
        let receiver_id = self.receiver_id;
        while let Some(msg) = subscriber.next().await {
            let payload = msg.payload.to_vec();

            // Decode message: 8-byte timestamp + Beast frame
            if payload.len() < 9 {
                warn!(
                    "Invalid Beast message: too short ({} bytes, expected at least 9)",
                    payload.len()
                );
                metrics::counter!("beast.nats.invalid_message_total").increment(1);
                continue;
            }

            // Extract timestamp (first 8 bytes, big-endian i64 microseconds)
            let timestamp_bytes: [u8; 8] = payload[0..8].try_into().unwrap();
            let timestamp_micros = i64::from_be_bytes(timestamp_bytes);
            let received_at =
                DateTime::from_timestamp_micros(timestamp_micros).unwrap_or_else(Utc::now);

            // Extract Beast frame (remaining bytes)
            let raw_frame = &payload[8..];

            // Decode the Beast frame using rs1090
            let decoded_json = match decode_beast_frame(raw_frame, received_at) {
                Ok(decoded) => {
                    metrics::counter!("beast.nats.decoded_total").increment(1);
                    // Convert to JSON for storage
                    match message_to_json(&decoded.message) {
                        Ok(json) => {
                            debug!("Decoded Beast message: {:?}", json);
                            Some(json.to_string())
                        }
                        Err(e) => {
                            warn!("Failed to serialize decoded message to JSON: {}", e);
                            metrics::counter!("beast.nats.json_error_total").increment(1);
                            None
                        }
                    }
                }
                Err(e) => {
                    // Log decode errors but still store the raw frame
                    debug!("Failed to decode Beast frame: {}", e);
                    metrics::counter!("beast.nats.decode_error_total").increment(1);
                    None
                }
            };

            // Create new Beast message with decoded JSON in unparsed field
            let message =
                NewBeastMessage::new(raw_frame.to_vec(), received_at, receiver_id, decoded_json);

            // Send to batch writer (blocking send for backpressure)
            match batch_tx.send_async(message).await {
                Ok(_) => {
                    metrics::counter!("beast.nats.consumed_total").increment(1);
                }
                Err(e) => {
                    // Channel closed - batch writer stopped
                    error!(
                        "Failed to send message to batch writer (channel closed): {}",
                        e
                    );
                    break;
                }
            }
        }

        // If we reach here, the subscriber stopped
        info!("Beast NATS subscriber stopped");

        // Drop batch_tx to signal batch writer to stop
        drop(batch_tx);

        // Wait for batch writer to finish
        batch_writer_handle.await?;

        Ok(())
    }

    /// Batch writer task that accumulates messages and writes them in batches
    ///
    /// This improves database throughput by reducing the number of transactions
    async fn batch_writer_task(
        batch_rx: flume::Receiver<NewBeastMessage>,
        repository: RawMessagesRepository,
    ) {
        info!("Starting Beast batch writer task");

        let mut batch = Vec::with_capacity(100);
        let mut last_write = std::time::Instant::now();
        let max_batch_wait = Duration::from_millis(100); // Write batch after 100ms even if not full

        loop {
            // Try to receive a message with timeout
            let message_result = tokio::select! {
                msg = batch_rx.recv_async() => msg,
                _ = sleep(max_batch_wait) => {
                    // Timeout - write batch if we have any messages
                    if !batch.is_empty() {
                        Self::write_batch(&repository, &mut batch, &mut last_write).await;
                    }
                    continue;
                }
            };

            match message_result {
                Ok(message) => {
                    batch.push(message);

                    // Write batch if it's full or if enough time has passed
                    if batch.len() >= 100 || last_write.elapsed() >= max_batch_wait {
                        Self::write_batch(&repository, &mut batch, &mut last_write).await;
                    }
                }
                Err(_) => {
                    // Channel closed - write remaining batch and exit
                    if !batch.is_empty() {
                        Self::write_batch(&repository, &mut batch, &mut last_write).await;
                    }
                    info!("Beast batch writer task stopped");
                    break;
                }
            }
        }
    }

    async fn write_batch(
        repository: &RawMessagesRepository,
        batch: &mut Vec<NewBeastMessage>,
        last_write: &mut std::time::Instant,
    ) {
        if batch.is_empty() {
            return;
        }

        let start = std::time::Instant::now();
        let batch_size = batch.len();

        match repository.insert_beast_batch(batch).await {
            Ok(_) => {
                let duration = start.elapsed();
                metrics::histogram!("beast.consumer.batch_write_ms")
                    .record(duration.as_millis() as f64);
                metrics::counter!("beast.consumer.messages_stored_total")
                    .increment(batch_size as u64);
                info!(
                    "Wrote batch of {} Beast messages in {:?}",
                    batch_size, duration
                );
            }
            Err(e) => {
                error!(
                    "Failed to write batch of {} Beast messages: {}",
                    batch_size, e
                );
                metrics::counter!("beast.consumer.write_errors_total").increment(1);
            }
        }

        batch.clear();
        *last_write = std::time::Instant::now();
    }
}
