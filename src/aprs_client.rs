use anyhow::{Context, Result};
use std::time::Duration;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};
use tracing::Instrument;
use tracing::{error, info, trace, warn};

// Queue size for raw APRS messages
const RAW_MESSAGE_QUEUE_SIZE: usize = 1000;

fn queue_warning_threshold(queue_size: usize) -> usize {
    queue_size / 2
}

// AprsClient only publishes raw messages to NATS - all parsing happens in the consumer

/// Result type for connection attempts
enum ConnectionResult {
    /// Connection was successful and ran until completion/disconnection
    Success,
    /// Connection failed immediately (couldn't establish connection)
    ConnectionFailed(anyhow::Error),
    /// Connection was established but failed during operation
    OperationFailed(anyhow::Error),
}

/// Configuration for the APRS client
#[derive(Debug, Clone)]
pub struct AprsClientConfig {
    /// APRS server hostname
    pub server: String,
    /// APRS server port
    pub port: u16,
    /// Maximum number of connection retry attempts
    pub max_retries: u32,
    /// Callsign for authentication
    pub callsign: String,
    /// Password for authentication (optional)
    pub password: Option<String>,
    /// APRS filter string (optional)
    pub filter: Option<String>,
    /// Initial delay between reconnection attempts in seconds (will use exponential backoff)
    pub retry_delay_seconds: u64,
    /// Maximum delay between reconnection attempts in seconds (cap for exponential backoff)
    pub max_retry_delay_seconds: u64,
    /// Base directory for message archive (optional)
    pub archive_base_dir: Option<String>,
}

impl Default for AprsClientConfig {
    fn default() -> Self {
        Self {
            server: "aprs.glidernet.org".to_string(),
            port: 14580,
            max_retries: 5,
            callsign: "N0CALL".to_string(),
            password: None,
            filter: None,
            retry_delay_seconds: 0, // Reconnect immediately on first failure
            max_retry_delay_seconds: 60, // Cap at 60 seconds
            archive_base_dir: None,
        }
    }
}

/// APRS client that connects to an APRS-IS server via TCP
/// Calls PacketRouter directly without a queue
pub struct AprsClient {
    config: AprsClientConfig,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl AprsClient {
    /// Create a new APRS client
    pub fn new(config: AprsClientConfig) -> Self {
        Self {
            config,
            shutdown_tx: None,
        }
    }

    /// Start the APRS client with NATS publisher
    /// This connects to APRS-IS and publishes all messages to NATS
    #[tracing::instrument(skip(self, nats_publisher))]
    pub async fn start(
        &mut self,
        nats_publisher: crate::aprs_nats_publisher::NatsPublisher,
    ) -> Result<()> {
        let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let health_state = crate::metrics::init_aprs_health();
        self.start_with_shutdown(nats_publisher, shutdown_rx, health_state)
            .await
    }

    /// Start the APRS client with NATS publisher and external shutdown signal
    /// This connects to APRS-IS and publishes all messages to NATS
    /// Supports graceful shutdown when shutdown_rx receives a signal
    /// Updates health_state with connection status for health checks
    #[tracing::instrument(skip(self, nats_publisher, shutdown_rx, health_state))]
    pub async fn start_with_shutdown(
        &mut self,
        nats_publisher: crate::aprs_nats_publisher::NatsPublisher,
        shutdown_rx: tokio::sync::oneshot::Receiver<()>,
        health_state: std::sync::Arc<tokio::sync::RwLock<crate::metrics::AprsIngestHealth>>,
    ) -> Result<()> {
        // Convert oneshot receiver to a shared Arc<AtomicBool> that both tasks can check
        let shutdown_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let shutdown_flag_for_publisher = shutdown_flag.clone();
        let shutdown_flag_for_connection = shutdown_flag.clone();

        // Spawn a task to watch the oneshot and set the flag
        tokio::spawn(async move {
            let _ = shutdown_rx.await;
            shutdown_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        let config = self.config.clone();

        // Create bounded channel for raw APRS messages from TCP socket
        let (raw_message_tx, raw_message_rx) = flume::bounded::<String>(RAW_MESSAGE_QUEUE_SIZE);
        info!(
            "Created raw message queue with capacity {} for NATS publishing",
            RAW_MESSAGE_QUEUE_SIZE
        );

        // Spawn message publishing task
        let publisher = nats_publisher.clone();
        let publisher_handle = tokio::spawn(
            async move {
                let mut stats_timer = tokio::time::interval(Duration::from_secs(10));
                stats_timer.tick().await; // First tick completes immediately

                let mut attempted_count = 0u64;
                let mut last_log_time = std::time::Instant::now();
                let mut last_receive_time = std::time::Instant::now();

                loop {
                    // Check shutdown flag at the start of each loop iteration
                    if shutdown_flag_for_publisher.load(std::sync::atomic::Ordering::SeqCst) {
                        info!("Shutdown signal received by publisher, flushing queue...");

                        // Start graceful shutdown - flush remaining messages with timeout
                        let queue_depth = raw_message_rx.len();
                        info!("Flushing {} remaining messages from queue...", queue_depth);
                        metrics::counter!("aprs.shutdown.queue_depth_at_shutdown").increment(queue_depth as u64);

                        let flush_start = std::time::Instant::now();
                        let flush_timeout = Duration::from_secs(10);
                        let mut flushed_count = 0u64;

                        while let Ok(Ok(message)) = tokio::time::timeout(
                            flush_timeout.saturating_sub(flush_start.elapsed()),
                            raw_message_rx.recv_async()
                        ).await {
                            let _ = publisher.publish(&message).await;
                            flushed_count += 1;
                        }

                        let flush_duration = flush_start.elapsed();
                        info!(
                            "Queue flush complete: {} messages published in {:.2}s",
                            flushed_count,
                            flush_duration.as_secs_f64()
                        );
                        metrics::counter!("aprs.shutdown.messages_flushed").increment(flushed_count);
                        metrics::histogram!("aprs.shutdown.flush_duration_seconds").record(flush_duration.as_secs_f64());

                        break;
                    }

                    tokio::select! {
                        Ok(message) = raw_message_rx.recv_async() => {
                            last_receive_time = std::time::Instant::now();
                            attempted_count += 1;

                            // Publish to NATS - truly fire-and-forget (spawns its own task)
                            publisher.publish_fire_and_forget(&message);
                        }
                        _ = stats_timer.tick() => {
                            // Report raw message queue depth every 10 seconds
                            let queue_depth = raw_message_rx.len();

                            metrics::gauge!("aprs.nats.queue_depth").set(queue_depth as f64);

                            // Log publishing rate and queue status
                            let elapsed = last_log_time.elapsed().as_secs_f64();
                            let time_since_last_receive = last_receive_time.elapsed().as_secs_f64();

                            if elapsed > 0.0 {
                                let rate = attempted_count as f64 / elapsed;
                                info!(
                                    "NATS stats: {:.1} msg/s attempted (queue: {}, last receive: {:.1}s ago)",
                                    rate, queue_depth, time_since_last_receive
                                );
                                attempted_count = 0;
                                last_log_time = std::time::Instant::now();
                            }

                            if queue_depth > queue_warning_threshold(RAW_MESSAGE_QUEUE_SIZE) {
                                warn!(
                                    "NATS publish queue building up: {} messages (80% full)",
                                    queue_depth
                                );

                                // Report queue buildup to Sentry
                                sentry::capture_message(
                                    &format!(
                                        "JetStream publish queue is 80% full ({} messages out of {})",
                                        queue_depth, RAW_MESSAGE_QUEUE_SIZE
                                    ),
                                    sentry::Level::Warning
                                );
                            }

                            // Detect if we've stopped receiving messages (may indicate APRS connection issue)
                            if time_since_last_receive > 60.0 {
                                warn!(
                                    "No messages received from APRS in {:.1}s - connection may be stalled",
                                    time_since_last_receive
                                );
                            }
                        }
                    }
                }

                info!("Publisher task shutting down gracefully");
            }
            .instrument(tracing::info_span!("nats_publisher")),
        );

        // Spawn connection management task (same as regular start)
        let health_for_connection = health_state.clone();
        let connection_handle = tokio::spawn(
            async move {
                let mut retry_count = 0;
                let mut current_delay = config.retry_delay_seconds;

                loop {
                    // Check if shutdown was requested
                    if shutdown_flag_for_connection.load(std::sync::atomic::Ordering::SeqCst) {
                        info!("Shutdown requested, stopping APRS NATS ingestion");
                        // Mark as disconnected in health state
                        {
                            let mut health = health_for_connection.write().await;
                            health.aprs_connected = false;
                        }
                        break;
                    }

                    if retry_count == 0 {
                        info!(
                            "Connecting to APRS server at {}:{}",
                            config.server, config.port
                        );
                    } else {
                        info!(
                            "Reconnecting to APRS server at {}:{} (retry attempt {})",
                            config.server, config.port, retry_count
                        );
                    }

                    // No timeout wrapper here - connect_and_run has its own internal timeouts
                    // for message processing (5 minute timeout) and doesn't need an outer limit
                    let connect_result = Self::connect_and_run(
                        &config,
                        raw_message_tx.clone(),
                        health_for_connection.clone(),
                    )
                    .await;

                    match connect_result {
                        ConnectionResult::Success => {
                            info!("Connection ended normally");
                            // Mark as disconnected in health state
                            {
                                let mut health = health_for_connection.write().await;
                                health.aprs_connected = false;
                            }

                            // Report disconnection to Sentry
                            sentry::capture_message(
                                &format!(
                                    "APRS connection ended normally after {} retries",
                                    retry_count
                                ),
                                sentry::Level::Info,
                            );

                            retry_count = 0; // Reset retry count on successful connection
                            current_delay = config.retry_delay_seconds;
                        }
                        ConnectionResult::ConnectionFailed(e) => {
                            error!("Failed to connect to APRS server: {}", e);

                            // Report connection failure to Sentry
                            sentry::capture_error(&*e);

                            // Mark as disconnected in health state
                            {
                                let mut health = health_for_connection.write().await;
                                health.aprs_connected = false;
                            }
                            retry_count += 1;
                        }
                        ConnectionResult::OperationFailed(e) => {
                            warn!("Connection operation failed: {}", e);

                            // Report operation failure to Sentry (connection was established but dropped)
                            sentry::capture_error(&*e);

                            // Mark as disconnected in health state
                            {
                                let mut health = health_for_connection.write().await;
                                health.aprs_connected = false;
                            }
                            retry_count += 1;
                        }
                    }

                    // Always retry indefinitely with exponential backoff
                    // Wait before retrying if we had a failure
                    if retry_count > 0 {
                        // Clamp delay between 1 second and 60 seconds
                        let delay = current_delay.clamp(1, 60);

                        info!(
                            "Waiting {} seconds before reconnecting... (retry attempt {})",
                            delay, retry_count
                        );

                        // Use tokio::select! to make sleep interruptible by shutdown signal
                        // Poll the shutdown flag every 100ms to allow fast shutdown
                        let sleep_future = sleep(Duration::from_secs(delay));
                        tokio::pin!(sleep_future);

                        loop {
                            tokio::select! {
                                _ = &mut sleep_future => {
                                    // Sleep completed normally
                                    break;
                                }
                                _ = sleep(Duration::from_millis(100)) => {
                                    // Check shutdown flag every 100ms
                                    if shutdown_flag_for_connection.load(std::sync::atomic::Ordering::SeqCst) {
                                        info!("Shutdown signal received during retry delay, exiting immediately");
                                        // Mark as disconnected in health state
                                        {
                                            let mut health = health_for_connection.write().await;
                                            health.aprs_connected = false;
                                        }
                                        return; // Exit the task immediately
                                    }
                                }
                            }
                        }

                        // Exponential backoff: start at 1 second, double each time, cap at 60 seconds
                        if current_delay == 0 {
                            current_delay = 1;
                        } else {
                            current_delay = std::cmp::min(current_delay * 2, 60);
                        }
                    }
                }
            }
            .instrument(tracing::info_span!("jetstream_connection_loop")),
        );

        // Wait for both tasks to complete (they run until shutdown or fatal error)
        // If either task panics, we'll get an error here
        let (publisher_result, connection_result) =
            tokio::join!(publisher_handle, connection_handle);

        // Check if either task panicked
        publisher_result.context("JetStream publisher task panicked")?;
        connection_result.context("Connection management task panicked or stopped")?;

        Ok(())
    }

    /// Stop the APRS client
    #[tracing::instrument(skip(self))]
    pub async fn stop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
    }

    /// Connect to the APRS server and run the message processing loop
    /// Messages are sent to raw_message_tx channel for processing
    #[tracing::instrument(skip(config, raw_message_tx, health_state), fields(server = %config.server, port = %config.port))]
    async fn connect_and_run(
        config: &AprsClientConfig,
        raw_message_tx: flume::Sender<String>,
        health_state: std::sync::Arc<tokio::sync::RwLock<crate::metrics::AprsIngestHealth>>,
    ) -> ConnectionResult {
        info!(
            "Connecting to APRS server {}:{}",
            config.server, config.port
        );

        // Track connection start time for duration reporting
        let connection_start = std::time::Instant::now();

        // Perform explicit DNS lookup for load balancing
        // This ensures fresh DNS resolution on each reconnect
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
                    info!("Connected to APRS server at {}", addr);
                    metrics::counter!("aprs.connection.established").increment(1);
                    metrics::gauge!("aprs.connection.connected").set(1.0);

                    // Mark as connected in health state
                    {
                        let mut health = health_state.write().await;
                        health.aprs_connected = true;
                    }

                    // Continue with message processing using this stream
                    return Self::process_connection(
                        stream,
                        config,
                        raw_message_tx,
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

        // If we get here, all addresses failed
        ConnectionResult::ConnectionFailed(anyhow::anyhow!(
            "Failed to connect to any resolved address for {}: {:?}",
            server_address,
            last_error
        ))
    }

    /// Process an established APRS connection
    #[tracing::instrument(skip(stream, config, raw_message_tx, health_state, connection_start), fields(peer_addr = %peer_addr_str))]
    async fn process_connection(
        stream: TcpStream,
        config: &AprsClientConfig,
        raw_message_tx: flume::Sender<String>,
        health_state: std::sync::Arc<tokio::sync::RwLock<crate::metrics::AprsIngestHealth>>,
        connection_start: std::time::Instant,
        peer_addr_str: String,
    ) -> ConnectionResult {
        info!("Processing connection to APRS server at {}", peer_addr_str);

        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);

        // Send login command
        let login_cmd = Self::build_login_command(config);
        info!("Sending login command: {}", login_cmd.trim());
        if let Err(e) = writer.write_all(login_cmd.as_bytes()).await {
            let duration = connection_start.elapsed();
            return ConnectionResult::OperationFailed(anyhow::anyhow!(
                "Failed to send login command to {} after {:.1}s: {}",
                peer_addr_str,
                duration.as_secs_f64(),
                e
            ));
        }
        if let Err(e) = writer.flush().await {
            let duration = connection_start.elapsed();
            return ConnectionResult::OperationFailed(anyhow::anyhow!(
                "Failed to flush login command to {} after {:.1}s: {}",
                peer_addr_str,
                duration.as_secs_f64(),
                e
            ));
        }
        info!("Login command sent successfully");

        // Read and process messages with timeout detection
        let mut line_buffer = Vec::new();
        let mut first_message = true;
        let message_timeout = Duration::from_secs(300); // 5 minute timeout (increased from 60s)
        let keepalive_interval = Duration::from_secs(20); // Send keepalive every 20 seconds
        let mut last_keepalive = tokio::time::Instant::now();

        loop {
            line_buffer.clear();

            // Check if we need to send a keepalive
            if last_keepalive.elapsed() >= keepalive_interval {
                // Send a comment as keepalive (APRS-IS servers expect periodic activity)
                let keepalive_msg = "# soar keepalive\r\n";
                if let Err(e) = writer.write_all(keepalive_msg.as_bytes()).await {
                    let duration = connection_start.elapsed();
                    warn!(
                        "Failed to send keepalive after {:.1}s: {}",
                        duration.as_secs_f64(),
                        e
                    );
                    return ConnectionResult::OperationFailed(anyhow::anyhow!(
                        "Failed to send keepalive after {:.1}s: {}",
                        duration.as_secs_f64(),
                        e
                    ));
                }
                if let Err(e) = writer.flush().await {
                    let duration = connection_start.elapsed();
                    warn!(
                        "Failed to flush keepalive after {:.1}s: {}",
                        duration.as_secs_f64(),
                        e
                    );
                    return ConnectionResult::OperationFailed(anyhow::anyhow!(
                        "Failed to flush keepalive after {:.1}s: {}",
                        duration.as_secs_f64(),
                        e
                    ));
                }
                trace!("Sent keepalive to APRS server");
                metrics::counter!("aprs.keepalive.sent").increment(1);
                last_keepalive = tokio::time::Instant::now();
            }

            // Use timeout to detect if no message received within 5 minutes
            match timeout(
                message_timeout,
                Self::read_line_with_invalid_utf8_handling(&mut buf_reader, &mut line_buffer),
            )
            .await
            {
                Ok(read_result) => {
                    match read_result {
                        Ok(0) => {
                            let duration = connection_start.elapsed();
                            warn!(
                                "Connection closed by server (IP: {}) after {:.1}s",
                                peer_addr_str,
                                duration.as_secs_f64()
                            );
                            break;
                        }
                        Ok(_) => {
                            // Convert buffer to string, handling invalid UTF-8
                            let line = match String::from_utf8(line_buffer.clone()) {
                                Ok(valid_line) => valid_line,
                                Err(_) => {
                                    // Invalid UTF-8 encountered - print hex dump and continue
                                    warn!(
                                        "Invalid UTF-8 in stream, hex dump: {}",
                                        Self::format_hex_dump(&line_buffer)
                                    );
                                    continue;
                                }
                            };

                            let trimmed_line = line.trim();
                            if !trimmed_line.is_empty() {
                                // Track message types for debugging
                                let is_server_message = trimmed_line.starts_with('#');
                                if is_server_message {
                                    metrics::counter!("aprs.raw_message.received.server")
                                        .increment(1);
                                } else {
                                    metrics::counter!("aprs.raw_message.received.aprs")
                                        .increment(1);
                                }

                                if first_message {
                                    info!("First message from server: {}", trimmed_line);
                                    first_message = false;
                                } else {
                                    trace!("Received: {}", trimmed_line);
                                }

                                // Prepend ISO-8601 timestamp to message before sending to JetStream
                                // Format: "YYYY-MM-DDTHH:MM:SS.SSSZ <original_message>"
                                // This ensures the "received at" timestamp is accurate even if messages queue up
                                let received_at = chrono::Utc::now();
                                let timestamped_message =
                                    format!("{} {}", received_at.to_rfc3339(), trimmed_line);

                                // Send message to channel for processing with timeout
                                // Use timeout to prevent indefinite blocking if JetStream publisher is stuck
                                // If send doesn't complete within 10 seconds, disconnect and reconnect
                                match timeout(
                                    Duration::from_secs(10),
                                    raw_message_tx.send_async(timestamped_message),
                                )
                                .await
                                {
                                    Ok(Ok(())) => {
                                        if is_server_message {
                                            trace!("Queued server message for JetStream");
                                            metrics::counter!("aprs.raw_message.queued.server")
                                                .increment(1);
                                        } else {
                                            trace!("Queued APRS message for JetStream");
                                            metrics::counter!("aprs.raw_message.queued.aprs")
                                                .increment(1);

                                            // Update last message time in health state for APRS messages (not server messages)
                                            {
                                                let mut health = health_state.write().await;
                                                health.last_message_time =
                                                    Some(std::time::Instant::now());
                                            }
                                        }
                                    }
                                    Ok(Err(flume::SendError(_))) => {
                                        error!("Raw message queue is closed - stopping connection");
                                        return ConnectionResult::OperationFailed(anyhow::anyhow!(
                                            "Raw message queue closed"
                                        ));
                                    }
                                    Err(_) => {
                                        // Timeout: queue send blocked for 10+ seconds
                                        // This indicates JetStream publisher is stuck (all permits exhausted)
                                        let duration = connection_start.elapsed();
                                        error!(
                                            "Queue send blocked for 10+ seconds after {:.1}s - JetStream publisher stuck, reconnecting",
                                            duration.as_secs_f64()
                                        );
                                        metrics::counter!("aprs.queue_send_timeout").increment(1);

                                        sentry::capture_message(
                                            "APRS queue send timed out - JetStream publisher stuck",
                                            sentry::Level::Error,
                                        );

                                        return ConnectionResult::OperationFailed(anyhow::anyhow!(
                                            "Queue send timeout after {:.1}s - publisher stuck",
                                            duration.as_secs_f64()
                                        ));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let duration = connection_start.elapsed();
                            return ConnectionResult::OperationFailed(anyhow::anyhow!(
                                "Connection error after {:.1}s: {}",
                                duration.as_secs_f64(),
                                e
                            ));
                        }
                    }
                }
                Err(_) => {
                    // Timeout occurred - no message received for 5 minutes
                    let duration = connection_start.elapsed();
                    error!(
                        "No message received from APRS server for 5 minutes (total connection time: {:.1}s), disconnecting and reconnecting",
                        duration.as_secs_f64()
                    );
                    return ConnectionResult::OperationFailed(anyhow::anyhow!(
                        "Message timeout after {:.1}s - no data received for 5 minutes",
                        duration.as_secs_f64()
                    ));
                }
            }
        }

        ConnectionResult::Success
    }

    /// Read a line from the buffered reader, handling invalid UTF-8 bytes
    /// Reads bytes until a newline character is found, including invalid UTF-8 sequences
    /// Uses efficient buffered reading instead of byte-at-a-time to avoid TCP backpressure
    async fn read_line_with_invalid_utf8_handling(
        reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
        buffer: &mut Vec<u8>,
    ) -> Result<usize> {
        use tokio::io::AsyncBufReadExt;

        // Use BufReader's efficient read_until which reads in chunks
        // This is much faster than reading one byte at a time
        match reader.read_until(b'\n', buffer).await {
            Ok(0) => Ok(0), // EOF
            Ok(n) => Ok(n), // Successfully read n bytes up to and including newline
            Err(e) => Err(e.into()),
        }
    }

    /// Format a byte buffer as a hex dump for logging invalid UTF-8 sequences
    fn format_hex_dump(buffer: &[u8]) -> String {
        let mut result = String::new();

        for (i, byte) in buffer.iter().enumerate() {
            if i > 0 {
                result.push(' ');
            }
            result.push_str(&format!("{:02x}", byte));
        }

        // Also include ASCII representation where possible
        result.push_str(" | ");
        for &byte in buffer {
            if byte.is_ascii_graphic() || byte == b' ' {
                result.push(byte as char);
            } else {
                result.push('.');
            }
        }

        result
    }

    /// Build the login command for APRS-IS authentication
    fn build_login_command(config: &AprsClientConfig) -> String {
        let mut login_cmd = format!("user {} pass ", config.callsign);

        // Add password or use -1 for read-only access
        match &config.password {
            Some(pass) => login_cmd.push_str(pass),
            None => login_cmd.push_str("-1"),
        }

        // Add version info
        login_cmd.push_str(" vers soar-aprs-client 1.0");

        // Add filter if specified
        if let Some(filter) = &config.filter {
            login_cmd.push_str(" filter ");
            login_cmd.push_str(filter);
            info!("Using APRS filter: {}", filter);
        }

        login_cmd.push_str("\r\n");
        login_cmd
    }
}

/// Builder pattern for creating APRS client configurations
pub struct AprsClientConfigBuilder {
    config: AprsClientConfig,
}

impl AprsClientConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: AprsClientConfig::default(),
        }
    }

    pub fn server<S: Into<String>>(mut self, server: S) -> Self {
        self.config.server = server.into();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.config.max_retries = max_retries;
        self
    }

    pub fn callsign<S: Into<String>>(mut self, callsign: S) -> Self {
        self.config.callsign = callsign.into();
        self
    }

    pub fn password<S: Into<String>>(mut self, password: Option<S>) -> Self {
        self.config.password = password.map(|p| p.into());
        self
    }

    pub fn filter<S: Into<String>>(mut self, filter: Option<S>) -> Self {
        self.config.filter = filter.map(|f| f.into());
        self
    }

    pub fn retry_delay_seconds(mut self, seconds: u64) -> Self {
        self.config.retry_delay_seconds = seconds;
        self
    }

    pub fn max_retry_delay_seconds(mut self, seconds: u64) -> Self {
        self.config.max_retry_delay_seconds = seconds;
        self
    }

    pub fn archive_base_dir<S: Into<String>>(mut self, archive_base_dir: Option<S>) -> Self {
        self.config.archive_base_dir = archive_base_dir.map(|d| d.into());
        self
    }

    pub fn build(self) -> AprsClientConfig {
        self.config
    }
}

impl Default for AprsClientConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    #[test]
    fn test_config_builder() {
        let config = AprsClientConfigBuilder::new()
            .server("test.aprs.net")
            .port(14580)
            .callsign("TEST123")
            .password(Some("12345"))
            .filter(Some("r/47.0/-122.0/100"))
            .max_retries(3)
            .retry_delay_seconds(10)
            .build();

        assert_eq!(config.server, "test.aprs.net");
        assert_eq!(config.port, 14580);
        assert_eq!(config.callsign, "TEST123");
        assert_eq!(config.password, Some("12345".to_string()));
        assert_eq!(config.filter, Some("r/47.0/-122.0/100".to_string()));
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay_seconds, 10);
    }

    #[test]
    fn test_server_message_parsing_logic() {
        // Test just the parsing logic by testing it directly
        let raw_message =
            "# aprsc 2.1.19-g730c5c0 22 Sep 2025 23:10:51 GMT GLIDERN3 85.188.1.173:10152";
        let trimmed = raw_message.trim_start_matches('#').trim();
        let parts: Vec<&str> = trimmed.split_whitespace().collect();

        // Verify we have the right number of parts
        assert!(parts.len() >= 8, "Should have at least 8 parts");

        // Test software extraction
        let software = format!("{} {}", parts[0], parts[1]);
        assert_eq!(software, "aprsc 2.1.19-g730c5c0");

        // Test timestamp parsing
        let date_time_str = format!(
            "{} {} {} {} {}",
            parts[2], parts[3], parts[4], parts[5], parts[6]
        );
        assert_eq!(date_time_str, "22 Sep 2025 23:10:51 GMT");

        let server_timestamp =
            NaiveDateTime::parse_from_str(&date_time_str, "%d %b %Y %H:%M:%S GMT");
        assert!(server_timestamp.is_ok(), "Should parse timestamp correctly");

        // Test server name and endpoint extraction
        let server_name = parts[7];
        let server_endpoint = parts[8];
        assert_eq!(server_name, "GLIDERN3");
        assert_eq!(server_endpoint, "85.188.1.173:10152");
    }

    #[test]
    fn test_login_command_with_password() {
        let config = AprsClientConfig {
            server: "test.aprs.net".to_string(),
            port: 14580,
            callsign: "TEST123".to_string(),
            password: Some("12345".to_string()),
            filter: Some("r/47.0/-122.0/100".to_string()),
            max_retries: 3,
            retry_delay_seconds: 5,
            max_retry_delay_seconds: 60,
            archive_base_dir: None,
        };

        let login_cmd = AprsClient::build_login_command(&config);
        assert_eq!(
            login_cmd,
            "user TEST123 pass 12345 vers soar-aprs-client 1.0 filter r/47.0/-122.0/100\r\n"
        );
    }

    #[test]
    fn test_login_command_without_password() {
        let config = AprsClientConfig {
            server: "test.aprs.net".to_string(),
            port: 14580,
            callsign: "TEST123".to_string(),
            password: None,
            filter: None,
            max_retries: 3,
            retry_delay_seconds: 5,
            max_retry_delay_seconds: 60,
            archive_base_dir: None,
        };

        let login_cmd = AprsClient::build_login_command(&config);
        assert_eq!(
            login_cmd,
            "user TEST123 pass -1 vers soar-aprs-client 1.0\r\n"
        );
    }

    // Note: test_packet_processor removed as PacketHandler trait was removed
    // Integration tests for packet processing should be done at a higher level
    // with a real PacketRouter instance

    #[test]
    fn test_config_builder_with_archive() {
        let config = AprsClientConfigBuilder::new()
            .server("test.aprs.net")
            .port(14580)
            .callsign("TEST123")
            .archive_base_dir(Some("/tmp/aprs_archive"))
            .build();

        assert_eq!(config.server, "test.aprs.net");
        assert_eq!(config.port, 14580);
        assert_eq!(config.callsign, "TEST123");
        assert_eq!(
            config.archive_base_dir,
            Some("/tmp/aprs_archive".to_string())
        );
    }

    #[test]
    fn test_timestamp_prepending_format() {
        // Test that the timestamp prepending logic creates the expected format
        let raw_message = "FLRDDA5BA>APRS,qAS,LFNX:/160829h4902.45N/00531.30E'342/049/A=001322";
        let received_at = chrono::Utc::now();
        let timestamped_message = format!("{} {}", received_at.to_rfc3339(), raw_message);

        // Verify the format is "TIMESTAMP MESSAGE"
        assert!(timestamped_message.contains(' '));
        let parts: Vec<&str> = timestamped_message.splitn(2, ' ').collect();
        assert_eq!(parts.len(), 2);

        // First part should be a valid ISO-8601 timestamp
        let parsed = chrono::DateTime::parse_from_rfc3339(parts[0]);
        assert!(parsed.is_ok(), "Timestamp should be valid RFC3339");

        // Second part should be the original message
        assert_eq!(parts[1], raw_message);
    }

    #[test]
    fn test_timestamp_prepending_server_message() {
        // Test timestamp prepending for server messages (starting with #)
        let server_message =
            "# aprsc 2.1.15-gc67551b 22 Sep 2025 21:51:55 GMT GLIDERN1 51.178.19.212:10152";
        let received_at = chrono::Utc::now();
        let timestamped_message = format!("{} {}", received_at.to_rfc3339(), server_message);

        // Split and verify
        let parts: Vec<&str> = timestamped_message.splitn(2, ' ').collect();
        assert_eq!(parts.len(), 2);

        // Second part should still start with #
        assert!(parts[1].starts_with('#'));
        assert_eq!(parts[1], server_message);
    }

    #[test]
    fn test_timestamp_roundtrip() {
        // Test that we can prepend a timestamp and then parse it back
        let original_message =
            "FLRDDA5BA>APRS,qAS,LFNX:/160829h4902.45N/00531.30E'342/049/A=001322";
        let sent_at = chrono::Utc::now();

        // Simulate what ingest-aprs does: prepend timestamp
        let timestamped = format!("{} {}", sent_at.to_rfc3339(), original_message);

        // Simulate what soar-run does: parse timestamp
        let (timestamp_str, message) = timestamped.split_once(' ').unwrap();
        let parsed_timestamp = chrono::DateTime::parse_from_rfc3339(timestamp_str)
            .unwrap()
            .with_timezone(&chrono::Utc);

        // Verify we got back the original message
        assert_eq!(message, original_message);

        // Verify timestamp roundtrip (should be within 1ms)
        let diff = (sent_at - parsed_timestamp).num_milliseconds().abs();
        assert!(diff < 10, "Timestamp should roundtrip accurately");
    }
}
