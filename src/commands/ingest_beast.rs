use anyhow::{Context, Result};
use soar::beast::{BeastClient, BeastClientConfig};
use soar::instance_lock::InstanceLock;
use std::env;
use tracing::Instrument;
use tracing::{error, info};

pub async fn handle_ingest_beast(
    server: String,
    port: u16,
    max_retries: u32,
    retry_delay: u64,
    nats_url: String,
) -> Result<()> {
    sentry::configure_scope(|scope| {
        scope.set_tag("operation", "ingest-beast");
    });

    // Determine environment and use appropriate NATS subject
    // Production: "beast.raw"
    // Staging: "staging.beast.raw"
    let soar_env = env::var("SOAR_ENV").unwrap_or_default();
    let is_production = soar_env == "production";
    let is_staging = soar_env == "staging";

    let nats_subject = if is_production {
        "beast.raw"
    } else {
        "staging.beast.raw"
    };

    info!(
        "Starting Beast ingestion service - server: {}:{}, NATS: {}, subject: {}",
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

    // Initialize all Beast ingester metrics to zero so they appear in Grafana even before events occur
    // This MUST happen before starting the metrics server to avoid race conditions where
    // Prometheus scrapes before metrics are initialized
    info!("Initializing Beast ingester metrics...");
    soar::metrics::initialize_beast_ingest_metrics();
    info!("Beast ingester metrics initialized");

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
        "beast-ingest-production"
    } else {
        "beast-ingest-dev"
    };
    let _lock = InstanceLock::new(lock_name)
        .context("Failed to acquire instance lock - is another beast-ingest instance running?")?;
    info!("Instance lock acquired for {}", lock_name);

    // Set up signal handling for immediate shutdown
    info!("Setting up shutdown handlers...");
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

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

    // Retry loop for NATS connection and Beast ingestion
    loop {
        // Check if shutdown was requested
        if shutdown_rx.try_recv().is_ok() {
            info!("Shutdown requested, exiting...");
            std::process::exit(0);
        }

        info!("Connecting to NATS at {}...", nats_url);
        let nats_client_name = if std::env::var("SOAR_ENV") == Ok("production".into()) {
            "soar-beast-ingester"
        } else {
            "soar-beast-ingester-staging"
        };
        let nats_result = async_nats::ConnectOptions::new()
            .name(nats_client_name)
            .connect(&nats_url)
            .await;

        let nats_client = match nats_result {
            Ok(client) => {
                info!("Connected to NATS successfully");
                client
            }
            Err(e) => {
                error!("Failed to connect to NATS: {} - retrying in 1s", e);
                metrics::counter!("beast.nats.connection_failed").increment(1);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        info!(
            "NATS ready - will publish Beast messages to subject '{}'",
            nats_subject
        );

        // Create Beast NATS publisher
        let nats_publisher =
            soar::beast_nats_publisher::NatsPublisher::new(nats_client, nats_subject.to_string());

        let mut client = BeastClient::new(config.clone());

        // Mark NATS as connected in health state
        {
            let mut health = health_state.write().await;
            health.jetstream_connected = true; // Reusing jetstream_connected field for NATS connection status
        }

        info!("Starting Beast client for ingestion...");

        // Run Beast client - this will block until failure or shutdown
        match client.start_jetstream(nats_publisher).await {
            Ok(_) => {
                info!("Beast ingestion stopped normally");
                break;
            }
            Err(e) => {
                error!("Beast ingestion failed: {} - retrying in 1s", e);
                metrics::counter!("beast.ingest_failed").increment(1);

                // Mark NATS as disconnected
                {
                    let mut health = health_state.write().await;
                    health.jetstream_connected = false;
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }

    Ok(())
}
