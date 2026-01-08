use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{info, warn};

const MAGIC_BYTES: &[u8; 8] = b"SOARQUE1";
const HEADER_SIZE: usize = 32;

/// State of the persistent queue
#[derive(Debug, Clone)]
pub enum QueueState {
    Connected {
        consumer_id: String,
        connected_at: Instant,
    },
    Disconnected {
        disconnected_at: Instant,
        messages_buffered: u64,
        bytes_buffered: u64,
    },
    Draining {
        drain_started_at: Instant,
        messages_drained: u64,
        messages_in_backlog: u64,
        new_messages_buffered: u64,
    },
}

/// Current depth of the queue
#[derive(Debug, Clone)]
pub struct QueueDepth {
    pub memory: usize,
    pub disk: usize,
}

/// A persistent file-backed queue with fast-path memory optimization
///
/// States:
/// - Connected: Messages go directly through memory channel (fast path)
/// - Disconnected: Messages buffer to disk file (slow path)
/// - Draining: Replay disk backlog while buffering new messages
///
/// Delivery Semantics:
/// - At-most-once: Messages are marked as consumed only after successful delivery
/// - If crash occurs after recv() but before commit(), message will be replayed
pub struct PersistentQueue<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    name: String,
    file_path: PathBuf,
    max_size_bytes: Option<u64>,
    state: Arc<RwLock<QueueState>>,
    memory_tx: flume::Sender<T>,
    memory_rx: flume::Receiver<T>,
    /// Pending read offset - set when message is read, committed when delivery succeeds
    /// This ensures at-most-once delivery: offset only advances after successful send
    pending_commit_offset: Arc<RwLock<Option<u64>>>,
}

impl<T> PersistentQueue<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    /// Create a new persistent queue
    ///
    /// # Arguments
    /// * `name` - Queue name (for metrics)
    /// * `file_path` - Path to persistent file
    /// * `max_size_bytes` - Optional maximum file size (disconnect on overflow)
    /// * `memory_capacity` - Bounded channel capacity for fast path
    pub fn new(
        name: String,
        file_path: PathBuf,
        max_size_bytes: Option<u64>,
        memory_capacity: usize,
    ) -> Result<Self> {
        // Create parent directory if needed
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let (memory_tx, memory_rx) = flume::bounded(memory_capacity);

        let queue = Self {
            name: name.clone(),
            file_path,
            max_size_bytes,
            state: Arc::new(RwLock::new(QueueState::Disconnected {
                disconnected_at: Instant::now(),
                messages_buffered: 0,
                bytes_buffered: 0,
            })),
            memory_tx,
            memory_rx,
            pending_commit_offset: Arc::new(RwLock::new(None)),
        };

        // Initialize metrics
        metrics::gauge!(format!("queue.{}.state", name)).set(0.0); // 0=disconnected, 1=connected, 2=draining

        Ok(queue)
    }

    /// Send a message to the queue
    ///
    /// Behavior depends on state:
    /// - Connected: Try to send through memory channel (non-blocking)
    ///   - If memory channel is full, overflow to disk
    /// - Disconnected/Draining: Append to disk file
    pub async fn send(&self, message: T) -> Result<()> {
        let state = { self.state.read().await.clone() };

        match state {
            QueueState::Connected { .. } => {
                // Fast path: try to send through memory channel (non-blocking)
                match self.memory_tx.try_send(message) {
                    Ok(_) => {
                        metrics::counter!(format!("queue.{}.messages.sent_total", self.name))
                            .increment(1);
                        Ok(())
                    }
                    Err(flume::TrySendError::Full(message)) => {
                        // Memory channel is full (publisher is slow/blocked)
                        // Overflow to disk to prevent blocking the entire pipeline
                        warn!(
                            "Queue {} memory channel full, overflowing to disk (publisher is slow)",
                            self.name
                        );
                        metrics::counter!(format!("queue.{}.overflow_to_disk_total", self.name))
                            .increment(1);
                        self.append_to_disk(message).await?;
                        metrics::counter!(format!("queue.{}.messages.buffered_total", self.name))
                            .increment(1);
                        Ok(())
                    }
                    Err(flume::TrySendError::Disconnected(_)) => {
                        Err(anyhow::anyhow!("Memory queue disconnected"))
                    }
                }
            }
            QueueState::Disconnected { .. } | QueueState::Draining { .. } => {
                // Slow path: append to disk
                self.append_to_disk(message).await?;
                metrics::counter!(format!("queue.{}.messages.buffered_total", self.name))
                    .increment(1);
                Ok(())
            }
        }
    }

    /// Receive a message from the queue
    ///
    /// In Draining mode, reads from disk backlog first, then switches to memory
    pub async fn recv(&self) -> Result<T> {
        let state = { self.state.read().await.clone() };

        match state {
            QueueState::Connected { .. } => {
                // Check if there's disk overflow (memory was full and messages went to disk)
                // This can happen when the publisher is slow/blocked
                if self.file_path.exists() {
                    // We have disk overflow - transition to Draining mode to process it
                    info!(
                        "Queue '{}' detected disk overflow in Connected state, switching to Draining",
                        self.name
                    );
                    {
                        let mut state_guard = self.state.write().await;
                        *state_guard = QueueState::Draining {
                            drain_started_at: Instant::now(),
                            messages_drained: 0,
                            messages_in_backlog: 0,
                            new_messages_buffered: 0,
                        };
                        metrics::gauge!(format!("queue.{}.state", self.name)).set(2.0);
                    }
                    // Now drain from disk first
                    if let Some(message) = self.read_from_disk().await? {
                        metrics::counter!(format!("queue.{}.messages.drained_total", self.name))
                            .increment(1);
                        return Ok(message);
                    }
                    // If disk is empty (race condition), fall through to recv from memory
                }

                // Fast path: receive from memory channel
                let message = self.memory_rx.recv_async().await?;
                metrics::counter!(format!("queue.{}.messages.received_total", self.name))
                    .increment(1);
                Ok(message)
            }
            QueueState::Draining { .. } => {
                // Drain mode: try to read from disk first
                if let Some(message) = self.read_from_disk().await? {
                    metrics::counter!(format!("queue.{}.messages.drained_total", self.name))
                        .increment(1);
                    Ok(message)
                } else {
                    // Backlog exhausted, switch to connected mode
                    self.finish_drain().await?;
                    // Now receive from memory
                    let message = self.memory_rx.recv_async().await?;
                    metrics::counter!(format!("queue.{}.messages.received_total", self.name))
                        .increment(1);
                    Ok(message)
                }
            }
            QueueState::Disconnected { .. } => {
                anyhow::bail!("Cannot receive from disconnected queue");
            }
        }
    }

    /// Commit a message delivery - marks it as consumed so it won't be replayed
    ///
    /// IMPORTANT: Must be called after successful delivery to ensure at-most-once semantics
    /// If crash occurs between recv() and commit(), message will be replayed
    pub async fn commit(&self) -> Result<()> {
        use std::io::{Seek, Write};

        // Get pending offset
        let offset = {
            let mut pending = self.pending_commit_offset.write().await;
            pending.take()
        };

        if let Some(new_read_offset) = offset {
            // Write new read_offset to disk
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .open(&self.file_path)?;

            file.seek(std::io::SeekFrom::Start(16))?;
            file.write_all(&new_read_offset.to_le_bytes())?;
            // Use sync_all() instead of flush() to force write to physical disk
            // This ensures durability and prevents duplicate messages on SIGKILL
            file.sync_all()?;

            metrics::counter!(format!("queue.{}.messages.committed_total", self.name)).increment(1);
        }

        Ok(())
    }

    /// Connect a consumer (transition to Connected or Draining state)
    pub async fn connect_consumer(&self, consumer_id: String) -> Result<()> {
        let mut state = self.state.write().await;

        // Check if there's a disk backlog
        let has_backlog = self.file_path.exists();

        if has_backlog {
            // Transition to Draining state
            *state = QueueState::Draining {
                drain_started_at: Instant::now(),
                messages_drained: 0,
                messages_in_backlog: 0, // Will be counted during drain
                new_messages_buffered: 0,
            };
            metrics::gauge!(format!("queue.{}.state", self.name)).set(2.0);
            info!(
                "Queue '{}' entering drain mode (consumer: {})",
                self.name, consumer_id
            );
        } else {
            // Transition to Connected state
            *state = QueueState::Connected {
                consumer_id,
                connected_at: Instant::now(),
            };
            metrics::gauge!(format!("queue.{}.state", self.name)).set(1.0);
            info!("Queue '{}' connected (fast path active)", self.name);
        }

        Ok(())
    }

    /// Disconnect consumer (transition to Disconnected state)
    pub async fn disconnect_consumer(&self) -> Result<()> {
        let mut state = self.state.write().await;
        *state = QueueState::Disconnected {
            disconnected_at: Instant::now(),
            messages_buffered: 0,
            bytes_buffered: 0,
        };
        metrics::gauge!(format!("queue.{}.state", self.name)).set(0.0);
        info!("Queue '{}' disconnected (buffering to disk)", self.name);
        Ok(())
    }

    /// Get current queue depth
    pub async fn depth(&self) -> QueueDepth {
        QueueDepth {
            memory: self.memory_tx.len(),
            disk: if self.file_path.exists() {
                std::fs::metadata(&self.file_path)
                    .map(|m| m.len() as usize)
                    .unwrap_or(0)
            } else {
                0
            },
        }
    }

    /// Check if the disk queue is at or near capacity
    /// Returns true if sending would likely fail due to size limit
    /// Uses 95% threshold to allow some buffer before hard limit
    pub fn is_at_capacity(&self) -> bool {
        if let Some(max_size) = self.max_size_bytes
            && self.file_path.exists()
            && let Ok(metadata) = std::fs::metadata(&self.file_path)
        {
            // Use 95% threshold to trigger backpressure before we hit the hard limit
            return metadata.len() >= (max_size * 95 / 100);
        }
        false
    }

    /// Get the queue name (for logging)
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Append a message to the disk file
    async fn append_to_disk(&self, message: T) -> Result<()> {
        use std::io::{Seek, Write};

        // Serialize message
        let data = bincode::serialize(&message)?;
        let data_len = data.len() as u32;

        // Calculate CRC32
        let checksum = crc32fast::hash(&data);

        // Open file in append mode
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false) // Don't truncate - we're appending
            .read(true)
            .write(true)
            .open(&self.file_path)?;

        // If file is empty, write header
        let file_size = file.metadata()?.len();
        if file_size == 0 {
            // Write header: MAGIC + write_offset + read_offset + reserved
            let header = [
                MAGIC_BYTES.as_slice(),
                &(HEADER_SIZE as u64).to_le_bytes(), // write_offset starts after header
                &(HEADER_SIZE as u64).to_le_bytes(), // read_offset starts after header
                &0u64.to_le_bytes(),                 // reserved
            ]
            .concat();
            file.write_all(&header)?;
        }

        // Check size limit
        if let Some(max_size) = self.max_size_bytes
            && file_size >= max_size
        {
            anyhow::bail!(
                "Queue file size limit exceeded: {}",
                self.file_path.display()
            );
        }

        // Seek to end
        file.seek(std::io::SeekFrom::End(0))?;

        // Write message: length + data + checksum
        file.write_all(&data_len.to_le_bytes())?;
        file.write_all(&data)?;
        file.write_all(&checksum.to_le_bytes())?;
        file.flush()?;

        // Update write offset in header
        let new_write_offset = file.metadata()?.len();
        file.seek(std::io::SeekFrom::Start(8))?;
        file.write_all(&new_write_offset.to_le_bytes())?;
        file.flush()?;

        Ok(())
    }

    /// Read a message from the disk file
    async fn read_from_disk(&self) -> Result<Option<T>> {
        use std::io::{Read, Seek, Write};

        if !self.file_path.exists() {
            return Ok(None);
        }

        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.file_path)?;

        // Read header
        let mut header = [0u8; HEADER_SIZE];
        file.read_exact(&mut header)?;

        // Verify magic bytes
        if &header[0..8] != MAGIC_BYTES {
            anyhow::bail!("Invalid queue file magic bytes");
        }

        // Read offsets
        let write_offset = u64::from_le_bytes(header[8..16].try_into()?);
        let read_offset = u64::from_le_bytes(header[16..24].try_into()?);

        // Check if we've read everything
        if read_offset >= write_offset {
            return Ok(None);
        }

        // Seek to read position
        file.seek(std::io::SeekFrom::Start(read_offset))?;

        // Read message length
        let mut len_bytes = [0u8; 4];
        file.read_exact(&mut len_bytes)?;
        let data_len = u32::from_le_bytes(len_bytes) as usize;

        // Read message data
        let mut data = vec![0u8; data_len];
        file.read_exact(&mut data)?;

        // Read checksum
        let mut checksum_bytes = [0u8; 4];
        file.read_exact(&mut checksum_bytes)?;
        let expected_checksum = u32::from_le_bytes(checksum_bytes);

        // Verify checksum
        let actual_checksum = crc32fast::hash(&data);
        if actual_checksum != expected_checksum {
            warn!("Checksum mismatch, skipping corrupted message");
            metrics::counter!(format!("queue.{}.corruption_total", self.name)).increment(1);
            // Skip to next message
            let new_read_offset = read_offset + 4 + data_len as u64 + 4;
            file.seek(std::io::SeekFrom::Start(16))?;
            file.write_all(&new_read_offset.to_le_bytes())?;
            file.flush()?;
            drop(file); // Drop file handle before recursive call
            return Box::pin(self.read_from_disk()).await;
        }

        // Deserialize message
        let message: T = bincode::deserialize(&data)?;

        // Calculate new read offset but DON'T write it yet
        // This ensures at-most-once delivery: message is only marked as consumed
        // after successful delivery (when commit() is called)
        let new_read_offset = read_offset + 4 + data_len as u64 + 4;

        // Store pending offset - will be committed after successful delivery
        {
            let mut pending = self.pending_commit_offset.write().await;
            *pending = Some(new_read_offset);
        }

        Ok(Some(message))
    }

    /// Finish draining (transition from Draining to Connected)
    async fn finish_drain(&self) -> Result<()> {
        let name = self.name.clone();

        // Delete the drained file
        if self.file_path.exists() {
            std::fs::remove_file(&self.file_path)?;
        }

        // Transition to Connected
        let mut state = self.state.write().await;
        let (drained, drain_start) = match &*state {
            QueueState::Draining {
                messages_drained,
                drain_started_at,
                ..
            } => (*messages_drained, *drain_started_at),
            _ => (0, Instant::now()),
        };

        let drain_duration = drain_start.elapsed().as_secs_f64();

        // Update state to Connected
        *state = QueueState::Connected {
            consumer_id: "drained".to_string(),
            connected_at: Instant::now(),
        };
        metrics::gauge!(format!("queue.{}.state", name)).set(1.0);

        metrics::histogram!(format!("queue.{}.drain_duration_seconds", name))
            .record(drain_duration);

        info!("Drained {} messages in {:.2}s", drained, drain_duration);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_basic_send_recv() {
        let temp_dir = TempDir::new().unwrap();
        let queue_path = temp_dir.path().join("test.queue");

        let queue =
            PersistentQueue::<String>::new("test".to_string(), queue_path.clone(), None, 10)
                .unwrap();

        // Connect consumer
        queue
            .connect_consumer("test-consumer".to_string())
            .await
            .unwrap();

        // Send and receive messages
        queue.send("hello".to_string()).await.unwrap();
        queue.send("world".to_string()).await.unwrap();

        let msg1 = queue.recv().await.unwrap();
        let msg2 = queue.recv().await.unwrap();

        assert_eq!(msg1, "hello");
        assert_eq!(msg2, "world");
    }

    #[tokio::test]
    async fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let queue_path = temp_dir.path().join("test.queue");

        // Create queue, send messages while disconnected
        {
            let queue =
                PersistentQueue::<String>::new("test".to_string(), queue_path.clone(), None, 10)
                    .unwrap();

            queue.send("message1".to_string()).await.unwrap();
            queue.send("message2".to_string()).await.unwrap();
            queue.send("message3".to_string()).await.unwrap();
        }

        // Create new queue instance (simulates restart)
        {
            let queue =
                PersistentQueue::<String>::new("test".to_string(), queue_path.clone(), None, 10)
                    .unwrap();

            // Connect and drain
            queue
                .connect_consumer("test-consumer".to_string())
                .await
                .unwrap();

            let msg1 = queue.recv().await.unwrap();
            queue.commit().await.unwrap(); // Commit after successful "delivery"
            let msg2 = queue.recv().await.unwrap();
            queue.commit().await.unwrap(); // Commit after successful "delivery"
            let msg3 = queue.recv().await.unwrap();
            queue.commit().await.unwrap(); // Commit after successful "delivery"

            assert_eq!(msg1, "message1");
            assert_eq!(msg2, "message2");
            assert_eq!(msg3, "message3");
        }
    }

    #[tokio::test]
    async fn test_drain_mode() {
        let temp_dir = TempDir::new().unwrap();
        let queue_path = temp_dir.path().join("test.queue");

        let queue = Arc::new(
            PersistentQueue::<String>::new("test".to_string(), queue_path.clone(), None, 100)
                .unwrap(),
        );

        // Buffer messages to disk
        for i in 0..10 {
            queue.send(format!("old-{}", i)).await.unwrap();
        }

        // Connect consumer (enters drain mode)
        queue
            .connect_consumer("test-consumer".to_string())
            .await
            .unwrap();

        // Send new messages while draining
        let queue_clone = queue.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            for i in 0..5 {
                queue_clone.send(format!("new-{}", i)).await.unwrap();
            }
        });

        // Receive messages - should get old ones first, then new ones
        let mut received = Vec::new();
        for _ in 0..15 {
            received.push(queue.recv().await.unwrap());
            queue.commit().await.unwrap(); // Commit after successful "delivery"
        }

        // Verify old messages come first
        for (i, item) in received.iter().enumerate().take(10) {
            assert_eq!(*item, format!("old-{}", i));
        }
        // New messages come after
        for i in 0..5 {
            assert_eq!(received[10 + i], format!("new-{}", i));
        }
    }

    #[tokio::test]
    async fn test_overflow_protection() {
        let temp_dir = TempDir::new().unwrap();
        let queue_path = temp_dir.path().join("test.queue");

        let queue = PersistentQueue::<String>::new(
            "test".to_string(),
            queue_path.clone(),
            Some(1024), // 1 KB limit
            10,
        )
        .unwrap();

        // Try to send messages until we hit the limit
        let mut sent = 0;
        loop {
            let large_message = "x".repeat(100);
            match queue.send(large_message).await {
                Ok(_) => sent += 1,
                Err(e) => {
                    assert!(e.to_string().contains("size limit exceeded"));
                    break;
                }
            }
        }

        assert!(sent > 0, "Should have sent at least some messages");
    }

    #[tokio::test]
    async fn test_binary_messages() {
        let temp_dir = TempDir::new().unwrap();
        let queue_path = temp_dir.path().join("test.queue");

        let queue =
            PersistentQueue::<Vec<u8>>::new("test".to_string(), queue_path, None, 10).unwrap();

        // Send binary data
        queue.send(vec![0x00, 0xFF, 0xAA, 0x55]).await.unwrap();
        queue.send(vec![0x12, 0x34, 0x56, 0x78]).await.unwrap();

        queue
            .connect_consumer("test-consumer".to_string())
            .await
            .unwrap();

        let msg1 = queue.recv().await.unwrap();
        queue.commit().await.unwrap(); // Commit after successful "delivery"
        let msg2 = queue.recv().await.unwrap();
        queue.commit().await.unwrap(); // Commit after successful "delivery"

        assert_eq!(msg1, vec![0x00, 0xFF, 0xAA, 0x55]);
        assert_eq!(msg2, vec![0x12, 0x34, 0x56, 0x78]);
    }

    #[tokio::test]
    async fn test_concurrent_send_recv() {
        let temp_dir = TempDir::new().unwrap();
        let queue_path = temp_dir.path().join("test.queue");

        let queue = Arc::new(
            PersistentQueue::<usize>::new("test".to_string(), queue_path, None, 1000).unwrap(),
        );

        queue
            .connect_consumer("test-consumer".to_string())
            .await
            .unwrap();

        // Spawn sender task
        let queue_send = queue.clone();
        let send_handle = tokio::spawn(async move {
            for i in 0..100 {
                queue_send.send(i).await.unwrap();
            }
        });

        // Spawn receiver task
        let queue_recv = queue.clone();
        let recv_handle = tokio::spawn(async move {
            let mut received = Vec::new();
            for _ in 0..100 {
                received.push(queue_recv.recv().await.unwrap());
            }
            received
        });

        send_handle.await.unwrap();
        let received = recv_handle.await.unwrap();

        // Verify all messages received in order
        assert_eq!(received.len(), 100);
        for (i, &msg) in received.iter().enumerate() {
            assert_eq!(msg, i);
        }
    }

    #[tokio::test]
    async fn test_state_transitions() {
        let temp_dir = TempDir::new().unwrap();
        let queue_path = temp_dir.path().join("test.queue");

        let queue =
            PersistentQueue::<String>::new("test".to_string(), queue_path.clone(), None, 10)
                .unwrap();

        // Initial state: Disconnected
        let state = queue.state.read().await.clone();
        assert!(matches!(state, QueueState::Disconnected { .. }));

        // Connect -> Connected (no backlog)
        queue
            .connect_consumer("test-consumer".to_string())
            .await
            .unwrap();
        let state = queue.state.read().await.clone();
        assert!(matches!(state, QueueState::Connected { .. }));

        // Disconnect -> Disconnected
        queue.disconnect_consumer().await.unwrap();
        let state = queue.state.read().await.clone();
        assert!(matches!(state, QueueState::Disconnected { .. }));

        // Buffer message while disconnected
        queue.send("buffered".to_string()).await.unwrap();

        // Connect -> Draining (has backlog)
        queue
            .connect_consumer("test-consumer".to_string())
            .await
            .unwrap();
        let state = queue.state.read().await.clone();
        assert!(matches!(state, QueueState::Draining { .. }));

        // Drain message
        let msg = queue.recv().await.unwrap();
        queue.commit().await.unwrap(); // Commit after successful "delivery"
        assert_eq!(msg, "buffered");
    }

    #[tokio::test]
    async fn test_depth() {
        let temp_dir = TempDir::new().unwrap();
        let queue_path = temp_dir.path().join("test.queue");

        let queue =
            PersistentQueue::<String>::new("test".to_string(), queue_path, None, 10).unwrap();

        queue
            .connect_consumer("test-consumer".to_string())
            .await
            .unwrap();

        // Send messages
        queue.send("msg1".to_string()).await.unwrap();
        queue.send("msg2".to_string()).await.unwrap();
        queue.send("msg3".to_string()).await.unwrap();

        let depth = queue.depth().await;
        assert_eq!(depth.memory, 3);

        // Receive one
        queue.recv().await.unwrap();
        queue.commit().await.unwrap(); // Commit after successful "delivery"

        let depth = queue.depth().await;
        assert_eq!(depth.memory, 2);
    }
}
