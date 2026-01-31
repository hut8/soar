use anyhow::{Context, Result};
use soar::aprs_client::{AprsClient, AprsClientConfigBuilder};
use soar::beast::{BeastClient, BeastClientConfig};
use soar::connection_status::ConnectionStatusPublisher;
use soar::instance_lock::InstanceLock;
use soar::sbs::{SbsClient, SbsClientConfig};
use std::env;
use std::sync::Arc;
use tracing::{error, info, warn};

/// Parse a "host:port" string into (hostname, port)
fn parse_server_address(addr: &str) -> Result<(String, u16)> {
    let parts: Vec<&str> = addr.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!(
            "Invalid server address '{}' - expected format 'host:port'",
            addr
        ));
    }
    let host = parts[0].to_string();
    let port = parts[1]
        .parse::<u16>()
        .context(format!("Invalid port in '{}'", addr))?;
    Ok((host, port))
}

/// Unified health state for the consolidated ingest service
/// Tracks health of OGN (APRS), Beast, and SBS connections
#[derive(Clone, Debug, Default)]
pub struct IngestHealth {
    #[allow(dead_code)] // Will be used for health endpoint in future
    pub ogn_connected: bool,
    #[allow(dead_code)] // Will be used for health endpoint in future
    pub beast_connected: bool,
    #[allow(dead_code)] // Will be used for health endpoint in future
    pub sbs_connected: bool,
    pub socket_connected: bool,
    #[allow(dead_code)] // Will be used for health endpoint in future
    pub last_ogn_message_time: Option<std::time::Instant>,
    #[allow(dead_code)] // Will be used for health endpoint in future
    pub last_beast_message_time: Option<std::time::Instant>,
    #[allow(dead_code)] // Will be used for health endpoint in future
    pub last_sbs_message_time: Option<std::time::Instant>,
}

/// Configuration for the unified ingest service
pub struct IngestConfig {
    // OGN parameters
    pub ogn_server: Option<String>,
    pub ogn_port: Option<u16>,
    pub ogn_callsign: Option<String>,
    pub ogn_filter: Option<String>,
    // ADS-B parameters
    pub beast_servers: Vec<String>,
    pub sbs_servers: Vec<String>,
    // Common parameters
    pub max_retries: u32,
    pub retry_delay: u64,
}

pub async fn handle_ingest(config: IngestConfig) -> Result<()> {
    let IngestConfig {
        ogn_server,
        ogn_port,
        ogn_callsign,
        ogn_filter,
        beast_servers,
        sbs_servers,
        max_retries,
        retry_delay,
    } = config;
    // Validate that at least one source is specified
    let ogn_enabled = ogn_server.is_some();
    let beast_enabled = !beast_servers.is_empty();
    let sbs_enabled = !sbs_servers.is_empty();

    if !ogn_enabled && !beast_enabled && !sbs_enabled {
        return Err(anyhow::anyhow!(
            "No sources specified - use --ogn-server, --beast, or --sbs to specify at least one source"
        ));
    }

    // Determine environment
    let soar_env = env::var("SOAR_ENV").unwrap_or_default();
    let is_production = soar_env == "production";
    let is_staging = soar_env == "staging";

    let socket_path = soar::socket_path();
    info!(
        "Starting unified ingest service - socket: {:?}",
        socket_path
    );

    if ogn_enabled {
        info!(
            "  OGN source enabled: {}:{}",
            ogn_server.as_ref().unwrap(),
            ogn_port.unwrap_or(14580)
        );
    }
    if beast_enabled {
        info!("  Beast sources enabled: {:?}", beast_servers);
    }
    if sbs_enabled {
        info!("  SBS sources enabled: {:?}", sbs_servers);
    }

    info!(
        "Environment: {}",
        if is_production {
            "production"
        } else if is_staging {
            "staging"
        } else {
            "development"
        }
    );

    // Initialize unified health state
    let health_state = Arc::new(tokio::sync::RwLock::new(IngestHealth::default()));

    // Initialize all ingest metrics to zero so they appear in Grafana even before events occur
    // This MUST happen before starting the metrics server to avoid race conditions where
    // Prometheus scrapes before metrics are initialized
    info!("Initializing unified ingest metrics...");
    initialize_ingest_metrics();
    info!("Unified ingest metrics initialized");

    // Start metrics server in production/staging mode (AFTER metrics are initialized)
    if is_production || is_staging {
        // Unified ingest service uses port 9095 for production, 9197 for staging
        let default_port = if is_staging { 9197 } else { 9095 };
        let metrics_port = env::var("METRICS_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(default_port);

        info!("Starting metrics server on port {}", metrics_port);
        tokio::spawn(async move {
            soar::metrics::start_metrics_server(metrics_port, Some("ingest")).await;
        });
    }

    // Acquire instance lock to prevent multiple ingest instances from running
    let lock_name = if is_production {
        "ingest-production"
    } else {
        "ingest-dev"
    };
    let _lock = InstanceLock::new(lock_name)
        .context("Failed to acquire instance lock - is another ingest instance running?")?;
    info!("Instance lock acquired for {}", lock_name);

    // Create a single unified persistent queue for all sources
    // Messages are stored as pre-serialized protobuf Envelopes containing:
    // - source type (OGN, Beast, SBS)
    // - timestamp (captured at receive time)
    // - raw payload data
    // Queue directory is environment-aware: /var/lib/soar/queues in prod/staging,
    // ~/.local/share/soar/queues in development (XDG spec)
    let queue_dir = soar::queue_dir();
    std::fs::create_dir_all(&queue_dir)
        .with_context(|| format!("Failed to create queue directory: {:?}", queue_dir))?;

    let queue_path = queue_dir.join("ingest.queue");
    let queue = Arc::new(
        soar::persistent_queue::PersistentQueue::<Vec<u8>>::new(
            "ingest".to_string(),
            queue_path,
            Some(10 * 1024 * 1024 * 1024), // 10 GB max
            3000,                          // Memory capacity (combined for all sources)
        )
        .expect("Failed to create unified ingest queue"),
    );

    info!("Created unified ingest queue at {:?}", queue_dir);

    // Create shared counters for stats tracking (aggregate across all sources)
    let stats_frames_received = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let stats_messages_sent = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let stats_bytes_sent = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let stats_send_time_total_ms = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let stats_slow_sends = Arc::new(std::sync::atomic::AtomicU64::new(0));

    // Per-source counters for individual metrics
    let stats_ogn_received = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let stats_beast_received = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let stats_sbs_received = Arc::new(std::sync::atomic::AtomicU64::new(0));

    // Shared health states for read stats tracking (one per source type)
    let aprs_health_shared = Arc::new(tokio::sync::RwLock::new(
        soar::metrics::AprsIngestHealth::default(),
    ));
    let beast_health_shared = Arc::new(tokio::sync::RwLock::new(
        soar::metrics::BeastIngestHealth::default(),
    ));
    let sbs_health_shared = Arc::new(tokio::sync::RwLock::new(
        soar::metrics::BeastIngestHealth::default(),
    ));

    // Create connection status publisher for broadcasting to NATS
    let status_publisher: Option<Arc<ConnectionStatusPublisher>> = match env::var("NATS_URL") {
        Ok(nats_url) => match ConnectionStatusPublisher::new(&nats_url).await {
            Ok(publisher) => {
                let publisher = Arc::new(publisher);
                // Start periodic 60-second publishing
                publisher.clone().start_periodic_publish();
                info!("Connection status publisher initialized for NATS");
                Some(publisher)
            }
            Err(e) => {
                warn!("Failed to create connection status publisher: {}", e);
                None
            }
        },
        Err(_) => {
            info!("NATS_URL not set, connection status publishing disabled");
            None
        }
    };

    // Spawn connection status monitoring task
    if let Some(publisher) = status_publisher.clone() {
        let aprs_health_for_status = aprs_health_shared.clone();
        let beast_health_for_status = beast_health_shared.clone();
        let sbs_health_for_status = sbs_health_shared.clone();
        let ogn_endpoint = if ogn_enabled {
            Some(format!(
                "{}:{}",
                ogn_server.as_ref().unwrap(),
                ogn_port.unwrap_or(14580)
            ))
        } else {
            None
        };
        let beast_endpoints: Vec<String> = beast_servers.clone();
        let sbs_endpoints: Vec<String> = sbs_servers.clone();

        tokio::spawn(async move {
            let mut last_ogn_connected = false;
            let mut last_adsb_connected = false;

            // Check status every second for fast change detection
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                interval.tick().await;

                // Check OGN status
                let ogn_connected = {
                    let health = aprs_health_for_status.read().await;
                    health.aprs_connected
                };

                // Check ADS-B status (Beast or SBS connected)
                let beast_connected = {
                    let health = beast_health_for_status.read().await;
                    health.beast_connected
                };
                let sbs_connected = {
                    let health = sbs_health_for_status.read().await;
                    health.beast_connected // SBS uses BeastIngestHealth too
                };
                let adsb_connected = beast_connected || sbs_connected;

                // Update OGN status if changed
                if ogn_connected != last_ogn_connected {
                    publisher
                        .set_ogn_status(ogn_connected, ogn_endpoint.clone())
                        .await;
                    last_ogn_connected = ogn_connected;
                }

                // Update ADS-B status if changed
                if adsb_connected != last_adsb_connected {
                    let endpoints: Vec<String> = if adsb_connected {
                        beast_endpoints
                            .iter()
                            .chain(sbs_endpoints.iter())
                            .cloned()
                            .collect()
                    } else {
                        Vec::new()
                    };
                    publisher.set_adsb_status(adsb_connected, endpoints).await;
                    last_adsb_connected = adsb_connected;
                }
            }
        });
    }

    // Create a single socket client for sending to soar-run
    // The source type is already encoded in each envelope, so we don't need per-source clients
    // We use Ogn as a dummy source since send_serialized() doesn't use it
    let mut socket_client = match soar::socket_client::SocketClient::connect(
        &socket_path,
        soar::protocol::IngestSource::Ogn, // Dummy - not used with send_serialized
    )
    .await
    {
        Ok(client) => {
            info!("Socket connected to soar-run at {:?}", socket_path);
            client
        }
        Err(e) => {
            warn!(
                "Failed to connect socket to soar-run (will buffer to queue): {}",
                e
            );
            soar::socket_client::SocketClient::new(&socket_path, soar::protocol::IngestSource::Ogn)
        }
    };

    // Update health state with socket connection status
    {
        let mut health = health_state.write().await;
        health.socket_connected = socket_client.is_connected();
    }

    // Queue starts in Disconnected state and auto-connects on first recv().
    // This ensures all messages are persisted to disk until a consumer
    // is actually ready to process them, preventing message loss on restart.

    // Spawn unified publisher task: reads from queue → sends to socket
    // Messages are pre-serialized protobuf Envelopes containing source type and timestamp
    {
        let queue_for_publisher = queue.clone();
        let stats_sent_clone = stats_messages_sent.clone();
        let stats_bytes_sent_clone = stats_bytes_sent.clone();
        let stats_time_clone = stats_send_time_total_ms.clone();
        let stats_slow_clone = stats_slow_sends.clone();
        tokio::spawn(async move {
            info!("Unified publisher task started");
            loop {
                match queue_for_publisher.recv().await {
                    Ok(serialized_envelope) => {
                        // Track send timing
                        let send_start = std::time::Instant::now();

                        // Send pre-serialized envelope directly to socket
                        match socket_client.send_serialized(&serialized_envelope).await {
                            Ok(_) => {
                                // Track stats - use microseconds for atomics, fractional ms for metrics
                                let elapsed = send_start.elapsed();
                                let send_duration_us = elapsed.as_micros() as u64;
                                let send_duration_ms_float = elapsed.as_secs_f64() * 1000.0;
                                stats_sent_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                stats_bytes_sent_clone.fetch_add(
                                    serialized_envelope.len() as u64,
                                    std::sync::atomic::Ordering::Relaxed,
                                );
                                stats_time_clone.fetch_add(
                                    send_duration_us,
                                    std::sync::atomic::Ordering::Relaxed,
                                );
                                if send_duration_us > 100_000 {
                                    // >100ms
                                    stats_slow_clone
                                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                }

                                // Update metric with fractional milliseconds for sub-ms precision
                                metrics::histogram!("ingest.socket_send_duration_ms")
                                    .record(send_duration_ms_float);

                                // Calculate and record message lag (time between message creation and send)
                                if let Ok(envelope) =
                                    soar::protocol::deserialize_envelope(&serialized_envelope)
                                {
                                    let now_micros = chrono::Utc::now().timestamp_micros();
                                    let lag_seconds = (now_micros - envelope.timestamp_micros)
                                        as f64
                                        / 1_000_000.0;
                                    metrics::gauge!("ingest_message_lag_seconds").set(lag_seconds);
                                }

                                // Successfully delivered - commit the message
                                if let Err(e) = queue_for_publisher.commit().await {
                                    error!("Failed to commit message offset: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("Failed to send message to socket: {}", e);
                                metrics::counter!("ingest.socket_send_error_total").increment(1);

                                // DON'T commit - message will be replayed on next recv()
                                if let Err(e) = socket_client.reconnect().await {
                                    error!("Failed to reconnect socket: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // Don't die on queue errors - they may be transient (e.g., race
                        // between overflow writes and drain reads, corrupted segment that
                        // gets cleaned up). Log and retry after a delay.
                        error!("Failed to receive from queue: {} (will retry)", e);
                        metrics::counter!("ingest.queue_recv_error_total").increment(1);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }
        });
    }

    // Spawn periodic stats reporting task
    let queue_for_stats = queue.clone();
    let stats_frames_rx = stats_frames_received.clone();
    let stats_msgs_sent = stats_messages_sent.clone();
    let stats_bytes = stats_bytes_sent.clone();
    let stats_send_time = stats_send_time_total_ms.clone();
    let stats_slow = stats_slow_sends.clone();
    let stats_ogn_rx = stats_ogn_received.clone();
    let stats_beast_rx = stats_beast_received.clone();
    let stats_sbs_rx = stats_sbs_received.clone();
    let aprs_health_for_stats = aprs_health_shared.clone();
    let beast_health_for_stats = beast_health_shared.clone();
    let sbs_health_for_stats = sbs_health_shared.clone();
    let ogn_enabled_for_stats = ogn_enabled;
    let beast_enabled_for_stats = beast_enabled;
    let sbs_enabled_for_stats = sbs_enabled;

    tokio::spawn(async move {
        const STATS_INTERVAL_SECS: f64 = 30.0;
        const HALF_LIFE_SECS: f64 = 15.0 * 60.0; // 15-minute half-life

        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(STATS_INTERVAL_SECS as u64));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        // EWMA smoothers for send and receive rates (15-minute half-life)
        let mut ewma_sent = soar::metrics::Ewma::new(HALF_LIFE_SECS);
        let mut ewma_bytes_sent = soar::metrics::Ewma::new(HALF_LIFE_SECS);
        let mut ewma_incoming = soar::metrics::Ewma::new(HALF_LIFE_SECS);
        let mut ewma_ogn = soar::metrics::Ewma::new(HALF_LIFE_SECS);
        let mut ewma_beast = soar::metrics::Ewma::new(HALF_LIFE_SECS);
        let mut ewma_sbs = soar::metrics::Ewma::new(HALF_LIFE_SECS);

        loop {
            interval.tick().await;

            // Get and reset counters atomically
            let total_frames = stats_frames_rx.swap(0, std::sync::atomic::Ordering::Relaxed);
            let sent_count = stats_msgs_sent.swap(0, std::sync::atomic::Ordering::Relaxed);
            let bytes_sent = stats_bytes.swap(0, std::sync::atomic::Ordering::Relaxed);
            let total_send_time = stats_send_time.swap(0, std::sync::atomic::Ordering::Relaxed);
            let slow_count = stats_slow.swap(0, std::sync::atomic::Ordering::Relaxed);

            let ogn_frames = stats_ogn_rx.swap(0, std::sync::atomic::Ordering::Relaxed);
            let beast_frames = stats_beast_rx.swap(0, std::sync::atomic::Ordering::Relaxed);
            let sbs_frames = stats_sbs_rx.swap(0, std::sync::atomic::Ordering::Relaxed);

            // Calculate instantaneous rates (per second) for this window
            let instant_incoming = total_frames as f64 / STATS_INTERVAL_SECS;
            let instant_sent = sent_count as f64 / STATS_INTERVAL_SECS;
            let instant_bytes_sent = bytes_sent as f64 / STATS_INTERVAL_SECS;
            let instant_ogn = ogn_frames as f64 / STATS_INTERVAL_SECS;
            let instant_beast = beast_frames as f64 / STATS_INTERVAL_SECS;
            let instant_sbs = sbs_frames as f64 / STATS_INTERVAL_SECS;

            // Update EWMAs with this window's samples
            ewma_sent.update(instant_sent, STATS_INTERVAL_SECS);
            ewma_bytes_sent.update(instant_bytes_sent, STATS_INTERVAL_SECS);
            ewma_incoming.update(instant_incoming, STATS_INTERVAL_SECS);
            ewma_ogn.update(instant_ogn, STATS_INTERVAL_SECS);
            ewma_beast.update(instant_beast, STATS_INTERVAL_SECS);
            ewma_sbs.update(instant_sbs, STATS_INTERVAL_SECS);

            let sent_per_sec = ewma_sent.value();
            let incoming_per_sec = ewma_incoming.value();

            // Update aggregate metrics
            metrics::gauge!("ingest.messages_per_second").set(incoming_per_sec);

            // Update per-source metrics
            if ogn_frames > 0 || ewma_ogn.value() > 0.0 {
                metrics::gauge!("ingest.messages_per_second", "source" => "OGN")
                    .set(ewma_ogn.value());
            }
            if beast_frames > 0 || ewma_beast.value() > 0.0 {
                metrics::gauge!("ingest.messages_per_second", "source" => "Beast")
                    .set(ewma_beast.value());
            }
            if sbs_frames > 0 || ewma_sbs.value() > 0.0 {
                metrics::gauge!("ingest.messages_per_second", "source" => "SBS")
                    .set(ewma_sbs.value());
            }

            // Format sent stats (rate + avg send time combined)
            // total_send_time is in microseconds
            let sent_stats = if sent_count > 0 {
                let avg_us = total_send_time as f64 / sent_count as f64;
                // Format duration: use µs if < 1000µs, otherwise ms
                let avg_str = if avg_us < 1000.0 {
                    format!("{:.1}µs", avg_us)
                } else {
                    format!("{:.2}ms", avg_us / 1000.0)
                };
                if slow_count > 0 {
                    format!(
                        "{{rate:{:.1}/s avg:{} ({} >100ms)}}",
                        sent_per_sec, avg_str, slow_count
                    )
                } else {
                    format!("{{rate:{:.1}/s avg:{}}}", sent_per_sec, avg_str)
                }
            } else {
                format!("{{rate:{:.1}/s}}", sent_per_sec)
            };

            // Get unified queue depth
            let queue_depth = queue_for_stats.depth().await;

            // Update queue depth metrics
            metrics::gauge!("ingest.queue_depth", "type" => "memory")
                .set(queue_depth.memory as f64);
            metrics::gauge!("ingest.queue_depth_bytes", "type" => "data")
                .set(queue_depth.disk_data_bytes as f64);
            metrics::gauge!("ingest.queue_depth_bytes", "type" => "file")
                .set(queue_depth.disk_file_bytes as f64);
            metrics::gauge!("ingest.queue_segment_count").set(queue_depth.segment_count as f64);

            // Calculate estimated drain time from actual byte throughput.
            // bytes_sent_per_sec is measured directly from serialized envelope sizes,
            // so no per-message size estimate is needed.
            let bytes_sent_per_sec = ewma_bytes_sent.value();
            let drain_time_estimate = if queue_depth.disk_data_bytes > 0 {
                if bytes_sent_per_sec > 0.0 {
                    let est_seconds = queue_depth.disk_data_bytes as f64 / bytes_sent_per_sec;
                    if est_seconds > 3600.0 {
                        format!("{:.1}h", est_seconds / 3600.0)
                    } else if est_seconds > 60.0 {
                        format!("{:.1}m", est_seconds / 60.0)
                    } else {
                        format!("{:.0}s", est_seconds)
                    }
                } else {
                    "stalled".to_string()
                }
            } else {
                "done".to_string()
            };

            // Format queue info with data size (not file size) and drain ETA
            let queue_info = if queue_depth.disk_data_bytes == 0 && queue_depth.disk_file_bytes == 0
            {
                format!(
                    "mem:{} drain_eta={}",
                    queue_depth.memory, drain_time_estimate
                )
            } else if queue_depth.disk_data_bytes == queue_depth.disk_file_bytes {
                format!(
                    "mem:{} data:{}B drain_eta={}",
                    queue_depth.memory, queue_depth.disk_data_bytes, drain_time_estimate
                )
            } else {
                // Show both data and file size when different (indicates need for compaction)
                format!(
                    "mem:{} data:{}B file:{}B drain_eta={}",
                    queue_depth.memory,
                    queue_depth.disk_data_bytes,
                    queue_depth.disk_file_bytes,
                    drain_time_estimate
                )
            };

            // Build read stats section for enabled sources (using EWMA rates)
            let mut read_parts = Vec::new();
            if ogn_enabled_for_stats {
                let health = aprs_health_for_stats.read().await;
                read_parts.push(format!(
                    "ogn={{records:{} rate:{:.1}/s}}",
                    health.total_messages,
                    ewma_ogn.value()
                ));
            }
            if beast_enabled_for_stats {
                let health = beast_health_for_stats.read().await;
                read_parts.push(format!(
                    "beast={{records:{} rate:{:.1}/s}}",
                    health.total_messages,
                    ewma_beast.value()
                ));
            }
            if sbs_enabled_for_stats {
                let health = sbs_health_for_stats.read().await;
                read_parts.push(format!(
                    "sbs={{records:{} rate:{:.1}/s}}",
                    health.total_messages,
                    ewma_sbs.value()
                ));
            }

            // Format read section
            let read_section = if read_parts.is_empty() {
                String::new()
            } else {
                format!(" read: {}", read_parts.join(" "))
            };

            // Log comprehensive stats
            info!(
                "stats: sent={} queue={{{}}}{}",
                sent_stats, queue_info, read_section
            );
        }
    });

    // Spawn OGN client if enabled
    if ogn_enabled {
        let ogn_server = ogn_server.unwrap();
        let ogn_callsign = ogn_callsign.unwrap_or_else(|| "N0CALL".to_string());
        let ogn_port_val = ogn_port.unwrap_or(14580);

        // Automatically switch to port 10152 for full feed if no filter specified
        let ogn_port_final = if ogn_filter.is_none() && ogn_port_val == 14580 {
            info!("No OGN filter specified, switching from port 14580 to 10152 for full feed");
            10152
        } else {
            ogn_port_val
        };

        info!("Starting OGN client for {}:{}", ogn_server, ogn_port_final);

        let config = AprsClientConfigBuilder::new()
            .server(ogn_server)
            .port(ogn_port_final)
            .callsign(ogn_callsign)
            .filter(ogn_filter)
            .max_retries(max_retries)
            .retry_delay_seconds(retry_delay)
            .build();

        let queue_for_ogn = queue.clone();
        let aprs_health = aprs_health_shared.clone();
        let stats_rx = stats_ogn_received.clone();

        tokio::spawn(async move {
            let mut client = AprsClient::new(config);

            // The AprsClient's start_with_envelope_queue method creates protobuf envelopes
            // with timestamps captured at receive time
            loop {
                match client
                    .start_with_envelope_queue(
                        queue_for_ogn.clone(),
                        aprs_health.clone(),
                        Some(stats_rx.clone()),
                    )
                    .await
                {
                    Ok(_) => {
                        info!("OGN client stopped normally");
                        break;
                    }
                    Err(e) => {
                        error!("OGN client failed: {}", e);
                        // Will retry via internal retry logic
                    }
                }
            }
        });
    }

    // Spawn Beast clients if enabled
    for beast_addr in &beast_servers {
        let (server, port) = parse_server_address(beast_addr)?;
        let config = BeastClientConfig {
            server: server.clone(),
            port,
            max_retries,
            retry_delay_seconds: retry_delay,
            max_retry_delay_seconds: 60,
        };

        let queue_for_beast = queue.clone();
        let stats_rx = stats_beast_received.clone();
        let beast_health = beast_health_shared.clone();

        info!("Spawning Beast client for {}:{}", server, port);

        tokio::spawn(async move {
            let mut client = BeastClient::new(config);

            // The Beast client's start_with_envelope_queue creates protobuf envelopes
            // with timestamps captured at receive time
            match client
                .start_with_envelope_queue(queue_for_beast, beast_health, Some(stats_rx))
                .await
            {
                Ok(_) => {
                    info!("Beast client {}:{} stopped normally", server, port);
                }
                Err(e) => error!("Beast client {}:{} failed: {}", server, port, e),
            }
        });
    }

    // Spawn SBS clients if enabled
    for sbs_addr in &sbs_servers {
        let (server, port) = parse_server_address(sbs_addr)?;
        let config = SbsClientConfig {
            server: server.clone(),
            port,
            max_retries,
            retry_delay_seconds: retry_delay,
            max_retry_delay_seconds: 60,
        };

        let queue_for_sbs = queue.clone();
        let stats_rx = stats_sbs_received.clone();
        let sbs_health = sbs_health_shared.clone();

        info!("Spawning SBS client for {}:{}", server, port);

        tokio::spawn(async move {
            let mut client = SbsClient::new(config);

            // The SBS client's start_with_envelope_queue creates protobuf envelopes
            // with timestamps captured at receive time
            match client
                .start_with_envelope_queue(queue_for_sbs, sbs_health, Some(stats_rx))
                .await
            {
                Ok(_) => {
                    info!("SBS client {}:{} stopped normally", server, port);
                }
                Err(e) => error!("SBS client {}:{} failed: {}", server, port, e),
            }
        });
    }

    info!(
        "Started {} OGN client(s), {} Beast client(s), and {} SBS client(s)",
        if ogn_enabled { 1 } else { 0 },
        beast_servers.len(),
        sbs_servers.len()
    );

    // Keep running until process is terminated
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
    }
}

/// Initialize unified ingest metrics to zero/default values
/// This ensures metrics always appear in Prometheus queries even if no events have occurred
fn initialize_ingest_metrics() {
    // Aggregate metrics (all sources combined)
    metrics::gauge!("ingest.messages_per_second").set(0.0);
    metrics::counter!("ingest.messages_received_total").absolute(0);
    metrics::counter!("ingest.messages_sent_total").absolute(0);
    metrics::counter!("ingest.socket_send_error_total").absolute(0);

    // Socket send duration histogram
    metrics::histogram!("ingest.socket_send_duration_ms").record(0.0);

    // Message lag (time between message creation and send to processor)
    metrics::gauge!("ingest_message_lag_seconds").set(0.0);

    // Per-source receive rate metrics
    for source in &["OGN", "Beast", "SBS"] {
        metrics::gauge!("ingest.messages_per_second", "source" => *source).set(0.0);
        metrics::counter!("ingest.messages_received_total", "source" => *source).absolute(0);
    }

    // Unified queue depth metrics
    metrics::gauge!("ingest.queue_depth", "type" => "memory").set(0.0);
    metrics::gauge!("ingest.queue_depth_bytes", "type" => "data").set(0.0);
    metrics::gauge!("ingest.queue_depth_bytes", "type" => "file").set(0.0);
    metrics::gauge!("ingest.queue_segment_count").set(0.0);

    // Connection health metrics
    metrics::gauge!("ingest.health.ogn_connected").set(0.0);
    metrics::gauge!("ingest.health.beast_connected").set(0.0);
    metrics::gauge!("ingest.health.sbs_connected").set(0.0);
    metrics::gauge!("ingest.health.socket_connected").set(0.0);

    // Queue capacity pause metrics (backpressure events)
    metrics::counter!("queue.capacity_pause_total", "queue" => "ingest").absolute(0);
}
