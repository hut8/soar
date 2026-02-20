use anyhow::Result;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::time::sleep;
use tracing::{error, info, trace, warn};

use crate::protocol::{IngestSource, create_serialized_envelope};

// Queue size for raw Beast messages from TCP socket
const RAW_MESSAGE_QUEUE_SIZE: usize = 1000;

/// Result type for connection attempts
enum ConnectionResult {
    /// Connection was successful and ran until completion/disconnection
    Success,
    /// Connection failed immediately (couldn't establish connection)
    ConnectionFailed(anyhow::Error),
    /// Connection was established but failed during operation
    OperationFailed(anyhow::Error),
}

/// Configuration for the Beast client
#[derive(Debug, Clone)]
pub struct BeastClientConfig {
    /// Beast server hostname
    pub server: String,
    /// Beast server port (typically 30005)
    pub port: u16,
    /// Initial delay between reconnection attempts in seconds (will use exponential backoff)
    pub retry_delay_seconds: u64,
    /// Maximum delay between reconnection attempts in seconds (cap for exponential backoff)
    pub max_retry_delay_seconds: u64,
}

impl Default for BeastClientConfig {
    fn default() -> Self {
        Self {
            server: "localhost".to_string(),
            port: 30005,
            retry_delay_seconds: 0, // Reconnect immediately on first failure
            max_retry_delay_seconds: 60, // Cap at 60 seconds
        }
    }
}

/// Beast client that connects to a Beast-format ADS-B server via TCP
/// Publishes raw Beast messages for processing
pub struct BeastClient {
    config: BeastClientConfig,
}

impl BeastClient {
    /// Create a new Beast client
    pub fn new(config: BeastClientConfig) -> Self {
        Self { config }
    }

    /// Start the Beast client with persistent queue
    /// This connects to the Beast server and sends all messages to the queue as serialized protobuf Envelopes
    /// Each envelope contains: source type (Beast), timestamp (captured at receive time), and raw payload
    #[tracing::instrument(skip(self, queue, health_state, stats_counter))]
    pub async fn start(
        &mut self,
        queue: std::sync::Arc<crate::persistent_queue::PersistentQueue<Vec<u8>>>,
        health_state: std::sync::Arc<tokio::sync::RwLock<crate::metrics::BeastIngestHealth>>,
        stats_counter: Option<std::sync::Arc<std::sync::atomic::AtomicU64>>,
    ) -> Result<()> {
        let config = self.config.clone();

        // Create bounded channel for serialized envelope bytes from TCP socket
        let (envelope_tx, envelope_rx) = flume::bounded::<Vec<u8>>(RAW_MESSAGE_QUEUE_SIZE);

        // Spawn queue feeding task - reads from channel, sends to persistent queue
        let queue_clone = queue.clone();
        let stats_counter_clone = stats_counter.clone();
        let _queue_handle = tokio::spawn(async move {
            let mut last_metrics_update = std::time::Instant::now();

            loop {
                match envelope_rx.recv_async().await {
                    Ok(envelope_bytes) => {
                        if let Err(e) = queue_clone.send(envelope_bytes.clone()).await {
                            warn!("Failed to send to persistent queue, retrying: {}", e);
                            metrics::counter!("beast.queue.send_error_total").increment(1);
                            // Retry with backoff until it succeeds
                            let mut retry_delay = Duration::from_millis(100);
                            loop {
                                sleep(retry_delay).await;
                                match queue_clone.send(envelope_bytes.clone()).await {
                                    Ok(()) => break,
                                    Err(e) => {
                                        warn!("Persistent queue send retry failed: {}", e);
                                        metrics::counter!("beast.queue.send_error_total")
                                            .increment(1);
                                        retry_delay =
                                            std::cmp::min(retry_delay * 2, Duration::from_secs(5));
                                    }
                                }
                            }
                        }
                        if let Some(ref counter) = stats_counter_clone {
                            counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }

                        // Update channel depth metric every second
                        if last_metrics_update.elapsed().as_secs() >= 1 {
                            let channel_depth = envelope_rx.len();
                            metrics::gauge!("beast.envelope_channel.depth")
                                .set(channel_depth as f64);
                            metrics::gauge!("beast.envelope_channel.capacity")
                                .set(RAW_MESSAGE_QUEUE_SIZE as f64);
                            last_metrics_update = std::time::Instant::now();
                        }
                    }
                    Err(_) => {
                        info!("Envelope channel closed, queue feeder exiting");
                        break;
                    }
                }
            }
        });

        // Connection retry loop - retries indefinitely with exponential backoff
        let mut retry_delay = config.retry_delay_seconds;

        loop {
            let result =
                Self::connect_and_run(&config, envelope_tx.clone(), health_state.clone()).await;

            match result {
                ConnectionResult::Success => {
                    info!("Beast connection completed normally");
                    retry_delay = config.retry_delay_seconds;
                }
                ConnectionResult::ConnectionFailed(e) => {
                    error!("Beast connection failed: {}", e);
                    retry_delay = std::cmp::min(retry_delay * 2, config.max_retry_delay_seconds);
                    info!("Retrying in {} seconds", retry_delay);
                    sleep(Duration::from_secs(retry_delay)).await;
                }
                ConnectionResult::OperationFailed(e) => {
                    error!("Beast operation failed: {}", e);
                    let operation_retry_delay = std::cmp::min(retry_delay, 5);
                    info!("Retrying in {} seconds", operation_retry_delay);
                    sleep(Duration::from_secs(operation_retry_delay)).await;
                }
            }
        }
    }

    /// Connect to the Beast server and run the message processing loop
    /// Creates protobuf envelopes with timestamps captured at receive time
    #[tracing::instrument(skip(config, envelope_tx, health_state), fields(server = %config.server, port = %config.port))]
    async fn connect_and_run(
        config: &BeastClientConfig,
        envelope_tx: flume::Sender<Vec<u8>>,
        health_state: std::sync::Arc<tokio::sync::RwLock<crate::metrics::BeastIngestHealth>>,
    ) -> ConnectionResult {
        info!(
            "Connecting to Beast server {}:{}",
            config.server, config.port
        );

        let connection_start = std::time::Instant::now();

        // DNS lookup
        let server_address = format!("{}:{}", config.server, config.port);
        let socket_addrs = match tokio::net::lookup_host(&server_address).await {
            Ok(addrs) => {
                let all_addrs: Vec<_> = addrs.collect();
                if all_addrs.is_empty() {
                    return ConnectionResult::ConnectionFailed(anyhow::anyhow!(
                        "DNS resolution returned no addresses for {}",
                        server_address
                    ));
                }

                let ipv4_addrs: Vec<_> = all_addrs
                    .iter()
                    .filter(|addr| addr.is_ipv4())
                    .cloned()
                    .collect();

                if ipv4_addrs.is_empty() {
                    warn!(
                        "No IPv4 addresses found for {}, falling back to all {} addresses",
                        server_address,
                        all_addrs.len()
                    );
                    all_addrs
                } else {
                    info!(
                        "DNS resolved {} to {} IPv4 address(es)",
                        server_address,
                        ipv4_addrs.len()
                    );
                    ipv4_addrs
                }
            }
            Err(e) => {
                return ConnectionResult::ConnectionFailed(anyhow::anyhow!(
                    "DNS resolution failed for {}: {}",
                    server_address,
                    e
                ));
            }
        };

        // Shuffle addresses
        let mut shuffled_addrs = socket_addrs;
        {
            use rand::seq::SliceRandom;
            let mut rng = rand::rng();
            shuffled_addrs.shuffle(&mut rng);
        }
        info!("Trying addresses in randomized order: {:?}", shuffled_addrs);

        // Try each address
        let mut last_error = None;
        for addr in &shuffled_addrs {
            match TcpStream::connect(addr).await {
                Ok(stream) => {
                    info!("Connected to Beast server at {}", addr);
                    metrics::counter!("beast.connection.established_total").increment(1);
                    metrics::gauge!("beast.connection.connected").set(1.0);

                    {
                        let mut health = health_state.write().await;
                        health.beast_connected = true;
                    }

                    return Self::process_connection(
                        stream,
                        envelope_tx,
                        health_state,
                        connection_start,
                        addr.to_string(),
                    )
                    .await;
                }
                Err(e) => {
                    warn!("Failed to connect to {}: {}", addr, e);
                    last_error = Some(e);
                    continue;
                }
            }
        }

        ConnectionResult::ConnectionFailed(anyhow::anyhow!(
            "Failed to connect to any resolved address for {}: {:?}",
            server_address,
            last_error
        ))
    }

    /// Process an established Beast connection
    /// Creates protobuf envelopes with timestamps at the moment frames are received
    #[tracing::instrument(skip(stream, envelope_tx, health_state, connection_start), fields(peer_addr = %peer_addr_str))]
    async fn process_connection(
        mut stream: TcpStream,
        envelope_tx: flume::Sender<Vec<u8>>,
        health_state: std::sync::Arc<tokio::sync::RwLock<crate::metrics::BeastIngestHealth>>,
        connection_start: std::time::Instant,
        peer_addr_str: String,
    ) -> ConnectionResult {
        info!("Processing connection to Beast server at {}", peer_addr_str);

        let message_timeout = Duration::from_secs(300);
        let mut buffer = vec![0u8; 8192];
        let mut frame_buffer = Vec::new();
        let mut pending_escape = false;
        let mut last_stats_log = std::time::Instant::now();

        // Initialize interval tracking
        {
            let mut health = health_state.write().await;
            if health.interval_start.is_none() {
                health.interval_start = Some(std::time::Instant::now());
            }
        }

        loop {
            let read_result = tokio::time::timeout(message_timeout, stream.read(&mut buffer)).await;
            match read_result {
                Ok(Ok(0)) => {
                    let duration = connection_start.elapsed();
                    let total_messages = {
                        let health = health_state.read().await;
                        health.total_messages
                    };
                    info!(
                        "Beast connection closed by server (IP: {}) after {:.1}s, received {} messages",
                        peer_addr_str,
                        duration.as_secs_f64(),
                        total_messages
                    );
                    metrics::counter!("beast.connection.server_closed_total").increment(1);
                    metrics::gauge!("beast.connection.connected").set(0.0);

                    {
                        let mut health = health_state.write().await;
                        health.beast_connected = false;
                    }

                    return ConnectionResult::Success;
                }
                Ok(Ok(n)) => {
                    trace!("Received {} bytes from Beast server", n);
                    metrics::counter!("beast.bytes.received_total").increment(n as u64);

                    // Process Beast frames (same escape logic as before)
                    let mut i = 0;

                    if pending_escape && n > 0 {
                        pending_escape = false;
                        if buffer[0] == 0x1A {
                            frame_buffer.push(0x1A);
                            i = 1;
                        } else {
                            if frame_buffer.len() > 1 {
                                frame_buffer.pop();
                                if !frame_buffer.is_empty() {
                                    Self::publish_frame(
                                        &envelope_tx,
                                        frame_buffer.clone(),
                                        &health_state,
                                    )
                                    .await;
                                }
                            }
                            frame_buffer.clear();
                            frame_buffer.push(0x1A);
                            frame_buffer.push(buffer[0]);
                            i = 1;
                        }
                    }

                    while i < n {
                        let byte = buffer[i];

                        if byte == 0x1A {
                            if i + 1 < n {
                                if buffer[i + 1] == 0x1A {
                                    frame_buffer.push(0x1A);
                                    i += 2;
                                } else {
                                    if !frame_buffer.is_empty() {
                                        Self::publish_frame(
                                            &envelope_tx,
                                            frame_buffer.clone(),
                                            &health_state,
                                        )
                                        .await;
                                    }
                                    frame_buffer.clear();
                                    frame_buffer.push(0x1A);
                                    i += 1;
                                }
                            } else {
                                frame_buffer.push(0x1A);
                                pending_escape = true;
                                i += 1;
                            }
                        } else {
                            frame_buffer.push(byte);
                            i += 1;
                        }
                    }

                    // Update metrics periodically
                    if last_stats_log.elapsed().as_secs() >= 10 {
                        let health = health_state.read().await;
                        if let Some(interval_start) = health.interval_start {
                            let elapsed = interval_start.elapsed().as_secs_f64();
                            if elapsed > 0.0 {
                                let rate = health.interval_messages as f64 / elapsed;
                                metrics::gauge!("beast.message_rate").set(rate);
                            }
                        }
                        drop(health);

                        {
                            let mut health = health_state.write().await;
                            health.interval_messages = 0;
                            health.interval_start = Some(std::time::Instant::now());
                        }
                        last_stats_log = std::time::Instant::now();
                    }
                }
                Ok(Err(e)) => {
                    let duration = connection_start.elapsed();
                    error!(
                        "Beast read error from {} after {:.1}s: {}",
                        peer_addr_str,
                        duration.as_secs_f64(),
                        e
                    );
                    metrics::gauge!("beast.connection.connected").set(0.0);

                    {
                        let mut health = health_state.write().await;
                        health.beast_connected = false;
                        health.last_error = Some(format!("Read error: {}", e));
                    }

                    return ConnectionResult::OperationFailed(anyhow::anyhow!(
                        "Beast read error: {}",
                        e
                    ));
                }
                Err(_) => {
                    let duration = connection_start.elapsed();
                    warn!(
                        "No data received from Beast server {} for {} seconds (after {:.1}s connected)",
                        peer_addr_str,
                        message_timeout.as_secs(),
                        duration.as_secs_f64()
                    );
                    metrics::counter!("beast.timeout_total").increment(1);
                    metrics::gauge!("beast.connection.connected").set(0.0);

                    {
                        let mut health = health_state.write().await;
                        health.beast_connected = false;
                        health.last_error = Some(format!(
                            "Timeout: no data for {}s",
                            message_timeout.as_secs()
                        ));
                    }

                    return ConnectionResult::OperationFailed(anyhow::anyhow!(
                        "No data received for {} seconds",
                        message_timeout.as_secs()
                    ));
                }
            }
        }
    }

    /// Publish a Beast frame as a protobuf envelope with timestamp captured now
    async fn publish_frame(
        envelope_tx: &flume::Sender<Vec<u8>>,
        frame: Vec<u8>,
        health_state: &std::sync::Arc<tokio::sync::RwLock<crate::metrics::BeastIngestHealth>>,
    ) {
        if frame.is_empty() {
            return;
        }

        // Create protobuf envelope with timestamp captured NOW
        let envelope_bytes = match create_serialized_envelope(IngestSource::Beast, frame.clone()) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to create Beast envelope: {}", e);
                metrics::counter!("beast.envelope.creation_error_total").increment(1);
                return;
            }
        };

        match envelope_tx.send_async(envelope_bytes).await {
            Ok(_) => {
                trace!("Published Beast frame ({} bytes) as envelope", frame.len());
                metrics::counter!("beast.frames.published_total").increment(1);

                // Update health stats
                {
                    let mut health = health_state.write().await;
                    health.total_messages += 1;
                    health.interval_messages += 1;
                    health.last_message_time = Some(std::time::Instant::now());
                }
            }
            Err(e) => {
                error!("Failed to send Beast envelope to channel: {}", e);
                metrics::counter!("beast.frames.dropped_total").increment(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    /// Helper function to process a buffer with escape sequences
    /// Returns the processed frames and any pending state
    fn process_buffer_with_escapes(
        buffer: &[u8],
        frame_buffer: &mut Vec<u8>,
        pending_escape: &mut bool,
    ) -> Vec<Vec<u8>> {
        let mut frames = Vec::new();
        let mut i = 0;
        let n = buffer.len();

        // Handle pending escape from previous buffer
        if *pending_escape && n > 0 {
            *pending_escape = false;
            if buffer[0] == 0x1A {
                // Escape sequence: the previous 0x1A + this 0x1A = one literal 0x1A
                // The previous 0x1A is already in frame_buffer, so we just skip the second one
                i = 1;
            } else {
                // Frame boundary: previous 0x1A started a new frame
                // Publish the previous frame (without the 0x1A which is already in frame_buffer)
                if frame_buffer.len() > 1 {
                    // Remove the trailing 0x1A that we added last time
                    frame_buffer.pop();
                    if !frame_buffer.is_empty() {
                        frames.push(frame_buffer.clone());
                    }
                }
                frame_buffer.clear();
                frame_buffer.push(0x1A);
                frame_buffer.push(buffer[0]);
                i = 1;
            }
        }

        while i < n {
            let byte = buffer[i];

            if byte == 0x1A {
                if i + 1 < n {
                    // We can peek at the next byte
                    if buffer[i + 1] == 0x1A {
                        // Escape sequence: <1A><1A> represents a literal 0x1A
                        frame_buffer.push(0x1A);
                        i += 2; // Skip both bytes
                    } else {
                        // Frame boundary: publish previous frame and start new one
                        if !frame_buffer.is_empty() {
                            frames.push(frame_buffer.clone());
                        }
                        frame_buffer.clear();
                        frame_buffer.push(0x1A);
                        i += 1;
                    }
                } else {
                    // 0x1A at end of buffer - we need to wait for next byte
                    frame_buffer.push(0x1A);
                    *pending_escape = true;
                    i += 1;
                }
            } else {
                // Regular data byte
                frame_buffer.push(byte);
                i += 1;
            }
        }

        frames
    }

    #[test]
    fn test_escape_sequence_simple() {
        let mut frame_buffer = Vec::new();
        let mut pending_escape = false;

        // Buffer with escape sequence: 1A 33 1A 1A 55
        // Should produce one frame: 1A 33 1A 55 (un-escaped)
        let buffer = vec![0x1A, 0x33, 0x1A, 0x1A, 0x55];
        let frames = process_buffer_with_escapes(&buffer, &mut frame_buffer, &mut pending_escape);

        assert_eq!(frames.len(), 0); // No complete frame yet
        assert_eq!(frame_buffer, vec![0x1A, 0x33, 0x1A, 0x55]);
        assert!(!pending_escape);
    }

    #[test]
    fn test_escape_sequence_multiple_frames() {
        let mut frame_buffer = Vec::new();
        let mut pending_escape = false;

        // Two frames: [1A 33 1A 1A 55] [1A 44 66]
        // First frame has escape sequence
        let buffer = vec![0x1A, 0x33, 0x1A, 0x1A, 0x55, 0x1A, 0x44, 0x66];
        let frames = process_buffer_with_escapes(&buffer, &mut frame_buffer, &mut pending_escape);

        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0], vec![0x1A, 0x33, 0x1A, 0x55]); // First frame un-escaped
        assert_eq!(frame_buffer, vec![0x1A, 0x44, 0x66]); // Second frame in progress
        assert!(!pending_escape);
    }

    #[test]
    fn test_escape_at_buffer_boundary() {
        let mut frame_buffer = Vec::new();
        let mut pending_escape = false;

        // First buffer ends with 0x1A
        let buffer1 = vec![0x1A, 0x33, 0x44, 0x1A];
        let frames1 = process_buffer_with_escapes(&buffer1, &mut frame_buffer, &mut pending_escape);

        assert_eq!(frames1.len(), 0);
        assert_eq!(frame_buffer, vec![0x1A, 0x33, 0x44, 0x1A]);
        assert!(pending_escape); // We're waiting to see if next byte is 0x1A

        // Second buffer starts with 0x1A (escape sequence)
        let buffer2 = vec![0x1A, 0x55];
        let frames2 = process_buffer_with_escapes(&buffer2, &mut frame_buffer, &mut pending_escape);

        assert_eq!(frames2.len(), 0);
        assert_eq!(frame_buffer, vec![0x1A, 0x33, 0x44, 0x1A, 0x55]); // Un-escaped
        assert!(!pending_escape);
    }

    #[test]
    fn test_frame_boundary_at_buffer_boundary() {
        let mut frame_buffer = Vec::new();
        let mut pending_escape = false;

        // First buffer ends with 0x1A
        let buffer1 = vec![0x1A, 0x33, 0x44, 0x1A];
        let frames1 = process_buffer_with_escapes(&buffer1, &mut frame_buffer, &mut pending_escape);

        assert_eq!(frames1.len(), 0);
        assert!(pending_escape);

        // Second buffer starts with different byte (frame boundary, not escape)
        let buffer2 = vec![0x55, 0x66];
        let frames2 = process_buffer_with_escapes(&buffer2, &mut frame_buffer, &mut pending_escape);

        assert_eq!(frames2.len(), 1);
        assert_eq!(frames2[0], vec![0x1A, 0x33, 0x44]); // First frame complete (without trailing 0x1A)
        assert_eq!(frame_buffer, vec![0x1A, 0x55, 0x66]); // New frame started
        assert!(!pending_escape);
    }

    #[test]
    fn test_multiple_escape_sequences() {
        let mut frame_buffer = Vec::new();
        let mut pending_escape = false;

        // Buffer: 1A 1A 1A 33 1A 1A 44
        // Processing:
        //   1A 1A -> literal 0x1A in buffer
        //   1A -> frame start, publish previous (just 0x1A), start new frame
        //   33 1A 1A 44 -> data with escape sequence
        let buffer = vec![0x1A, 0x1A, 0x1A, 0x33, 0x1A, 0x1A, 0x44];
        let frames = process_buffer_with_escapes(&buffer, &mut frame_buffer, &mut pending_escape);

        // Should produce one frame containing just the first escaped 0x1A
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0], vec![0x1A]);
        // Current frame should be: 1A 33 1A 44
        assert_eq!(frame_buffer, vec![0x1A, 0x33, 0x1A, 0x44]);
        assert!(!pending_escape);
    }

    #[test]
    fn test_consecutive_frame_boundaries() {
        let mut frame_buffer = Vec::new();
        let mut pending_escape = false;

        // Multiple frame starts: 1A 33 1A 44 1A 55
        // Should produce 2 complete frames
        let buffer = vec![0x1A, 0x33, 0x1A, 0x44, 0x1A, 0x55];
        let frames = process_buffer_with_escapes(&buffer, &mut frame_buffer, &mut pending_escape);

        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0], vec![0x1A, 0x33]);
        assert_eq!(frames[1], vec![0x1A, 0x44]);
        assert_eq!(frame_buffer, vec![0x1A, 0x55]); // Third frame in progress
        assert!(!pending_escape);
    }

    #[test]
    fn test_realistic_beast_frame_with_escape() {
        let mut frame_buffer = Vec::new();
        let mut pending_escape = false;

        // Realistic Beast frame with escape in timestamp
        // 1A (start) + 33 (type) + [00 01 02 1A 1A 03] (6-byte timestamp with escape) + 80 (signal) + [AB CD] (payload)
        let buffer = vec![
            0x1A, 0x33, 0x00, 0x01, 0x02, 0x1A, 0x1A, 0x03, 0x80, 0xAB, 0xCD,
        ];
        let frames = process_buffer_with_escapes(&buffer, &mut frame_buffer, &mut pending_escape);

        assert_eq!(frames.len(), 0);
        // Un-escaped frame: 1A 33 00 01 02 1A 03 80 AB CD (11 bytes - minimum valid Beast frame)
        assert_eq!(
            frame_buffer,
            vec![0x1A, 0x33, 0x00, 0x01, 0x02, 0x1A, 0x03, 0x80, 0xAB, 0xCD]
        );
        assert!(!pending_escape);
    }

    #[test]
    fn test_empty_buffer() {
        let mut frame_buffer = Vec::new();
        let mut pending_escape = false;

        let buffer = vec![];
        let frames = process_buffer_with_escapes(&buffer, &mut frame_buffer, &mut pending_escape);

        assert_eq!(frames.len(), 0);
        assert!(frame_buffer.is_empty());
        assert!(!pending_escape);
    }

    #[test]
    fn test_single_0x1a_at_start() {
        let mut frame_buffer = Vec::new();
        let mut pending_escape = false;

        let buffer = vec![0x1A];
        let frames = process_buffer_with_escapes(&buffer, &mut frame_buffer, &mut pending_escape);

        assert_eq!(frames.len(), 0);
        assert_eq!(frame_buffer, vec![0x1A]);
        assert!(pending_escape); // Waiting for next byte
    }

    #[test]
    fn test_escape_sequence_across_three_buffers() {
        let mut frame_buffer = Vec::new();
        let mut pending_escape = false;

        // Buffer 1: data ending with 0x1A
        let buffer1 = vec![0x1A, 0x33, 0x44, 0x1A];
        let frames1 = process_buffer_with_escapes(&buffer1, &mut frame_buffer, &mut pending_escape);
        assert_eq!(frames1.len(), 0);
        assert!(pending_escape);

        // Buffer 2: starts with 0x1A (escape), then more data ending with 0x1A
        let buffer2 = vec![0x1A, 0x55, 0x1A];
        let frames2 = process_buffer_with_escapes(&buffer2, &mut frame_buffer, &mut pending_escape);
        assert_eq!(frames2.len(), 0);
        assert_eq!(frame_buffer, vec![0x1A, 0x33, 0x44, 0x1A, 0x55, 0x1A]);
        assert!(pending_escape);

        // Buffer 3: starts with different byte (frame boundary)
        let buffer3 = vec![0x66];
        let frames3 = process_buffer_with_escapes(&buffer3, &mut frame_buffer, &mut pending_escape);
        assert_eq!(frames3.len(), 1);
        assert_eq!(frames3[0], vec![0x1A, 0x33, 0x44, 0x1A, 0x55]);
        assert_eq!(frame_buffer, vec![0x1A, 0x66]);
    }
}
