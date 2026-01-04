use anyhow::{Context, Result};
use soar::aprs_client::{AprsClient, AprsClientConfigBuilder};
use soar::instance_lock::InstanceLock;
use std::env;
use tracing::Instrument;
use tracing::{error, info, warn};

pub async fn handle_ingest_ogn(
    server: String,
    mut port: u16,
    callsign: String,
    filter: Option<String>,
    max_retries: u32,
    retry_delay: u64,
    nats_url: String,
) -> Result<()> {
    sentry::configure_scope(|scope| {
        scope.set_tag("operation", "ingest-ogn");
    });

    // Automatically switch to port 10152 for full feed if no filter specified
    // Port 14580 requires a filter, port 10152 provides the full global feed
    if filter.is_none() && port == 14580 {
        info!("No filter specified, switching from port 14580 to 10152 for full feed");
        port = 10152;
    }

    // Determine environment and use appropriate NATS subject
    // Production: "ogn.raw"
    // Staging: "staging.ogn.raw"
    let soar_env = env::var("SOAR_ENV").unwrap_or_default();
    let is_production = soar_env == "production";
    let is_staging = soar_env == "staging";

    let nats_subject = if is_production {
        "ogn.raw"
    } else {
        "staging.ogn.raw"
    };

    info!(
        "Starting OGN ingestion service - server: {}:{}, NATS: {}, subject: {}",
        server, port, nats_url, nats_subject
    );

    info!(
        "Environment: {}, using NATS subject '{}'",
        if is_production {
            "production"
        } else {
            "staging"
        },
        nats_subject
    );

    // Initialize health state for this ingester
    let health_state = soar::metrics::init_aprs_health();
    soar::metrics::set_aprs_health(health_state.clone());

    // Initialize all OGN ingester metrics to zero so they appear in Grafana even before events occur
    // This MUST happen before starting the metrics server to avoid race conditions where
    // Prometheus scrapes before metrics are initialized
    info!("Initializing OGN ingester metrics...");
    soar::metrics::initialize_aprs_ingest_metrics();
    info!("OGN ingester metrics initialized");

    // Start metrics server in production/staging mode (AFTER metrics are initialized)
    if is_production || is_staging {
        // Allow overriding metrics port via METRICS_PORT env var (for blue-green deployment)
        // Auto-assign default based on environment: production=9093, staging=9094
        let default_port = if is_staging { 9094 } else { 9093 };
        let metrics_port = env::var("METRICS_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(default_port);

        info!("Starting metrics server on port {}", metrics_port);
        tokio::spawn(
            async move {
                soar::metrics::start_metrics_server(metrics_port).await;
            }
            .instrument(tracing::info_span!("metrics_server")),
        );
    }

    // Acquire instance lock to prevent multiple ingest instances from running
    let lock_name = if is_production {
        "ogn-ingest-production"
    } else {
        "ogn-ingest-dev"
    };
    let _lock = InstanceLock::new(lock_name)
        .context("Failed to acquire instance lock - is another ogn-ingest instance running?")?;
    info!("Instance lock acquired for {}", lock_name);

    // Set up signal handling for immediate shutdown
    info!("Setting up shutdown handlers...");
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Spawn signal handler task for both SIGINT and SIGTERM
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};

            let mut sigterm =
                signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");
            let mut sigint =
                signal(SignalKind::interrupt()).expect("Failed to register SIGINT handler");

            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, exiting immediately...");
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT (Ctrl+C), exiting immediately...");
                }
            }
        }

        #[cfg(not(unix))]
        {
            match tokio::signal::ctrl_c().await {
                Ok(()) => {
                    info!("Received SIGINT (Ctrl+C), exiting immediately...");
                }
                Err(err) => {
                    error!("Failed to listen for SIGINT signal: {}", err);
                    return;
                }
            }
        }

        // Signal shutdown
        let _ = shutdown_tx.send(());
    });

    // Create OGN (APRS) client config
    let config = AprsClientConfigBuilder::new()
        .server(server)
        .port(port)
        .callsign(callsign)
        .filter(filter)
        .max_retries(max_retries)
        .retry_delay_seconds(retry_delay)
        .build();

    // Create persistent queue for buffering messages
    let queue_path = std::path::PathBuf::from("/var/lib/soar/queues/ogn.queue");
    let queue = std::sync::Arc::new(
        soar::persistent_queue::PersistentQueue::<String>::new(
            "ogn".to_string(),
            queue_path,
            Some(10 * 1024 * 1024 * 1024), // 10 GB max
            1000,                          // Memory capacity
        )
        .expect("Failed to create persistent queue"),
    );

    info!("Created persistent queue at /var/lib/soar/queues/ogn.queue");

    // Create socket client for sending to soar-run
    let socket_path = std::path::PathBuf::from("/var/run/soar/run.sock");
    let mut socket_client = match soar::socket_client::SocketClient::connect(
        &socket_path,
        soar::protocol::IngestSource::Ogn,
    )
    .await
    {
        Ok(client) => {
            info!("Connected to soar-run at {:?}", socket_path);
            client
        }
        Err(e) => {
            warn!(
                "Failed to connect to soar-run (will buffer to queue): {}",
                e
            );
            // Create disconnected client - it will connect later when soar-run is available
            soar::socket_client::SocketClient::new(&socket_path, soar::protocol::IngestSource::Ogn)
        }
    };

    // Mark socket as connected in health state
    {
        let mut health = health_state.write().await;
        health.nats_connected = socket_client.is_connected(); // Reuse nats_connected field for now
    }

    // Connect consumer to queue (transition from Disconnected to Connected/Draining state)
    queue
        .connect_consumer("ogn-publisher".to_string())
        .await
        .expect("Failed to connect consumer to queue");

    // Spawn publisher task: reads from queue → sends to socket
    let queue_for_publisher = queue.clone();
    let publisher_handle = tokio::spawn(async move {
        info!("Publisher task started");
        loop {
            match queue_for_publisher.recv().await {
                Ok(message) => {
                    // Send to socket
                    if let Err(e) = socket_client.send(message.into_bytes()).await {
                        error!("Failed to send to socket: {}", e);
                        metrics::counter!("aprs.socket.send_error_total").increment(1);

                        // Reconnect and retry
                        if let Err(e) = socket_client.reconnect().await {
                            error!("Failed to reconnect to socket: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to receive from queue: {}", e);
                    break;
                }
            }
        }
        info!("Publisher task stopped");
    });

    let mut client = AprsClient::new(config.clone());

    info!("Starting OGN (APRS) client for ingestion...");

    // Run OGN (APRS) client with shutdown signal
    let client_result = client
        .start_with_queue(queue.clone(), shutdown_rx, health_state.clone())
        .await;

    // Wait for publisher to finish
    let _ = publisher_handle.await;

    match client_result {
        Ok(_) => {
            info!("OGN ingestion stopped normally");
        }
        Err(e) => {
            error!("OGN ingestion failed: {}", e);
            metrics::counter!("aprs.ingest_failed_total").increment(1);
        }
    }

    Ok(())
}
