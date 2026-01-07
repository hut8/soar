use anyhow::{Context, Result};
use soar::aprs_client::{AprsClient, AprsClientConfigBuilder};
use soar::beast::{BeastClient, BeastClientConfig};
use soar::instance_lock::InstanceLock;
use soar::sbs::{SbsClient, SbsClientConfig};
use std::env;
use std::sync::Arc;
use tracing::Instrument;
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
#[allow(dead_code)] // Health tracking will be implemented in future iterations
pub struct IngestHealth {
    pub ogn_connected: bool,
    pub beast_connected: bool,
    pub sbs_connected: bool,
    pub socket_connected: bool,
    pub last_ogn_message_time: Option<std::time::Instant>,
    pub last_beast_message_time: Option<std::time::Instant>,
    pub last_sbs_message_time: Option<std::time::Instant>,
}

pub async fn handle_ingest(
    // OGN parameters
    ogn_server: Option<String>,
    ogn_port: Option<u16>,
    ogn_callsign: Option<String>,
    ogn_filter: Option<String>,
    // ADS-B parameters
    beast_servers: Vec<String>,
    sbs_servers: Vec<String>,
    // Common parameters
    max_retries: u32,
    retry_delay: u64,
) -> Result<()> {
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
        tokio::spawn(
            async move {
                soar::metrics::start_metrics_server(metrics_port, Some("ingest")).await;
            }
            .instrument(tracing::info_span!("metrics_server")),
        );
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

    // Create persistent queues for buffering messages (separate queues per source)
    let ogn_queue_path = std::path::PathBuf::from("/var/lib/soar/queues/ogn.queue");
    let ogn_queue = Arc::new(
        soar::persistent_queue::PersistentQueue::<String>::new(
            "ogn".to_string(),
            ogn_queue_path,
            Some(10 * 1024 * 1024 * 1024), // 10 GB max
            1000,                          // Memory capacity
        )
        .expect("Failed to create OGN persistent queue"),
    );

    let beast_queue_path = std::path::PathBuf::from("/var/lib/soar/queues/adsb-beast.queue");
    let beast_queue = Arc::new(
        soar::persistent_queue::PersistentQueue::<Vec<u8>>::new(
            "adsb-beast".to_string(),
            beast_queue_path,
            Some(10 * 1024 * 1024 * 1024), // 10 GB max
            1000,                          // Memory capacity
        )
        .expect("Failed to create Beast persistent queue"),
    );

    let sbs_queue_path = std::path::PathBuf::from("/var/lib/soar/queues/adsb-sbs.queue");
    let sbs_queue = Arc::new(
        soar::persistent_queue::PersistentQueue::<Vec<u8>>::new(
            "adsb-sbs".to_string(),
            sbs_queue_path,
            Some(10 * 1024 * 1024 * 1024), // 10 GB max
            1000,                          // Memory capacity
        )
        .expect("Failed to create SBS persistent queue"),
    );

    info!("Created persistent queues at /var/lib/soar/queues/");

    // Create shared counters for stats tracking (aggregate across all sources)
    let stats_frames_received = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let stats_messages_sent = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let stats_send_time_total_ms = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let stats_slow_sends = Arc::new(std::sync::atomic::AtomicU64::new(0));

    // Per-source counters for individual metrics
    let stats_ogn_received = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let stats_beast_received = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let stats_sbs_received = Arc::new(std::sync::atomic::AtomicU64::new(0));

    // Create socket clients for sending to soar-run
    let mut ogn_socket_client = match soar::socket_client::SocketClient::connect(
        &socket_path,
        soar::protocol::IngestSource::Ogn,
    )
    .await
    {
        Ok(client) => {
            info!("OGN socket connected to soar-run at {:?}", socket_path);
            client
        }
        Err(e) => {
            warn!(
                "Failed to connect OGN socket to soar-run (will buffer to queue): {}",
                e
            );
            soar::socket_client::SocketClient::new(&socket_path, soar::protocol::IngestSource::Ogn)
        }
    };

    let mut beast_socket_client = match soar::socket_client::SocketClient::connect(
        &socket_path,
        soar::protocol::IngestSource::Beast,
    )
    .await
    {
        Ok(client) => {
            info!("Beast socket connected to soar-run at {:?}", socket_path);
            client
        }
        Err(e) => {
            warn!(
                "Failed to connect Beast socket to soar-run (will buffer to queue): {}",
                e
            );
            soar::socket_client::SocketClient::new(
                &socket_path,
                soar::protocol::IngestSource::Beast,
            )
        }
    };

    let mut sbs_socket_client = match soar::socket_client::SocketClient::connect(
        &socket_path,
        soar::protocol::IngestSource::Sbs,
    )
    .await
    {
        Ok(client) => {
            info!("SBS socket connected to soar-run at {:?}", socket_path);
            client
        }
        Err(e) => {
            warn!(
                "Failed to connect SBS socket to soar-run (will buffer to queue): {}",
                e
            );
            soar::socket_client::SocketClient::new(&socket_path, soar::protocol::IngestSource::Sbs)
        }
    };

    // Update health state with socket connection status
    {
        let mut health = health_state.write().await;
        health.socket_connected = ogn_socket_client.is_connected()
            || beast_socket_client.is_connected()
            || sbs_socket_client.is_connected();
    }

    // Connect consumers to queues
    ogn_queue
        .connect_consumer("ogn-publisher".to_string())
        .await
        .expect("Failed to connect OGN consumer to queue");
    beast_queue
        .connect_consumer("beast-publisher".to_string())
        .await
        .expect("Failed to connect Beast consumer to queue");
    sbs_queue
        .connect_consumer("sbs-publisher".to_string())
        .await
        .expect("Failed to connect SBS consumer to queue");

    // Spawn OGN publisher task: reads from queue → sends to socket
    if ogn_enabled {
        let queue_for_publisher = ogn_queue.clone();
        let stats_sent_clone = stats_messages_sent.clone();
        let stats_time_clone = stats_send_time_total_ms.clone();
        let stats_slow_clone = stats_slow_sends.clone();
        tokio::spawn(async move {
            info!("OGN publisher task started");
            loop {
                match queue_for_publisher.recv().await {
                    Ok(message) => {
                        // Track send timing
                        let send_start = std::time::Instant::now();

                        // Send to socket
                        match ogn_socket_client.send(message.into_bytes()).await {
                            Ok(_) => {
                                // Track stats
                                let send_duration_ms = send_start.elapsed().as_millis() as u64;
                                stats_sent_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                stats_time_clone
                                    .fetch_add(send_duration_ms, std::sync::atomic::Ordering::Relaxed);
                                if send_duration_ms > 100 {
                                    stats_slow_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                }

                                // Update per-source metric
                                metrics::histogram!("ingest.socket_send_duration_ms", "source" => "ogn")
                                    .record(send_duration_ms as f64);

                                // Successfully delivered - commit the message
                                if let Err(e) = queue_for_publisher.commit().await {
                                    error!("Failed to commit OGN message offset: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("Failed to send OGN message to socket: {}", e);
                                metrics::counter!("ingest.socket_send_error_total", "source" => "ogn")
                                    .increment(1);

                                // DON'T commit - message will be replayed on next recv()
                                if let Err(e) = ogn_socket_client.reconnect().await {
                                    error!("Failed to reconnect OGN socket: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to receive from OGN queue: {}", e);
                        break;
                    }
                }
            }
            info!("OGN publisher task stopped");
        });
    }

    // Spawn Beast publisher task: reads from queue → sends to socket
    if beast_enabled {
        let queue_for_publisher = beast_queue.clone();
        let stats_sent_clone = stats_messages_sent.clone();
        let stats_time_clone = stats_send_time_total_ms.clone();
        let stats_slow_clone = stats_slow_sends.clone();
        tokio::spawn(async move {
            info!("Beast publisher task started");
            loop {
                match queue_for_publisher.recv().await {
                    Ok(message) => {
                        // Track send timing
                        let send_start = std::time::Instant::now();

                        // Send to socket
                        match beast_socket_client.send(message).await {
                            Ok(_) => {
                                // Track stats
                                let send_duration_ms = send_start.elapsed().as_millis() as u64;
                                stats_sent_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                stats_time_clone
                                    .fetch_add(send_duration_ms, std::sync::atomic::Ordering::Relaxed);
                                if send_duration_ms > 100 {
                                    stats_slow_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                }

                                // Update per-source metric
                                metrics::histogram!("ingest.socket_send_duration_ms", "source" => "beast")
                                    .record(send_duration_ms as f64);

                                // Successfully delivered - commit the message
                                if let Err(e) = queue_for_publisher.commit().await {
                                    error!("Failed to commit Beast message offset: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("Failed to send Beast message to socket: {}", e);
                                metrics::counter!("ingest.socket_send_error_total", "source" => "beast")
                                    .increment(1);

                                // DON'T commit - message will be replayed on next recv()
                                if let Err(e) = beast_socket_client.reconnect().await {
                                    error!("Failed to reconnect Beast socket: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to receive from Beast queue: {}", e);
                        break;
                    }
                }
            }
            info!("Beast publisher task stopped");
        });
    }

    // Spawn SBS publisher task: reads from queue → sends to socket
    if sbs_enabled {
        let queue_for_publisher = sbs_queue.clone();
        let stats_sent_clone = stats_messages_sent.clone();
        let stats_time_clone = stats_send_time_total_ms.clone();
        let stats_slow_clone = stats_slow_sends.clone();
        tokio::spawn(async move {
            info!("SBS publisher task started");
            loop {
                match queue_for_publisher.recv().await {
                    Ok(message) => {
                        // Track send timing
                        let send_start = std::time::Instant::now();

                        // Send to socket
                        match sbs_socket_client.send(message).await {
                            Ok(_) => {
                                // Track stats
                                let send_duration_ms = send_start.elapsed().as_millis() as u64;
                                stats_sent_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                stats_time_clone
                                    .fetch_add(send_duration_ms, std::sync::atomic::Ordering::Relaxed);
                                if send_duration_ms > 100 {
                                    stats_slow_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                }

                                // Update per-source metric
                                metrics::histogram!("ingest.socket_send_duration_ms", "source" => "sbs")
                                    .record(send_duration_ms as f64);

                                // Successfully delivered - commit the message
                                if let Err(e) = queue_for_publisher.commit().await {
                                    error!("Failed to commit SBS message offset: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("Failed to send SBS message to socket: {}", e);
                                metrics::counter!("ingest.socket_send_error_total", "source" => "sbs")
                                    .increment(1);

                                // DON'T commit - message will be replayed on next recv()
                                if let Err(e) = sbs_socket_client.reconnect().await {
                                    error!("Failed to reconnect SBS socket: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to receive from SBS queue: {}", e);
                        break;
                    }
                }
            }
            info!("SBS publisher task stopped");
        });
    }

    // Spawn periodic stats reporting task
    let ogn_queue_for_stats = ogn_queue.clone();
    let beast_queue_for_stats = beast_queue.clone();
    let sbs_queue_for_stats = sbs_queue.clone();
    let stats_frames_rx = stats_frames_received.clone();
    let stats_msgs_sent = stats_messages_sent.clone();
    let stats_send_time = stats_send_time_total_ms.clone();
    let stats_slow = stats_slow_sends.clone();
    let stats_ogn_rx = stats_ogn_received.clone();
    let stats_beast_rx = stats_beast_received.clone();
    let stats_sbs_rx = stats_sbs_received.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            // Get and reset counters atomically
            let total_frames = stats_frames_rx.swap(0, std::sync::atomic::Ordering::Relaxed);
            let sent_count = stats_msgs_sent.swap(0, std::sync::atomic::Ordering::Relaxed);
            let total_send_time = stats_send_time.swap(0, std::sync::atomic::Ordering::Relaxed);
            let slow_count = stats_slow.swap(0, std::sync::atomic::Ordering::Relaxed);

            let ogn_frames = stats_ogn_rx.swap(0, std::sync::atomic::Ordering::Relaxed);
            let beast_frames = stats_beast_rx.swap(0, std::sync::atomic::Ordering::Relaxed);
            let sbs_frames = stats_sbs_rx.swap(0, std::sync::atomic::Ordering::Relaxed);

            // Calculate rates (per second)
            let total_per_sec = total_frames as f64 / 30.0;
            let sent_per_sec = sent_count as f64 / 30.0;

            // Update aggregate metrics
            metrics::gauge!("ingest.messages_per_second").set(total_per_sec);

            // Update per-source metrics
            if ogn_frames > 0 {
                metrics::gauge!("ingest.messages_per_second", "source" => "ogn")
                    .set(ogn_frames as f64 / 30.0);
            }
            if beast_frames > 0 {
                metrics::gauge!("ingest.messages_per_second", "source" => "beast")
                    .set(beast_frames as f64 / 30.0);
            }
            if sbs_frames > 0 {
                metrics::gauge!("ingest.messages_per_second", "source" => "sbs")
                    .set(sbs_frames as f64 / 30.0);
            }

            // Calculate average socket send time
            let avg_send_time_ms = if sent_count > 0 {
                let avg = total_send_time as f64 / sent_count as f64;
                if slow_count > 0 {
                    format!("{:.1}ms ({} >100ms)", avg, slow_count)
                } else {
                    format!("{:.1}ms", avg)
                }
            } else {
                "N/A".to_string()
            };

            // Get queue depths
            let ogn_depth = ogn_queue_for_stats.depth().await;
            let beast_depth = beast_queue_for_stats.depth().await;
            let sbs_depth = sbs_queue_for_stats.depth().await;

            // Update queue depth metrics
            metrics::gauge!("ingest.queue_depth", "source" => "ogn", "type" => "memory")
                .set(ogn_depth.memory as f64);
            metrics::gauge!("ingest.queue_depth", "source" => "ogn", "type" => "disk")
                .set(ogn_depth.disk as f64);
            metrics::gauge!("ingest.queue_depth", "source" => "beast", "type" => "memory")
                .set(beast_depth.memory as f64);
            metrics::gauge!("ingest.queue_depth", "source" => "beast", "type" => "disk")
                .set(beast_depth.disk as f64);
            metrics::gauge!("ingest.queue_depth", "source" => "sbs", "type" => "memory")
                .set(sbs_depth.memory as f64);
            metrics::gauge!("ingest.queue_depth", "source" => "sbs", "type" => "disk")
                .set(sbs_depth.disk as f64);

            // Log comprehensive stats
            info!(
                "Ingest Stats (30s): recv={:.1}/s (OGN:{:.1} Beast:{:.1} SBS:{:.1}) sent={:.1}/s | socket_send={} | queues: ogn={{mem:{} disk:{}B}} beast={{mem:{} disk:{}B}} sbs={{mem:{} disk:{}B}}",
                total_per_sec,
                ogn_frames as f64 / 30.0,
                beast_frames as f64 / 30.0,
                sbs_frames as f64 / 30.0,
                sent_per_sec,
                avg_send_time_ms,
                ogn_depth.memory,
                ogn_depth.disk,
                beast_depth.memory,
                beast_depth.disk,
                sbs_depth.memory,
                sbs_depth.disk
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

        let queue = ogn_queue.clone();
        let _stats_rx = stats_ogn_received.clone();
        let _stats_total = stats_frames_received.clone();

        // Create wrapper health state for APRS client compatibility
        let aprs_health = Arc::new(tokio::sync::RwLock::new(soar::metrics::AprsIngestHealth::default()));

        tokio::spawn(async move {
            let mut client = AprsClient::new(config);
            
            // The AprsClient's start_with_queue method will handle stats internally
            // We wrap the queue to track our own stats as well
            loop {
                match client.start_with_queue(queue.clone(), aprs_health.clone()).await {
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

        let queue = beast_queue.clone();
        let stats_rx = stats_beast_received.clone();
        let _stats_total = stats_frames_received.clone();

        info!("Spawning Beast client for {}:{}", server, port);
        let server_clone = server.clone();

        // Create wrapper health state for Beast client compatibility
        let beast_health = Arc::new(tokio::sync::RwLock::new(soar::metrics::BeastIngestHealth::default()));

        tokio::spawn(
            async move {
                let mut client = BeastClient::new(config);
                
                // The Beast client's start_with_queue tracks stats - pass our counters
                // The stats counter updates both per-source and total
                match client.start_with_queue(queue, beast_health, Some(stats_rx)).await {
                    Ok(_) => {
                        info!("Beast client {}:{} stopped normally", server, port);
                    }
                    Err(e) => error!("Beast client {}:{} failed: {}", server, port, e),
                }
            }
            .instrument(tracing::info_span!("beast_client", server = %server_clone, port = %port)),
        );
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

        let queue = sbs_queue.clone();
        let stats_rx = stats_sbs_received.clone();
        let _stats_total = stats_frames_received.clone();

        info!("Spawning SBS client for {}:{}", server, port);
        let server_clone = server.clone();

        // Create wrapper health state for SBS client compatibility
        let beast_health = Arc::new(tokio::sync::RwLock::new(soar::metrics::BeastIngestHealth::default()));

        tokio::spawn(
            async move {
                let mut client = SbsClient::new(config);
                
                // The SBS client's start_with_queue tracks stats - pass our counters
                match client.start_with_queue(queue, beast_health, Some(stats_rx)).await {
                    Ok(_) => {
                        info!("SBS client {}:{} stopped normally", server, port);
                    }
                    Err(e) => error!("SBS client {}:{} failed: {}", server, port, e),
                }
            }
            .instrument(tracing::info_span!("sbs_client", server = %server_clone, port = %port)),
        );
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

    // Per-source metrics (OGN, Beast, SBS)
    for source in &["ogn", "beast", "sbs"] {
        metrics::gauge!("ingest.messages_per_second", "source" => *source).set(0.0);
        metrics::counter!("ingest.messages_received_total", "source" => *source).absolute(0);
        metrics::counter!("ingest.messages_sent_total", "source" => *source).absolute(0);
        metrics::counter!("ingest.socket_send_error_total", "source" => *source).absolute(0);
        
        // Socket send duration histogram
        metrics::histogram!("ingest.socket_send_duration_ms", "source" => *source).record(0.0);
        
        // Queue depth metrics
        for queue_type in &["memory", "disk"] {
            metrics::gauge!("ingest.queue_depth", "source" => *source, "type" => *queue_type).set(0.0);
        }
    }

    // Connection health metrics
    metrics::gauge!("ingest.health.ogn_connected").set(0.0);
    metrics::gauge!("ingest.health.beast_connected").set(0.0);
    metrics::gauge!("ingest.health.sbs_connected").set(0.0);
    metrics::gauge!("ingest.health.socket_connected").set(0.0);
}
