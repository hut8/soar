use anyhow::Result;
use chrono::{DateTime, Utc};
use tokio::time::{Duration, sleep};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::beast_jetstream_consumer::JetStreamConsumer;
use crate::raw_messages_repo::{BeastMessagesRepository, NewBeastMessage};

/// Task that consumes Beast messages from JetStream and stores them in the database
///
/// This task:
/// 1. Reads binary messages from JetStream (8-byte timestamp + Beast frame)
/// 2. Decodes the timestamp and frame
/// 3. Stores the message in the raw_messages table via BeastMessagesRepository
/// 4. Provides backpressure by blocking when database writes are slow
pub struct BeastConsumerTask {
    consumer: JetStreamConsumer,
    repository: BeastMessagesRepository,
    receiver_id: Uuid,
    batch_size: usize,
}

impl BeastConsumerTask {
    pub fn new(
        consumer: JetStreamConsumer,
        repository: BeastMessagesRepository,
        receiver_id: Uuid,
    ) -> Self {
        Self {
            consumer,
            repository,
            receiver_id,
            batch_size: 100, // Process messages in batches
        }
    }

    /// Start consuming and storing Beast messages
    ///
    /// This will run indefinitely, consuming messages from JetStream and storing them
    /// in the database. Messages are processed in batches for efficiency.
    pub async fn run(&self) -> Result<()> {
        info!(
            "Starting Beast consumer task for receiver {}",
            self.receiver_id
        );

        // Create bounded channel for batch processing
        let (batch_tx, batch_rx) = flume::bounded::<NewBeastMessage>(self.batch_size * 2);

        // Spawn batch writer task
        let repository = self.repository.clone();
        let batch_writer_handle = tokio::spawn(async move {
            Self::batch_writer_task(batch_rx, repository).await;
        });

        // Start consuming messages from JetStream
        let receiver_id = self.receiver_id;
        let consume_result = self
            .consumer
            .consume(move |payload, _jetstream_msg| {
                let batch_tx = batch_tx.clone();
                async move {
                    // Decode message: 8-byte timestamp + Beast frame
                    if payload.len() < 9 {
                        warn!(
                            "Invalid Beast message: too short ({} bytes, expected at least 9)",
                            payload.len()
                        );
                        metrics::counter!("beast.consumer.invalid_message").increment(1);
                        return Ok(()); // ACK invalid messages
                    }

                    // Extract timestamp (first 8 bytes, big-endian i64 microseconds)
                    let timestamp_bytes: [u8; 8] = payload[0..8].try_into().unwrap();
                    let timestamp_micros = i64::from_be_bytes(timestamp_bytes);
                    let received_at =
                        DateTime::from_timestamp_micros(timestamp_micros).unwrap_or_else(Utc::now);

                    // Extract Beast frame (remaining bytes)
                    let raw_message = payload[8..].to_vec();

                    // Create new Beast message
                    let message = NewBeastMessage::new(raw_message, received_at, receiver_id, None);

                    // Send to batch writer (blocking send for backpressure)
                    match batch_tx.send_async(message).await {
                        Ok(_) => {
                            metrics::counter!("beast.consumer.received").increment(1);
                            Ok(())
                        }
                        Err(e) => {
                            error!("Failed to send message to batch writer: {}", e);
                            metrics::counter!("beast.consumer.send_errors").increment(1);
                            Ok(()) // Still ACK to JetStream
                        }
                    }
                }
            })
            .await;

        // If we reach here, the consumer stopped
        match consume_result {
            Ok(_) => {
                info!("Beast consumer stopped normally");
            }
            Err(e) => {
                error!("Beast consumer failed: {}", e);
            }
        }

        // Wait for batch writer to finish
        // batch_tx is dropped when consume() returns, which closes the channel
        // causing batch_writer_task to exit gracefully
        batch_writer_handle.await?;

        Ok(())
    }

    /// Batch writer task that accumulates messages and writes them in batches
    ///
    /// This improves database throughput by reducing the number of transactions
    async fn batch_writer_task(
        batch_rx: flume::Receiver<NewBeastMessage>,
        repository: BeastMessagesRepository,
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
        repository: &BeastMessagesRepository,
        batch: &mut Vec<NewBeastMessage>,
        last_write: &mut std::time::Instant,
    ) {
        if batch.is_empty() {
            return;
        }

        let start = std::time::Instant::now();
        let batch_size = batch.len();

        match repository.insert_batch(batch).await {
            Ok(_) => {
                let duration = start.elapsed();
                metrics::histogram!("beast.consumer.batch_write_ms")
                    .record(duration.as_millis() as f64);
                metrics::counter!("beast.consumer.messages_stored").increment(batch_size as u64);
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
                metrics::counter!("beast.consumer.write_errors").increment(1);
            }
        }

        batch.clear();
        *last_write = std::time::Instant::now();
    }
}
