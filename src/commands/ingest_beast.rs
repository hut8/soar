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
    use soar::queue_config::{
        BEAST_RAW_STREAM, BEAST_RAW_STREAM_STAGING, BEAST_RAW_SUBJECT, BEAST_RAW_SUBJECT_STAGING,
    };

    sentry::configure_scope(|scope| {
        scope.set_tag("operation", "ingest-beast");
    });

    // Determine environment and use appropriate stream/subject names
    // Production: "BEAST_RAW" and "beast.raw"
    // Staging: "STAGING_BEAST_RAW" and "staging.beast.raw"
    let is_production = env::var("SOAR_ENV")
        .map(|env| env == "production")
        .unwrap_or(false);

    let (final_stream_name, final_subject) = if is_production {
        (BEAST_RAW_STREAM.to_string(), BEAST_RAW_SUBJECT.to_string())
    } else {
        (
            BEAST_RAW_STREAM_STAGING.to_string(),
            BEAST_RAW_SUBJECT_STAGING.to_string(),
        )
    };

    info!(
        "Starting Beast ingestion service - server: {}:{}, NATS: {}, stream: {}, subject: {}",
        server, port, nats_url, final_stream_name, final_subject
    );

    info!(
        "Environment: {}, using stream '{}' and subject '{}'",
        if is_production {
            "production"
        } else {
            "staging"
        },
        final_stream_name,
        final_subject
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

    // Start metrics server in production mode (AFTER metrics are initialized)
    if is_production {
        // Allow overriding metrics port via METRICS_PORT env var (for blue-green deployment)
        let metrics_port = env::var("METRICS_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(9094); // Use different port from APRS ingest (9093)

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

    // Retry loop for JetStream connection and Beast ingestion
    loop {
        // Check if shutdown was requested
        if shutdown_rx.try_recv().is_ok() {
            info!("Shutdown requested, exiting...");
            std::process::exit(0);
        }

        info!("Connecting to NATS at {}...", nats_url);
        let nats_result = async_nats::ConnectOptions::new()
            .name("soar-beast-ingester")
            .connect(&nats_url)
            .await;

        let nats_client = match nats_result {
            Ok(client) => {
                info!("Connected to NATS successfully");
                client
            }
            Err(e) => {
                error!("Failed to connect to NATS: {} - retrying in 1s", e);
                metrics::counter!("beast.jetstream.connection_failed").increment(1);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        let jetstream = async_nats::jetstream::new(nats_client);

        // Create or get the stream for raw Beast messages
        info!(
            "Setting up JetStream stream '{}' for subject '{}'...",
            final_stream_name, final_subject
        );

        let stream = match jetstream.get_stream(&final_stream_name).await {
            Ok(stream) => {
                info!("JetStream stream '{}' already exists", final_stream_name);
                stream
            }
            Err(_) => {
                info!("Creating new JetStream stream '{}'...", final_stream_name);
                match jetstream
                    .create_stream(async_nats::jetstream::stream::Config {
                        name: final_stream_name.clone(),
                        subjects: vec![final_subject.clone()],
                        max_messages: 100_000_000, // Store up to 100M messages (no limit)
                        storage: async_nats::jetstream::stream::StorageType::File,
                        num_replicas: 1,
                        ..Default::default()
                    })
                    .await
                {
                    Ok(stream) => stream,
                    Err(e) => {
                        error!("Failed to create JetStream stream: {} - retrying in 1s", e);
                        metrics::counter!("beast.jetstream.stream_setup_failed").increment(1);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue;
                    }
                }
            }
        };

        info!(
            "JetStream stream ready - will publish to subject '{}'",
            final_subject
        );

        // Create Beast JetStream publisher
        let jetstream_publisher = soar::beast_jetstream_publisher::JetStreamPublisher::new(
            jetstream,
            final_subject.clone(),
            stream,
        );

        let mut client = BeastClient::new(config.clone());

        // Mark JetStream as connected in health state
        {
            let mut health = health_state.write().await;
            health.jetstream_connected = true;
        }

        info!("Starting Beast client for ingestion...");

        // Run Beast client - this will block until failure or shutdown
        match client.start_jetstream(jetstream_publisher).await {
            Ok(_) => {
                info!("Beast ingestion stopped normally");
                break;
            }
            Err(e) => {
                error!("Beast ingestion failed: {} - retrying in 1s", e);
                metrics::counter!("beast.ingest_failed").increment(1);

                // Mark JetStream as disconnected
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
