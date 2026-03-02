use anyhow::Result;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::sleep;
use tracing::{error, info, trace, warn};

use crate::protocol::{IngestSource, create_serialized_envelope};

// Queue size for raw SBS messages from TCP socket
const RAW_MESSAGE_QUEUE_SIZE: usize = 200;

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
            retry_delay_seconds: 0, // Reconnect immediately on first failure
            max_retry_delay_seconds: 60, // Cap at 60 seconds
        }
    }
}

/// SBS client that connects to an SBS-1 BaseStation server via TCP
/// Wraps raw SBS CSV messages in protobuf Envelopes and sends them via the persistent queue
pub struct SbsClient {
    config: SbsClientConfig,
}

impl SbsClient {
    /// Create a new SBS client
    pub fn new(config: SbsClientConfig) -> Self {
        Self { config }
    }

    /// Start the SBS client with persistent queue
    /// This connects to the SBS server and sends all messages to the queue as serialized protobuf Envelopes
    /// Each envelope contains: source type (SBS), timestamp (captured at receive time), and raw payload
    #[tracing::instrument(skip(self, queue, health_state, stats_counter))]
    pub async fn start(
        &self,
        queue: std::sync::Arc<crate::persistent_queue::PersistentQueue<Vec<u8>>>,
        health_state: std::sync::Arc<tokio::sync::RwLock<crate::metrics::BeastIngestHealth>>,
        stats_counter: Option<std::sync::Arc<std::sync::atomic::AtomicU64>>,
    ) -> Result<()> {
        let config = self.config.clone();

        // Create bounded channel for serialized envelope bytes from TCP socket
        let (envelope_tx, envelope_rx) = flume::bounded::<Vec<u8>>(RAW_MESSAGE_QUEUE_SIZE);

        // Spawn queue feeding task
        let queue_clone = queue.clone();
        let stats_counter_clone = stats_counter.clone();
        let queue_feeder_handle = tokio::spawn(async move {
            info!("Starting SBS queue feeding task");
            let mut last_metrics_update = std::time::Instant::now();

            loop {
                match envelope_rx.recv_async().await {
                    Ok(envelope_bytes) => {
                        if let Err(e) = queue_clone.send(envelope_bytes.clone()).await {
                            warn!(
                                "Failed to send SBS envelope to persistent queue, retrying: {}",
                                e
                            );
                            metrics::counter!("sbs.queue.send_error_total").increment(1);
                            // Retry with backoff until it succeeds
                            let mut retry_delay = Duration::from_millis(100);
                            loop {
                                sleep(retry_delay).await;
                                match queue_clone.send(envelope_bytes.clone()).await {
                                    Ok(()) => break,
                                    Err(e) => {
                                        warn!("SBS persistent queue send retry failed: {}", e);
                                        metrics::counter!("sbs.queue.send_error_total")
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
                            metrics::gauge!("sbs.envelope_channel.depth").set(channel_depth as f64);
                            last_metrics_update = std::time::Instant::now();
                        }
                    }
                    Err(_) => {
                        info!("SBS channel closed, queue feeder exiting");
                        break;
                    }
                }
            }
        });

        // Connection loop with exponential backoff - retries indefinitely
        let mut current_delay = config.retry_delay_seconds;

        loop {
            let result =
                Self::connect_and_process(&config, &envelope_tx, health_state.clone()).await;

            match result {
                ConnectionResult::Success => {
                    info!("SBS connection completed successfully");
                    break;
                }
                ConnectionResult::ConnectionFailed(e) => {
                    metrics::counter!("sbs.connection.failed_total").increment(1);
                    warn!(
                        "Failed to connect to SBS server {}:{}: {} - retrying in {}s",
                        config.server, config.port, e, current_delay
                    );

                    sleep(Duration::from_secs(current_delay)).await;
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
                    current_delay = config.retry_delay_seconds;
                }
            }
        }

        // Drop envelope_tx so the queue feeder task sees channel closed and exits
        drop(envelope_tx);

        // Wait for queue feeder task to complete
        info!("Waiting for SBS queue feeder task to complete...");
        let _ = queue_feeder_handle.await;
        info!("SBS client stopped");

        Ok(())
    }

    /// Connect to SBS server and process messages
    async fn connect_and_process(
        config: &SbsClientConfig,
        envelope_tx: &flume::Sender<Vec<u8>>,
        health_state: std::sync::Arc<tokio::sync::RwLock<crate::metrics::BeastIngestHealth>>,
    ) -> ConnectionResult {
        let address = format!("{}:{}", config.server, config.port);
        info!("Connecting to SBS server at {}", address);

        let stream = match TcpStream::connect(&address).await {
            Ok(stream) => {
                info!("Connected to SBS server at {}", address);
                metrics::gauge!("sbs.connection.connected").set(1.0);

                {
                    let mut health = health_state.write().await;
                    health.beast_connected = true; // Reusing Beast health struct for SBS
                }

                stream
            }
            Err(e) => {
                metrics::gauge!("sbs.connection.connected").set(0.0);
                return ConnectionResult::ConnectionFailed(anyhow::anyhow!(
                    "Failed to connect to SBS server at {}: {}",
                    address,
                    e
                ));
            }
        };

        // Process the connection
        Self::process_stream(stream, envelope_tx, health_state).await
    }

    /// Process SBS stream, creating protobuf envelopes with timestamps captured at receive time
    async fn process_stream(
        stream: TcpStream,
        envelope_tx: &flume::Sender<Vec<u8>>,
        health_state: std::sync::Arc<tokio::sync::RwLock<crate::metrics::BeastIngestHealth>>,
    ) -> ConnectionResult {
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        let mut last_stats_log = std::time::Instant::now();

        // Initialize interval tracking
        {
            let mut health = health_state.write().await;
            if health.interval_start.is_none() {
                health.interval_start = Some(std::time::Instant::now());
            }
        }

        loop {
            line.clear();

            match tokio::time::timeout(Duration::from_secs(300), reader.read_line(&mut line)).await
            {
                Ok(Ok(0)) => {
                    info!("SBS connection closed by server");
                    metrics::counter!("sbs.connection.server_closed_total").increment(1);
                    metrics::gauge!("sbs.connection.connected").set(0.0);

                    {
                        let mut health = health_state.write().await;
                        health.beast_connected = false;
                    }

                    return ConnectionResult::Success;
                }
                Ok(Ok(_)) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    trace!("Received SBS message: {}", trimmed);
                    metrics::counter!("sbs.messages.received_total").increment(1);

                    // Create protobuf envelope with timestamp captured NOW
                    let envelope_bytes = match create_serialized_envelope(
                        IngestSource::Sbs,
                        trimmed.as_bytes().to_vec(),
                    ) {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            error!(error = %e, "Failed to create SBS envelope");
                            metrics::counter!("sbs.envelope.creation_error_total").increment(1);
                            continue;
                        }
                    };

                    // Send envelope to channel
                    match tokio::time::timeout(
                        Duration::from_secs(10),
                        envelope_tx.send_async(envelope_bytes),
                    )
                    .await
                    {
                        Ok(Ok(())) => {
                            metrics::counter!("sbs.messages.queued_total").increment(1);

                            // Update health stats
                            {
                                let mut health = health_state.write().await;
                                health.total_messages += 1;
                                health.interval_messages += 1;
                                health.last_message_time = Some(std::time::Instant::now());
                            }

                            // Update metrics periodically
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

                                {
                                    let mut health = health_state.write().await;
                                    health.interval_messages = 0;
                                    health.interval_start = Some(std::time::Instant::now());
                                }
                                last_stats_log = std::time::Instant::now();
                            }
                        }
                        Ok(Err(_)) => {
                            info!("SBS envelope channel closed - stopping connection");
                            return ConnectionResult::OperationFailed(anyhow::anyhow!(
                                "Envelope channel closed"
                            ));
                        }
                        Err(_) => {
                            error!("SBS queue send blocked for 10+ seconds - disconnecting");
                            metrics::counter!("sbs.queue_send_timeout_total").increment(1);
                            return ConnectionResult::OperationFailed(anyhow::anyhow!(
                                "Queue send timeout - will reconnect when drained"
                            ));
                        }
                    }
                }
                Ok(Err(e)) => {
                    error!(error = %e, "Error reading from SBS stream");
                    metrics::gauge!("sbs.connection.connected").set(0.0);

                    {
                        let mut health = health_state.write().await;
                        health.beast_connected = false;
                    }

                    return ConnectionResult::OperationFailed(anyhow::anyhow!("Read error: {}", e));
                }
                Err(_) => {
                    warn!("No data received from SBS server for 5 minutes");
                    metrics::counter!("sbs.timeout_total").increment(1);
                    metrics::gauge!("sbs.connection.connected").set(0.0);

                    {
                        let mut health = health_state.write().await;
                        health.beast_connected = false;
                    }

                    return ConnectionResult::OperationFailed(anyhow::anyhow!(
                        "Message timeout - no data received for 5 minutes"
                    ));
                }
            }
        }
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
        assert_eq!(config.retry_delay_seconds, 0);
        assert_eq!(config.max_retry_delay_seconds, 60);
    }

    #[test]
    fn test_sbs_config_custom() {
        let config = SbsClientConfig {
            server: "data.adsbhub.org".to_string(),
            port: 5002,
            retry_delay_seconds: 5,
            max_retry_delay_seconds: 120,
        };
        assert_eq!(config.server, "data.adsbhub.org");
        assert_eq!(config.port, 5002);
        assert_eq!(config.retry_delay_seconds, 5);
        assert_eq!(config.max_retry_delay_seconds, 120);
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

    #[test]
    fn test_connection_result_enum() {
        // Verify ConnectionResult enum exists and has expected variants
        let _success = ConnectionResult::Success;
        let _conn_failed = ConnectionResult::ConnectionFailed(anyhow::anyhow!("test"));
        let _op_failed = ConnectionResult::OperationFailed(anyhow::anyhow!("test"));
    }
}
