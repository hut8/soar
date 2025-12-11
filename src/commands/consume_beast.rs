#![allow(dead_code)] // Will be used in future commits
use anyhow::{Context, Result};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use soar::beast_consumer_task::BeastConsumerTask;
use soar::beast_jetstream_consumer::JetStreamConsumer;
use soar::instance_lock::InstanceLock;
use soar::raw_messages_repo::BeastMessagesRepository;
use std::env;
use tracing::Instrument;
use tracing::{error, info};
use uuid::Uuid;

pub async fn handle_consume_beast(
    database_url: String,
    nats_url: String,
    receiver_id: Uuid,
) -> Result<()> {
    use soar::queue_config::{
        BEAST_RAW_STREAM, BEAST_RAW_STREAM_STAGING, BEAST_RAW_SUBJECT, BEAST_RAW_SUBJECT_STAGING,
    };

    sentry::configure_scope(|scope| {
        scope.set_tag("operation", "consume-beast");
    });

    // Determine environment and use appropriate stream/subject names
    let soar_env = env::var("SOAR_ENV").unwrap_or_default();
    let is_production = soar_env == "production";
    let is_staging = soar_env == "staging";

    let (final_stream_name, final_subject, final_consumer_name) = if is_production {
        (
            BEAST_RAW_STREAM.to_string(),
            BEAST_RAW_SUBJECT.to_string(),
            "beast-consumer-production".to_string(),
        )
    } else {
        (
            BEAST_RAW_STREAM_STAGING.to_string(),
            BEAST_RAW_SUBJECT_STAGING.to_string(),
            "beast-consumer-dev".to_string(),
        )
    };

    info!(
        "Starting Beast consumer service - receiver: {}, NATS: {}, stream: {}, subject: {}",
        receiver_id, nats_url, final_stream_name, final_subject
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

    // Initialize health state for this consumer
    let health_state = soar::metrics::init_beast_health();
    soar::metrics::set_beast_health(health_state.clone());

    // Initialize all Beast consumer metrics to zero
    info!("Initializing Beast consumer metrics...");
    soar::metrics::initialize_beast_consumer_metrics();
    info!("Beast consumer metrics initialized");

    // Start metrics server in production/staging mode
    if is_production || is_staging {
        let metrics_port = env::var("METRICS_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(9095); // Different port from ingest-beast (9094)

        info!("Starting metrics server on port {}", metrics_port);
        tokio::spawn(
            async move {
                soar::metrics::start_metrics_server(metrics_port).await;
            }
            .instrument(tracing::info_span!("metrics_server")),
        );
    }

    // Acquire instance lock to prevent multiple consumer instances from running
    let lock_name = if is_production {
        "beast-consumer-production"
    } else {
        "beast-consumer-dev"
    };
    let _lock = InstanceLock::new(lock_name)
        .context("Failed to acquire instance lock - is another beast-consumer instance running?")?;
    info!("Instance lock acquired for {}", lock_name);

    // Set up signal handling for immediate shutdown
    info!("Setting up shutdown handlers...");
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Spawn signal handler task
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

    // Set up database connection pool
    info!("Connecting to database at {}...", database_url);
    let manager = ConnectionManager::<PgConnection>::new(&database_url);
    let pool = Pool::builder()
        .max_size(20) // Moderate pool size for batch writes
        .build(manager)
        .context("Failed to create database connection pool")?;
    info!("Database connection pool created");

    // Create repository
    let repository = BeastMessagesRepository::new(pool);

    // Retry loop for JetStream connection and Beast consumption
    loop {
        // Check if shutdown was requested
        if shutdown_rx.try_recv().is_ok() {
            info!("Shutdown requested, exiting...");
            std::process::exit(0);
        }

        info!("Connecting to NATS at {}...", nats_url);
        let nats_result = async_nats::ConnectOptions::new()
            .name("soar-beast-consumer")
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

        // Create JetStream consumer
        info!(
            "Creating JetStream consumer '{}' for stream '{}', subject '{}'...",
            final_consumer_name, final_stream_name, final_subject
        );

        let consumer = match JetStreamConsumer::new(
            jetstream,
            final_stream_name.clone(),
            final_subject.clone(),
            final_consumer_name.clone(),
        )
        .await
        {
            Ok(consumer) => {
                info!("JetStream consumer ready, starting message processing...");
                consumer
            }
            Err(e) => {
                error!(
                    "Failed to create JetStream consumer: {} - retrying in 1s",
                    e
                );
                metrics::counter!("beast.jetstream.consumer_setup_failed").increment(1);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        // Mark JetStream as connected in health state
        {
            let mut health = health_state.write().await;
            health.jetstream_connected = true;
        }

        info!("Starting Beast consumer task...");

        // Create and run the consumer task
        let consumer_task = BeastConsumerTask::new(consumer, repository.clone(), receiver_id);

        match consumer_task.run().await {
            Ok(_) => {
                info!("Beast consumer stopped normally");
                break;
            }
            Err(e) => {
                error!("Beast consumer failed: {} - retrying in 1s", e);
                metrics::counter!("beast.consume_failed").increment(1);

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
