use anyhow::{Context, Result};
use soar::beast::{BeastClient, BeastClientConfig};
use soar::instance_lock::InstanceLock;
use soar::sbs::{SbsClient, SbsClientConfig};
use std::env;
use tracing::Instrument;
use tracing::{error, info};

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

pub async fn handle_ingest_adsb(
    beast_servers: Vec<String>,
    sbs_servers: Vec<String>,
    max_retries: u32,
    retry_delay: u64,
    nats_url: String,
) -> Result<()> {
    sentry::configure_scope(|scope| {
        scope.set_tag("operation", "ingest-adsb");
    });

    // Validate that at least one server is specified
    if beast_servers.is_empty() && sbs_servers.is_empty() {
        return Err(anyhow::anyhow!(
            "No servers specified - use --beast or --sbs to specify at least one server"
        ));
    }

    // Determine environment and use appropriate NATS subjects
    // Production: "adsb.raw" (Beast), "adsb.sbs" (SBS)
    // Staging: "staging.adsb.raw" (Beast), "staging.adsb.sbs" (SBS)
    let soar_env = env::var("SOAR_ENV").unwrap_or_default();
    let is_production = soar_env == "production";
    let is_staging = soar_env == "staging";

    let beast_subject = if is_production {
        "adsb.raw"
    } else {
        "staging.adsb.raw"
    };

    let sbs_subject = if is_production {
        "adsb.sbs"
    } else {
        "staging.adsb.sbs"
    };

    info!(
        "Starting ADS-B ingestion service - Beast servers: {:?}, SBS servers: {:?}, NATS: {}",
        beast_servers, sbs_servers, nats_url
    );

    info!(
        "Environment: {}, Beast subject: '{}', SBS subject: '{}'",
        if is_production {
            "production"
        } else {
            "staging"
        },
        beast_subject,
        sbs_subject
    );

    // Initialize health state for this ingester
    let health_state = soar::metrics::init_beast_health();
    soar::metrics::set_beast_health(health_state.clone());

    // Initialize all ADS-B ingester metrics to zero so they appear in Grafana even before events occur
    // This MUST happen before starting the metrics server to avoid race conditions where
    // Prometheus scrapes before metrics are initialized
    info!("Initializing ADS-B ingester metrics...");
    soar::metrics::initialize_beast_ingest_metrics();
    info!("ADS-B ingester metrics initialized");

    // Start metrics server in production/staging mode (AFTER metrics are initialized)
    if is_production || is_staging {
        // Allow overriding metrics port via METRICS_PORT env var (for blue-green deployment)
        // Auto-assign default based on environment: production=9094, staging=9096
        let default_port = if is_staging { 9096 } else { 9094 };
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
        "adsb-ingest-production"
    } else {
        "adsb-ingest-dev"
    };
    let _lock = InstanceLock::new(lock_name)
        .context("Failed to acquire instance lock - is another adsb-ingest instance running?")?;
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

    // Connect to NATS
    info!("Connecting to NATS at {}...", nats_url);
    let nats_client_name = soar::nats_client_name("adsb-ingester");
    let nats_client = async_nats::ConnectOptions::new()
        .name(&nats_client_name)
        .connect(&nats_url)
        .await
        .context("Failed to connect to NATS")?;
    info!("Connected to NATS successfully");

    // Mark NATS as connected in health state
    {
        let mut health = health_state.write().await;
        health.nats_connected = true;
    }

    // Create publishers
    let beast_publisher = soar::beast_nats_publisher::NatsPublisher::new(
        nats_client.clone(),
        beast_subject.to_string(),
    );
    let sbs_publisher =
        soar::sbs_nats_publisher::SbsNatsPublisher::new(nats_client, sbs_subject.to_string());

    // Create broadcast channel for shutdown signal (multiple tasks need to receive it)
    let (shutdown_broadcast_tx, _) = tokio::sync::broadcast::channel::<()>(1);

    // Spawn signal handler task
    let shutdown_tx_clone = shutdown_broadcast_tx.clone();
    tokio::spawn(async move {
        let _ = shutdown_rx.await;
        info!("Shutdown signal received, broadcasting to all client tasks...");
        let _ = shutdown_tx_clone.send(());
    });

    // Spawn tasks for each Beast server
    let mut client_handles = vec![];

    for beast_addr in &beast_servers {
        let (server, port) = parse_server_address(beast_addr)?;
        let config = BeastClientConfig {
            server: server.clone(),
            port,
            max_retries,
            retry_delay_seconds: retry_delay,
            max_retry_delay_seconds: 60,
        };

        let publisher = beast_publisher.clone();
        let health = health_state.clone();
        let shutdown_rx = shutdown_broadcast_tx.subscribe();
        let (client_shutdown_tx, client_shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        // Bridge broadcast to oneshot
        tokio::spawn(async move {
            let mut rx = shutdown_rx;
            if rx.recv().await.is_ok() {
                let _ = client_shutdown_tx.send(());
            }
        });

        info!("Spawning Beast client for {}:{}", server, port);
        let server_clone = server.clone();
        let handle = tokio::spawn(
            async move {
                let mut client = BeastClient::new(config);
                match client
                    .start_with_shutdown(publisher, client_shutdown_rx, health)
                    .await
                {
                    Ok(_) => info!("Beast client {}:{} stopped normally", server, port),
                    Err(e) => error!("Beast client {}:{} failed: {}", server, port, e),
                }
            }
            .instrument(tracing::info_span!("beast_client", server = %server_clone, port = %port)),
        );
        client_handles.push(handle);
    }

    // Spawn tasks for each SBS server
    for sbs_addr in &sbs_servers {
        let (server, port) = parse_server_address(sbs_addr)?;
        let config = SbsClientConfig {
            server: server.clone(),
            port,
            max_retries,
            retry_delay_seconds: retry_delay,
            max_retry_delay_seconds: 60,
        };

        let publisher = sbs_publisher.clone();
        let health = health_state.clone();
        let shutdown_rx = shutdown_broadcast_tx.subscribe();
        let (client_shutdown_tx, client_shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        // Bridge broadcast to oneshot
        tokio::spawn(async move {
            let mut rx = shutdown_rx;
            if rx.recv().await.is_ok() {
                let _ = client_shutdown_tx.send(());
            }
        });

        info!("Spawning SBS client for {}:{}", server, port);
        let server_clone = server.clone();
        let handle = tokio::spawn(
            async move {
                let mut client = SbsClient::new(config);
                match client
                    .start_with_shutdown(publisher, client_shutdown_rx, health)
                    .await
                {
                    Ok(_) => info!("SBS client {}:{} stopped normally", server, port),
                    Err(e) => error!("SBS client {}:{} failed: {}", server, port, e),
                }
            }
            .instrument(tracing::info_span!("sbs_client", server = %server_clone, port = %port)),
        );
        client_handles.push(handle);
    }

    info!(
        "Started {} Beast client(s) and {} SBS client(s)",
        beast_servers.len(),
        sbs_servers.len()
    );

    // Wait for all client tasks to complete
    for handle in client_handles {
        let _ = handle.await;
    }

    info!("All client tasks completed, shutting down");

    // Mark NATS as disconnected
    {
        let mut health = health_state.write().await;
        health.nats_connected = false;
    }

    Ok(())
}
