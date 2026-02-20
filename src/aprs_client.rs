use anyhow::{Context, Result};
use std::time::Duration;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, error, info, trace, warn};

use crate::protocol::{IngestSource, create_serialized_envelope};

// Queue size for raw APRS messages
const RAW_MESSAGE_QUEUE_SIZE: usize = 1000;

// AprsClient only publishes raw messages to queue - all parsing happens in the consumer

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

    /// Start the APRS client with persistent queue
    /// This connects to APRS-IS and sends all messages to the queue as serialized protobuf Envelopes
    /// Each envelope contains: source type (OGN), timestamp (captured at receive time), and raw payload
    #[tracing::instrument(skip(self, queue, health_state, stats_counter))]
    pub async fn start(
        &mut self,
        queue: std::sync::Arc<crate::persistent_queue::PersistentQueue<Vec<u8>>>,
        health_state: std::sync::Arc<tokio::sync::RwLock<crate::metrics::AprsIngestHealth>>,
        stats_counter: Option<std::sync::Arc<std::sync::atomic::AtomicU64>>,
    ) -> Result<()> {
        let config = self.config.clone();

        // Create bounded channel for serialized envelope bytes from TCP socket to queue
        let (envelope_tx, envelope_rx) = flume::bounded::<Vec<u8>>(RAW_MESSAGE_QUEUE_SIZE);
        info!(
            "Created envelope queue channel with capacity {} for unified queue",
            RAW_MESSAGE_QUEUE_SIZE
        );

        // Spawn queue feeding task - reads from channel, sends to persistent queue
        let queue_clone = queue.clone();
        let stats_counter_clone = stats_counter.clone();
        let queue_handle = tokio::spawn(async move {
            let mut last_metrics_update = std::time::Instant::now();

            loop {
                match envelope_rx.recv_async().await {
                    Ok(envelope_bytes) => {
                        if let Err(e) = queue_clone.send(envelope_bytes).await {
                            warn!("Failed to send to persistent queue (will retry): {}", e);
                            metrics::counter!("aprs.queue.send_error_total").increment(1);
                        } else if let Some(ref counter) = stats_counter_clone {
                            counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }

                        // Update channel depth metric every second
                        if last_metrics_update.elapsed().as_secs() >= 1 {
                            let channel_depth = envelope_rx.len();
                            metrics::gauge!("aprs.envelope_channel.depth")
                                .set(channel_depth as f64);
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

        // Spawn connection management task
        let health_for_connection = health_state.clone();
        let connection_handle = tokio::spawn(async move {
            let mut retry_count = 0;
            let mut current_delay = config.retry_delay_seconds;

            loop {
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

                let connect_result = Self::connect_and_run(
                    &config,
                    envelope_tx.clone(),
                    health_for_connection.clone(),
                )
                .await;

                match connect_result {
                    ConnectionResult::Success => {
                        info!("APRS connection completed normally");
                        retry_count = 0;
                        current_delay = config.retry_delay_seconds;
                    }
                    ConnectionResult::ConnectionFailed(e) => {
                        error!("APRS connection failed: {}", e);
                        retry_count += 1;
                        metrics::counter!("aprs.connection_failed_total").increment(1);
                    }
                    ConnectionResult::OperationFailed(e) => {
                        error!("APRS operation failed: {}", e);
                        retry_count = 0;
                        metrics::counter!("aprs.connection.operation_failed_total").increment(1);
                    }
                }

                // Sleep before retrying
                if current_delay > 0 {
                    info!("Waiting {} seconds before retry", current_delay);
                    tokio::time::sleep(tokio::time::Duration::from_secs(current_delay)).await;
                    current_delay =
                        std::cmp::min(current_delay * 2, config.max_retry_delay_seconds);
                }
            }
        });

        let (queue_result, connection_result) = tokio::join!(queue_handle, connection_handle);

        queue_result.context("Queue feeder task panicked")?;
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
    /// Creates protobuf envelopes with timestamps captured at receive time
    #[tracing::instrument(skip(config, envelope_tx, health_state), fields(server = %config.server, port = %config.port))]
    async fn connect_and_run(
        config: &AprsClientConfig,
        envelope_tx: flume::Sender<Vec<u8>>,
        health_state: std::sync::Arc<tokio::sync::RwLock<crate::metrics::AprsIngestHealth>>,
    ) -> ConnectionResult {
        info!(
            "Connecting to APRS server {}:{}",
            config.server, config.port
        );

        let connection_start = std::time::Instant::now();

        // DNS lookup (reuse the same logic as connect_and_run)
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
                        "No IPv4 addresses found for {}, falling back to all addresses",
                        server_address
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

        // Shuffle and try addresses
        let mut shuffled_addrs = socket_addrs;
        {
            use rand::seq::SliceRandom;
            let mut rng = rand::rng();
            shuffled_addrs.shuffle(&mut rng);
        }

        let mut last_error = None;
        for addr in &shuffled_addrs {
            match TcpStream::connect(addr).await {
                Ok(stream) => {
                    info!("Connected to APRS server at {}", addr);
                    metrics::counter!("aprs.connection.established_total").increment(1);
                    metrics::gauge!("aprs.connection.connected").set(1.0);

                    {
                        let mut health = health_state.write().await;
                        health.aprs_connected = true;
                    }

                    return Self::process_connection(
                        stream,
                        config,
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

    /// Process an established APRS connection
    /// Creates protobuf envelopes with timestamps at the moment messages are received
    #[tracing::instrument(skip(stream, config, envelope_tx, health_state, connection_start), fields(peer_addr = %peer_addr_str))]
    async fn process_connection(
        stream: TcpStream,
        config: &AprsClientConfig,
        envelope_tx: flume::Sender<Vec<u8>>,
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
            return ConnectionResult::OperationFailed(anyhow::anyhow!(
                "Failed to send login command: {}",
                e
            ));
        }
        if let Err(e) = writer.flush().await {
            return ConnectionResult::OperationFailed(anyhow::anyhow!(
                "Failed to flush login command: {}",
                e
            ));
        }
        info!("Login command sent successfully");

        let mut line_buffer = Vec::new();
        let mut first_message = true;
        let message_timeout = Duration::from_secs(300);
        let keepalive_interval = Duration::from_secs(20);
        let mut last_keepalive = tokio::time::Instant::now();
        let mut last_stats_log = std::time::Instant::now();

        loop {
            line_buffer.clear();

            // Send keepalive if needed
            if last_keepalive.elapsed() >= keepalive_interval {
                let keepalive_msg = "# soar keepalive\r\n";
                if let Err(e) = writer.write_all(keepalive_msg.as_bytes()).await {
                    return ConnectionResult::OperationFailed(anyhow::anyhow!(
                        "Failed to send keepalive: {}",
                        e
                    ));
                }
                if let Err(e) = writer.flush().await {
                    return ConnectionResult::OperationFailed(anyhow::anyhow!(
                        "Failed to flush keepalive: {}",
                        e
                    ));
                }
                trace!("Sent keepalive to APRS server");
                metrics::counter!("aprs.keepalive.sent_total").increment(1);
                last_keepalive = tokio::time::Instant::now();
            }

            let read_result = timeout(
                message_timeout,
                Self::read_line_with_invalid_utf8_handling(&mut buf_reader, &mut line_buffer),
            )
            .await;

            match read_result {
                Ok(inner_result) => {
                    match inner_result {
                        Ok(0) => {
                            let duration = connection_start.elapsed();
                            // Use error! so sentry_tracing forwards this as a Sentry event
                            error!(
                                "APRS connection closed by server after {:.1}s",
                                duration.as_secs_f64()
                            );
                            metrics::counter!("aprs.connection.server_closed_total").increment(1);
                            break;
                        }
                        Ok(_) => {
                            let line = match String::from_utf8(line_buffer.clone()) {
                                Ok(valid_line) => valid_line,
                                Err(_) => {
                                    debug!("Invalid UTF-8 in stream, skipping");
                                    continue;
                                }
                            };

                            let trimmed_line = line.trim();
                            if !trimmed_line.is_empty() {
                                let is_server_message = trimmed_line.starts_with('#');
                                if is_server_message {
                                    metrics::counter!("aprs.raw_message.received.server_total")
                                        .increment(1);
                                } else {
                                    metrics::counter!("aprs.raw_message.received.aprs_total")
                                        .increment(1);
                                }

                                if first_message {
                                    info!("First message from server: {}", trimmed_line);
                                    first_message = false;
                                } else {
                                    trace!("Received: {}", trimmed_line);
                                }

                                // Create protobuf envelope with timestamp captured NOW
                                // This preserves the true receive time even if the message sits in queue
                                let envelope_bytes = match create_serialized_envelope(
                                    IngestSource::Ogn,
                                    trimmed_line.as_bytes().to_vec(),
                                ) {
                                    Ok(bytes) => bytes,
                                    Err(e) => {
                                        error!("Failed to create envelope: {}", e);
                                        continue;
                                    }
                                };

                                // Send envelope to channel (blocking - backpressure
                                // from the queue will stall reads from the socket,
                                // which is preferable to disconnecting)
                                if envelope_tx.is_full() {
                                    metrics::counter!("aprs.envelope_channel.blocked_total")
                                        .increment(1);
                                    warn!("Envelope channel full, blocking until space available");
                                }
                                match envelope_tx.send_async(envelope_bytes).await {
                                    Ok(()) => {
                                        if !is_server_message {
                                            metrics::counter!("aprs.raw_message.queued.aprs_total")
                                                .increment(1);

                                            {
                                                let mut health = health_state.write().await;
                                                health.last_message_time =
                                                    Some(std::time::Instant::now());
                                                health.total_messages += 1;
                                                health.interval_messages += 1;

                                                if health.interval_start.is_none() {
                                                    health.interval_start =
                                                        Some(std::time::Instant::now());
                                                }
                                            }

                                            if last_stats_log.elapsed().as_secs() >= 10 {
                                                let health = health_state.read().await;
                                                if let Some(interval_start) = health.interval_start
                                                {
                                                    let elapsed =
                                                        interval_start.elapsed().as_secs_f64();
                                                    if elapsed > 0.0 {
                                                        let rate = health.interval_messages as f64
                                                            / elapsed;
                                                        metrics::gauge!("aprs.message_rate")
                                                            .set(rate);
                                                    }
                                                }
                                                drop(health);

                                                {
                                                    let mut health = health_state.write().await;
                                                    health.interval_messages = 0;
                                                    health.interval_start =
                                                        Some(std::time::Instant::now());
                                                }
                                                last_stats_log = std::time::Instant::now();
                                            }
                                        }
                                    }
                                    Err(flume::SendError(_)) => {
                                        info!("Envelope channel closed - stopping connection");
                                        return ConnectionResult::OperationFailed(anyhow::anyhow!(
                                            "Envelope channel closed"
                                        ));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            return ConnectionResult::OperationFailed(anyhow::anyhow!(
                                "Connection error: {}",
                                e
                            ));
                        }
                    }
                }
                Err(_) => {
                    error!("No message received for 5 minutes, disconnecting");
                    return ConnectionResult::OperationFailed(anyhow::anyhow!(
                        "Message timeout - no data received for 5 minutes"
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
            .retry_delay_seconds(10)
            .build();

        assert_eq!(config.server, "test.aprs.net");
        assert_eq!(config.port, 14580);
        assert_eq!(config.callsign, "TEST123");
        assert_eq!(config.password, Some("12345".to_string()));
        assert_eq!(config.filter, Some("r/47.0/-122.0/100".to_string()));
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
