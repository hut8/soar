use anyhow::{Context, Result};
use soar::aprs_client::{AprsClient, AprsClientConfigBuilder};
use soar::instance_lock::InstanceLock;
use std::env;
use tracing::Instrument;
use tracing::info;

pub async fn handle_ingest_aprs(
    server: String,
    mut port: u16,
    callsign: String,
    filter: Option<String>,
    max_retries: u32,
    retry_delay: u64,
    nats_url: String,
) -> Result<()> {
    use soar::queue_config::{
        APRS_RAW_STREAM, APRS_RAW_STREAM_STAGING, APRS_RAW_SUBJECT, APRS_RAW_SUBJECT_STAGING,
    };

    sentry::configure_scope(|scope| {
        scope.set_tag("operation", "ingest-aprs");
    });

    // Automatically switch to port 10152 for full feed if no filter specified
    // Port 14580 requires a filter, port 10152 provides the full global feed
    if filter.is_none() && port == 14580 {
        info!("No filter specified, switching from port 14580 to 10152 for full feed");
        port = 10152;
    }

    // Determine environment and use appropriate stream/subject names
    // Production: "APRS_RAW" and "aprs.raw"
    // Staging: "STAGING_APRS_RAW" and "staging.aprs.raw"
    let is_production = env::var("SOAR_ENV")
        .map(|env| env == "production")
        .unwrap_or(false);

    let (final_stream_name, final_subject) = if is_production {
        (APRS_RAW_STREAM.to_string(), APRS_RAW_SUBJECT.to_string())
    } else {
        (
            APRS_RAW_STREAM_STAGING.to_string(),
            APRS_RAW_SUBJECT_STAGING.to_string(),
        )
    };

    info!(
        "Starting APRS ingestion service - server: {}:{}, NATS: {}, stream: {}, subject: {}",
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
    let health_state = soar::metrics::init_aprs_health();
    soar::metrics::set_aprs_health(health_state.clone());

    // Start metrics server in production mode
    if is_production {
        // Allow overriding metrics port via METRICS_PORT env var (for blue-green deployment)
        let metrics_port = env::var("METRICS_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(9093);

        info!("Starting metrics server on port {}", metrics_port);
        tokio::spawn(
            async move {
                soar::metrics::start_metrics_server(metrics_port).await;
            }
            .instrument(tracing::info_span!("metrics_server")),
        );
    }

    // Initialize all APRS ingester metrics to zero so they appear in Grafana even before events occur
    info!("Initializing APRS ingester metrics...");

    // Connection metrics
    metrics::counter!("aprs.connection.established").absolute(0);
    metrics::counter!("aprs.connection.ended").absolute(0);
    metrics::counter!("aprs.connection.failed").absolute(0);
    metrics::counter!("aprs.connection.operation_failed").absolute(0);
    metrics::gauge!("aprs.connection.connected").set(0.0);

    // Keepalive metrics
    metrics::counter!("aprs.keepalive.sent").absolute(0);

    // Message processing metrics
    metrics::counter!("aprs.raw_message.processed").absolute(0);
    metrics::counter!("aprs.raw_message_queue.full").absolute(0);
    metrics::gauge!("aprs.raw_message_queue.depth").set(0.0);

    // Message type tracking (received from APRS-IS)
    metrics::counter!("aprs.raw_message.received.server").absolute(0);
    metrics::counter!("aprs.raw_message.received.aprs").absolute(0);
    metrics::counter!("aprs.raw_message.queued.server").absolute(0);
    metrics::counter!("aprs.raw_message.queued.aprs").absolute(0);

    // JetStream publishing metrics
    metrics::counter!("aprs.jetstream.published").absolute(0);
    metrics::counter!("aprs.jetstream.published.server").absolute(0);
    metrics::counter!("aprs.jetstream.published.aprs").absolute(0);
    metrics::counter!("aprs.jetstream.publish_error").absolute(0);
    metrics::gauge!("aprs.jetstream.queue_depth").set(0.0);

    info!("APRS ingester metrics initialized");

    // Acquire instance lock to prevent multiple ingest instances from running
    let lock_name = if is_production {
        "aprs-ingest-production"
    } else {
        "aprs-ingest-dev"
    };
    let _lock = InstanceLock::new(lock_name)
        .context("Failed to acquire instance lock - is another aprs-ingest instance running?")?;
    info!("Instance lock acquired for {}", lock_name);

    // Connect to NATS and set up JetStream
    info!("Connecting to NATS at {}...", nats_url);
    let nats_client = async_nats::ConnectOptions::new()
        .name("soar-aprs-ingester")
        .connect(&nats_url)
        .await
        .context("Failed to connect to NATS")?;
    info!("Connected to NATS successfully");

    let jetstream = async_nats::jetstream::new(nats_client);

    // Create or get the stream for raw APRS messages
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
            jetstream
                .create_stream(async_nats::jetstream::stream::Config {
                    name: final_stream_name.clone(),
                    subjects: vec![final_subject.clone()],
                    max_messages: 10_000_000, // Store up to 10M messages
                    max_bytes: 10 * 1024 * 1024 * 1024, // 10GB max
                    max_age: std::time::Duration::from_secs(24 * 60 * 60), // 24 hours retention
                    storage: async_nats::jetstream::stream::StorageType::File,
                    num_replicas: 1,
                    ..Default::default()
                })
                .await
                .context("Failed to create JetStream stream")?
        }
    };

    info!(
        "JetStream stream ready - will publish to subject '{}'",
        final_subject
    );

    // Create a simplified APRS client that publishes to JetStream
    let config = AprsClientConfigBuilder::new()
        .server(server)
        .port(port)
        .callsign(callsign)
        .filter(filter)
        .max_retries(max_retries)
        .retry_delay_seconds(retry_delay)
        .build();

    // Create a custom "router" that just publishes to JetStream
    let jetstream_publisher = soar::aprs_jetstream_publisher::JetStreamPublisher::new(
        jetstream,
        final_subject.clone(),
        stream,
    );

    let mut client = AprsClient::new(config);

    // Set up signal handling for graceful shutdown
    info!("Setting up graceful shutdown handlers...");
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

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
                    info!("Received SIGTERM, initiating graceful shutdown...");
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT (Ctrl+C), initiating graceful shutdown...");
                }
            }
        }

        #[cfg(not(unix))]
        {
            match tokio::signal::ctrl_c().await {
                Ok(()) => {
                    info!("Received SIGINT (Ctrl+C), initiating graceful shutdown...");
                }
                Err(err) => {
                    error!("Failed to listen for SIGINT signal: {}", err);
                    return;
                }
            }
        }

        let _ = shutdown_tx.send(());
    });

    info!("Starting APRS client for ingestion...");

    // Mark JetStream as connected in health state
    {
        let mut health = health_state.write().await;
        health.jetstream_connected = true;
    }

    client
        .start_jetstream_with_shutdown(jetstream_publisher, shutdown_rx, health_state)
        .await
        .context("APRS ingestion client failed")?;

    Ok(())
}
