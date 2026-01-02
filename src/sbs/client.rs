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
            // Create a new subscription for this iteration
            let shutdown_rx_for_connection = shutdown_rx_for_loop.resubscribe();

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
                    shutdown_rx_for_connection,
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

    /// Connect to SBS server and process messages
    async fn connect_and_process(
        config: &SbsClientConfig,
        raw_message_tx: &flume::Sender<Vec<u8>>,
        shutdown_rx: tokio::sync::broadcast::Receiver<()>,
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

        Self::process_connection(
            stream,
            peer_addr_str,
            raw_message_tx,
            shutdown_rx,
            health_state,
        )
        .await
    }

    /// Process an active SBS connection
    async fn process_connection(
        stream: TcpStream,
        peer_addr_str: String,
        raw_message_tx: &flume::Sender<Vec<u8>>,
        mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
        health_state: std::sync::Arc<tokio::sync::RwLock<crate::metrics::BeastIngestHealth>>,
    ) -> ConnectionResult {
        info!("Processing connection to SBS server at {}", peer_addr_str);

        let message_timeout = Duration::from_secs(300); // 5 minute timeout
        let reader = BufReader::new(stream);
        let mut lines = reader.lines();
        let mut message_count = 0u64;
        let mut last_stats_log = std::time::Instant::now();
        let connection_start = std::time::Instant::now();

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    let duration = connection_start.elapsed();
                    info!(
                        "Shutdown signal received, closing SBS connection (IP: {}) after {:.1}s, received {} messages",
                        peer_addr_str,
                        duration.as_secs_f64(),
                        message_count
                    );
                    metrics::gauge!("sbs.connection.connected").set(0.0);

                    // Mark as disconnected in health state
                    {
                        let mut health = health_state.write().await;
                        health.beast_connected = false;
                    }

                    return ConnectionResult::Success;
                }
                result = tokio::time::timeout(message_timeout, lines.next_line()) => {
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
                            info!(
                                "SBS connection closed by server (IP: {}) after {:.1}s, received {} messages",
                                peer_addr_str,
                                duration.as_secs_f64(),
                                message_count
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
                            message_count += 1;

                            // Log stats every 10 seconds
                            if last_stats_log.elapsed().as_secs() >= 10 {
                                let elapsed = last_stats_log.elapsed().as_secs_f64();
                                let rate = message_count as f64 / elapsed;
                                info!(
                                    "SBS stats: {:.1} msg/s, {} total messages",
                                    rate, message_count
                                );
                                metrics::gauge!("sbs.message_rate").set(rate);
                                message_count = 0;
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
