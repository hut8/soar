use anyhow::{Context, Result};
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use soar::elevation::ElevationDB;
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
    APRS_RAW_SUBJECT, APRS_RAW_SUBJECT_STAGING, ELEVATION_QUEUE_SIZE, RECEIVER_POSITION_QUEUE_SIZE,
    RECEIVER_STATUS_QUEUE_SIZE, SERVER_STATUS_QUEUE_SIZE, SOAR_RUN_CONSUMER,
    SOAR_RUN_CONSUMER_STAGING, queue_warning_threshold,
};
use soar::receiver_repo::ReceiverRepository;
use soar::receiver_status_repo::ReceiverStatusRepository;
use soar::server_messages_repo::ServerMessagesRepository;
use std::env;
use tracing::Instrument;
use tracing::{info, trace, warn};

/// Process a received APRS message from JetStream by parsing and routing through PacketRouter
async fn process_aprs_message(
    message: &str,
    packet_router: &soar::packet_processors::PacketRouter,
) {
    // Route server messages (starting with #) differently
    if message.starts_with('#') {
        info!("Server message: {}", message);
        packet_router.process_server_message(message);
        return;
    }

    // Try to parse the message using ogn-parser
    match ogn_parser::parse(message) {
        Ok(parsed) => {
            // Call PacketRouter to archive, process, and route to queues
            packet_router.process_packet(parsed, message).await;
        }
        Err(e) => {
            // For OGNFNT sources with invalid lat/lon, log as trace instead of error
            // These are common and expected issues with this data source
            let error_str = e.to_string();
            if message.contains("OGNFNT")
                && (error_str.contains("Invalid Latitude")
                    || error_str.contains("Invalid Longitude"))
            {
                trace!("Failed to parse APRS message '{message}': {e}");
            } else {
                info!("Failed to parse APRS message '{message}': {e}");
            }
        }
    }
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

    // Start metrics server in the background
    if is_production {
        info!("Starting metrics server on port 9091");
        tokio::spawn(
            async {
                soar::metrics::start_metrics_server(9091).await;
            }
            .instrument(tracing::info_span!("metrics_server")),
        );
    }

    // Initialize all soar-run metrics to zero so they appear in Grafana even before events occur
    info!("Initializing soar-run metrics...");

    // JetStream consumer metrics
    metrics::counter!("aprs.jetstream.consumed").absolute(0);
    metrics::counter!("aprs.jetstream.consumer_error").absolute(0);

    // Receiver cache metrics
    metrics::counter!("generic_processor.receiver_cache.hit").absolute(0);
    metrics::counter!("generic_processor.receiver_cache.miss").absolute(0);

    // Flight tracker metrics
    metrics::counter!("flight_tracker_timeouts_detected").absolute(0);

    info!("soar-run metrics initialized");

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
        tokio::sync::mpsc::channel::<soar::elevation::ElevationTask>(ELEVATION_QUEUE_SIZE);

    info!("Created bounded elevation processing queue with capacity 1,000");

    // Create separate bounded channel for AGL database updates
    // This separates the fast elevation calculation from the slower database updates
    // and allows batching of database writes for much better throughput
    let (agl_db_tx, agl_db_rx) =
        tokio::sync::mpsc::channel::<soar::elevation::AglDatabaseTask>(AGL_DATABASE_QUEUE_SIZE);

    info!("Created bounded AGL database update queue with capacity 10,000");

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

    // Set up shutdown handler
    tokio::spawn(
        async move {
            match tokio::signal::ctrl_c().await {
                Ok(()) => {
                    info!("Received Ctrl+C, exiting...");
                    std::process::exit(0);
                }
                Err(err) => {
                    eprintln!("Unable to listen for shutdown signal: {}", err);
                }
            }
        }
        .instrument(tracing::info_span!("shutdown_handler")),
    );

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
    let (aircraft_tx, aircraft_rx) = tokio::sync::mpsc::channel(AIRCRAFT_QUEUE_SIZE);
    info!(
        "Created aircraft position queue with capacity {}",
        AIRCRAFT_QUEUE_SIZE
    );

    // Receiver status: high capacity
    let (receiver_status_tx, receiver_status_rx) =
        tokio::sync::mpsc::channel(RECEIVER_STATUS_QUEUE_SIZE);
    info!(
        "Created receiver status queue with capacity {}",
        RECEIVER_STATUS_QUEUE_SIZE
    );

    // Receiver position: medium capacity
    let (receiver_position_tx, receiver_position_rx) =
        tokio::sync::mpsc::channel(RECEIVER_POSITION_QUEUE_SIZE);
    info!(
        "Created receiver position queue with capacity {}",
        RECEIVER_POSITION_QUEUE_SIZE
    );

    // Server status: low capacity (rare messages)
    let (server_status_tx, server_status_rx) = tokio::sync::mpsc::channel(SERVER_STATUS_QUEUE_SIZE);
    info!(
        "Created server status queue with capacity {}",
        SERVER_STATUS_QUEUE_SIZE
    );

    // Create PacketRouter with per-processor queues
    let packet_router = PacketRouter::new(generic_processor)
        .with_aircraft_position_queue(aircraft_tx)
        .with_receiver_status_queue(receiver_status_tx)
        .with_receiver_position_queue(receiver_position_tx)
        .with_server_status_queue(server_status_tx);

    info!("Created PacketRouter with per-processor queues");

    // Initialize queue drop counters so they're always exported to Prometheus
    // even when queues are healthy and not dropping packets
    metrics::counter!("aprs.raw_message_queue.full").absolute(0);
    metrics::counter!("aprs.aircraft_queue.full").absolute(0);
    metrics::counter!("aprs.aircraft_queue.closed").absolute(0);
    metrics::counter!("aprs.receiver_status_queue.full").absolute(0);
    metrics::counter!("aprs.receiver_status_queue.closed").absolute(0);
    metrics::counter!("aprs.receiver_position_queue.full").absolute(0);
    metrics::counter!("aprs.receiver_position_queue.closed").absolute(0);
    metrics::counter!("aprs.server_status_queue.full").absolute(0);
    metrics::counter!("aprs.server_status_queue.closed").absolute(0);

    // Initialize APRS connection metrics
    metrics::gauge!("aprs.connection.connected").set(0.0);
    metrics::counter!("aprs.connection.established").absolute(0);
    metrics::counter!("aprs.connection.ended").absolute(0);
    metrics::counter!("aprs.connection.failed").absolute(0);
    metrics::counter!("aprs.connection.operation_failed").absolute(0);
    metrics::counter!("aprs.keepalive.sent").absolute(0);

    // Initialize JetStream metrics
    metrics::gauge!("aprs.jetstream.queue_depth").set(0.0);
    metrics::counter!("aprs.jetstream.published").absolute(0);
    metrics::counter!("aprs.jetstream.publish_error").absolute(0);
    metrics::counter!("aprs.jetstream.consumed").absolute(0);
    metrics::counter!("aprs.jetstream.decode_error").absolute(0);
    metrics::counter!("aprs.jetstream.ack_error").absolute(0);
    metrics::counter!("aprs.jetstream.process_error").absolute(0);
    metrics::counter!("aprs.jetstream.receive_error").absolute(0);

    // Initialize NATS publisher error counter
    metrics::counter!("nats_publisher_errors").absolute(0);

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

    // Wrap elevation receiver in Arc<Mutex> to share among elevation workers
    let shared_elevation_rx = std::sync::Arc::new(tokio::sync::Mutex::new(elevation_rx));

    for elevation_worker_id in 0..num_elevation_workers {
        let worker_elevation_rx = shared_elevation_rx.clone();
        let worker_elevation_db = elevation_db.clone();
        let worker_agl_db_tx = agl_db_tx.clone();

        tokio::spawn(
            async move {
                loop {
                    // Lock the receiver and try to get an elevation task
                    let task = {
                        let mut rx = worker_elevation_rx.lock().await;
                        rx.recv().await
                    };

                    match task {
                        Some(task) => {
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

                            // Send calculated AGL to database batch writer
                            let agl_task = soar::elevation::AglDatabaseTask {
                                fix_id: task.fix_id,
                                altitude_agl_feet: agl,
                            };

                            if let Err(e) = worker_agl_db_tx.try_send(agl_task) {
                                match e {
                                    tokio::sync::mpsc::error::TrySendError::Full(_) => {
                                        warn!("AGL database queue is FULL (10,000 tasks) - dropping database update for fix {}", task.fix_id);
                                        metrics::counter!("agl_db_queue.dropped_total").increment(1);
                                    }
                                    tokio::sync::mpsc::error::TrySendError::Closed(_) => {
                                        warn!("AGL database queue is closed");
                                    }
                                }
                            }
                        }
                        None => {
                            // Channel closed, exit worker
                            break;
                        }
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

    // Spawn AGL backfill background task
    // This task runs continuously, backfilling AGL altitudes for old fixes that were missed
    // It gets its own dedicated ElevationDB with a larger GDAL dataset cache (1000 vs 100)
    // to avoid contention with real-time elevation processing
    let backfill_fixes_repo = FixesRepository::new(diesel_pool.clone());
    let backfill_elevation_db = ElevationDB::with_custom_cache_sizes(500_000, 1000)
        .context("Failed to create dedicated ElevationDB for AGL backfill")?;
    info!(
        "Created dedicated ElevationDB for AGL backfill (dataset cache: 1000, elevation cache: 500,000)"
    );
    tokio::spawn(
        async move {
            soar::agl_backfill::agl_backfill_task(backfill_fixes_repo, backfill_elevation_db).await;
        }
        .instrument(tracing::info_span!("agl_backfill")),
    );
    info!("Spawned AGL backfill background task with dedicated elevation database");

    // Spawn dedicated worker pools for each processor type
    // Aircraft position workers (20 workers - heaviest processing due to FixProcessor + flight tracking)
    let num_aircraft_workers = 20;
    info!(
        "Spawning {} aircraft position workers",
        num_aircraft_workers
    );
    let shared_aircraft_rx = std::sync::Arc::new(tokio::sync::Mutex::new(aircraft_rx));
    for worker_id in 0..num_aircraft_workers {
        let worker_rx = shared_aircraft_rx.clone();
        let processor = aircraft_position_processor.clone();
        tokio::spawn(
            async move {
                loop {
                    let task = {
                        let mut rx = worker_rx.lock().await;
                        rx.recv().await
                    };
                    match task {
                        Some((packet, context)) => {
                            let start = std::time::Instant::now();
                            processor.process_aircraft_position(&packet, context).await;
                            let duration = start.elapsed();
                            metrics::histogram!("aprs.aircraft.duration_ms")
                                .record(duration.as_millis() as f64);
                            metrics::counter!("aprs.aircraft.processed").increment(1);
                        }
                        None => break,
                    }
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
    let shared_receiver_status_rx =
        std::sync::Arc::new(tokio::sync::Mutex::new(receiver_status_rx));
    for worker_id in 0..num_receiver_status_workers {
        let worker_rx = shared_receiver_status_rx.clone();
        let processor = receiver_status_processor.clone();
        tokio::spawn(
            async move {
                loop {
                    let task = {
                        let mut rx = worker_rx.lock().await;
                        rx.recv().await
                    };
                    match task {
                        Some((packet, context)) => {
                            let start = std::time::Instant::now();
                            processor.process_status_packet(&packet, context).await;
                            let duration = start.elapsed();
                            metrics::histogram!("aprs.receiver_status.duration_ms")
                                .record(duration.as_millis() as f64);
                            metrics::counter!("aprs.receiver_status.processed").increment(1);
                        }
                        None => break,
                    }
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
    let shared_receiver_position_rx =
        std::sync::Arc::new(tokio::sync::Mutex::new(receiver_position_rx));
    for worker_id in 0..num_receiver_position_workers {
        let worker_rx = shared_receiver_position_rx.clone();
        let processor = receiver_position_processor.clone();
        tokio::spawn(
            async move {
                loop {
                    let task = {
                        let mut rx = worker_rx.lock().await;
                        rx.recv().await
                    };
                    match task {
                        Some((packet, context)) => {
                            let start = std::time::Instant::now();
                            processor.process_receiver_position(&packet, context).await;
                            let duration = start.elapsed();
                            metrics::histogram!("aprs.receiver_position.duration_ms")
                                .record(duration.as_millis() as f64);
                            metrics::counter!("aprs.receiver_position.processed").increment(1);
                        }
                        None => break,
                    }
                }
            }
            .instrument(tracing::info_span!("receiver_position_worker", worker_id)),
        );
    }

    // Server status workers (2 workers - very light processing)
    info!("Spawning 2 server status workers");
    let shared_server_status_rx = std::sync::Arc::new(tokio::sync::Mutex::new(server_status_rx));
    for worker_id in 0..2 {
        let worker_rx = shared_server_status_rx.clone();
        let processor = server_status_processor.clone();
        tokio::spawn(
            async move {
                loop {
                    let task = {
                        let mut rx = worker_rx.lock().await;
                        rx.recv().await
                    };
                    match task {
                        Some(message) => {
                            let start = std::time::Instant::now();
                            processor.process_server_message(&message).await;
                            let duration = start.elapsed();
                            metrics::histogram!("aprs.server_status.duration_ms")
                                .record(duration.as_millis() as f64);
                            metrics::counter!("aprs.server_status.processed").increment(1);
                        }
                        None => break,
                    }
                }
            }
            .instrument(tracing::info_span!("server_status_worker", worker_id)),
        );
    }

    // Spawn queue depth metrics reporter
    // Reports the depth of all processing queues to Prometheus every 10 seconds
    let metrics_aircraft_rx = shared_aircraft_rx.clone();
    let metrics_receiver_status_rx = shared_receiver_status_rx.clone();
    let metrics_receiver_position_rx = shared_receiver_position_rx.clone();
    let metrics_server_status_rx = shared_server_status_rx.clone();
    let metrics_elevation_rx = shared_elevation_rx.clone();
    tokio::spawn(
        async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            interval.tick().await; // First tick completes immediately

            loop {
                interval.tick().await;

                // Sample queue depths (non-blocking reads)
                let aircraft_depth = {
                    if let Ok(rx) = metrics_aircraft_rx.try_lock() {
                        rx.len()
                    } else {
                        0 // Skip if locked
                    }
                };
                let receiver_status_depth = {
                    if let Ok(rx) = metrics_receiver_status_rx.try_lock() {
                        rx.len()
                    } else {
                        0
                    }
                };
                let receiver_position_depth = {
                    if let Ok(rx) = metrics_receiver_position_rx.try_lock() {
                        rx.len()
                    } else {
                        0
                    }
                };
                let server_status_depth = {
                    if let Ok(rx) = metrics_server_status_rx.try_lock() {
                        rx.len()
                    } else {
                        0
                    }
                };
                let elevation_depth = {
                    if let Ok(rx) = metrics_elevation_rx.try_lock() {
                        rx.len()
                    } else {
                        0
                    }
                };

                // Report to Prometheus
                metrics::gauge!("aprs.aircraft_queue.depth").set(aircraft_depth as f64);
                metrics::gauge!("aprs.receiver_status_queue.depth")
                    .set(receiver_status_depth as f64);
                metrics::gauge!("aprs.receiver_position_queue.depth")
                    .set(receiver_position_depth as f64);
                metrics::gauge!("aprs.server_status_queue.depth").set(server_status_depth as f64);
                metrics::gauge!("aprs.elevation_queue.depth").set(elevation_depth as f64);

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

    // Spawn periodic performance metrics logger before starting consumption
    // This helps diagnose what's slowing down processing
    let _metrics_handle = tokio::spawn(
        async move {
            use std::time::{Duration, Instant};
            let mut last_log = Instant::now();
            let log_interval = Duration::from_secs(30); // Log every 30 seconds

            loop {
                tokio::time::sleep(log_interval).await;

                let elapsed_secs = last_log.elapsed().as_secs_f64();
                last_log = Instant::now();

                // Log performance summary
                // The metrics are tracked via the metrics crate and available in the metrics backend
                info!(
                    "=== PERFORMANCE METRICS (last {:.0}s) ===",
                    elapsed_secs
                );
                info!(
                    "Worker pools: {} aircraft, {} receiver_status, {} receiver_position, 2 server_status",
                    num_aircraft_workers, num_receiver_status_workers, num_receiver_position_workers
                );
                info!(
                    "Per-processor metrics: aprs.aircraft.duration_ms, aprs.receiver_status.duration_ms, aprs.receiver_position.duration_ms, aprs.server_status.duration_ms"
                );
                info!(
                    "Queue drops: aprs.aircraft_queue.full, aprs.receiver_status_queue.full, aprs.receiver_position_queue.full, aprs.server_status_queue.full"
                );
                info!(
                    "Elevation metrics: aprs.elevation.duration_ms, aprs.elevation.queued, aprs.elevation.dropped"
                );
            }
        }
        .instrument(tracing::info_span!("performance_metrics_logger")),
    );

    info!("APRS client started. Press Ctrl+C to stop.");

    // Start consuming messages from JetStream
    // This runs indefinitely until the stream ends or an error occurs
    // Each message will be parsed and routed through PacketRouter
    consumer
        .consume(move |message| {
            let packet_router = packet_router.clone();
            async move {
                // Process the APRS message: parse and route through PacketRouter
                process_aprs_message(&message, &packet_router).await;
                Ok(())
            }
        })
        .await?;

    // If we reach here, the consumer stream ended unexpectedly
    warn!("JetStream consumer stopped unexpectedly");
    Ok(())
}
