use anyhow::Result;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::time::sleep;
use tracing::{error, info, trace, warn};

// Queue size for raw Beast messages from TCP socket
const RAW_MESSAGE_QUEUE_SIZE: usize = 10000;

fn queue_warning_threshold(queue_size: usize) -> usize {
    queue_size * 4 / 5 // 80% threshold
}

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
    /// Maximum number of connection retry attempts
    pub max_retries: u32,
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
            max_retries: 5,
            retry_delay_seconds: 0, // Reconnect immediately on first failure
            max_retry_delay_seconds: 60, // Cap at 60 seconds
        }
    }
}

/// Beast client that connects to a Beast-format ADS-B server via TCP
/// Publishes raw Beast messages to JetStream for processing
pub struct BeastClient {
    config: BeastClientConfig,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl BeastClient {
    /// Create a new Beast client
    pub fn new(config: BeastClientConfig) -> Self {
        Self {
            config,
            shutdown_tx: None,
        }
    }

    /// Start the Beast client with a publisher
    /// This connects to the Beast server and publishes all messages to NATS
    #[tracing::instrument(skip(self, publisher))]
    pub async fn start<P: crate::beast::BeastPublisher>(&mut self, publisher: P) -> Result<()> {
        let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let health_state = crate::metrics::init_beast_health();
        self.start_with_shutdown(publisher, shutdown_rx, health_state)
            .await
    }

    /// Start the Beast client with a publisher and external shutdown signal
    /// This connects to the Beast server and publishes all messages to NATS
    /// Supports graceful shutdown when shutdown_rx receives a signal
    /// Updates health_state with connection status for health checks
    #[tracing::instrument(skip(self, publisher, shutdown_rx, health_state))]
    pub async fn start_with_shutdown<P: crate::beast::BeastPublisher>(
        &mut self,
        publisher: P,
        shutdown_rx: tokio::sync::oneshot::Receiver<()>,
        health_state: std::sync::Arc<tokio::sync::RwLock<crate::metrics::BeastIngestHealth>>,
    ) -> Result<()> {
        let (internal_shutdown_tx, mut internal_shutdown_rx) =
            tokio::sync::oneshot::channel::<()>();
        self.shutdown_tx = Some(internal_shutdown_tx);

        let config = self.config.clone();

        // Use a broadcast channel to share shutdown signal with both publisher and connection loop
        let (shutdown_broadcast_tx, _) = tokio::sync::broadcast::channel::<()>(1);
        let mut shutdown_rx_for_loop = shutdown_broadcast_tx.subscribe();

        // Create bounded channel for raw Beast messages from TCP socket
        let (raw_message_tx, raw_message_rx) = flume::bounded::<Vec<u8>>(RAW_MESSAGE_QUEUE_SIZE);
        info!(
            "Created raw message queue with capacity {} for JetStream publishing",
            RAW_MESSAGE_QUEUE_SIZE
        );

        // Spawn task to forward external shutdown signal to broadcast channel
        let shutdown_broadcast_tx_clone = shutdown_broadcast_tx.clone();
        tokio::spawn(async move {
            let _ = shutdown_rx.await;
            info!("External shutdown signal received, broadcasting to all tasks");
            let _ = shutdown_broadcast_tx_clone.send(());
        });

        // Spawn message publishing task
        let publisher_for_task = publisher.clone();
        let mut shutdown_rx_for_publisher = shutdown_broadcast_tx.subscribe();
        let publisher_handle = tokio::spawn(async move {
            let publisher = publisher_for_task;
            let mut stats_timer = tokio::time::interval(Duration::from_secs(10));
            stats_timer.tick().await; // First tick completes immediately

            let mut attempted_count = 0u64;
            let mut last_log_time = std::time::Instant::now();
            let mut last_receive_time = std::time::Instant::now();

            // Limit concurrent publishes to prevent spawning too many tasks
            let publish_semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(100));

            loop {
                tokio::select! {
                    Ok(message) = raw_message_rx.recv_async() => {
                        last_receive_time = std::time::Instant::now();
                        attempted_count += 1;

                        // Use semaphore to limit concurrent publishes to 100
                        let permit = publish_semaphore.clone().acquire_owned().await.unwrap();
                        let pub_clone = publisher.clone();

                        tokio::spawn(async move {
                            let _permit = permit; // Hold permit until publish completes
                            let publish_start = std::time::Instant::now();

                            // Add 5-second timeout to prevent indefinite blocking
                            match tokio::time::timeout(
                                Duration::from_secs(5),
                                pub_clone.publish_fire_and_forget(&message)
                            ).await {
                                Ok(_) => {
                                    let publish_duration = publish_start.elapsed();

                                    // Warn if publish took more than 100ms
                                    if publish_duration.as_millis() > 100 {
                                        warn!(
                                            "Slow JetStream publish: {}ms",
                                            publish_duration.as_millis()
                                        );
                                        metrics::counter!("beast.nats.slow_publish_total").increment(1);
                                    }
                                }
                                Err(_) => {
                                    error!("JetStream publish timed out after 5 seconds - NATS may be blocked");
                                    metrics::counter!("beast.nats.publish_timeout_total").increment(1);

                                    // Report timeout to Sentry (throttled)
                                    sentry::capture_message(
                                        "Beast NATS publish timed out after 5 seconds",
                                        sentry::Level::Error
                                    );
                                }
                            }
                        });
                    }
                    _ = stats_timer.tick() => {
                        // Report raw message queue depth every 10 seconds
                        let queue_depth = raw_message_rx.len();
                        let available_permits = publish_semaphore.available_permits();
                        let in_flight_publishes = 100 - available_permits;

                        metrics::gauge!("beast.nats.queue_depth").set(queue_depth as f64);
                        metrics::gauge!("beast.nats.in_flight").set(in_flight_publishes as f64);

                        // Log publishing rate and queue status
                        let elapsed = last_log_time.elapsed().as_secs_f64();
                        let time_since_last_receive = last_receive_time.elapsed().as_secs_f64();

                        if elapsed > 0.0 {
                            let rate = attempted_count as f64 / elapsed;
                            info!(
                                "Beast NATS stats: {:.1} msg/s attempted (queue: {}, in-flight: {}, last receive: {:.1}s ago)",
                                rate, queue_depth, in_flight_publishes, time_since_last_receive
                            );
                            attempted_count = 0;
                            last_log_time = std::time::Instant::now();
                        }

                        if queue_depth > queue_warning_threshold(RAW_MESSAGE_QUEUE_SIZE) {
                            warn!(
                                "Beast NATS publish queue building up: {} messages (80% full)",
                                queue_depth
                            );
                        }
                    }
                    _ = shutdown_rx_for_publisher.recv() => {
                        info!("Beast publisher received shutdown signal");
                        break;
                    }
                }
            }

            // Flush remaining messages before shutdown
            info!(
                "Beast publisher shutting down, flushing {} remaining messages",
                raw_message_rx.len()
            );

            let mut flushed_count = 0;
            while let Ok(message) = raw_message_rx.recv_async().await {
                if let Err(e) = publisher.publish_with_retry(&message, 3).await {
                    error!("Failed to flush Beast message during shutdown: {}", e);
                } else {
                    flushed_count += 1;
                }
            }

            info!(
                "Beast publisher shutdown complete, flushed {} messages",
                flushed_count
            );
        });

        // Main connection loop
        let mut retry_count = 0;
        let mut retry_delay = config.retry_delay_seconds;

        loop {
            tokio::select! {
                _ = &mut internal_shutdown_rx => {
                    info!("Beast client received internal shutdown signal");
                    break;
                }
                _ = shutdown_rx_for_loop.recv() => {
                    info!("Beast client received external shutdown signal");
                    break;
                }
                result = Self::connect_and_run(&config, raw_message_tx.clone(), health_state.clone(), shutdown_broadcast_tx.subscribe()) => {
                    match result {
                        ConnectionResult::Success => {
                            info!("Beast connection completed successfully");
                            retry_count = 0; // Reset retry count on successful connection
                            retry_delay = config.retry_delay_seconds; // Reset delay
                        }
                        ConnectionResult::ConnectionFailed(e) => {
                            error!("Beast connection failed: {}", e);
                            metrics::counter!("beast.connection.failed_total").increment(1);

                            // Mark as disconnected in health state
                            {
                                let mut health = health_state.write().await;
                                health.beast_connected = false;
                                health.last_error = Some(e.to_string());
                            }

                            retry_count += 1;
                            if retry_count >= config.max_retries {
                                error!(
                                    "Max retries ({}) reached, giving up",
                                    config.max_retries
                                );
                                return Err(anyhow::anyhow!(
                                    "Failed to connect to Beast server after {} retries",
                                    config.max_retries
                                ));
                            }

                            // Exponential backoff: double the delay each time, up to max
                            retry_delay = std::cmp::min(
                                if retry_delay == 0 {
                                    1
                                } else {
                                    retry_delay * 2
                                },
                                config.max_retry_delay_seconds,
                            );

                            info!(
                                "Retrying in {} seconds (attempt {}/{})",
                                retry_delay, retry_count, config.max_retries
                            );
                            sleep(Duration::from_secs(retry_delay)).await;
                        }
                        ConnectionResult::OperationFailed(e) => {
                            error!("Beast operation failed: {}", e);
                            metrics::counter!("beast.operation.failed_total").increment(1);

                            // Mark as disconnected in health state
                            {
                                let mut health = health_state.write().await;
                                health.beast_connected = false;
                                health.last_error = Some(e.to_string());
                            }

                            retry_count += 1;
                            if retry_count >= config.max_retries {
                                error!(
                                    "Max retries ({}) reached, giving up",
                                    config.max_retries
                                );
                                return Err(anyhow::anyhow!(
                                    "Beast operation failed after {} retries",
                                    config.max_retries
                                ));
                            }

                            // Use shorter delay for operation failures (connection was successful)
                            let operation_retry_delay = std::cmp::min(retry_delay, 5);

                            info!(
                                "Retrying in {} seconds (attempt {}/{})",
                                operation_retry_delay, retry_count, config.max_retries
                            );
                            sleep(Duration::from_secs(operation_retry_delay)).await;
                        }
                    }
                }
            }
        }

        // Shutdown publisher task
        drop(raw_message_tx);
        let _ = publisher_handle.await;

        info!("Beast client shutdown complete");
        Ok(())
    }

    /// Connect to the Beast server and run the message processing loop
    /// Messages are sent to raw_message_tx channel for processing
    #[tracing::instrument(skip(config, raw_message_tx, health_state, shutdown_rx), fields(server = %config.server, port = %config.port))]
    async fn connect_and_run(
        config: &BeastClientConfig,
        raw_message_tx: flume::Sender<Vec<u8>>,
        health_state: std::sync::Arc<tokio::sync::RwLock<crate::metrics::BeastIngestHealth>>,
        shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> ConnectionResult {
        info!(
            "Connecting to Beast server {}:{}",
            config.server, config.port
        );

        // Track connection start time for duration reporting
        let connection_start = std::time::Instant::now();

        // Perform explicit DNS lookup
        let server_address = format!("{}:{}", config.server, config.port);
        let socket_addrs = match tokio::net::lookup_host(&server_address).await {
            Ok(addrs) => {
                let addrs_vec: Vec<_> = addrs.collect();
                if addrs_vec.is_empty() {
                    return ConnectionResult::ConnectionFailed(anyhow::anyhow!(
                        "DNS resolution returned no addresses for {}",
                        server_address
                    ));
                }
                info!(
                    "DNS resolved {} to {} address(es): {:?}",
                    server_address,
                    addrs_vec.len(),
                    addrs_vec
                );
                addrs_vec
            }
            Err(e) => {
                return ConnectionResult::ConnectionFailed(anyhow::anyhow!(
                    "DNS resolution failed for {}: {}",
                    server_address,
                    e
                ));
            }
        };

        // Try each resolved address until one succeeds
        let mut last_error = None;
        for addr in &socket_addrs {
            match TcpStream::connect(addr).await {
                Ok(stream) => {
                    info!("Connected to Beast server at {}", addr);
                    metrics::counter!("beast.connection.established_total").increment(1);
                    metrics::gauge!("beast.connection.connected").set(1.0);

                    // Mark as connected in health state
                    {
                        let mut health = health_state.write().await;
                        health.beast_connected = true;
                    }

                    // Continue with message processing using this stream
                    return Self::process_connection(
                        stream,
                        raw_message_tx,
                        health_state,
                        connection_start,
                        addr.to_string(),
                        shutdown_rx,
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

        // If we get here, all addresses failed
        ConnectionResult::ConnectionFailed(anyhow::anyhow!(
            "Failed to connect to any resolved address for {}: {:?}",
            server_address,
            last_error
        ))
    }

    /// Process an established Beast connection
    /// Reads raw Beast frames and publishes them to JetStream
    #[tracing::instrument(skip(stream, raw_message_tx, health_state, connection_start, shutdown_rx), fields(peer_addr = %peer_addr_str))]
    async fn process_connection(
        mut stream: TcpStream,
        raw_message_tx: flume::Sender<Vec<u8>>,
        health_state: std::sync::Arc<tokio::sync::RwLock<crate::metrics::BeastIngestHealth>>,
        connection_start: std::time::Instant,
        peer_addr_str: String,
        mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> ConnectionResult {
        info!("Processing connection to Beast server at {}", peer_addr_str);

        let message_timeout = Duration::from_secs(300); // 5 minute timeout
        let mut buffer = vec![0u8; 8192]; // 8KB buffer for Beast messages
        let mut frame_buffer = Vec::new();
        let mut pending_escape = false; // Track if we saw 0x1A at end of previous buffer
        let mut message_count = 0u64;
        let mut last_stats_log = std::time::Instant::now();

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("Beast connection received shutdown signal, closing connection");
                    metrics::gauge!("beast.connection.connected").set(0.0);

                    // Mark as disconnected in health state
                    {
                        let mut health = health_state.write().await;
                        health.beast_connected = false;
                    }

                    return ConnectionResult::Success;
                }
                read_result = tokio::time::timeout(message_timeout, stream.read(&mut buffer)) => {
                    match read_result {
                Ok(Ok(0)) => {
                    // Connection closed
                    let duration = connection_start.elapsed();
                    info!(
                        "Beast connection closed by server (IP: {}) after {:.1}s, received {} messages",
                        peer_addr_str,
                        duration.as_secs_f64(),
                        message_count
                    );
                    metrics::gauge!("beast.connection.connected").set(0.0);

                    // Mark as disconnected in health state
                    {
                        let mut health = health_state.write().await;
                        health.beast_connected = false;
                    }

                    return ConnectionResult::Success;
                }
                Ok(Ok(n)) => {
                    // Data received
                    trace!("Received {} bytes from Beast server", n);
                    metrics::counter!("beast.bytes.received_total").increment(n as u64);

                    // Process Beast frames with proper escape sequence handling
                    // Beast format: <1A> <message_type> <6-byte timestamp> <signal> <payload>
                    // Escape rule: Any 0x1A in the data is escaped as <1A><1A>
                    // We need to un-escape and detect frame boundaries correctly

                    let mut i = 0;

                    // Handle pending escape from previous buffer
                    if pending_escape && n > 0 {
                        pending_escape = false;
                        if buffer[0] == 0x1A {
                            // Escape sequence: the previous 0x1A + this 0x1A = one literal 0x1A
                            frame_buffer.push(0x1A);
                            i = 1;
                        } else {
                            // Frame boundary: previous 0x1A started a new frame
                            // Publish the previous frame (without the 0x1A which is already in frame_buffer)
                            if frame_buffer.len() > 1 {
                                // Remove the trailing 0x1A that we added last time
                                frame_buffer.pop();
                                if !frame_buffer.is_empty() {
                                    Self::publish_frame(&raw_message_tx, frame_buffer.clone()).await;
                                    message_count += 1;
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
                                        Self::publish_frame(&raw_message_tx, frame_buffer.clone()).await;
                                        message_count += 1;
                                    }
                                    frame_buffer.clear();
                                    frame_buffer.push(0x1A);
                                    i += 1;
                                }
                            } else {
                                // 0x1A at end of buffer - we need to wait for next byte
                                frame_buffer.push(0x1A);
                                pending_escape = true;
                                i += 1;
                            }
                        } else {
                            // Regular data byte
                            frame_buffer.push(byte);
                            i += 1;
                        }
                    }

                    // Log stats every 10 seconds
                    if last_stats_log.elapsed().as_secs() >= 10 {
                        let elapsed = last_stats_log.elapsed().as_secs_f64();
                        let rate = message_count as f64 / elapsed;
                        info!(
                            "Beast stats: {:.1} msg/s, {} total messages",
                            rate, message_count
                        );
                        metrics::gauge!("beast.message_rate").set(rate);
                        message_count = 0;
                        last_stats_log = std::time::Instant::now();
                    }
                }
                Ok(Err(e)) => {
                    // Read error
                    let duration = connection_start.elapsed();
                    error!(
                        "Beast read error from {} after {:.1}s: {}",
                        peer_addr_str,
                        duration.as_secs_f64(),
                        e
                    );
                    metrics::gauge!("beast.connection.connected").set(0.0);

                    return ConnectionResult::OperationFailed(anyhow::anyhow!(
                        "Beast read error: {}",
                        e
                    ));
                }
                Err(_) => {
                    // Timeout
                    let duration = connection_start.elapsed();
                    warn!(
                        "No data received from Beast server {} for {} seconds (after {:.1}s connected)",
                        peer_addr_str,
                        message_timeout.as_secs(),
                        duration.as_secs_f64()
                    );
                    metrics::counter!("beast.timeout_total").increment(1);
                    metrics::gauge!("beast.connection.connected").set(0.0);

                    return ConnectionResult::OperationFailed(anyhow::anyhow!(
                        "No data received for {} seconds",
                        message_timeout.as_secs()
                    ));
                }
                    }
                }
            }
        }
    }

    /// Publish a Beast frame to JetStream with timestamp
    /// Format: 8-byte Unix timestamp (microseconds as i64) + raw Beast frame bytes
    async fn publish_frame(raw_message_tx: &flume::Sender<Vec<u8>>, frame: Vec<u8>) {
        if frame.is_empty() {
            return;
        }

        // Get current timestamp as microseconds since Unix epoch
        let timestamp_micros = chrono::Utc::now().timestamp_micros();

        // Build message: 8-byte timestamp + frame bytes
        let mut message = Vec::with_capacity(8 + frame.len());
        message.extend_from_slice(&timestamp_micros.to_be_bytes());
        message.extend_from_slice(&frame);

        match raw_message_tx.send_async(message).await {
            Ok(_) => {
                trace!("Published Beast frame ({} bytes)", frame.len());
                metrics::counter!("beast.frames.published_total").increment(1);
            }
            Err(e) => {
                error!("Failed to send Beast frame to queue: {}", e);
                metrics::counter!("beast.frames.dropped_total").increment(1);
            }
        }
    }
}
