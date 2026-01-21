use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

const MAGIC_BYTES: &[u8; 8] = b"SOARQUE1";
const HEADER_SIZE: usize = 32;
const DEFAULT_MAX_SEGMENT_SIZE: u64 = 10 * 1024 * 1024; // 10 MB

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
    /// Actual data bytes remaining across all segments
    pub disk_data_bytes: u64,
    /// Total file size on disk across all segments
    pub disk_file_bytes: u64,
}

/// A persistent file-backed queue with fast-path memory optimization
///
/// States:
/// - Connected: Messages go directly through memory channel (fast path)
/// - Disconnected: Messages buffer to disk file (slow path)
/// - Draining: Replay disk backlog while buffering new messages
///
/// Storage:
/// - Uses a directory containing segment files
/// - Each segment file is named with a 16-digit zero-padded unix timestamp (milliseconds)
/// - Segment files are rotated when they exceed max_segment_size_bytes
/// - Segments are deleted after being fully drained
/// - This limits deadspace to at most one segment's worth
///
/// Delivery Semantics:
/// - At-most-once: Messages are marked as consumed only after successful delivery
/// - If crash occurs after recv() but before commit(), message will be replayed
pub struct PersistentQueue<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    name: String,
    dir_path: PathBuf,
    max_total_size_bytes: Option<u64>,
    max_segment_size_bytes: u64,
    state: Arc<RwLock<QueueState>>,
    memory_tx: flume::Sender<T>,
    memory_rx: flume::Receiver<T>,
    /// Pending read offset - set when message is read, committed when delivery succeeds
    /// This ensures at-most-once delivery: offset only advances after successful send
    /// Format: (segment_name, new_read_offset)
    pending_commit: Arc<RwLock<Option<(String, u64)>>>,
    /// Track whether we've logged the overflow warning (to avoid spamming logs)
    /// Set to true on first overflow, reset to false after drain completes
    overflow_warned: Arc<AtomicBool>,
    /// Cached data size for is_at_capacity() - avoids expensive recalculation on every call
    /// Value is the data size in bytes
    cached_data_size: Arc<AtomicU64>,
    /// Timestamp (millis since queue creation) when cached_data_size was last updated
    cached_data_size_time_ms: Arc<AtomicU64>,
    /// Instant when the queue was created (for calculating cache age)
    created_at: Instant,
}

impl<T> PersistentQueue<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    /// Create a new persistent queue
    ///
    /// # Arguments
    /// * `name` - Queue name (for metrics)
    /// * `dir_path` - Path to persistent queue directory
    /// * `max_total_size_bytes` - Optional maximum total size across all segments (disconnect on overflow)
    /// * `memory_capacity` - Bounded channel capacity for fast path
    pub fn new(
        name: String,
        dir_path: PathBuf,
        max_total_size_bytes: Option<u64>,
        memory_capacity: usize,
    ) -> Result<Self> {
        Self::with_segment_size(
            name,
            dir_path,
            max_total_size_bytes,
            memory_capacity,
            DEFAULT_MAX_SEGMENT_SIZE,
        )
    }

    /// Create a new persistent queue with custom segment size
    ///
    /// # Arguments
    /// * `name` - Queue name (for metrics)
    /// * `dir_path` - Path to persistent queue directory
    /// * `max_total_size_bytes` - Optional maximum total size across all segments
    /// * `memory_capacity` - Bounded channel capacity for fast path
    /// * `max_segment_size_bytes` - Maximum size of each segment file before rotation
    pub fn with_segment_size(
        name: String,
        dir_path: PathBuf,
        max_total_size_bytes: Option<u64>,
        memory_capacity: usize,
        max_segment_size_bytes: u64,
    ) -> Result<Self> {
        // Create directory if needed
        std::fs::create_dir_all(&dir_path)?;

        let (memory_tx, memory_rx) = flume::bounded(memory_capacity);

        let created_at = Instant::now();
        let queue = Self {
            name: name.clone(),
            dir_path,
            max_total_size_bytes,
            max_segment_size_bytes,
            state: Arc::new(RwLock::new(QueueState::Disconnected {
                disconnected_at: Instant::now(),
                messages_buffered: 0,
                bytes_buffered: 0,
            })),
            memory_tx,
            memory_rx,
            pending_commit: Arc::new(RwLock::new(None)),
            overflow_warned: Arc::new(AtomicBool::new(false)),
            cached_data_size: Arc::new(AtomicU64::new(0)),
            cached_data_size_time_ms: Arc::new(AtomicU64::new(0)),
            created_at,
        };

        // Initialize metrics
        metrics::gauge!(format!("queue.{}.state", name)).set(0.0); // 0=disconnected, 1=connected, 2=draining

        Ok(queue)
    }

    /// List segment files sorted by name (oldest first)
    fn list_segments(&self) -> Vec<String> {
        let mut segments = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.dir_path) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                // Segment files are 16-digit numbers
                if name.len() == 16 && name.chars().all(|c| c.is_ascii_digit()) {
                    segments.push(name);
                }
            }
        }

        // Sort asciibetically (works because we use zero-padded numbers)
        segments.sort();
        segments
    }

    /// Get path to a segment file
    fn segment_path(&self, segment_name: &str) -> PathBuf {
        self.dir_path.join(segment_name)
    }

    /// Generate a new segment name based on current timestamp
    fn new_segment_name(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        // 16-digit zero-padded timestamp
        format!("{:016}", timestamp)
    }

    /// Get the current write segment (create if needed)
    fn get_or_create_write_segment(&self) -> Result<String> {
        let segments = self.list_segments();

        if let Some(latest) = segments.last() {
            // Check if current segment is under size limit
            let path = self.segment_path(latest);
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

            if size < self.max_segment_size_bytes {
                return Ok(latest.clone());
            }
            // Current segment is full, create new one
        }

        // Create new segment
        let name = self.new_segment_name();
        Ok(name)
    }

    /// Get the oldest segment for reading (returns None if no segments)
    fn get_read_segment(&self) -> Option<String> {
        self.list_segments().first().cloned()
    }

    /// Read the offsets from a segment file header
    fn read_segment_offsets(&self, segment_name: &str) -> Option<(u64, u64)> {
        use std::io::Read;

        let path = self.segment_path(segment_name);
        if !path.exists() {
            return None;
        }

        let mut file = std::fs::File::open(&path).ok()?;
        let mut header = [0u8; HEADER_SIZE];
        file.read_exact(&mut header).ok()?;

        // Verify magic bytes
        if &header[0..8] != MAGIC_BYTES {
            return None;
        }

        let write_offset = u64::from_le_bytes(header[8..16].try_into().ok()?);
        let read_offset = u64::from_le_bytes(header[16..24].try_into().ok()?);

        Some((write_offset, read_offset))
    }

    /// Check if a segment is fully drained
    fn is_segment_drained(&self, segment_name: &str) -> bool {
        if let Some((write_offset, read_offset)) = self.read_segment_offsets(segment_name) {
            read_offset >= write_offset
        } else {
            true // Invalid or missing segment is considered drained
        }
    }

    /// Delete a segment file
    fn delete_segment(&self, segment_name: &str) -> Result<()> {
        let path = self.segment_path(segment_name);
        if path.exists() {
            std::fs::remove_file(&path)?;
            debug!("Deleted drained segment: {}", segment_name);
            metrics::counter!(format!("queue.{}.segments_deleted_total", self.name)).increment(1);
        }
        Ok(())
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
                        // Only log warning once per overflow episode (not on every send)
                        if !self.overflow_warned.swap(true, Ordering::Relaxed) {
                            warn!(
                                "Queue {} memory channel full, overflowing to disk (publisher is slow)",
                                self.name
                            );
                        }
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
    /// Auto-connects on first call if disconnected. This ensures messages are
    /// buffered to disk until a consumer actually starts receiving, preventing
    /// message loss if the process restarts before consumption begins.
    ///
    /// In Draining mode, reads from disk backlog first, then switches to memory.
    pub async fn recv(&self) -> Result<T> {
        // Auto-connect on first recv if disconnected
        {
            let state = self.state.read().await;
            if matches!(*state, QueueState::Disconnected { .. }) {
                drop(state); // Release read lock before acquiring write lock
                self.auto_connect().await?;
            }
        }

        let state = { self.state.read().await.clone() };

        match state {
            QueueState::Connected { .. } => {
                // Check if there's disk overflow (memory was full and messages went to disk)
                // This can happen when the publisher is slow/blocked
                if self.has_segments() {
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
                // This shouldn't happen after auto_connect, but handle gracefully
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

        // Get pending commit info
        let commit_info = {
            let mut pending = self.pending_commit.write().await;
            pending.take()
        };

        if let Some((segment_name, new_read_offset)) = commit_info {
            let path = self.segment_path(&segment_name);

            // Check if segment still exists (might have been deleted in a race)
            if !path.exists() {
                return Ok(());
            }

            // Get write_offset to check if segment is fully drained
            let (write_offset, _) = self
                .read_segment_offsets(&segment_name)
                .ok_or_else(|| anyhow::anyhow!("Failed to read segment offsets"))?;

            // If this commit drains the segment, we may be able to delete it
            if new_read_offset >= write_offset {
                // Only delete if this is NOT the newest segment (the write segment).
                // If it's the newest, concurrent appends may be writing to it, and
                // we'd race with append_to_disk() which writes data before updating
                // the header's write_offset. Deleting based on stale write_offset
                // would drop those concurrent writes.
                let segments = self.list_segments();
                let is_write_segment = segments.last().map(|s| s == &segment_name).unwrap_or(true);

                if is_write_segment {
                    // Can't delete - this is the current write segment.
                    // Just update read_offset to mark it as empty.
                    let mut file = std::fs::OpenOptions::new().write(true).open(&path)?;
                    file.seek(std::io::SeekFrom::Start(16))?;
                    file.write_all(&new_read_offset.to_le_bytes())?;
                    file.sync_all()?;
                } else {
                    // Safe to delete - writes are going to newer segments
                    self.delete_segment(&segment_name)?;
                }
            } else {
                // Write new read_offset to disk
                let mut file = std::fs::OpenOptions::new().write(true).open(&path)?;

                file.seek(std::io::SeekFrom::Start(16))?;
                file.write_all(&new_read_offset.to_le_bytes())?;
                file.sync_all()?;
            }

            metrics::counter!(format!("queue.{}.messages.committed_total", self.name)).increment(1);

            // Update messages_drained counter if in Draining state
            let mut state = self.state.write().await;
            if let QueueState::Draining {
                messages_drained, ..
            } = &mut *state
            {
                *messages_drained += 1;
            }
        }

        Ok(())
    }

    /// Connect a consumer (transition to Connected or Draining state)
    ///
    /// Note: Prefer letting `recv()` auto-connect rather than calling this directly.
    /// This ensures messages are buffered to disk until consumption actually begins.
    pub async fn connect_consumer(&self, consumer_id: String) -> Result<()> {
        self.do_connect(consumer_id).await
    }

    /// Auto-connect when first recv() is called
    ///
    /// This is called internally by recv() to transition from Disconnected state.
    /// By deferring connection until recv(), we ensure all messages are persisted
    /// to disk until a consumer is actually ready to process them.
    async fn auto_connect(&self) -> Result<()> {
        self.do_connect("auto".to_string()).await
    }

    /// Internal connection logic shared by connect_consumer and auto_connect
    async fn do_connect(&self, consumer_id: String) -> Result<()> {
        let mut state = self.state.write().await;

        // Check if there's a disk backlog
        let has_backlog = self.has_segments();

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
            disk_data_bytes: self.data_size_bytes(),
            disk_file_bytes: self.total_file_size_bytes(),
        }
    }

    /// Check if there are any segment files
    fn has_segments(&self) -> bool {
        !self.list_segments().is_empty()
    }

    /// Get the actual data size in the queue (bytes not yet consumed) across all segments
    pub fn data_size_bytes(&self) -> u64 {
        let mut total = 0u64;

        for segment in self.list_segments() {
            if let Some((write_offset, read_offset)) = self.read_segment_offsets(&segment) {
                total += write_offset.saturating_sub(read_offset);
            }
        }

        total
    }

    /// Get the total file size across all segments
    pub fn total_file_size_bytes(&self) -> u64 {
        let mut total = 0u64;

        for segment in self.list_segments() {
            let path = self.segment_path(&segment);
            if let Ok(meta) = std::fs::metadata(&path) {
                total += meta.len();
            }
        }

        total
    }

    /// Get data size with caching to avoid expensive filesystem operations
    /// Cache is refreshed at most every 250ms
    fn cached_data_size_bytes(&self) -> u64 {
        const CACHE_TTL_MS: u64 = 250; // Refresh at most every 250ms

        let now_ms = self.created_at.elapsed().as_millis() as u64;
        let cached_time = self.cached_data_size_time_ms.load(Ordering::Relaxed);

        // Check if cache is still valid
        if now_ms.saturating_sub(cached_time) < CACHE_TTL_MS {
            return self.cached_data_size.load(Ordering::Relaxed);
        }

        // Cache expired, recalculate
        let data_size = self.data_size_bytes();

        // Update cache (relaxed ordering is fine - we don't need strict consistency)
        self.cached_data_size.store(data_size, Ordering::Relaxed);
        self.cached_data_size_time_ms
            .store(now_ms, Ordering::Relaxed);

        data_size
    }

    /// Check if the disk queue is at or near capacity
    /// Returns true if sending would likely fail due to size limit
    /// Uses 95% threshold on actual data size to trigger backpressure
    /// Uses cached data size to avoid expensive filesystem operations on every call
    pub fn is_at_capacity(&self) -> bool {
        if let Some(max_size) = self.max_total_size_bytes {
            let data_size = self.cached_data_size_bytes();
            // Backpressure when actual data is >= 95% of max
            if data_size >= (max_size * 95 / 100) {
                return true;
            }
        }
        false
    }

    /// Check if the queue is ready to accept new connections
    /// Returns true if below 75% capacity (allows reconnection after backpressure)
    /// Uses cached data size to avoid expensive filesystem operations
    pub fn is_ready_for_connection(&self) -> bool {
        if let Some(max_size) = self.max_total_size_bytes {
            let data_size = self.cached_data_size_bytes();

            // Allow reconnection when actual data is below 75% of max
            if data_size < (max_size * 75 / 100) {
                return true;
            }
            return false;
        }
        // If no max size or no segments, always ready
        true
    }

    /// Get current capacity usage as a percentage (0-100)
    /// Based on actual data size, not file size
    pub fn capacity_percent(&self) -> u8 {
        if let Some(max_size) = self.max_total_size_bytes {
            let data_size = self.data_size_bytes();
            return ((data_size * 100) / max_size) as u8;
        }
        0
    }

    /// Get the queue name (for logging)
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the number of segment files
    pub fn segment_count(&self) -> usize {
        self.list_segments().len()
    }

    /// Append a message to the disk file
    async fn append_to_disk(&self, message: T) -> Result<()> {
        use std::io::{Seek, Write};

        // Serialize message
        let data = bincode::serialize(&message)?;
        let data_len = data.len() as u32;

        // Calculate CRC32
        let checksum = crc32fast::hash(&data);

        // Message size on disk: 4 bytes length + data + 4 bytes checksum
        let message_disk_size = 4 + data.len() as u64 + 4;

        // Get or create write segment
        let segment_name = self.get_or_create_write_segment()?;
        let path = self.segment_path(&segment_name);

        // Check total size limit
        if let Some(max_size) = self.max_total_size_bytes {
            let current_size = self.total_file_size_bytes();
            if current_size >= max_size {
                anyhow::bail!(
                    "Queue total size limit exceeded: {}",
                    self.dir_path.display()
                );
            }
        }

        // Open file in append mode
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false) // Don't truncate - we're appending
            .read(true)
            .write(true)
            .open(&path)?;

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
            metrics::counter!(format!("queue.{}.segments_created_total", self.name)).increment(1);
        }

        // Check if this message would exceed segment size
        let current_file_size = file.metadata()?.len();
        if current_file_size > 0
            && current_file_size + message_disk_size > self.max_segment_size_bytes
        {
            // Need to rotate to a new segment
            drop(file);

            // Wait a moment to ensure unique timestamp
            std::thread::sleep(std::time::Duration::from_millis(1));

            let new_segment = self.new_segment_name();
            let new_path = self.segment_path(&new_segment);

            let mut new_file = std::fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(&new_path)?;

            // Write header
            let header = [
                MAGIC_BYTES.as_slice(),
                &(HEADER_SIZE as u64).to_le_bytes(),
                &(HEADER_SIZE as u64).to_le_bytes(),
                &0u64.to_le_bytes(),
            ]
            .concat();
            new_file.write_all(&header)?;
            metrics::counter!(format!("queue.{}.segments_created_total", self.name)).increment(1);

            file = new_file;
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

    /// Read a message from the disk file (from oldest segment)
    async fn read_from_disk(&self) -> Result<Option<T>> {
        use std::io::{Read, Seek, Write};

        // Get the oldest segment
        let segment_name = match self.get_read_segment() {
            Some(name) => name,
            None => return Ok(None),
        };

        let path = self.segment_path(&segment_name);
        if !path.exists() {
            return Ok(None);
        }

        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)?;

        // Read header
        let mut header = [0u8; HEADER_SIZE];
        file.read_exact(&mut header)?;

        // Verify magic bytes
        if &header[0..8] != MAGIC_BYTES {
            anyhow::bail!("Invalid queue file magic bytes in segment {}", segment_name);
        }

        // Read offsets
        let write_offset = u64::from_le_bytes(header[8..16].try_into()?);
        let read_offset = u64::from_le_bytes(header[16..24].try_into()?);

        // Check if we've read everything in this segment
        if read_offset >= write_offset {
            // Segment is drained - only delete if it's NOT the write segment
            // to avoid race with concurrent appends (same fix as in commit())
            drop(file);
            let segments = self.list_segments();
            let is_write_segment = segments.last().map(|s| s == &segment_name).unwrap_or(true);

            if !is_write_segment {
                // Safe to delete - not the write segment
                self.delete_segment(&segment_name)?;
                // Try to read from the next segment
                return Box::pin(self.read_from_disk()).await;
            } else {
                // This is the write segment - don't delete, just return None
                // to indicate no messages available from disk
                return Ok(None);
            }
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
            warn!(
                "Checksum mismatch in segment {}, skipping corrupted message",
                segment_name
            );
            metrics::counter!(format!("queue.{}.corruption_total", self.name)).increment(1);
            // Skip to next message
            let new_read_offset = read_offset + 4 + data_len as u64 + 4;
            file.seek(std::io::SeekFrom::Start(16))?;
            file.write_all(&new_read_offset.to_le_bytes())?;
            file.flush()?;
            drop(file);
            return Box::pin(self.read_from_disk()).await;
        }

        // Deserialize message
        let message: T = bincode::deserialize(&data)?;

        // Calculate new read offset but DON'T write it yet
        // This ensures at-most-once delivery: message is only marked as consumed
        // after successful delivery (when commit() is called)
        let new_read_offset = read_offset + 4 + data_len as u64 + 4;

        // Store pending commit info - will be committed after successful delivery
        {
            let mut pending = self.pending_commit.write().await;
            *pending = Some((segment_name, new_read_offset));
        }

        Ok(Some(message))
    }

    /// Finish draining (transition from Draining to Connected)
    async fn finish_drain(&self) -> Result<()> {
        let name = self.name.clone();

        // All segments should already be deleted during commit()
        // But clean up any remaining empty segments just in case
        for segment in self.list_segments() {
            if self.is_segment_drained(&segment) {
                self.delete_segment(&segment)?;
            }
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

        // Log recovery if we were in overflow mode, and reset the warning flag
        if self.overflow_warned.swap(false, Ordering::Relaxed) {
            info!(
                "Queue {} recovered from disk overflow, back to memory channel (drained {} messages in {:.2}s)",
                name, drained, drain_duration
            );
        } else {
            info!("Drained {} messages in {:.2}s", drained, drain_duration);
        }

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
        let queue_path = temp_dir.path().join("test_queue");

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
        let queue_path = temp_dir.path().join("test_queue");

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
        let queue_path = temp_dir.path().join("test_queue");

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
        let queue_path = temp_dir.path().join("test_queue");

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
        let queue_path = temp_dir.path().join("test_queue");

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
        let queue_path = temp_dir.path().join("test_queue");

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
        let queue_path = temp_dir.path().join("test_queue");

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
        let queue_path = temp_dir.path().join("test_queue");

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

    #[tokio::test]
    async fn test_segment_rotation() {
        let temp_dir = TempDir::new().unwrap();
        let queue_path = temp_dir.path().join("test_queue");

        // Use a small segment size to force rotation
        let queue = PersistentQueue::<String>::with_segment_size(
            "test".to_string(),
            queue_path.clone(),
            None,
            10,
            500, // 500 bytes per segment
        )
        .unwrap();

        // Send enough messages to create multiple segments
        for i in 0..50 {
            queue.send(format!("message-{:04}", i)).await.unwrap();
        }

        // Should have created multiple segments
        let segment_count = queue.segment_count();
        assert!(
            segment_count > 1,
            "Expected multiple segments, got {}",
            segment_count
        );

        // Connect and drain
        queue
            .connect_consumer("test-consumer".to_string())
            .await
            .unwrap();

        for i in 0..50 {
            let msg = queue.recv().await.unwrap();
            assert_eq!(msg, format!("message-{:04}", i));
            queue.commit().await.unwrap();
        }

        // After draining, only the write segment may remain (empty but not deleted
        // to avoid race with concurrent appends). Non-write segments are deleted.
        let remaining_segments = queue.segment_count();
        assert!(
            remaining_segments <= 1,
            "Expected at most 1 segment (the write segment) after drain, got {}",
            remaining_segments
        );
    }

    #[tokio::test]
    async fn test_segment_cleanup_on_drain() {
        let temp_dir = TempDir::new().unwrap();
        let queue_path = temp_dir.path().join("test_queue");

        // Small segments for testing
        let queue = PersistentQueue::<String>::with_segment_size(
            "test".to_string(),
            queue_path.clone(),
            None,
            10,
            200, // Very small segments
        )
        .unwrap();

        // Write messages to create multiple segments
        for i in 0..20 {
            queue.send(format!("msg-{:02}", i)).await.unwrap();
        }

        let initial_segments = queue.segment_count();
        assert!(initial_segments > 1, "Should have multiple segments");

        // Connect and drain half the messages
        queue
            .connect_consumer("test-consumer".to_string())
            .await
            .unwrap();

        for _ in 0..10 {
            queue.recv().await.unwrap();
            queue.commit().await.unwrap();
        }

        // Some segments should have been deleted (all except the write segment)
        let mid_segments = queue.segment_count();
        assert!(
            mid_segments < initial_segments,
            "Segments should decrease during drain"
        );

        // Drain remaining messages
        for _ in 0..10 {
            queue.recv().await.unwrap();
            queue.commit().await.unwrap();
        }

        // After draining, only the write segment may remain (empty but not deleted
        // to avoid race with concurrent appends). Non-write segments are deleted.
        assert!(
            queue.segment_count() <= 1,
            "At most 1 segment (the write segment) should remain"
        );
    }

    #[tokio::test]
    async fn test_concurrent_drain_and_append_single_segment() {
        // This test verifies that concurrent appends to the write segment
        // are not lost when commit() drains that same segment.
        // Regression test for: commit() deleting segment based on stale write_offset
        // while append_to_disk() is concurrently writing to it.

        let temp_dir = TempDir::new().unwrap();
        let queue_path = temp_dir.path().join("test_queue");

        let queue = Arc::new(
            PersistentQueue::<String>::new("test".to_string(), queue_path.clone(), None, 10)
                .unwrap(),
        );

        // Buffer a single message to disk (creates one segment)
        queue.send("initial".to_string()).await.unwrap();
        assert_eq!(queue.segment_count(), 1);

        // Connect consumer (enters drain mode since there's a backlog)
        queue
            .connect_consumer("test-consumer".to_string())
            .await
            .unwrap();

        // Drain the initial message
        let msg = queue.recv().await.unwrap();
        assert_eq!(msg, "initial");

        // Before commit, append a new message to the same segment
        // This simulates concurrent append during drain
        queue.send("concurrent".to_string()).await.unwrap();

        // Now commit - this should NOT delete the segment because:
        // 1. It's the only/newest segment (the write segment)
        // 2. The concurrent append just added data to it
        queue.commit().await.unwrap();

        // The concurrent message should NOT be lost
        let msg2 = queue.recv().await.unwrap();
        assert_eq!(msg2, "concurrent");
        queue.commit().await.unwrap();

        // Segment should still exist (it's the write segment) but be empty
        // It will be cleaned up when finish_drain() runs after memory channel recv
    }

    #[tokio::test]
    async fn test_write_segment_not_deleted_prematurely() {
        // Verify that the current write segment is never deleted by commit(),
        // even if it appears fully drained.

        let temp_dir = TempDir::new().unwrap();
        let queue_path = temp_dir.path().join("test_queue");

        let queue =
            PersistentQueue::<String>::new("test".to_string(), queue_path.clone(), None, 10)
                .unwrap();

        // Send and drain a single message
        queue.send("message1".to_string()).await.unwrap();
        queue
            .connect_consumer("test-consumer".to_string())
            .await
            .unwrap();

        let msg = queue.recv().await.unwrap();
        assert_eq!(msg, "message1");
        queue.commit().await.unwrap();

        // The segment should NOT be deleted because it's the write segment
        // (even though it's fully drained)
        assert_eq!(
            queue.segment_count(),
            1,
            "Write segment should not be deleted"
        );

        // Verify we can still write to it
        queue.send("message2".to_string()).await.unwrap();

        // And read the new message
        let msg2 = queue.recv().await.unwrap();
        assert_eq!(msg2, "message2");
        queue.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_auto_connect_on_recv() {
        // Verify that recv() auto-connects when queue is in Disconnected state.
        // This ensures messages are buffered to disk until consumption begins,
        // preventing message loss if process restarts before recv() is called.

        let temp_dir = TempDir::new().unwrap();
        let queue_path = temp_dir.path().join("test_queue");

        let queue =
            PersistentQueue::<String>::new("test".to_string(), queue_path.clone(), None, 10)
                .unwrap();

        // Initial state: Disconnected
        let state = queue.state.read().await.clone();
        assert!(matches!(state, QueueState::Disconnected { .. }));

        // Send messages while disconnected - they go to disk
        queue.send("msg1".to_string()).await.unwrap();
        queue.send("msg2".to_string()).await.unwrap();

        // Verify still disconnected and messages are on disk
        let state = queue.state.read().await.clone();
        assert!(matches!(state, QueueState::Disconnected { .. }));
        assert!(queue.data_size_bytes() > 0, "Messages should be on disk");

        // recv() should auto-connect and drain from disk
        let msg = queue.recv().await.unwrap();
        assert_eq!(msg, "msg1");
        queue.commit().await.unwrap();

        // Should now be in Draining state (since there was a disk backlog)
        let state = queue.state.read().await.clone();
        assert!(
            matches!(
                state,
                QueueState::Draining { .. } | QueueState::Connected { .. }
            ),
            "Should be Draining or Connected after recv, got {:?}",
            state
        );

        // Continue draining
        let msg = queue.recv().await.unwrap();
        assert_eq!(msg, "msg2");
        queue.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_auto_connect_no_backlog() {
        // Verify that recv() auto-connects to Connected state when no disk backlog

        let temp_dir = TempDir::new().unwrap();
        let queue_path = temp_dir.path().join("test_queue");

        let queue = Arc::new(
            PersistentQueue::<String>::new("test".to_string(), queue_path.clone(), None, 10)
                .unwrap(),
        );

        // Initial state: Disconnected
        let state = queue.state.read().await.clone();
        assert!(matches!(state, QueueState::Disconnected { .. }));

        // No messages sent yet, so no disk backlog

        // Spawn a task to send a message after a short delay
        let queue_sender = queue.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            // By now, recv() should have auto-connected to Connected state
            queue_sender.send("hello".to_string()).await.unwrap();
        });

        // recv() should auto-connect to Connected (no backlog) and wait for message
        let msg = queue.recv().await.unwrap();
        assert_eq!(msg, "hello");

        // Should be in Connected state
        let state = queue.state.read().await.clone();
        assert!(
            matches!(state, QueueState::Connected { .. }),
            "Should be Connected after recv with no backlog, got {:?}",
            state
        );
    }

    #[tokio::test]
    async fn test_messages_not_lost_on_restart_simulation() {
        // Simulate the scenario where process restarts before consumption begins.
        // Messages should be persisted to disk and recoverable.

        let temp_dir = TempDir::new().unwrap();
        let queue_path = temp_dir.path().join("test_queue");

        // First "process" - sends messages but never calls recv()
        {
            let queue =
                PersistentQueue::<String>::new("test".to_string(), queue_path.clone(), None, 10)
                    .unwrap();

            // Send messages while disconnected - they go to disk
            queue.send("persistent1".to_string()).await.unwrap();
            queue.send("persistent2".to_string()).await.unwrap();
            queue.send("persistent3".to_string()).await.unwrap();

            // Verify messages are on disk
            assert!(queue.data_size_bytes() > 0);

            // Queue is dropped here (simulating process restart)
        }

        // Second "process" - should recover messages
        {
            let queue =
                PersistentQueue::<String>::new("test".to_string(), queue_path.clone(), None, 10)
                    .unwrap();

            // Verify still disconnected initially
            let state = queue.state.read().await.clone();
            assert!(matches!(state, QueueState::Disconnected { .. }));

            // Now recv() - should auto-connect and drain persisted messages
            let msg1 = queue.recv().await.unwrap();
            queue.commit().await.unwrap();
            let msg2 = queue.recv().await.unwrap();
            queue.commit().await.unwrap();
            let msg3 = queue.recv().await.unwrap();
            queue.commit().await.unwrap();

            assert_eq!(msg1, "persistent1");
            assert_eq!(msg2, "persistent2");
            assert_eq!(msg3, "persistent3");
        }
    }
}
