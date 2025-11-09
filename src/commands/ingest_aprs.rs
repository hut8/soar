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

    // Initialize all APRS ingester metrics to zero so they appear in Grafana even before events occur
    // This MUST happen before starting the metrics server to avoid race conditions where
    // Prometheus scrapes before metrics are initialized
    info!("Initializing APRS ingester metrics...");
    soar::metrics::initialize_aprs_ingest_metrics();
    info!("APRS ingester metrics initialized");

    // Start metrics server in production mode (AFTER metrics are initialized)
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
                    max_messages: 100_000_000, // Store up to 100M messages
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

    // Set up signal handling for immediate shutdown
    info!("Setting up shutdown handlers...");

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

        // Exit immediately without graceful shutdown
        std::process::exit(0);
    });

    info!("Starting APRS client for ingestion...");

    // Mark JetStream as connected in health state
    {
        let mut health = health_state.write().await;
        health.jetstream_connected = true;
    }

    client
        .start_jetstream(jetstream_publisher)
        .await
        .context("APRS ingestion client failed")?;

    Ok(())
}
