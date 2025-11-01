use crate::queue_config::{ARCHIVE_QUEUE_SIZE, RAW_MESSAGE_QUEUE_SIZE, queue_warning_threshold};
use anyhow::{Context, Result};
use std::time::Duration;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};
use tracing::Instrument;
use tracing::{error, info, trace, warn};

// AprsClient only publishes raw messages to JetStream - all parsing happens in the consumer

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
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl AprsClient {
    /// Create a new APRS client
    pub fn new(config: AprsClientConfig) -> Self {
        Self {
            config,
            shutdown_tx: None,
        }
    }

    /// Start the APRS client with JetStream publisher
    /// This connects to APRS-IS and publishes all messages to a durable NATS JetStream queue
    #[tracing::instrument(skip(self, jetstream_publisher))]
    pub async fn start_jetstream(
        &mut self,
        jetstream_publisher: crate::aprs_jetstream_publisher::JetStreamPublisher,
    ) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let config = self.config.clone();

        // Create bounded channel for raw APRS messages from TCP socket
        let (raw_message_tx, mut raw_message_rx) = mpsc::channel::<String>(RAW_MESSAGE_QUEUE_SIZE);
        info!(
            "Created raw message queue with capacity {} for JetStream publishing",
            RAW_MESSAGE_QUEUE_SIZE
        );

        // Spawn message publishing task
        let publisher = jetstream_publisher.clone();
        let publisher_handle = tokio::spawn(
            async move {
                let mut stats_timer = tokio::time::interval(Duration::from_secs(10));
                stats_timer.tick().await; // First tick completes immediately

                let mut published_count = 0u64;
                let mut last_log_time = std::time::Instant::now();

                loop {
                    tokio::select! {
                        Some(message) = raw_message_rx.recv() => {
                            // Publish in fire-and-forget mode for maximum throughput
                            // We accept the risk of message loss during crashes since:
                            // 1. APRS is a continuous stream - we'll get new data
                            // 2. Process restarts lose queued messages anyway
                            // 3. JetStream persists messages once received
                            publisher.publish_fire_and_forget(&message).await;
                            published_count += 1;
                        }
                        _ = stats_timer.tick() => {
                            // Report raw message queue depth every 10 seconds
                            let queue_depth = raw_message_rx.len();
                            metrics::gauge!("aprs.jetstream.queue_depth").set(queue_depth as f64);

                            // Log publishing rate
                            let elapsed = last_log_time.elapsed().as_secs_f64();
                            if elapsed > 0.0 {
                                let rate = published_count as f64 / elapsed;
                                info!("JetStream publishing rate: {:.1} msg/s (queue depth: {})", rate, queue_depth);
                                published_count = 0;
                                last_log_time = std::time::Instant::now();
                            }

                            if queue_depth > queue_warning_threshold(RAW_MESSAGE_QUEUE_SIZE) {
                                warn!(
                                    "JetStream publish queue building up: {} messages (80% full)",
                                    queue_depth
                                );
                            }
                        }
                    }
                }
            }
            .instrument(tracing::info_span!("jetstream_publisher")),
        );

        // Spawn connection management task (same as regular start)
        let connection_handle = tokio::spawn(
            async move {
                let mut retry_count = 0;
                let mut current_delay = config.retry_delay_seconds;

                loop {
                    // Check if shutdown was requested
                    if shutdown_rx.try_recv().is_ok() {
                        info!("Shutdown requested, stopping APRS JetStream ingestion");
                        break;
                    }

                    info!(
                        "Connecting to APRS server at {}:{} (attempt {}/{})",
                        config.server,
                        config.port,
                        retry_count + 1,
                        config.max_retries
                    );

                    match Self::connect_and_run(&config, raw_message_tx.clone()).await {
                        ConnectionResult::Success => {
                            info!("Connection ended normally");
                            retry_count = 0; // Reset retry count on successful connection
                            current_delay = config.retry_delay_seconds;
                        }
                        ConnectionResult::ConnectionFailed(e) => {
                            error!("Failed to connect to APRS server: {}", e);
                            retry_count += 1;
                        }
                        ConnectionResult::OperationFailed(e) => {
                            warn!("Connection operation failed: {}", e);
                            retry_count += 1;
                        }
                    }

                    // Check if we should retry
                    if retry_count >= config.max_retries && config.max_retries > 0 {
                        error!(
                            "Maximum retry attempts ({}) reached. Stopping.",
                            config.max_retries
                        );
                        break;
                    }

                    // Wait before retrying with exponential backoff
                    if retry_count > 0 {
                        info!(
                            "Waiting {} seconds before reconnecting... (attempt {}/{})",
                            current_delay,
                            retry_count + 1,
                            config.max_retries
                        );
                        sleep(Duration::from_secs(current_delay)).await;

                        // Exponential backoff with maximum cap
                        current_delay =
                            std::cmp::min(current_delay * 2, config.max_retry_delay_seconds);
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
            let _ = shutdown_tx.send(()).await;
        }
    }

    /// Connect to the APRS server and run the message processing loop
    /// Messages are sent to raw_message_tx channel for processing
    #[tracing::instrument(skip(config, raw_message_tx), fields(server = %config.server, port = %config.port))]
    async fn connect_and_run(
        config: &AprsClientConfig,
        raw_message_tx: mpsc::Sender<String>,
    ) -> ConnectionResult {
        info!(
            "Connecting to APRS server {}:{}",
            config.server, config.port
        );

        // Track connection start time for duration reporting
        let connection_start = std::time::Instant::now();

        // Connect to the APRS server
        let stream = match TcpStream::connect(format!("{}:{}", config.server, config.port)).await {
            Ok(stream) => {
                info!("Connected to APRS server");
                metrics::counter!("aprs.connection.established").increment(1);
                metrics::gauge!("aprs.connection.connected").set(1.0);
                stream
            }
            Err(e) => {
                return ConnectionResult::ConnectionFailed(e.into());
            }
        };

        // Log the resolved IP address we're actually connected to
        let peer_addr = stream.peer_addr().ok();
        let peer_addr_str = peer_addr
            .map(|a| a.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        info!("Connected to APRS server (resolved IP: {})", peer_addr_str);
        metrics::counter!("aprs.connection.established").increment(1);

        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);

        // Send login command
        let login_cmd = Self::build_login_command(config);
        info!("Sending login command: {}", login_cmd.trim());
        if let Err(e) = writer.write_all(login_cmd.as_bytes()).await {
            let duration = connection_start.elapsed();
            return ConnectionResult::OperationFailed(anyhow::anyhow!(
                "Failed to send login command after {:.1}s: {}",
                duration.as_secs_f64(),
                e
            ));
        }
        if let Err(e) = writer.flush().await {
            let duration = connection_start.elapsed();
            return ConnectionResult::OperationFailed(anyhow::anyhow!(
                "Failed to flush login command after {:.1}s: {}",
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

                                // Send message to channel for processing (non-blocking)
                                match raw_message_tx.try_send(trimmed_line.to_string()) {
                                    Ok(()) => {
                                        if is_server_message {
                                            trace!("Queued server message for JetStream");
                                            metrics::counter!("aprs.raw_message.queued.server")
                                                .increment(1);
                                        } else {
                                            trace!("Queued APRS message for JetStream");
                                            metrics::counter!("aprs.raw_message.queued.aprs")
                                                .increment(1);
                                        }
                                    }
                                    Err(mpsc::error::TrySendError::Full(_)) => {
                                        warn!(
                                            "Raw message queue FULL ({} messages buffered) - dropping message. \
                                             This indicates JetStream publishing is slower than APRS message rate.",
                                            RAW_MESSAGE_QUEUE_SIZE
                                        );
                                        metrics::counter!("aprs.raw_message_queue.full")
                                            .increment(1);
                                        // Message is dropped to prevent blocking TCP reads
                                    }
                                    Err(mpsc::error::TrySendError::Closed(_)) => {
                                        error!("Raw message queue is closed - stopping connection");
                                        return ConnectionResult::OperationFailed(anyhow::anyhow!(
                                            "Raw message queue closed"
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

/// Archive service for managing daily log files and compression
#[derive(Clone)]
pub struct ArchiveService {
    sender: mpsc::Sender<String>,
}

impl ArchiveService {
    /// Create a new archive service and start the background archival task
    pub async fn new(base_dir: String) -> Result<Self> {
        use chrono::Local;
        use std::fs;
        use std::io::Write;
        use std::path::PathBuf;

        // Create base directory if it doesn't exist
        fs::create_dir_all(&base_dir)?;

        // Check for yesterday's uncompressed file and compress it
        let yesterday = Local::now().date_naive() - chrono::Duration::days(1);
        let yesterday_file = PathBuf::from(&base_dir).join(format!("{}.log", yesterday));
        if yesterday_file.exists() {
            info!(
                "Found uncompressed file from yesterday: {:?}, compressing...",
                yesterday_file
            );
            if let Err(e) = Self::compress_file(&yesterday_file).await {
                warn!("Failed to compress yesterday's file: {}", e);
            }
        }

        // Use bounded channel to prevent unbounded memory growth
        // Should handle bursts while limiting memory to ~1.5MB (assuming ~150 bytes per APRS message)
        let (sender, mut receiver) = mpsc::channel::<String>(ARCHIVE_QUEUE_SIZE);

        info!(
            "Archive service initialized with bounded channel (capacity: {} messages, ~1.5MB buffer)",
            ARCHIVE_QUEUE_SIZE
        );

        // Spawn background task for file writing and management
        tokio::spawn(async move {
            let mut current_file: Option<std::io::BufWriter<std::fs::File>> = None;
            let mut current_date: Option<chrono::NaiveDate> = None;
            let mut messages_written = 0u64;
            let mut messages_since_flush = 0u64;
            let mut last_stats_log = std::time::Instant::now();
            let mut last_flush = std::time::Instant::now();

            while let Some(message) = receiver.recv().await {
                let now = Local::now();
                let today = now.date_naive();

                // Check if we need to rotate to a new file
                if current_date != Some(today) {
                    // Close current file if exists
                    if let Some(mut file) = current_file.take() {
                        if let Err(e) = file.flush() {
                            warn!("Failed to flush archive file: {}", e);
                        }
                        drop(file);

                        // Compress the previous day's file
                        if let Some(prev_date) = current_date {
                            let prev_file =
                                PathBuf::from(&base_dir).join(format!("{}.log", prev_date));
                            tokio::spawn(async move {
                                if let Err(e) = Self::compress_file(&prev_file).await {
                                    warn!("Failed to compress archive file: {}", e);
                                }
                            });
                        }
                    }

                    // Open new file for today
                    let log_path = PathBuf::from(&base_dir).join(format!("{}.log", today));
                    match std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&log_path)
                    {
                        Ok(file) => {
                            info!("Opened new archive file: {:?}", log_path);
                            // Wrap file in BufWriter with 1MB buffer for efficient writes
                            current_file =
                                Some(std::io::BufWriter::with_capacity(1024 * 1024, file));
                            current_date = Some(today);
                            messages_since_flush = 0;
                            last_flush = std::time::Instant::now();
                        }
                        Err(e) => {
                            error!("Failed to open archive file {:?}: {}", log_path, e);
                            continue;
                        }
                    }
                }

                // Write message to current file
                if let Some(file) = &mut current_file {
                    if let Err(e) = writeln!(file, "{}", message) {
                        error!(
                            "Failed to write to archive file: {} - this may cause message backlog",
                            e
                        );
                    } else {
                        messages_written += 1;
                        messages_since_flush += 1;
                    }
                }

                // Flush buffer periodically to ensure data durability
                // Flush every 1000 messages OR every 30 seconds, whichever comes first
                let should_flush =
                    messages_since_flush >= 1000 || last_flush.elapsed().as_secs() >= 30;

                if should_flush && let Some(file) = &mut current_file {
                    let flush_start = std::time::Instant::now();
                    if let Err(e) = file.flush() {
                        error!("Failed to flush archive buffer: {}", e);
                    } else {
                        let flush_duration = flush_start.elapsed();

                        // Warn if flush takes more than 100ms (indicates I/O issues)
                        if flush_duration.as_millis() > 100 {
                            warn!(
                                "Slow archive flush detected: {}ms for {} messages - disk I/O may be bottleneck",
                                flush_duration.as_millis(),
                                messages_since_flush
                            );
                        }

                        messages_since_flush = 0;
                        last_flush = std::time::Instant::now();
                    }
                }

                // Log statistics every 5 minutes
                if last_stats_log.elapsed().as_secs() >= 300 {
                    let queue_len = receiver.len();
                    info!(
                        "Archive stats: {} messages written in last 5min, {} messages queued",
                        messages_written, queue_len
                    );
                    if queue_len > 5000 {
                        warn!(
                            "Archive queue is building up ({} messages) - disk writes may be too slow",
                            queue_len
                        );
                    }
                    messages_written = 0;
                    last_stats_log = std::time::Instant::now();
                }
            }

            // Flush final file on shutdown
            if let Some(mut file) = current_file {
                let _ = file.flush();
            }
        });

        Ok(Self { sender })
    }

    /// Compress a log file using zstd
    async fn compress_file(file_path: &std::path::PathBuf) -> Result<()> {
        use std::fs::File;
        use std::io::{BufReader, BufWriter};

        let compressed_path = file_path.with_extension("log.zst");

        // Read the original file
        let input_file = File::open(file_path)?;
        let reader = BufReader::new(input_file);

        // Create the compressed file
        let output_file = File::create(&compressed_path)?;
        let writer = BufWriter::new(output_file);

        // Compress with zstd (compression level 3 is a good balance)
        let mut encoder = zstd::Encoder::new(writer, 3)?;
        std::io::copy(&mut BufReader::new(reader), &mut encoder)?;
        encoder.finish()?;

        // Delete the original file after successful compression
        std::fs::remove_file(file_path)?;

        info!(
            "Compressed {:?} to {:?} and deleted original",
            file_path, compressed_path
        );

        Ok(())
    }

    /// Archive a message
    pub fn archive(&self, message: &str) {
        // Use try_send to avoid blocking the caller
        // This provides backpressure if the archive writer can't keep up
        match self.sender.try_send(message.to_string()) {
            Ok(_) => {
                // Message successfully queued
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                // Channel is full - indicates archive writer is falling behind
                // This is a sign of disk I/O issues or excessive message rate
                warn!(
                    "Archive channel is FULL (10,000 messages buffered) - dropping message. \
                     This indicates disk writes are slower than incoming APRS messages. \
                     Check disk I/O performance and consider increasing channel capacity."
                );
                // Message is dropped to prevent unbounded memory growth
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                // Archive service has shut down
                error!("Archive service channel is closed - cannot archive message");
            }
        }
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
}
