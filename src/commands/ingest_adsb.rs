use anyhow::{Context, Result};
use soar::beast::{BeastClient, BeastClientConfig};
use soar::instance_lock::InstanceLock;
use std::env;
use tracing::Instrument;
use tracing::{error, info};

pub async fn handle_ingest_adsb(
    server: String,
    port: u16,
    max_retries: u32,
    retry_delay: u64,
    nats_url: String,
) -> Result<()> {
    sentry::configure_scope(|scope| {
        scope.set_tag("operation", "ingest-adsb");
    });

    // Determine environment and use appropriate NATS subject
    // Production: "adsb.raw"
    // Staging: "staging.adsb.raw"
    let soar_env = env::var("SOAR_ENV").unwrap_or_default();
    let is_production = soar_env == "production";
    let is_staging = soar_env == "staging";

    let nats_subject = if is_production {
        "adsb.raw"
    } else {
        "staging.adsb.raw"
    };

    info!(
        "Starting ADS-B ingestion service - server: {}:{}, NATS: {}, subject: {}",
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

    // Create Beast client config
    let config = BeastClientConfig {
        server,
        port,
        max_retries,
        retry_delay_seconds: retry_delay,
        max_retry_delay_seconds: 60,
    };

    // Retry loop for NATS connection
    loop {
        info!("Connecting to NATS at {}...", nats_url);
        let nats_client_name = soar::nats_client_name("adsb-ingester");
        let nats_result = async_nats::ConnectOptions::new()
            .name(&nats_client_name)
            .connect(&nats_url)
            .await;

        let nats_client = match nats_result {
            Ok(client) => {
                info!("Connected to NATS successfully");
                client
            }
            Err(e) => {
                error!("Failed to connect to NATS: {} - retrying in 1s", e);
                metrics::counter!("beast.nats.connection_failed_total").increment(1);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        info!(
            "NATS ready - will publish ADS-B messages to subject '{}'",
            nats_subject
        );

        // Create ADS-B (Beast) NATS publisher
        let nats_publisher =
            soar::beast_nats_publisher::NatsPublisher::new(nats_client, nats_subject.to_string());

        let mut client = BeastClient::new(config.clone());

        // Mark NATS as connected in health state
        {
            let mut health = health_state.write().await;
            health.nats_connected = true;
        }

        info!("Starting ADS-B (Beast) client for ingestion...");

        // Run Beast client with shutdown signal - this will exit immediately on SIGINT/SIGTERM
        match client
            .start_with_shutdown(nats_publisher, shutdown_rx, health_state.clone())
            .await
        {
            Ok(_) => {
                info!("ADS-B ingestion stopped normally");
                break;
            }
            Err(e) => {
                error!("ADS-B ingestion failed: {} - retrying in 1s", e);
                metrics::counter!("beast.ingest_failed_total").increment(1);

                // Mark NATS as disconnected
                {
                    let mut health = health_state.write().await;
                    health.nats_connected = false;
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }

        // If we get here, either shutdown was requested or connection failed
        // In either case, exit the loop
        break;
    }

    Ok(())
}
