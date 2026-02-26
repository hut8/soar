use anyhow::{Context, Result};
use soar::connection_status::ConnectionStatusPublisher;
use soar::ingest_config::{IngestConfigFile, ingest_config_path};
use soar::instance_lock::InstanceLock;
use soar::stream_manager::{SharedResources, StreamManager};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};

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
    /// Optional explicit path to the config file (overrides default resolution)
    pub config_path: Option<PathBuf>,
}

pub async fn handle_ingest(config: IngestConfig) -> Result<()> {
    // Resolve config file path
    let config_path = config.config_path.unwrap_or_else(ingest_config_path);

    info!("Loading ingest config from {:?}", config_path);

    let ingest_config = IngestConfigFile::load(&config_path)
        .with_context(|| format!("Failed to load ingest config from {:?}", config_path))?;

    let enabled_count = ingest_config.streams.iter().filter(|s| s.enabled).count();
    if enabled_count == 0 {
        warn!("No enabled streams in config file {:?}", config_path);
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

    for stream in &ingest_config.streams {
        if stream.enabled {
            info!(
                "  {} ({}) -> {}:{}",
                stream.name, stream.format, stream.host, stream.port
            );
        } else {
            info!("  {} (disabled)", stream.name);
        }
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
    let queue_dir = soar::queue_dir();
    std::fs::create_dir_all(&queue_dir)
        .with_context(|| format!("Failed to create queue directory: {:?}", queue_dir))?;

    let queue_path = queue_dir.join("ingest.queue");
    let queue = Arc::new(
        soar::persistent_queue::PersistentQueue::<Vec<u8>>::new(
            "ingest".to_string(),
            queue_path,
            3000, // Memory capacity (combined for all sources)
        )
        .expect("Failed to create unified ingest queue"),
    );

    info!("Created unified ingest queue at {:?}", queue_dir);

    // Create shared counters for stats tracking (aggregate across all sources)
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
                    publisher.set_ogn_status(ogn_connected, None).await;
                    last_ogn_connected = ogn_connected;
                }

                // Update ADS-B status if changed
                if adsb_connected != last_adsb_connected {
                    publisher.set_adsb_status(adsb_connected, Vec::new()).await;
                    last_adsb_connected = adsb_connected;
                }
            }
        });
    }

    // Create a single socket client for sending to soar-run
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

    // Spawn unified publisher task: reads from queue → sends to socket
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
                                    error!(error = %e, "Failed to commit message offset");
                                }
                            }
                            Err(e) => {
                                error!(error = %e, "Failed to send message to socket");
                                metrics::counter!("ingest.socket_send_error_total").increment(1);

                                // DON'T commit - message will be replayed on next recv()
                                if let Err(e) = socket_client.reconnect().await {
                                    error!(error = %e, "Failed to reconnect socket");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to receive from queue (will retry)");
                        metrics::counter!("ingest.queue_recv_error_total").increment(1);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }
        });
    }

    // Create shared resources for the stream manager
    let shared_resources = Arc::new(SharedResources {
        queue: queue.clone(),
        aprs_health: aprs_health_shared.clone(),
        beast_health: beast_health_shared.clone(),
        sbs_health: sbs_health_shared.clone(),
        stats_ogn_received: stats_ogn_received.clone(),
        stats_beast_received: stats_beast_received.clone(),
        stats_sbs_received: stats_sbs_received.clone(),
    });

    // Create the stream manager and apply initial config
    let manager = Arc::new(tokio::sync::Mutex::new(StreamManager::new(
        shared_resources,
        ingest_config.retry_delay,
    )));

    {
        let mut mgr = manager.lock().await;
        mgr.apply_config(&ingest_config);
        info!("Started {} stream(s) from config", mgr.running_count());
    }

    // Spawn config file watcher for hot-reload
    let _watcher_handle =
        soar::stream_manager::spawn_config_watcher(config_path.clone(), manager.clone());

    // Spawn periodic stats reporting task
    let queue_for_stats = queue.clone();
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
    tokio::spawn(async move {
        const STATS_INTERVAL_SECS: u64 = 30;

        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(STATS_INTERVAL_SECS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let mut last_stats_time = std::time::Instant::now();

        loop {
            interval.tick().await;

            // Measure actual elapsed time since last stats log
            let elapsed = last_stats_time.elapsed().as_secs_f64();
            last_stats_time = std::time::Instant::now();

            // Get and reset counters atomically
            let sent_count = stats_msgs_sent.swap(0, std::sync::atomic::Ordering::Relaxed);
            let bytes_sent = stats_bytes.swap(0, std::sync::atomic::Ordering::Relaxed);
            let total_send_time = stats_send_time.swap(0, std::sync::atomic::Ordering::Relaxed);
            let slow_count = stats_slow.swap(0, std::sync::atomic::Ordering::Relaxed);

            let ogn_frames = stats_ogn_rx.swap(0, std::sync::atomic::Ordering::Relaxed);
            let beast_frames = stats_beast_rx.swap(0, std::sync::atomic::Ordering::Relaxed);
            let sbs_frames = stats_sbs_rx.swap(0, std::sync::atomic::Ordering::Relaxed);

            // Calculate rates from count / actual elapsed time
            let total_frames = ogn_frames + beast_frames + sbs_frames;
            let incoming_per_sec = if elapsed > 0.0 {
                total_frames as f64 / elapsed
            } else {
                0.0
            };
            let sent_per_sec = if elapsed > 0.0 {
                sent_count as f64 / elapsed
            } else {
                0.0
            };
            let bytes_sent_per_sec = if elapsed > 0.0 {
                bytes_sent as f64 / elapsed
            } else {
                0.0
            };
            let ogn_per_sec = if elapsed > 0.0 {
                ogn_frames as f64 / elapsed
            } else {
                0.0
            };
            let beast_per_sec = if elapsed > 0.0 {
                beast_frames as f64 / elapsed
            } else {
                0.0
            };
            let sbs_per_sec = if elapsed > 0.0 {
                sbs_frames as f64 / elapsed
            } else {
                0.0
            };

            // Update aggregate metrics
            metrics::gauge!("ingest.messages_per_second").set(incoming_per_sec);

            // Update per-source metrics
            if ogn_frames > 0 {
                metrics::gauge!("ingest.messages_per_second", "source" => "OGN").set(ogn_per_sec);
            }
            if beast_frames > 0 {
                metrics::gauge!("ingest.messages_per_second", "source" => "Beast")
                    .set(beast_per_sec);
            }
            if sbs_frames > 0 {
                metrics::gauge!("ingest.messages_per_second", "source" => "SBS").set(sbs_per_sec);
            }

            // Format sent stats (rate + avg send time combined)
            let sent_stats = if sent_count > 0 {
                let avg_us = total_send_time as f64 / sent_count as f64;
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

            // Calculate estimated drain time from actual byte throughput
            let drain_time_estimate = if queue_depth.disk_data_bytes > 0 {
                if bytes_sent_per_sec < 1.0 {
                    "∞".to_string()
                } else {
                    let est_seconds = queue_depth.disk_data_bytes as f64 / bytes_sent_per_sec;
                    if est_seconds > 3600.0 {
                        format!("{:.1}h", est_seconds / 3600.0)
                    } else if est_seconds > 60.0 {
                        format!("{:.1}m", est_seconds / 60.0)
                    } else {
                        format!("{:.0}s", est_seconds)
                    }
                }
            } else {
                "done".to_string()
            };

            // Format queue info
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
                format!(
                    "mem:{} data:{}B file:{}B drain_eta={}",
                    queue_depth.memory,
                    queue_depth.disk_data_bytes,
                    queue_depth.disk_file_bytes,
                    drain_time_estimate
                )
            };

            // Build read stats section aggregated by format type
            // Health objects are shared per format, so we report one entry per format
            let mut read_parts = Vec::new();
            if ogn_frames > 0 || {
                let h = aprs_health_for_stats.read().await;
                h.aprs_connected
            } {
                let health = aprs_health_for_stats.read().await;
                read_parts.push(format!(
                    "aprs={{records:{} rate:{:.1}/s}}",
                    health.total_messages, ogn_per_sec
                ));
            }
            if beast_frames > 0 || {
                let h = beast_health_for_stats.read().await;
                h.beast_connected
            } {
                let health = beast_health_for_stats.read().await;
                read_parts.push(format!(
                    "beast={{records:{} rate:{:.1}/s}}",
                    health.total_messages, beast_per_sec
                ));
            }
            if sbs_frames > 0 || {
                let h = sbs_health_for_stats.read().await;
                h.beast_connected
            } {
                let health = sbs_health_for_stats.read().await;
                read_parts.push(format!(
                    "sbs={{records:{} rate:{:.1}/s}}",
                    health.total_messages, sbs_per_sec
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
}
