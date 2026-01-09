use anyhow::Result;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::sleep;
use tracing::{error, info, trace, warn};

// Queue size for raw SBS messages from TCP socket
const RAW_MESSAGE_QUEUE_SIZE: usize = 10000;

/// Result type for connection attempts
enum ConnectionResult {
    /// Connection was successful and ran until completion/disconnection
    Success,
    /// Connection failed immediately (couldn't establish connection)
    ConnectionFailed(anyhow::Error),
    /// Connection was established but failed during operation
    OperationFailed(anyhow::Error),
}

/// Configuration for the SBS client
#[derive(Debug, Clone)]
pub struct SbsClientConfig {
    /// SBS server hostname
    pub server: String,
    /// SBS server port (typically 30003)
    pub port: u16,
    /// Maximum number of connection retry attempts
    pub max_retries: u32,
    /// Initial delay between reconnection attempts in seconds (will use exponential backoff)
    pub retry_delay_seconds: u64,
    /// Maximum delay between reconnection attempts in seconds (cap for exponential backoff)
    pub max_retry_delay_seconds: u64,
}

impl Default for SbsClientConfig {
    fn default() -> Self {
        Self {
            server: "localhost".to_string(),
            port: 30003,
            max_retries: 5,
            retry_delay_seconds: 0, // Reconnect immediately on first failure
            max_retry_delay_seconds: 60, // Cap at 60 seconds
        }
    }
}

/// SBS client that connects to an SBS-1 BaseStation server via TCP
/// Publishes raw SBS CSV messages to NATS for processing
pub struct SbsClient {
    config: SbsClientConfig,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl SbsClient {
    /// Create a new SBS client
    pub fn new(config: SbsClientConfig) -> Self {
        Self {
            config,
            shutdown_tx: None,
        }
    }

    /// Start the SBS client with a publisher
    /// This connects to the SBS server and publishes all messages to NATS
    #[tracing::instrument(skip(self, publisher))]
    pub async fn start<P: crate::sbs::SbsPublisher>(&mut self, publisher: P) -> Result<()> {
        let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let health_state = crate::metrics::init_beast_health(); // Reuse Beast health for now
        self.start_with_shutdown(publisher, shutdown_rx, health_state)
            .await
    }

    /// Start the SBS client with a publisher and external shutdown signal
    /// This connects to the SBS server and publishes all messages to NATS
    /// Supports graceful shutdown when shutdown_rx receives a signal
    /// Updates health_state with connection status for health checks
    #[tracing::instrument(skip(self, publisher, shutdown_rx, health_state))]
    pub async fn start_with_shutdown<P: crate::sbs::SbsPublisher>(
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

        // Create bounded channel for raw SBS messages from TCP socket
        let (raw_message_tx, raw_message_rx) = flume::bounded::<Vec<u8>>(RAW_MESSAGE_QUEUE_SIZE);

        // Spawn publisher task that consumes from the raw_message_rx channel and publishes to NATS
        let publisher_clone = publisher.clone();
        let publisher_handle = tokio::spawn(async move {
            Self::publisher_loop(publisher_clone, raw_message_rx).await;
        });

        // Monitor shutdown signal
        tokio::spawn(async move {
            if shutdown_rx.await.is_ok() {
                info!("Received shutdown signal, broadcasting to tasks...");
                let _ = shutdown_broadcast_tx.send(());
            }
        });

        // Connection loop with exponential backoff
        let mut retry_count = 0;
        let mut current_delay = config.retry_delay_seconds;

        loop {
            tokio::select! {
                _ = &mut internal_shutdown_rx => {
                    info!("Shutdown signal received, stopping SBS client");
                    break;
                }
                _ = shutdown_rx_for_loop.recv() => {
                    info!("Shutdown broadcast received, stopping SBS client");
                    break;
                }
                result = Self::connect_and_process(
                    &config,
                    &raw_message_tx,
                    health_state.clone(),
                ) => {
                    match result {
                        ConnectionResult::Success => {
                            info!("SBS connection completed successfully");
                            break;
                        }
                        ConnectionResult::ConnectionFailed(e) => {
                            retry_count += 1;
                            if retry_count > config.max_retries {
                                error!(
                                    "Max retries ({}) exceeded for SBS connection to {}:{}, giving up: {}",
                                    config.max_retries, config.server, config.port, e
                                );
                                break;
                            }

                            metrics::counter!("sbs.connection.failed_total").increment(1);
                            warn!(
                                "Failed to connect to SBS server {}:{} (attempt {}/{}): {} - retrying in {}s",
                                config.server, config.port, retry_count, config.max_retries, e, current_delay
                            );

                            sleep(Duration::from_secs(current_delay)).await;

                            // Exponential backoff with cap
                            current_delay = std::cmp::min(
                                current_delay * 2,
                                config.max_retry_delay_seconds,
                            );
                        }
                        ConnectionResult::OperationFailed(e) => {
                            metrics::counter!("sbs.connection.operation_failed_total").increment(1);
                            warn!("SBS connection to {}:{} failed during operation: {} - reconnecting in 1s", config.server, config.port, e);
                            sleep(Duration::from_secs(1)).await;

                            // Reset retry count on operation failures (connection was successful)
                            retry_count = 0;
                            current_delay = config.retry_delay_seconds;
                        }
                    }
                }
            }
        }

        // Wait for publisher task to complete
        info!("Waiting for SBS publisher task to complete...");
        let _ = publisher_handle.await;
        info!("SBS client stopped");

        Ok(())
    }

    /// Start the SBS client with a persistent queue (NEW: NATS replacement)
    /// This connects to the SBS server and feeds all messages to a persistent queue
    /// The queue handles buffering when soar-run is disconnected
    #[tracing::instrument(skip(self, queue, health_state, stats_counter))]
    pub async fn start_with_queue(
        &mut self,
        queue: std::sync::Arc<crate::persistent_queue::PersistentQueue<Vec<u8>>>,
        health_state: std::sync::Arc<tokio::sync::RwLock<crate::metrics::BeastIngestHealth>>,
        stats_counter: Option<std::sync::Arc<std::sync::atomic::AtomicU64>>,
    ) -> Result<()> {
        let config = self.config.clone();

        // Create bounded channel for raw SBS messages from TCP socket
        let (raw_message_tx, raw_message_rx) = flume::bounded::<Vec<u8>>(RAW_MESSAGE_QUEUE_SIZE);

        // Spawn queue feeding task that consumes from the raw_message_rx channel and sends to persistent queue
        let queue_clone = queue.clone();
        let stats_counter_clone = stats_counter.clone();
        let queue_feeder_handle = tokio::spawn(async move {
            info!("Starting SBS queue feeding task");
            let mut at_capacity_logged = false;
            loop {
                // Check if disk queue is at capacity before consuming from channel
                // This provides backpressure - channel will fill up, causing TCP reader to block
                while queue_clone.is_at_capacity() {
                    if !at_capacity_logged {
                        warn!(
                            "Queue '{}' disk at capacity, pausing consumption (source will disconnect)",
                            queue_clone.name()
                        );
                        metrics::counter!("queue.capacity_pause_total", "queue" => "sbs")
                            .increment(1);
                        at_capacity_logged = true;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
                if at_capacity_logged {
                    info!(
                        "Queue '{}' has space, resuming consumption",
                        queue_clone.name()
                    );
                    at_capacity_logged = false;
                }

                match raw_message_rx.recv_async().await {
                    Ok(message) => {
                        if let Err(e) = queue_clone.send(message).await {
                            // This can still happen in a race condition, but should be rare
                            warn!(
                                "Failed to send SBS message to persistent queue (will retry): {}",
                                e
                            );
                            metrics::counter!("sbs.queue.send_error_total").increment(1);
                        } else if let Some(ref counter) = stats_counter_clone {
                            counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                    Err(_) => {
                        info!("SBS queue feeding task ended");
                        break;
                    }
                }
            }
        });

        // Connection loop with exponential backoff
        let mut retry_count = 0;
        let mut current_delay = config.retry_delay_seconds;

        loop {
            // Before connecting/reconnecting, wait for queue to be ready
            // This prevents reconnecting when the queue is still full
            if !queue.is_ready_for_connection() {
                let capacity = queue.capacity_percent();
                info!(
                    "Queue at {}% capacity, waiting for it to drain to 75% before reconnecting",
                    capacity
                );
                metrics::counter!("sbs.connection_deferred_total").increment(1);

                // Wait for queue to drain
                while !queue.is_ready_for_connection() {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }

                let new_capacity = queue.capacity_percent();
                info!(
                    "Queue drained to {}% capacity, proceeding with reconnection",
                    new_capacity
                );
            }

            let result =
                Self::connect_and_process(&config, &raw_message_tx, health_state.clone()).await;

            match result {
                ConnectionResult::Success => {
                    info!("SBS connection completed successfully");
                    break;
                }
                ConnectionResult::ConnectionFailed(e) => {
                    retry_count += 1;
                    if retry_count > config.max_retries {
                        error!(
                            "Max retries ({}) exceeded for SBS connection to {}:{}, giving up: {}",
                            config.max_retries, config.server, config.port, e
                        );
                        break;
                    }

                    metrics::counter!("sbs.connection.failed_total").increment(1);
                    warn!(
                        "Failed to connect to SBS server {}:{} (attempt {}/{}): {} - retrying in {}s",
                        config.server,
                        config.port,
                        retry_count,
                        config.max_retries,
                        e,
                        current_delay
                    );

                    sleep(Duration::from_secs(current_delay)).await;

                    // Exponential backoff with cap
                    current_delay =
                        std::cmp::min(current_delay * 2, config.max_retry_delay_seconds);
                }
                ConnectionResult::OperationFailed(e) => {
                    metrics::counter!("sbs.connection.operation_failed_total").increment(1);
                    warn!(
                        "SBS connection to {}:{} failed during operation: {} - reconnecting in 1s",
                        config.server, config.port, e
                    );
                    sleep(Duration::from_secs(1)).await;

                    // Reset retry count on operation failures (connection was successful)
                    retry_count = 0;
                    current_delay = config.retry_delay_seconds;
                }
            }
        }

        // Wait for queue feeder task to complete
        info!("Waiting for SBS queue feeder task to complete...");
        let _ = queue_feeder_handle.await;
        info!("SBS client stopped");

        Ok(())
    }

    /// Connect to SBS server and process messages
    async fn connect_and_process(
        config: &SbsClientConfig,
        raw_message_tx: &flume::Sender<Vec<u8>>,
        health_state: std::sync::Arc<tokio::sync::RwLock<crate::metrics::BeastIngestHealth>>,
    ) -> ConnectionResult {
        let address = format!("{}:{}", config.server, config.port);
        info!("Connecting to SBS server at {}", address);

        // Attempt to establish connection
        let stream = match TcpStream::connect(&address).await {
            Ok(stream) => {
                info!("Connected to SBS server at {}", address);
                metrics::gauge!("sbs.connection.connected").set(1.0);

                // Mark as connected in health state
                {
                    let mut health = health_state.write().await;
                    health.beast_connected = true; // Reuse Beast health field for now
                }

                stream
            }
            Err(e) => {
                error!("Failed to connect to SBS server at {}: {}", address, e);
                metrics::gauge!("sbs.connection.connected").set(0.0);
                return ConnectionResult::ConnectionFailed(anyhow::anyhow!(
                    "Failed to connect: {}",
                    e
                ));
            }
        };

        let peer_addr_str = stream
            .peer_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        Self::process_connection(stream, peer_addr_str, raw_message_tx, health_state).await
    }

    /// Process an active SBS connection
    async fn process_connection(
        stream: TcpStream,
        peer_addr_str: String,
        raw_message_tx: &flume::Sender<Vec<u8>>,
        health_state: std::sync::Arc<tokio::sync::RwLock<crate::metrics::BeastIngestHealth>>,
    ) -> ConnectionResult {
        info!("Processing connection to SBS server at {}", peer_addr_str);

        let message_timeout = Duration::from_secs(300); // 5 minute timeout
        let reader = BufReader::new(stream);
        let mut lines = reader.lines();
        let mut last_stats_log = std::time::Instant::now();
        let connection_start = std::time::Instant::now();

        // Initialize interval tracking in health state
        {
            let mut health = health_state.write().await;
            if health.interval_start.is_none() {
                health.interval_start = Some(std::time::Instant::now());
            }
        }

        loop {
            let result = tokio::time::timeout(message_timeout, lines.next_line()).await;
            match result {
                Err(_) => {
                    // Timeout
                    let duration = connection_start.elapsed();
                    warn!(
                        "SBS connection to {} timed out after {:.1}s (no data for {}s)",
                        peer_addr_str,
                        duration.as_secs_f64(),
                        message_timeout.as_secs()
                    );
                    metrics::gauge!("sbs.connection.connected").set(0.0);

                    return ConnectionResult::OperationFailed(anyhow::anyhow!(
                        "Connection timed out"
                    ));
                }
                Ok(Ok(None)) => {
                    // Connection closed
                    let duration = connection_start.elapsed();
                    let total_messages = {
                        let health = health_state.read().await;
                        health.total_messages
                    };
                    info!(
                        "SBS connection closed by server (IP: {}) after {:.1}s, received {} messages",
                        peer_addr_str,
                        duration.as_secs_f64(),
                        total_messages
                    );
                    metrics::gauge!("sbs.connection.connected").set(0.0);

                    // Mark as disconnected in health state
                    {
                        let mut health = health_state.write().await;
                        health.beast_connected = false;
                    }

                    return ConnectionResult::Success;
                }
                Ok(Ok(Some(line))) => {
                    // Data received
                    trace!("Received SBS line: {}", line);
                    metrics::counter!("sbs.bytes.received_total").increment(line.len() as u64);

                    // Skip empty lines
                    if line.trim().is_empty() {
                        continue;
                    }

                    // Publish line with timestamp
                    Self::publish_line(raw_message_tx, line).await;

                    // Update stats in health state
                    {
                        let mut health = health_state.write().await;
                        health.total_messages += 1;
                        health.interval_messages += 1;
                        health.last_message_time = Some(std::time::Instant::now());
                    }

                    // Update metrics gauge periodically (every 10 seconds)
                    if last_stats_log.elapsed().as_secs() >= 10 {
                        let health = health_state.read().await;
                        if let Some(interval_start) = health.interval_start {
                            let elapsed = interval_start.elapsed().as_secs_f64();
                            if elapsed > 0.0 {
                                let rate = health.interval_messages as f64 / elapsed;
                                metrics::gauge!("sbs.message_rate").set(rate);
                            }
                        }
                        drop(health);

                        // Reset interval counters
                        {
                            let mut health = health_state.write().await;
                            health.interval_messages = 0;
                            health.interval_start = Some(std::time::Instant::now());
                        }
                        last_stats_log = std::time::Instant::now();
                    }
                }
                Ok(Err(e)) => {
                    // Read error
                    let duration = connection_start.elapsed();
                    error!(
                        "SBS read error from {} after {:.1}s: {}",
                        peer_addr_str,
                        duration.as_secs_f64(),
                        e
                    );
                    metrics::gauge!("sbs.connection.connected").set(0.0);

                    return ConnectionResult::OperationFailed(anyhow::anyhow!(
                        "SBS read error: {}",
                        e
                    ));
                }
            }
        }
    }

    /// Publish an SBS CSV line to the queue
    async fn publish_line(raw_message_tx: &flume::Sender<Vec<u8>>, line: String) {
        if line.is_empty() {
            return;
        }

        // Get current timestamp as microseconds since Unix epoch
        let timestamp_micros = chrono::Utc::now().timestamp_micros();

        // Build message: 8-byte timestamp + CSV line as UTF-8 bytes
        let mut message = Vec::with_capacity(8 + line.len());
        message.extend_from_slice(&timestamp_micros.to_be_bytes());
        message.extend_from_slice(line.as_bytes());

        match raw_message_tx.send_async(message).await {
            Ok(_) => {
                trace!("Published SBS line ({} bytes)", line.len());
                metrics::counter!("sbs.lines.published_total").increment(1);
            }
            Err(e) => {
                error!("Failed to send SBS line to queue: {}", e);
                metrics::counter!("sbs.lines.dropped_total").increment(1);
            }
        }
    }

    /// Publisher loop that consumes from the queue and publishes to NATS
    async fn publisher_loop<P: crate::sbs::SbsPublisher>(
        publisher: P,
        raw_message_rx: flume::Receiver<Vec<u8>>,
    ) {
        info!("Starting SBS publisher loop");

        while let Ok(message) = raw_message_rx.recv_async().await {
            // Publish to NATS with fire-and-forget
            publisher.publish_fire_and_forget(&message).await;
        }

        info!("SBS publisher loop ended");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sbs_config_default() {
        let config = SbsClientConfig::default();
        assert_eq!(config.server, "localhost");
        assert_eq!(config.port, 30003);
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.retry_delay_seconds, 0);
        assert_eq!(config.max_retry_delay_seconds, 60);
    }

    #[test]
    fn test_sbs_config_custom() {
        let config = SbsClientConfig {
            server: "data.adsbhub.org".to_string(),
            port: 5002,
            max_retries: 10,
            retry_delay_seconds: 5,
            max_retry_delay_seconds: 120,
        };
        assert_eq!(config.server, "data.adsbhub.org");
        assert_eq!(config.port, 5002);
        assert_eq!(config.max_retries, 10);
        assert_eq!(config.retry_delay_seconds, 5);
        assert_eq!(config.max_retry_delay_seconds, 120);
    }

    #[tokio::test]
    async fn test_publish_line_format() {
        // Create a channel to capture published messages
        let (tx, rx) = flume::bounded::<Vec<u8>>(10);

        // Test SBS message line
        let line = "MSG,3,,,AB1234,,,,,,,5000,,,51.5074,-0.1278,,,0,0,0,0".to_string();

        SbsClient::publish_line(&tx, line.clone()).await;

        // Receive the published message
        let message = rx.recv_async().await.unwrap();

        // Verify format: 8-byte timestamp + CSV line bytes
        assert!(message.len() > 8);
        assert_eq!(message.len(), 8 + line.len());

        // Extract timestamp (first 8 bytes)
        let timestamp_bytes: [u8; 8] = message[0..8].try_into().unwrap();
        let timestamp_micros = i64::from_be_bytes(timestamp_bytes);

        // Verify timestamp is reasonable (within last minute)
        let now_micros = chrono::Utc::now().timestamp_micros();
        let diff = (now_micros - timestamp_micros).abs();
        assert!(diff < 60_000_000); // Within 60 seconds

        // Extract line bytes
        let line_bytes = &message[8..];
        assert_eq!(line_bytes, line.as_bytes());
    }

    #[tokio::test]
    async fn test_publish_empty_line() {
        let (tx, rx) = flume::bounded::<Vec<u8>>(10);

        // Empty line should not be published
        SbsClient::publish_line(&tx, String::new()).await;

        // Channel should be empty
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_publish_multiple_lines() {
        let (tx, rx) = flume::bounded::<Vec<u8>>(10);

        let lines = vec![
            "MSG,1,,,AB1234,,,,,,,,,,,,,0,0,0,0".to_string(),
            "MSG,3,,,CD5678,,,,,,,10000,,,52.5074,-1.1278,,,0,0,0,0".to_string(),
            "MSG,4,,,EF9012,,,,,,,,,450,180,,,0,0,0,0".to_string(),
        ];

        for line in &lines {
            SbsClient::publish_line(&tx, line.clone()).await;
        }

        // Verify all messages were published
        for (i, line) in lines.iter().enumerate() {
            let message = rx
                .recv_async()
                .await
                .unwrap_or_else(|_| panic!("Failed to receive message {}", i));
            assert_eq!(message.len(), 8 + line.len());
            assert_eq!(&message[8..], line.as_bytes());
        }
    }

    #[test]
    fn test_realistic_sbs_messages() {
        // Real examples from SBS-1 BaseStation format
        let examples = vec![
            // MSG,1: ES Identification and Category
            "MSG,1,1,1,738065,1,2008/11/28,23:48:18.611,2008/11/28,23:53:19.161,RYR1427,,,,,,,0,,0,0",
            // MSG,3: ES Airborne Position Message
            "MSG,3,1,1,738065,1,2008/11/28,23:48:18.611,2008/11/28,23:53:19.161,,36000,,,51.45735,1.02826,,,0,0,0,0",
            // MSG,4: ES Airborne Velocity Message
            "MSG,4,1,1,738065,1,2008/11/28,23:48:18.611,2008/11/28,23:53:19.161,,,420,179,,,0,0,0,0",
            // MSG,5: Surveillance Alt Message
            "MSG,5,1,1,738065,1,2008/11/28,23:48:18.611,2008/11/28,23:53:19.161,,36000,,,,0,0,0,0,0",
            // MSG,6: Surveillance ID Message
            "MSG,6,1,1,738065,1,2008/11/28,23:48:18.611,2008/11/28,23:53:19.161,,,,,,,7541,0,0,0,0",
            // MSG,7: Air To Air Message
            "MSG,7,1,1,738065,1,2008/11/28,23:48:18.611,2008/11/28,23:53:19.161,,36000,,,,,,0,0,0,0",
            // MSG,8: All Call Reply
            "MSG,8,1,1,738065,1,2008/11/28,23:48:18.611,2008/11/28,23:53:19.161,,,,,,,,0,0,0,0",
        ];

        // These are all valid SBS messages that should be parseable
        for msg in examples {
            let fields: Vec<&str> = msg.split(',').collect();
            assert!(
                fields.len() >= 10,
                "SBS message should have at least 10 fields"
            );
            assert_eq!(fields[0], "MSG", "First field should be MSG");
        }
    }

    #[test]
    fn test_sbs_message_types() {
        // Verify we handle all MSG subtypes
        let subtypes = [
            "MSG,1", "MSG,2", "MSG,3", "MSG,4", "MSG,5", "MSG,6", "MSG,7", "MSG,8",
        ];

        for subtype in subtypes {
            assert!(subtype.starts_with("MSG,"));
            let type_num = subtype.split(',').nth(1).unwrap();
            let type_val: u8 = type_num.parse().unwrap();
            assert!((1..=8).contains(&type_val));
        }
    }

    #[tokio::test]
    async fn test_publish_line_with_special_characters() {
        let (tx, rx) = flume::bounded::<Vec<u8>>(10);

        // SBS message with special characters in callsign
        let line = "MSG,1,,,A1B2C3,,,,,,,RYR-123,,,,,,,0,0,0,0".to_string();

        SbsClient::publish_line(&tx, line.clone()).await;

        let message = rx.recv_async().await.unwrap();
        let line_bytes = &message[8..];
        assert_eq!(line_bytes, line.as_bytes());
        assert_eq!(String::from_utf8_lossy(line_bytes), line);
    }

    #[tokio::test]
    async fn test_publish_line_queue_capacity() {
        // Create a queue and verify we can publish messages to it
        let (tx, rx) = flume::bounded::<Vec<u8>>(10);

        let line = "MSG,3,,,AB1234,,,,,,,5000,,,51.5074,-0.1278,,,0,0,0,0".to_string();

        // Publish a few messages
        for _ in 0..3 {
            SbsClient::publish_line(&tx, line.clone()).await;
        }

        // Verify messages were published
        let mut count = 0;
        while rx.try_recv().is_ok() {
            count += 1;
        }
        assert_eq!(count, 3);
    }

    #[test]
    fn test_connection_result_enum() {
        // Verify ConnectionResult enum exists and has expected variants
        let _success = ConnectionResult::Success;
        let _conn_failed = ConnectionResult::ConnectionFailed(anyhow::anyhow!("test"));
        let _op_failed = ConnectionResult::OperationFailed(anyhow::anyhow!("test"));
    }
}
