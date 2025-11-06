use anyhow::{Context, Result};
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use soar::fix_processor::FixProcessor;
use soar::fixes_repo::FixesRepository;
use soar::flight_tracker::FlightTracker;
use soar::instance_lock::InstanceLock;
use soar::packet_processors::{
    AircraftPositionProcessor, GenericProcessor, PacketRouter, ReceiverPositionProcessor,
    ReceiverStatusProcessor, ServerStatusProcessor,
};
use soar::queue_config::{
    AGL_DATABASE_QUEUE_SIZE, AIRCRAFT_QUEUE_SIZE, APRS_RAW_STREAM, APRS_RAW_STREAM_STAGING,
    APRS_RAW_SUBJECT, APRS_RAW_SUBJECT_STAGING, ELEVATION_QUEUE_SIZE, JETSTREAM_INTAKE_QUEUE_SIZE,
    RECEIVER_POSITION_QUEUE_SIZE, RECEIVER_STATUS_QUEUE_SIZE, SERVER_STATUS_QUEUE_SIZE,
    SOAR_RUN_CONSUMER, SOAR_RUN_CONSUMER_STAGING, queue_warning_threshold,
};
use soar::receiver_repo::ReceiverRepository;
use soar::receiver_status_repo::ReceiverStatusRepository;
use soar::server_messages_repo::ServerMessagesRepository;
use std::env;
use tracing::Instrument;
use tracing::{info, trace, warn};

/// Process a received APRS message by parsing and routing through PacketRouter
/// The message format is: "YYYY-MM-DDTHH:MM:SS.SSSZ <original_message>"
/// We extract the timestamp and pass it through the processing pipeline
/// NOTE: With AckPolicy::None, messages are implicitly ACKed when delivered from JetStream
async fn process_aprs_message(
    message: &str,
    packet_router: &soar::packet_processors::PacketRouter,
) {
    let start_time = std::time::Instant::now();

    // Extract timestamp from the beginning of the message
    // Format: "YYYY-MM-DDTHH:MM:SS.SSSZ <rest_of_message>"
    let (received_at, actual_message) = match message.split_once(' ') {
        Some((timestamp_str, rest)) => match chrono::DateTime::parse_from_rfc3339(timestamp_str) {
            Ok(timestamp) => (timestamp.with_timezone(&chrono::Utc), rest),
            Err(e) => {
                warn!(
                    "Failed to parse timestamp from message: {} - using current time",
                    e
                );
                (chrono::Utc::now(), message)
            }
        },
        None => {
            warn!("Message does not contain timestamp prefix - using current time");
            (chrono::Utc::now(), message)
        }
    };

    // Calculate and record lag (difference between now and packet timestamp)
    let now = chrono::Utc::now();
    let lag_seconds = (now - received_at).num_milliseconds() as f64 / 1000.0;
    metrics::gauge!("aprs.jetstream.lag_seconds").set(lag_seconds);

    // Route server messages (starting with #) differently
    // Server messages don't create PacketContext
    if actual_message.starts_with('#') {
        info!("Server message: {}", actual_message);
        packet_router
            .process_server_message(actual_message, received_at)
            .await;
        return;
    }

    // Try to parse the message using ogn-parser
    match ogn_parser::parse(actual_message) {
        Ok(parsed) => {
            // Call PacketRouter to archive, process, and route to queues
            packet_router
                .process_packet(parsed, actual_message, received_at)
                .await;
        }
        Err(e) => {
            // For OGNFNT sources with invalid lat/lon, log as trace instead of error
            // These are common and expected issues with this data source
            let error_str = e.to_string();
            if actual_message.contains("OGNFNT")
                && (error_str.contains("Invalid Latitude")
                    || error_str.contains("Invalid Longitude"))
            {
                trace!("Failed to parse APRS message '{actual_message}': {e}");
            } else {
                info!("Failed to parse APRS message '{actual_message}': {e}");
            }
        }
    }

    // Record processing latency
    let elapsed_micros = start_time.elapsed().as_micros() as f64 / 1000.0; // Convert to milliseconds
    metrics::histogram!("aprs.message_processing_latency_ms").record(elapsed_micros);
}

#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip_all)]
pub async fn handle_run(
    archive_dir: Option<String>,
    nats_url: String,
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> Result<()> {
    sentry::configure_scope(|scope| {
        scope.set_tag("operation", "run");
    });

    // Determine environment and use appropriate stream/subject/consumer names
    let is_production = env::var("SOAR_ENV")
        .map(|env| env == "production")
        .unwrap_or(false);

    let (final_stream_name, final_subject, final_consumer_name) = if is_production {
        (
            APRS_RAW_STREAM.to_string(),
            APRS_RAW_SUBJECT.to_string(),
            SOAR_RUN_CONSUMER.to_string(),
        )
    } else {
        (
            APRS_RAW_STREAM_STAGING.to_string(),
            APRS_RAW_SUBJECT_STAGING.to_string(),
            SOAR_RUN_CONSUMER_STAGING.to_string(),
        )
    };

    info!(
        "Starting APRS processing service consuming from JetStream stream: {}, subject: {}, consumer: {}",
        final_stream_name, final_subject, final_consumer_name
    );

    // Initialize all soar-run metrics to zero so they appear in Grafana even before events occur
    // This MUST happen before starting the metrics server to avoid race conditions where
    // Prometheus scrapes before metrics are initialized
    info!("Initializing soar-run metrics...");
    soar::metrics::initialize_run_metrics();
    info!("soar-run metrics initialized");

    // Start metrics server in the background (AFTER metrics are initialized)
    if is_production {
        info!("Starting metrics server on port 9091");
        tokio::spawn(
            async {
                soar::metrics::start_metrics_server(9091).await;
            }
            .instrument(tracing::info_span!("metrics_server")),
        );
    }

    let lock_name = if is_production {
        "soar-run-production"
    } else {
        "soar-run-dev"
    };
    let _instance_lock = InstanceLock::new(lock_name)?;
    info!(
        "Acquired instance lock: {}",
        _instance_lock.path().display()
    );

    // Log elevation data storage path
    let elevation_path =
        env::var("ELEVATION_DATA_PATH").unwrap_or_else(|_| "/var/soar/elevation".to_string());
    info!("Elevation data path: {}", elevation_path);

    info!(
        "Environment: {}, using stream '{}', subject '{}', consumer '{}'",
        if is_production {
            "production"
        } else {
            "staging"
        },
        final_stream_name,
        final_subject,
        final_consumer_name
    );

    // Create FlightTracker
    let flight_tracker = FlightTracker::new(&diesel_pool);

    // Initialize flight tracker from database:
    // 1. Timeout old incomplete flights (older than 1 hour)
    // 2. Load recent active flights into memory
    let timeout_duration = chrono::Duration::hours(1);
    match flight_tracker
        .initialize_from_database(timeout_duration)
        .await
    {
        Ok((timed_out, loaded)) => {
            info!(
                "Flight tracker initialized: {} flights timed out, {} flights loaded",
                timed_out, loaded
            );
        }
        Err(e) => {
            warn!("Failed to initialize flight tracker from database: {}", e);
        }
    }

    // Start flight timeout checker (every 60 seconds)
    flight_tracker.start_timeout_checker(60);

    // Create separate bounded channel for elevation/AGL calculations
    // This prevents elevation lookups (which can be slow) from blocking the main processing queue
    let (elevation_tx, elevation_rx) =
        flume::bounded::<soar::elevation::ElevationTask>(ELEVATION_QUEUE_SIZE);

    info!("Created bounded elevation processing queue with capacity 1,000");

    // Create separate bounded channel for AGL database updates
    // This separates the fast elevation calculation from the slower database updates
    // and allows batching of database writes for much better throughput
    let (agl_db_tx, agl_db_rx) =
        flume::bounded::<soar::elevation::AglDatabaseTask>(AGL_DATABASE_QUEUE_SIZE);

    info!("Created bounded AGL database update queue with capacity 100");

    // Create database fix processor to save all valid fixes to the database
    // Try to create with NATS first, fall back to without NATS if connection fails
    let fix_processor = match FixProcessor::with_flight_tracker_and_nats(
        diesel_pool.clone(),
        flight_tracker.clone(),
        &nats_url,
    )
    .await
    {
        Ok(processor_with_nats) => {
            info!("Created FixProcessor with NATS publisher");
            processor_with_nats.with_elevation_channel(elevation_tx.clone())
        }
        Err(e) => {
            warn!(
                "Failed to create FixProcessor with NATS ({}), falling back to processor without NATS",
                e
            );
            FixProcessor::with_flight_tracker(diesel_pool.clone(), flight_tracker.clone())
                .with_elevation_channel(elevation_tx.clone())
        }
    };

    // Create server status processor for server messages
    let server_messages_repo = ServerMessagesRepository::new(diesel_pool.clone());
    let server_status_processor = ServerStatusProcessor::new(server_messages_repo);

    // Create repositories
    let receiver_repo = ReceiverRepository::new(diesel_pool.clone());
    let receiver_status_repo = ReceiverStatusRepository::new(diesel_pool.clone());
    let aprs_messages_repo =
        soar::aprs_messages_repo::AprsMessagesRepository::new(diesel_pool.clone());

    // Create GenericProcessor for archiving, receiver identification, and APRS message insertion
    let generic_processor = if let Some(archive_path) = archive_dir.clone() {
        let archive_service = soar::aprs_client::ArchiveService::new(archive_path).await?;
        GenericProcessor::new(receiver_repo.clone(), aprs_messages_repo)
            .with_archive_service(archive_service)
    } else {
        GenericProcessor::new(receiver_repo.clone(), aprs_messages_repo)
    };

    // Create receiver status processor for receiver status messages
    let receiver_status_processor =
        ReceiverStatusProcessor::new(receiver_status_repo, receiver_repo.clone());

    // Create receiver position processor for receiver position messages
    let receiver_position_processor = ReceiverPositionProcessor::new(receiver_repo.clone());

    // Create aircraft position processor
    // Note: FlightDetectionProcessor is now handled inside FixProcessor
    let aircraft_position_processor =
        AircraftPositionProcessor::new().with_fix_processor(fix_processor.clone());

    info!(
        "Setting up APRS client with PacketRouter - archive directory: {:?}, NATS URL: {}",
        archive_dir, nats_url
    );

    // Create bounded channels for per-processor queues
    // Aircraft positions: highest capacity due to high volume and heavy processing
    let (aircraft_tx, aircraft_rx) = flume::bounded(AIRCRAFT_QUEUE_SIZE);
    info!(
        "Created aircraft position queue with capacity {}",
        AIRCRAFT_QUEUE_SIZE
    );

    // Receiver status: high capacity
    let (receiver_status_tx, receiver_status_rx) = flume::bounded(RECEIVER_STATUS_QUEUE_SIZE);
    info!(
        "Created receiver status queue with capacity {}",
        RECEIVER_STATUS_QUEUE_SIZE
    );

    // Receiver position: medium capacity
    let (receiver_position_tx, receiver_position_rx) = flume::bounded(RECEIVER_POSITION_QUEUE_SIZE);
    info!(
        "Created receiver position queue with capacity {}",
        RECEIVER_POSITION_QUEUE_SIZE
    );

    // Server status: low capacity (rare messages)
    // Channel now includes timestamp: (message, received_at)
    let (server_status_tx, server_status_rx) =
        flume::bounded::<(String, chrono::DateTime<chrono::Utc>)>(SERVER_STATUS_QUEUE_SIZE);
    info!(
        "Created server status queue with capacity {}",
        SERVER_STATUS_QUEUE_SIZE
    );

    // JetStream intake queue: buffers raw APRS messages from JetStream consumer
    // This allows graceful shutdown by stopping JetStream reads and draining this queue
    let (jetstream_intake_tx, jetstream_intake_rx) =
        flume::bounded::<String>(JETSTREAM_INTAKE_QUEUE_SIZE);
    info!(
        "Created JetStream intake queue with capacity {}",
        JETSTREAM_INTAKE_QUEUE_SIZE
    );

    // Create PacketRouter with per-processor queues and internal worker pool
    const PACKET_ROUTER_WORKERS: usize = 10;
    let packet_router = PacketRouter::new(generic_processor, PACKET_ROUTER_WORKERS)
        .with_aircraft_position_queue(aircraft_tx)
        .with_receiver_status_queue(receiver_status_tx)
        .with_receiver_position_queue(receiver_position_tx)
        .with_server_status_queue(server_status_tx);

    info!(
        "Created PacketRouter with {} workers and per-processor queues",
        PACKET_ROUTER_WORKERS
    );

    // Spawn intake queue processor
    // This task reads raw APRS messages from the intake queue and processes them
    // Separating JetStream consumption from processing allows graceful shutdown
    let intake_router = packet_router.clone();
    tokio::spawn(
        async move {
            info!("Intake queue processor started");
            let mut messages_processed = 0u64;
            while let Ok(message) = jetstream_intake_rx.recv_async().await {
                process_aprs_message(&message, &intake_router).await;
                messages_processed += 1;
                metrics::counter!("aprs.intake.processed").increment(1);

                // Update intake queue depth metric
                metrics::gauge!("aprs.jetstream.intake_queue_depth")
                    .set(jetstream_intake_rx.len() as f64);
            }
            info!(
                "Intake queue processor stopped after processing {} messages",
                messages_processed
            );
        }
        .instrument(tracing::info_span!("intake_processor")),
    );
    info!("Spawned intake queue processor");

    // Spawn AGL batch database writer
    // This worker receives calculated AGL values and writes them to database in batches
    // Batching dramatically reduces database load (100+ individual UPDATEs become 1 batch UPDATE)
    let batch_writer_fixes_repo = FixesRepository::new(diesel_pool.clone());
    tokio::spawn(
        async move {
            soar::agl_batch_writer::batch_writer_task(agl_db_rx, batch_writer_fixes_repo).await;
        }
        .instrument(tracing::info_span!("agl_batch_writer")),
    );
    info!("Spawned AGL batch database writer (batch size: 100, timeout: 5s)");

    // Spawn multiple dedicated elevation processing workers
    // These workers handle AGL calculations separately to prevent them from blocking main processing
    // Using multiple workers allows parallel elevation lookups, which can be I/O intensive
    // The ElevationDB uses Arc<Mutex<LruCache>> internally, so all workers share the same cache
    let num_elevation_workers = 8;
    let elevation_db = flight_tracker.elevation_db().clone();

    for elevation_worker_id in 0..num_elevation_workers {
        let worker_elevation_rx = elevation_rx.clone();
        let worker_elevation_db = elevation_db.clone();
        let worker_agl_db_tx = agl_db_tx.clone();

        tokio::spawn(
            async move {
                while let Ok(task) = worker_elevation_rx.recv_async().await {
                            let start = std::time::Instant::now();

                            // Calculate AGL (no database update here, just calculation)
                            let agl = soar::flight_tracker::altitude::calculate_altitude_agl(
                                &worker_elevation_db,
                                &task.fix,
                            )
                            .await;

                            let duration = start.elapsed();
                            metrics::histogram!("aprs.elevation.duration_ms")
                                .record(duration.as_millis() as f64);
                            metrics::counter!("aprs.elevation.processed", "worker_id" => elevation_worker_id.to_string()).increment(1);

                            // Send calculated AGL to database batch writer (blocking)
                            let agl_task = soar::elevation::AglDatabaseTask {
                                fix_id: task.fix_id,
                                altitude_agl_feet: agl,
                            };

                            // Block until space is available - never drop AGL updates
                            if let Err(e) = worker_agl_db_tx.send_async(agl_task).await {
                                warn!("AGL database queue is closed: {}", e);
                            }
                }
            }
            .instrument(tracing::info_span!("elevation_worker", worker_id = elevation_worker_id)),
        );
    }

    info!(
        "Spawned {} dedicated elevation processing workers (sharing elevation and dataset caches)",
        num_elevation_workers
    );

    // Spawn dedicated worker pools for each processor type
    // Aircraft position workers (80 workers - heaviest processing due to FixProcessor + flight tracking)
    // Increased from 20 to 80 because aircraft queue was constantly full at 1,000 capacity
    // Most APRS messages (~80-90%) are aircraft positions, so this queue needs the most workers
    let num_aircraft_workers = 80;
    info!(
        "Spawning {} aircraft position workers",
        num_aircraft_workers
    );
    for worker_id in 0..num_aircraft_workers {
        let worker_rx = aircraft_rx.clone();
        let processor = aircraft_position_processor.clone();
        tokio::spawn(
            async move {
                while let Ok((packet, context)) = worker_rx.recv_async().await {
                    let start = std::time::Instant::now();
                    processor.process_aircraft_position(&packet, context).await;
                    let duration = start.elapsed();
                    metrics::histogram!("aprs.aircraft.duration_ms")
                        .record(duration.as_millis() as f64);
                    metrics::counter!("aprs.aircraft.processed").increment(1);
                    metrics::counter!("aprs.messages.processed.aircraft").increment(1);
                    metrics::counter!("aprs.messages.processed.total").increment(1);
                }
            }
            .instrument(tracing::info_span!("aircraft_worker", worker_id)),
        );
    }

    // Receiver status workers (6 workers - medium processing)
    let num_receiver_status_workers = 6;
    info!(
        "Spawning {} receiver status workers",
        num_receiver_status_workers
    );
    for worker_id in 0..num_receiver_status_workers {
        let worker_rx = receiver_status_rx.clone();
        let processor = receiver_status_processor.clone();
        tokio::spawn(
            async move {
                while let Ok((packet, context)) = worker_rx.recv_async().await {
                    let start = std::time::Instant::now();
                    processor.process_status_packet(&packet, context).await;
                    let duration = start.elapsed();
                    metrics::histogram!("aprs.receiver_status.duration_ms")
                        .record(duration.as_millis() as f64);
                    metrics::counter!("aprs.receiver_status.processed").increment(1);
                    metrics::counter!("aprs.messages.processed.receiver_status").increment(1);
                    metrics::counter!("aprs.messages.processed.total").increment(1);
                }
            }
            .instrument(tracing::info_span!("receiver_status_worker", worker_id)),
        );
    }

    // Receiver position workers (4 workers - light processing)
    let num_receiver_position_workers = 4;
    info!(
        "Spawning {} receiver position workers",
        num_receiver_position_workers
    );
    for worker_id in 0..num_receiver_position_workers {
        let worker_rx = receiver_position_rx.clone();
        let processor = receiver_position_processor.clone();
        tokio::spawn(
            async move {
                while let Ok((packet, context)) = worker_rx.recv_async().await {
                    let start = std::time::Instant::now();
                    processor.process_receiver_position(&packet, context).await;
                    let duration = start.elapsed();
                    metrics::histogram!("aprs.receiver_position.duration_ms")
                        .record(duration.as_millis() as f64);
                    metrics::counter!("aprs.receiver_position.processed").increment(1);
                    metrics::counter!("aprs.messages.processed.receiver_position").increment(1);
                    metrics::counter!("aprs.messages.processed.total").increment(1);
                }
            }
            .instrument(tracing::info_span!("receiver_position_worker", worker_id)),
        );
    }

    // Server status workers (2 workers - very light processing)
    info!("Spawning 2 server status workers");
    for worker_id in 0..2 {
        let worker_rx = server_status_rx.clone();
        let processor = server_status_processor.clone();
        tokio::spawn(
            async move {
                while let Ok((message, received_at)) = worker_rx.recv_async().await {
                    let start = std::time::Instant::now();
                    processor
                        .process_server_message(&message, received_at)
                        .await;
                    let duration = start.elapsed();
                    metrics::histogram!("aprs.server_status.duration_ms")
                        .record(duration.as_millis() as f64);
                    metrics::counter!("aprs.server_status.processed").increment(1);
                    metrics::counter!("aprs.messages.processed.server").increment(1);
                    metrics::counter!("aprs.messages.processed.total").increment(1);
                }
            }
            .instrument(tracing::info_span!("server_status_worker", worker_id)),
        );
    }

    // Spawn queue depth and system metrics reporter
    // Reports the depth of all processing queues and DB pool state to Prometheus every 10 seconds
    let metrics_aircraft_rx = aircraft_rx.clone();
    let metrics_receiver_status_rx = receiver_status_rx.clone();
    let metrics_receiver_position_rx = receiver_position_rx.clone();
    let metrics_server_status_rx = server_status_rx.clone();
    let metrics_elevation_rx = elevation_rx.clone();
    let metrics_db_pool = diesel_pool.clone();
    tokio::spawn(
        async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            interval.tick().await; // First tick completes immediately

            loop {
                interval.tick().await;

                // Sample queue depths (lock-free with flume!)
                let aircraft_depth = metrics_aircraft_rx.len();
                let receiver_status_depth = metrics_receiver_status_rx.len();
                let receiver_position_depth = metrics_receiver_position_rx.len();
                let server_status_depth = metrics_server_status_rx.len();
                let elevation_depth = metrics_elevation_rx.len();

                // Get database pool state
                let pool_state = metrics_db_pool.state();
                let active_connections = pool_state.connections - pool_state.idle_connections;

                // Report queue depths to Prometheus
                metrics::gauge!("aprs.aircraft_queue.depth").set(aircraft_depth as f64);
                metrics::gauge!("aprs.receiver_status_queue.depth")
                    .set(receiver_status_depth as f64);
                metrics::gauge!("aprs.receiver_position_queue.depth")
                    .set(receiver_position_depth as f64);
                metrics::gauge!("aprs.server_status_queue.depth").set(server_status_depth as f64);
                metrics::gauge!("aprs.elevation_queue.depth").set(elevation_depth as f64);

                // Report database pool state to Prometheus
                metrics::gauge!("aprs.db_pool.total_connections")
                    .set(pool_state.connections as f64);
                metrics::gauge!("aprs.db_pool.active_connections").set(active_connections as f64);
                metrics::gauge!("aprs.db_pool.idle_connections")
                    .set(pool_state.idle_connections as f64);

                // Warn if queues are building up (50% full)
                if aircraft_depth > queue_warning_threshold(AIRCRAFT_QUEUE_SIZE) {
                    warn!(
                        "Aircraft position queue building up: {} messages (50% full)",
                        aircraft_depth
                    );
                }
                if receiver_status_depth > queue_warning_threshold(RECEIVER_STATUS_QUEUE_SIZE) {
                    warn!(
                        "Receiver status queue building up: {} messages (50% full)",
                        receiver_status_depth
                    );
                }
                if receiver_position_depth > queue_warning_threshold(RECEIVER_POSITION_QUEUE_SIZE) {
                    warn!(
                        "Receiver position queue building up: {} messages (50% full)",
                        receiver_position_depth
                    );
                }
                if elevation_depth > queue_warning_threshold(ELEVATION_QUEUE_SIZE) {
                    warn!(
                        "Elevation queue building up: {} tasks (50% full)",
                        elevation_depth
                    );
                }
            }
        }
        .instrument(tracing::info_span!("queue_metrics_reporter")),
    );
    info!("Spawned queue depth metrics reporter (reports every 10 seconds to Prometheus)");

    // Set up graceful shutdown handler
    let shutdown_aircraft_rx = aircraft_rx.clone();
    let shutdown_receiver_status_rx = receiver_status_rx.clone();
    let shutdown_receiver_position_rx = receiver_position_rx.clone();
    let shutdown_server_status_rx = server_status_rx.clone();
    let shutdown_elevation_rx = elevation_rx.clone();
    let shutdown_intake_rx = jetstream_intake_tx.clone(); // Use tx to check queue depth

    tokio::spawn(
        async move {
            match tokio::signal::ctrl_c().await {
                Ok(()) => {
                    info!("Received shutdown signal (Ctrl+C), initiating graceful shutdown...");
                    info!("JetStream consumer will stop reading, allowing queues to drain...");

                    // Wait for queues to drain (check every second, max 10 minutes)
                    for i in 1..=600 {
                        let intake_depth = shutdown_intake_rx.len();
                        let aircraft_depth = shutdown_aircraft_rx.len();
                        let receiver_status_depth = shutdown_receiver_status_rx.len();
                        let receiver_position_depth = shutdown_receiver_position_rx.len();
                        let server_status_depth = shutdown_server_status_rx.len();
                        let elevation_depth = shutdown_elevation_rx.len();

                        let total_queued = intake_depth + aircraft_depth + receiver_status_depth + receiver_position_depth + server_status_depth + elevation_depth;

                        if total_queued == 0 {
                            info!("All queues drained, shutting down now");
                            break;
                        }

                        info!(
                            "Waiting for queues to drain ({}/600s): {} intake, {} aircraft, {} rx_status, {} rx_pos, {} server, {} elevation",
                            i, intake_depth, aircraft_depth, receiver_status_depth, receiver_position_depth, server_status_depth, elevation_depth
                        );

                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }

                    info!("Graceful shutdown complete");
                    std::process::exit(0);
                }
                Err(err) => {
                    eprintln!("Unable to listen for shutdown signal: {}", err);
                }
            }
        }
        .instrument(tracing::info_span!("shutdown_handler")),
    );
    info!("Graceful shutdown handler configured");

    // Set up JetStream consumer to read from durable queue
    info!("Connecting to NATS at {}...", nats_url);
    let nats_client = async_nats::ConnectOptions::new()
        .name("soar-run")
        .connect(&nats_url)
        .await
        .context("Failed to connect to NATS")?;
    info!("Connected to NATS successfully");

    let jetstream = async_nats::jetstream::new(nats_client);

    // Create JetStream consumer
    info!(
        "Creating JetStream consumer '{}' for stream '{}', subject '{}'...",
        final_consumer_name, final_stream_name, final_subject
    );
    let consumer = soar::aprs_jetstream_consumer::JetStreamConsumer::new(
        jetstream,
        final_stream_name.clone(),
        final_subject.clone(),
        final_consumer_name.clone(),
    )
    .await
    .context("Failed to create JetStream consumer")?;

    info!("JetStream consumer ready, starting message processing...");

    info!("APRS client started. Press Ctrl+C to stop.");

    // Start consuming messages from JetStream and sending them to the intake queue
    // Messages are implicitly ACKed when delivered (AckPolicy::None)
    // The intake queue processor will handle parsing and routing
    // Blocking sends to the intake queue provide natural backpressure to JetStream
    consumer
        .consume(move |message, _jetstream_msg| {
            let intake_tx = jetstream_intake_tx.clone();
            async move {
                // Send message to intake queue (blocking send for backpressure)
                // When intake queue is full, this will block and stop reading from JetStream
                match intake_tx.send_async(message).await {
                    Ok(_) => {
                        metrics::counter!("aprs.jetstream.received").increment(1);
                        Ok(())
                    }
                    Err(e) => {
                        warn!("Failed to send message to intake queue: {}", e);
                        metrics::counter!("aprs.jetstream.send_errors").increment(1);
                        Ok(()) // Still ack the message to JetStream
                    }
                }
            }
        })
        .await?;

    // If we reach here, the consumer stream ended unexpectedly
    warn!("JetStream consumer stopped unexpectedly");
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_timestamp_parsing_valid() {
        // Test that a valid ISO-8601 timestamp is correctly parsed
        let message = "2025-01-15T12:34:56.789Z FLRDDA5BA>APRS,qAS,LFNX:/160829h4902.45N/00531.30E'342/049/A=001322";

        // We can't directly test process_aprs_message since it's async and requires PacketRouter,
        // but we can test the parsing logic
        let (timestamp_str, rest) = message.split_once(' ').unwrap();
        let parsed = chrono::DateTime::parse_from_rfc3339(timestamp_str);

        assert!(parsed.is_ok());
        let timestamp = parsed.unwrap().with_timezone(&chrono::Utc);
        assert_eq!(timestamp.year(), 2025);
        assert_eq!(timestamp.month(), 1);
        assert_eq!(timestamp.day(), 15);
        assert_eq!(timestamp.hour(), 12);
        assert_eq!(timestamp.minute(), 34);
        assert_eq!(timestamp.second(), 56);
        assert_eq!(
            rest,
            "FLRDDA5BA>APRS,qAS,LFNX:/160829h4902.45N/00531.30E'342/049/A=001322"
        );
    }

    #[test]
    fn test_timestamp_parsing_invalid() {
        // Test that an invalid timestamp doesn't crash
        let message = "INVALID-TIMESTAMP FLRDDA5BA>APRS,qAS,LFNX:/160829h4902.45N/00531.30E";

        let (timestamp_str, _rest) = message.split_once(' ').unwrap();
        let parsed = chrono::DateTime::parse_from_rfc3339(timestamp_str);

        assert!(parsed.is_err());
    }

    #[test]
    fn test_timestamp_parsing_missing() {
        // Test that a message without a space (no timestamp) is handled
        let message = "FLRDDA5BA>APRS,qAS,LFNX:/160829h4902.45N/00531.30E'342/049/A=001322";

        let result = message.split_once(' ');
        assert!(result.is_none());
    }

    #[test]
    fn test_timestamp_parsing_server_message() {
        // Test that server messages with timestamps are handled correctly
        let message = "2025-01-15T12:34:56.789Z # aprsc 2.1.15-gc67551b 22 Sep 2025 21:51:55 GMT GLIDERN1 51.178.19.212:10152";

        let (timestamp_str, rest) = message.split_once(' ').unwrap();
        let parsed = chrono::DateTime::parse_from_rfc3339(timestamp_str);

        assert!(parsed.is_ok());
        assert!(rest.starts_with('#'));
        assert_eq!(
            rest,
            "# aprsc 2.1.15-gc67551b 22 Sep 2025 21:51:55 GMT GLIDERN1 51.178.19.212:10152"
        );
    }

    #[test]
    fn test_timestamp_format_rfc3339() {
        // Test that Utc::now().to_rfc3339() produces a parseable timestamp
        let now = chrono::Utc::now();
        let timestamp_str = now.to_rfc3339();

        let parsed = chrono::DateTime::parse_from_rfc3339(&timestamp_str);
        assert!(parsed.is_ok());

        let parsed_utc = parsed.unwrap().with_timezone(&chrono::Utc);
        // Should be within 1 second (to account for processing time)
        let diff = (now - parsed_utc).num_milliseconds().abs();
        assert!(diff < 1000);
    }
}
