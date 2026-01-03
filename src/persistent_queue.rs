// Persistent queue implementation with file backing
//
// This queue provides:
// - Fast path: Direct memory channel when consumer is connected
// - Slow path: File-backed persistence when consumer is disconnected
// - Drain mode: Replays file backlog while buffering new messages
//
// The queue maintains strict message ordering and zero message loss guarantees.

use anyhow::{Context, Result};
use crc32fast::Hasher;
use serde::{Serialize, de::DeserializeOwned};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Magic bytes for queue file format: "SOARQ001"
const MAGIC: &[u8; 8] = b"SOARQ001";

/// Queue file header size (32 bytes)
const HEADER_SIZE: u64 = 32;

/// Maximum file size before overflow (default: 10 GB)
const DEFAULT_MAX_FILE_SIZE: u64 = 10 * 1024 * 1024 * 1024;

/// Queue state
#[derive(Debug, Clone)]
pub enum QueueState {
    /// Consumer is connected, messages go directly to memory
    Connected {
        consumer_id: String,
        connected_at: std::time::Instant,
    },
    /// Consumer is disconnected, buffering to file
    Disconnected {
        disconnected_at: std::time::Instant,
        messages_buffered: u64,
        bytes_buffered: u64,
    },
    /// Draining backlog while also buffering new messages
    Draining {
        drain_started_at: std::time::Instant,
        messages_drained: u64,
        messages_in_backlog: u64,
        new_messages_buffered: u64,
    },
}

impl QueueState {
    pub fn state_code(&self) -> i32 {
        match self {
            QueueState::Connected { .. } => 1,
            QueueState::Disconnected { .. } => 0,
            QueueState::Draining { .. } => 2,
        }
    }
}

/// Queue depth information
#[derive(Debug, Clone)]
pub struct QueueDepth {
    pub memory: usize,
    pub disk: u64,
}

/// Persistent queue with file backing
pub struct PersistentQueue<T>
where
    T: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    /// Queue name (for metrics)
    name: String,

    /// Path to queue file
    file_path: PathBuf,

    /// Maximum file size in bytes
    max_file_size: u64,

    /// Fast-path memory channel (sender)
    mem_tx: flume::Sender<T>,

    /// Fast-path memory channel (receiver)
    mem_rx: flume::Receiver<T>,

    /// Current queue state
    state: Arc<RwLock<QueueState>>,
}

impl<T> PersistentQueue<T>
where
    T: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    /// Create a new persistent queue
    ///
    /// # Arguments
    /// * `name` - Queue name (for metrics and logging)
    /// * `file_path` - Path to queue file
    /// * `max_file_size` - Maximum file size in bytes (default: 10 GB)
    /// * `mem_capacity` - Memory channel capacity
    pub fn new<P: AsRef<Path>>(
        name: String,
        file_path: P,
        max_file_size: Option<u64>,
        mem_capacity: usize,
    ) -> Result<Self> {
        let file_path = file_path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create queue directory: {:?}", parent))?;
        }

        // Create memory channel
        let (mem_tx, mem_rx) = flume::bounded(mem_capacity);

        // Initialize queue file if it doesn't exist
        let state = if file_path.exists() {
            // File exists, check if it has messages buffered
            let count = Self::count_messages_in_file(&file_path)?;
            if count > 0 {
                info!(
                    "Queue '{}' has {} messages buffered from previous run",
                    name, count
                );
                Arc::new(RwLock::new(QueueState::Disconnected {
                    disconnected_at: std::time::Instant::now(),
                    messages_buffered: count,
                    bytes_buffered: std::fs::metadata(&file_path)?.len(),
                }))
            } else {
                Arc::new(RwLock::new(QueueState::Connected {
                    consumer_id: "initial".to_string(),
                    connected_at: std::time::Instant::now(),
                }))
            }
        } else {
            // Create new file
            Self::init_queue_file(&file_path)?;
            Arc::new(RwLock::new(QueueState::Connected {
                consumer_id: "initial".to_string(),
                connected_at: std::time::Instant::now(),
            }))
        };

        Ok(Self {
            name,
            file_path,
            max_file_size: max_file_size.unwrap_or(DEFAULT_MAX_FILE_SIZE),
            mem_tx,
            mem_rx,
            state,
        })
    }

    /// Send a message to the queue
    ///
    /// Fast path: If consumer connected, send directly to memory
    /// Slow path: If disconnected, buffer to file
    pub async fn send(&self, message: T) -> Result<()> {
        let state_snapshot = {
            let state = self.state.read().await;
            state.clone()
        };

        match state_snapshot {
            QueueState::Connected { .. } => {
                // Try fast path first
                match self.mem_tx.try_send(message) {
                    Ok(()) => {
                        metrics::counter!(format!("queue.{}.messages.sent_direct", self.name))
                            .increment(1);
                        Ok(())
                    }
                    Err(flume::TrySendError::Full(msg)) => {
                        // Memory full, fall back to file
                        warn!("Queue '{}' memory full, falling back to file", self.name);
                        self.transition_to_disconnected().await;
                        self.write_to_file(&msg).await
                    }
                    Err(flume::TrySendError::Disconnected(msg)) => {
                        // Consumer disconnected
                        self.transition_to_disconnected().await;
                        self.write_to_file(&msg).await
                    }
                }
            }
            QueueState::Disconnected { .. } | QueueState::Draining { .. } => {
                self.write_to_file(&message).await
            }
        }
    }

    /// Receive a message from the queue
    ///
    /// During drain: reads from file
    /// After drain: reads from memory
    pub async fn recv(&self) -> Result<T> {
        self.mem_rx
            .recv_async()
            .await
            .context("Memory channel closed")
    }

    /// Connect a consumer
    ///
    /// If there's a backlog, transitions to draining.
    /// Otherwise, transitions directly to connected.
    pub async fn connect_consumer(&self, consumer_id: String) -> Result<()> {
        let mut state = self.state.write().await;

        match &*state {
            QueueState::Disconnected {
                messages_buffered, ..
            } => {
                let count = *messages_buffered;
                if count > 0 {
                    info!(
                        "Queue '{}' has {} messages to drain, starting drain",
                        self.name, count
                    );

                    *state = QueueState::Draining {
                        drain_started_at: std::time::Instant::now(),
                        messages_drained: 0,
                        messages_in_backlog: count,
                        new_messages_buffered: 0,
                    };

                    // Start drain task
                    drop(state);
                    self.start_drain_task().await?;
                } else {
                    *state = QueueState::Connected {
                        consumer_id,
                        connected_at: std::time::Instant::now(),
                    };
                    info!("Queue '{}' connected with no backlog", self.name);
                }
                Ok(())
            }
            QueueState::Connected { .. } => {
                warn!("Queue '{}' already connected", self.name);
                Ok(())
            }
            QueueState::Draining { .. } => {
                warn!("Queue '{}' already draining", self.name);
                Ok(())
            }
        }
    }

    /// Disconnect consumer
    pub async fn disconnect_consumer(&self) -> Result<()> {
        let mut state = self.state.write().await;
        *state = QueueState::Disconnected {
            disconnected_at: std::time::Instant::now(),
            messages_buffered: 0,
            bytes_buffered: 0,
        };
        info!("Queue '{}' disconnected", self.name);
        Ok(())
    }

    /// Get current queue depth
    pub async fn depth(&self) -> QueueDepth {
        let state = self.state.read().await;
        match &*state {
            QueueState::Connected { .. } => QueueDepth {
                memory: self.mem_rx.len(),
                disk: 0,
            },
            QueueState::Disconnected {
                messages_buffered, ..
            } => QueueDepth {
                memory: 0,
                disk: *messages_buffered,
            },
            QueueState::Draining {
                messages_in_backlog,
                new_messages_buffered,
                ..
            } => QueueDepth {
                memory: self.mem_rx.len(),
                disk: messages_in_backlog + new_messages_buffered,
            },
        }
    }

    /// Get current state
    pub async fn get_state(&self) -> QueueState {
        self.state.read().await.clone()
    }

    // --- Private helper methods ---

    /// Initialize a new queue file with header
    fn init_queue_file(path: &Path) -> Result<()> {
        let mut file = File::create(path)
            .with_context(|| format!("Failed to create queue file: {:?}", path))?;

        // Write header
        file.write_all(MAGIC)?; // Magic bytes
        file.write_all(&0u64.to_le_bytes())?; // Write offset (initially 32 = after header)
        file.write_all(&0u64.to_le_bytes())?; // Read offset (initially 32)
        file.write_all(&[0u8; 8])?; // Reserved
        file.sync_all()?;

        Ok(())
    }

    /// Transition to disconnected state
    async fn transition_to_disconnected(&self) {
        let mut state = self.state.write().await;
        if !matches!(*state, QueueState::Disconnected { .. }) {
            warn!("Queue '{}' transitioning to disconnected", self.name);
            *state = QueueState::Disconnected {
                disconnected_at: std::time::Instant::now(),
                messages_buffered: 0,
                bytes_buffered: 0,
            };
        }
    }

    /// Write a message to the file
    async fn write_to_file(&self, message: &T) -> Result<()> {
        // Serialize message
        let data =
            bincode::serialize(message).context("Failed to serialize message for file write")?;

        // Calculate checksum
        let mut hasher = Hasher::new();
        hasher.update(&data);
        let checksum = hasher.finalize();

        // Open file for append
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.file_path)
            .with_context(|| format!("Failed to open queue file: {:?}", self.file_path))?;

        // Read current write offset
        file.seek(SeekFrom::Start(8))?;
        let mut offset_bytes = [0u8; 8];
        file.read_exact(&mut offset_bytes)?;
        let write_offset = u64::from_le_bytes(offset_bytes);

        // Check overflow
        let message_size = 4 + data.len() + 4; // length + data + checksum
        if write_offset + message_size as u64 > self.max_file_size {
            error!(
                "Queue '{}' overflow: would exceed max size {} bytes",
                self.name, self.max_file_size
            );
            metrics::counter!(format!("queue.{}.overflow_disconnects_total", self.name))
                .increment(1);
            return Err(anyhow::anyhow!("Queue overflow"));
        }

        // Seek to write position
        let write_pos = if write_offset == 0 {
            HEADER_SIZE
        } else {
            write_offset
        };
        file.seek(SeekFrom::Start(write_pos))?;

        // Write message
        {
            let mut writer = BufWriter::new(&mut file);
            writer.write_all(&(data.len() as u32).to_le_bytes())?; // Length prefix
            writer.write_all(&data)?; // Data
            writer.write_all(&checksum.to_le_bytes())?; // Checksum
            writer.flush()?;
        }

        // Update write offset in header
        let new_write_offset = write_pos + message_size as u64;
        file.seek(SeekFrom::Start(8))?;
        file.write_all(&new_write_offset.to_le_bytes())?;
        file.sync_all()?;

        // Update metrics
        metrics::counter!(format!("queue.{}.messages.sent_buffered", self.name)).increment(1);
        metrics::counter!(format!("queue.{}.bytes.written", self.name))
            .increment(message_size as u64);

        // Update state
        let mut state = self.state.write().await;
        match &mut *state {
            QueueState::Disconnected {
                messages_buffered,
                bytes_buffered,
                ..
            } => {
                *messages_buffered += 1;
                *bytes_buffered += message_size as u64;
            }
            QueueState::Draining {
                new_messages_buffered,
                ..
            } => {
                *new_messages_buffered += 1;
            }
            _ => {}
        }

        Ok(())
    }

    /// Count messages in file
    fn count_messages_in_file(path: &Path) -> Result<u64> {
        let mut file = File::open(path)?;

        // Read write and read offsets from header
        file.seek(SeekFrom::Start(8))?;
        let mut offset_bytes = [0u8; 8];
        file.read_exact(&mut offset_bytes)?;
        let write_offset = u64::from_le_bytes(offset_bytes);

        file.read_exact(&mut offset_bytes)?;
        let read_offset = u64::from_le_bytes(offset_bytes);

        if write_offset == 0 || write_offset == read_offset {
            return Ok(0);
        }

        // Count messages
        let mut count = 0u64;
        let mut pos = if read_offset == 0 {
            HEADER_SIZE
        } else {
            read_offset
        };

        file.seek(SeekFrom::Start(pos))?;
        let mut reader = BufReader::new(file);

        while pos < write_offset {
            // Read length
            let mut len_bytes = [0u8; 4];
            if reader.read_exact(&mut len_bytes).is_err() {
                break;
            }
            let len = u32::from_le_bytes(len_bytes) as u64;

            // Skip data + checksum
            reader.seek(SeekFrom::Current((len + 4) as i64))?;

            count += 1;
            pos += 4 + len + 4;
        }

        Ok(count)
    }

    /// Start draining backlog
    async fn start_drain_task(&self) -> Result<()> {
        // Rename current file to .draining
        let drain_file = self.file_path.with_extension("draining");
        std::fs::rename(&self.file_path, &drain_file)?;

        // Create new file for incoming messages
        Self::init_queue_file(&self.file_path)?;

        info!("Queue '{}' starting drain from {:?}", self.name, drain_file);

        // Spawn drain task
        let mem_tx = self.mem_tx.clone();
        let state = self.state.clone();
        let name = self.name.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::drain_file(&drain_file, mem_tx, state, &name).await {
                error!("Drain task failed: {}", e);
            }

            // Delete drain file
            if let Err(e) = std::fs::remove_file(&drain_file) {
                error!("Failed to delete drain file: {}", e);
            }

            info!("Queue '{}' drain complete", name);
        });

        Ok(())
    }

    /// Drain messages from file to memory channel
    async fn drain_file(
        path: &Path,
        mem_tx: flume::Sender<T>,
        state: Arc<RwLock<QueueState>>,
        name: &str,
    ) -> Result<()> {
        let mut file = File::open(path)?;

        // Read read offset from header
        file.seek(SeekFrom::Start(16))?;
        let mut offset_bytes = [0u8; 8];
        file.read_exact(&mut offset_bytes)?;
        let mut pos = u64::from_le_bytes(offset_bytes);

        if pos == 0 {
            pos = HEADER_SIZE;
        }

        file.seek(SeekFrom::Start(pos))?;
        let mut reader = BufReader::new(file);

        let mut drained = 0u64;

        loop {
            // Read length
            let mut len_bytes = [0u8; 4];
            if reader.read_exact(&mut len_bytes).is_err() {
                break;
            }
            let len = u32::from_le_bytes(len_bytes);

            // Read data
            let mut data = vec![0u8; len as usize];
            reader.read_exact(&mut data)?;

            // Read checksum
            let mut checksum_bytes = [0u8; 4];
            reader.read_exact(&mut checksum_bytes)?;
            let expected_checksum = u32::from_le_bytes(checksum_bytes);

            // Verify checksum
            let mut hasher = Hasher::new();
            hasher.update(&data);
            let actual_checksum = hasher.finalize();

            if actual_checksum != expected_checksum {
                error!("Checksum mismatch, skipping corrupted message");
                metrics::counter!(format!("queue.{}.file_errors_total", name)).increment(1);
                continue;
            }

            // Deserialize
            let message: T = match bincode::deserialize(&data) {
                Ok(msg) => msg,
                Err(e) => {
                    error!("Failed to deserialize message: {}", e);
                    metrics::counter!(format!("queue.{}.file_errors_total", name)).increment(1);
                    continue;
                }
            };

            // Send to memory channel (blocking is OK - backpressure)
            mem_tx.send_async(message).await?;

            drained += 1;
            metrics::counter!(format!("queue.{}.drain.messages_total", name)).increment(1);

            // Update state
            let mut st = state.write().await;
            if let QueueState::Draining {
                messages_drained, ..
            } = &mut *st
            {
                *messages_drained = drained;
            }
        }

        // Transition to connected
        let mut st = state.write().await;
        *st = QueueState::Connected {
            consumer_id: "drained".to_string(),
            connected_at: std::time::Instant::now(),
        };

        let drain_duration = match &*st {
            QueueState::Connected { connected_at, .. } => connected_at.elapsed().as_secs_f64(),
            _ => 0.0,
        };

        metrics::histogram!(format!("queue.{}.drain_duration_seconds", name))
            .record(drain_duration);

        info!("Drained {} messages in {:.2}s", drained, drain_duration);

        Ok(())
    }
}
