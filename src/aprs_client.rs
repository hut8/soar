use crate::packet_processors::PacketRouter;
use anyhow::Result;

/// Result type for connection attempts
enum ConnectionResult {
    /// Connection was successful and ran until completion/disconnection
    Success,
    /// Connection failed immediately (couldn't establish connection)
    ConnectionFailed(anyhow::Error),
    /// Connection was established but failed during operation
    OperationFailed(anyhow::Error),
}
use ogn_parser::{AprsData, AprsPacket};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};
use tracing::Instrument;
use tracing::trace;
use tracing::{error, info, warn};

/// Trait for processing APRS packets
/// Implementors can define custom logic for handling received APRS packets
pub trait PacketHandler: Send + Sync + std::any::Any {
    /// Process a received APRS packet
    ///
    /// # Arguments
    /// * `packet` - The parsed APRS packet
    fn process_packet(&self, _packet: AprsPacket) {
        // Default implementation does nothing
    }

    /// Process a raw APRS message (optional - for logging, archiving, etc.)
    ///
    /// # Arguments
    /// * `raw_message` - The raw APRS message string received from the server
    fn process_raw_message(&self, _raw_message: &str) {
        // Default implementation does nothing
    }

    /// Support for downcasting to concrete types
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Unified processor struct that contains the packet processor
#[derive(Clone)]
pub struct AprsProcessors {
    /// Processor for general APRS packets and raw message handling
    pub packet_processor: Arc<dyn PacketHandler>,
}

impl AprsProcessors {
    /// Create a new AprsProcessors with a PacketRouter
    pub fn with_packet_router(router: PacketRouter) -> Self {
        Self {
            packet_processor: Arc::new(router),
        }
    }
}

/// Type alias for boxed packet processor trait objects
pub type BoxedPacketProcessor = Box<dyn PacketHandler>;

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
    /// Delay between reconnection attempts in seconds
    pub retry_delay_seconds: u64,
    /// Base directory for message archive (optional)
    pub archive_base_dir: Option<String>,
    /// Path to CSV log file for unparsed APRS fragments (optional)
    pub unparsed_log_path: Option<String>,
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
            retry_delay_seconds: 5,
            archive_base_dir: None,
            unparsed_log_path: None,
        }
    }
}

/// APRS client that connects to an APRS-IS server via TCP
pub struct AprsClient {
    config: AprsClientConfig,
    processors: AprsProcessors,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl AprsClient {
    /// Create a new APRS client with unified processors
    /// This is the recommended constructor that takes an AprsProcessors struct
    pub fn new(config: AprsClientConfig, processors: AprsProcessors) -> Self {
        Self {
            config,
            processors,
            shutdown_tx: None,
        }
    }

    /// Create a new APRS client with a PacketRouter
    pub fn with_packet_router(config: AprsClientConfig, router: PacketRouter) -> Self {
        Self::new(config, AprsProcessors::with_packet_router(router))
    }

    /// Start the APRS client
    /// This will connect to the server and begin processing messages
    #[tracing::instrument(skip(self))]
    pub async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let config = self.config.clone();
        let processors = self.processors.clone();

        tokio::spawn(
            async move {
                let mut retry_count = 0;

                loop {
                    // Check if shutdown was requested
                    if shutdown_rx.try_recv().is_ok() {
                        info!("Shutdown requested, stopping APRS client");
                        break;
                    }

                    match Self::connect_and_run(&config, processors.clone()).await {
                        ConnectionResult::Success => {
                            info!("APRS client connection ended normally");
                            retry_count = 0; // Reset retry count on successful connection
                        }
                        ConnectionResult::ConnectionFailed(e) => {
                            error!("APRS client connection failed: {}", e);
                            retry_count += 1;

                            if retry_count >= config.max_retries {
                                error!(
                                    "Maximum retry attempts ({}) reached, stopping client",
                                    config.max_retries
                                );
                                break;
                            }

                            warn!(
                                "Retrying connection in {} seconds (attempt {}/{})",
                                config.retry_delay_seconds, retry_count, config.max_retries
                            );
                            sleep(Duration::from_secs(config.retry_delay_seconds)).await;
                        }
                        ConnectionResult::OperationFailed(e) => {
                            error!(
                                "APRS client operation failed after successful connection: {}",
                                e
                            );
                            // Reset retry count since connection was initially successful
                            retry_count = 0;

                            warn!(
                                "Retrying connection in {} seconds (connection was successful)",
                                config.retry_delay_seconds
                            );
                            sleep(Duration::from_secs(config.retry_delay_seconds)).await;
                        }
                    }
                }
            }
            .instrument(tracing::info_span!("aprs_client_connection_loop")),
        );

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
    #[tracing::instrument(skip(config, processors), fields(server = %config.server, port = %config.port))]
    async fn connect_and_run(
        config: &AprsClientConfig,
        processors: AprsProcessors,
    ) -> ConnectionResult {
        info!(
            "Connecting to APRS server {}:{}",
            config.server, config.port
        );

        // Connect to the APRS server
        let stream = match TcpStream::connect(format!("{}:{}", config.server, config.port)).await {
            Ok(stream) => {
                info!("Connected to APRS server");
                stream
            }
            Err(e) => {
                return ConnectionResult::ConnectionFailed(e.into());
            }
        };

        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);

        // Send login command
        let login_cmd = Self::build_login_command(config);
        info!("Sending login command: {}", login_cmd.trim());
        if let Err(e) = writer.write_all(login_cmd.as_bytes()).await {
            return ConnectionResult::OperationFailed(e.into());
        }
        if let Err(e) = writer.flush().await {
            return ConnectionResult::OperationFailed(e.into());
        }
        info!("Login command sent successfully");

        // Read and process messages with timeout detection
        let mut line_buffer = Vec::new();
        let mut first_message = true;
        let message_timeout = Duration::from_secs(60); // 1 minute timeout

        loop {
            line_buffer.clear();

            // Use timeout to detect if no message received within 1 minute
            match timeout(
                message_timeout,
                Self::read_line_with_invalid_utf8_handling(&mut buf_reader, &mut line_buffer),
            )
            .await
            {
                Ok(read_result) => {
                    match read_result {
                        Ok(0) => {
                            warn!("Connection closed by server");
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
                                if first_message {
                                    info!("First message from server: {}", trimmed_line);
                                    first_message = false;
                                } else {
                                    trace!("Received: {}", trimmed_line);
                                }
                                // Route server messages (lines starting with #) and regular APRS messages differently
                                if !trimmed_line.starts_with('#') {
                                    Self::process_message(trimmed_line, &processors, config).await;
                                } else {
                                    Self::process_server_message(trimmed_line, &processors).await;
                                }
                            }
                        }
                        Err(e) => {
                            return ConnectionResult::OperationFailed(e);
                        }
                    }
                }
                Err(_) => {
                    // Timeout occurred - no message received for 1 minute
                    error!(
                        "No message received from APRS server for 1 minute, disconnecting and reconnecting"
                    );
                    return ConnectionResult::OperationFailed(anyhow::anyhow!(
                        "Message timeout - no data received for 1 minute"
                    ));
                }
            }
        }

        ConnectionResult::Success
    }

    /// Read a line from the buffered reader, handling invalid UTF-8 bytes
    /// Reads bytes until a newline character is found, including invalid UTF-8 sequences
    async fn read_line_with_invalid_utf8_handling(
        reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
        buffer: &mut Vec<u8>,
    ) -> Result<usize> {
        let mut byte = [0u8; 1];
        let mut total_bytes_read = 0;

        loop {
            match reader.read(&mut byte).await {
                Ok(0) => {
                    // End of stream
                    if total_bytes_read == 0 {
                        return Ok(0); // No bytes read, stream closed
                    } else {
                        break; // Some bytes read before EOF
                    }
                }
                Ok(1) => {
                    total_bytes_read += 1;
                    buffer.push(byte[0]);

                    // Stop at newline character
                    if byte[0] == b'\n' {
                        break;
                    }
                }
                Ok(_) => {
                    // This shouldn't happen with a 1-byte buffer, but handle it just in case
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        Ok(total_bytes_read)
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

    /// Process a server message (line starting with #)
    async fn process_server_message(message: &str, processors: &AprsProcessors) {
        // First log the server message for backward compatibility
        info!("Server message: {}", message);

        // Try to process with PacketRouter if available
        // We need to use Any to check if it's a PacketRouter since we only have dyn PacketHandler
        if let Some(router) = processors
            .packet_processor
            .as_any()
            .downcast_ref::<PacketRouter>()
        {
            trace!("Successfully downcast to PacketRouter, processing server message");
            router.process_server_message(message).await;
        } else {
            trace!("Packet processor is not a PacketRouter, server message only logged");
        }
    }

    /// Process a received APRS message
    async fn process_message(
        message: &str,
        processors: &AprsProcessors,
        config: &AprsClientConfig,
    ) {
        // Always call process_raw_message first (for logging/archiving)
        processors.packet_processor.process_raw_message(message);

        // Try to parse the message using ogn-parser
        match ogn_parser::parse(message) {
            Ok(parsed) => {
                // Log unparsed fragments if present and configured
                Self::log_unparsed_fragments_if_configured(&parsed, message, config).await;

                // Dispatch the packet to the packet processor
                processors.packet_processor.process_packet(parsed);
            }
            Err(e) => {
                // For OGNFNT sources with invalid lat/lon, log as trace instead of error
                // These are common and expected issues with this data source
                let error_str = e.to_string();
                if message.contains("OGNFNT")
                    && (error_str.contains("Invalid Latitude")
                        || error_str.contains("Invalid Longitude"))
                {
                    trace!("Failed to parse APRS message '{message}': {e}");
                } else {
                    info!("Failed to parse APRS message '{message}': {e}");
                    // Log entire message to unparsed log if configured
                    if let Some(log_path) = &config.unparsed_log_path {
                        Self::log_unparsed_to_csv(log_path, "unparsed", message, message)
                            .await
                            .unwrap_or_else(|e| {
                                warn!("Failed to write to unparsed log: {}", e);
                            });
                    }
                }
            }
        }
    }

    /// Handle unparsed fragment logging if configured
    async fn log_unparsed_fragments_if_configured(
        packet: &AprsPacket,
        message: &str,
        config: &AprsClientConfig,
    ) {
        if let Some(log_path) = &config.unparsed_log_path {
            match &packet.data {
                AprsData::Position(pos) => {
                    if let Some(unparsed) = &pos.comment.unparsed {
                        Self::log_unparsed_to_csv(log_path, "position", unparsed, message)
                            .await
                            .unwrap_or_else(|e| {
                                warn!("Failed to write to unparsed log: {}", e);
                            });
                    }
                }
                AprsData::Status(status) => {
                    if let Some(unparsed) = &status.comment.unparsed {
                        Self::log_unparsed_to_csv(log_path, "status", unparsed, message)
                            .await
                            .unwrap_or_else(|e| {
                                warn!("Failed to write to unparsed log: {}", e);
                            })
                    }
                }
                _ => {}
            }
        }
    }

    /// Log unparsed APRS fragments to CSV file
    async fn log_unparsed_to_csv(
        log_path: &str,
        fragment_type: &str,
        unparsed_fragment: &str,
        whole_message: &str,
    ) -> Result<()> {
        // Escape CSV fields by wrapping in quotes and escaping internal quotes
        let escaped_fragment = unparsed_fragment.replace('"', "\"\"");
        let escaped_message = whole_message.replace('"', "\"\"");

        let csv_line = format!(
            "{},\"{}\",\"{}\"\n",
            fragment_type, escaped_fragment, escaped_message
        );

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .await?;

        file.write_all(csv_line.as_bytes()).await?;
        file.flush().await?;

        Ok(())
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

    pub fn archive_base_dir<S: Into<String>>(mut self, archive_base_dir: Option<S>) -> Self {
        self.config.archive_base_dir = archive_base_dir.map(|d| d.into());
        self
    }

    pub fn unparsed_log_path<S: Into<String>>(mut self, unparsed_log_path: Option<S>) -> Self {
        self.config.unparsed_log_path = unparsed_log_path.map(|p| p.into());
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
        // Capacity of 10,000 messages should handle bursts while limiting memory to ~1.5MB
        // (assuming ~150 bytes per APRS message)
        let (sender, mut receiver) = mpsc::channel::<String>(10_000);

        info!(
            "Archive service initialized with bounded channel (capacity: 10,000 messages, ~1.5MB buffer)"
        );

        // Spawn background task for file writing and management
        tokio::spawn(async move {
            let mut current_file: Option<std::fs::File> = None;
            let mut current_date: Option<chrono::NaiveDate> = None;
            let mut messages_written = 0u64;
            let mut last_stats_log = std::time::Instant::now();

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
                            current_file = Some(file);
                            current_date = Some(today);
                        }
                        Err(e) => {
                            error!("Failed to open archive file {:?}: {}", log_path, e);
                            continue;
                        }
                    }
                }

                // Write message to current file
                if let Some(file) = &mut current_file {
                    let write_start = std::time::Instant::now();
                    if let Err(e) = writeln!(file, "{}", message) {
                        error!(
                            "Failed to write to archive file: {} - this may cause message backlog",
                            e
                        );
                    } else {
                        messages_written += 1;
                        let write_duration = write_start.elapsed();

                        // Warn if a single write takes more than 100ms (indicates I/O issues)
                        if write_duration.as_millis() > 100 {
                            warn!(
                                "Slow archive write detected: {}ms - disk I/O may be bottleneck",
                                write_duration.as_millis()
                            );
                        }
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
    use std::sync::atomic::{AtomicUsize, Ordering};

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
            archive_base_dir: None,
            unparsed_log_path: None,
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
            archive_base_dir: None,
            unparsed_log_path: None,
        };

        let login_cmd = AprsClient::build_login_command(&config);
        assert_eq!(
            login_cmd,
            "user TEST123 pass -1 vers soar-aprs-client 1.0\r\n"
        );
    }

    struct TestPacketProcessor {
        counter: Arc<AtomicUsize>,
    }

    impl PacketHandler for TestPacketProcessor {
        fn process_packet(&self, _packet: AprsPacket) {
            self.counter.fetch_add(1, Ordering::SeqCst);
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[tokio::test]
    async fn test_packet_processor() {
        let counter = Arc::new(AtomicUsize::new(0));
        let test_processor = TestPacketProcessor {
            counter: Arc::clone(&counter),
        };

        // Directly assign the test processor (this is a test workaround)
        let processors = AprsProcessors {
            packet_processor: Arc::new(test_processor),
        };

        let config = AprsClientConfig::default();

        // Simulate processing a message
        AprsClient::process_message("TEST>APRS:>Test message", &processors, &config).await;

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

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
