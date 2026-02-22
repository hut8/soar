mod aprs;
mod beast;
mod constants;
mod monitoring;
mod sbs;
mod shutdown;
mod workers;

use anyhow::{Context, Result};
use chrono::DateTime;
use constants::*;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use soar::adsb_accumulator::AdsbAccumulator;
use soar::aircraft_repo::{AircraftCache, AircraftRepository};
use soar::aircraft_types::AircraftCategory;
use soar::fix_processor::FixProcessor;
use soar::flight_tracker::FlightTracker;
use soar::instance_lock::InstanceLock;
use soar::ogn::{
    OgnGenericProcessor, ReceiverPositionProcessor, ReceiverStatusProcessor, ServerStatusProcessor,
};
use soar::raw_messages_repo::RawMessagesRepository;
use soar::receiver_repo::ReceiverRepository;
use soar::receiver_status_repo::ReceiverStatusRepository;
use soar::server_messages_repo::ServerMessagesRepository;
use std::env;
use std::sync::Arc;
use tracing::{error, info, warn};

#[allow(clippy::too_many_arguments)]
// Note: Intentionally NOT using #[tracing::instrument] here because it creates a parent span
// that causes all spawned worker tasks to inherit the same trace context, leading to
// TRACE_TOO_LARGE errors in Tempo as the trace accumulates indefinitely.
pub async fn handle_run(
    archive_dir: Option<String>,
    nats_url: String,
    suppress_aprs_types: &[String],
    skip_ogn_aircraft_types: &[String],
    no_aprs: bool,
    no_adsb: bool,
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> Result<()> {
    // Validate that at least one consumer is enabled
    if no_aprs && no_adsb {
        anyhow::bail!(
            "Cannot disable both APRS and ADS-B consumers. At least one must be enabled."
        );
    }

    // Determine environment
    let soar_env = env::var("SOAR_ENV").unwrap_or_default();
    let is_production = soar_env == "production";
    let is_staging = soar_env == "staging";

    // Log which consumers are enabled
    info!("Starting run command with:");
    info!(
        "  APRS consumer: {}",
        if no_aprs { "DISABLED" } else { "ENABLED" }
    );
    info!(
        "  ADS-B consumer: {}",
        if no_adsb { "DISABLED" } else { "ENABLED" }
    );

    // Initialize the metrics recorder FIRST so the global labels are applied
    // before we initialize any metrics to zero
    if is_production || is_staging {
        info!("Initializing metrics recorder with component=run...");
        soar::metrics::init_metrics_recorder(Some("run"));
    }

    // Initialize all soar-run metrics to zero so they appear in Grafana even before events occur
    // This MUST happen after the recorder is initialized (so global labels apply)
    // and before starting the metrics server (to avoid race conditions where
    // Prometheus scrapes before metrics are initialized)
    info!("Initializing soar-run metrics...");
    soar::metrics::initialize_run_metrics();
    info!("soar-run metrics initialized");

    // Start metrics server in the background (AFTER metrics are initialized)
    if is_production || is_staging {
        // Auto-assign port based on environment: production=9091, staging=9192
        let metrics_port = if is_staging { 9192 } else { 9091 };
        info!("Starting metrics server on port {}", metrics_port);
        let metrics_handle = tokio::spawn(async move {
            soar::metrics::start_metrics_server(metrics_port, Some("run")).await;
        });
        // Monitor the metrics server so we know if it dies
        tokio::spawn(async move {
            match metrics_handle.await {
                Ok(()) => error!("Metrics server exited unexpectedly"),
                Err(e) => error!("Metrics server task panicked: {}", e),
            }
        });
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
        env::var("ELEVATION_DATA_PATH").unwrap_or_else(|_| "/var/lib/soar/elevation".to_string());
    info!("Elevation data path: {}", elevation_path);

    info!(
        "Environment: {}",
        if is_production {
            "production"
        } else if is_staging {
            "staging"
        } else {
            "development"
        }
    );

    // Create and preload aircraft cache (loads aircraft with fixes in last 7 days)
    let aircraft_cache = AircraftCache::new(diesel_pool.clone());
    aircraft_cache
        .preload()
        .await
        .context("Failed to preload aircraft cache")?;

    // Create FlightTracker
    let flight_tracker = FlightTracker::new(&diesel_pool, aircraft_cache.clone());

    // Initialize flight tracker from database:
    // 1. Timeout old incomplete flights (last_fix_at older than 1 hour)
    // 2. Restore aircraft states:
    //    - Active flights: last_fix_at within last 1 hour
    //    - Timed-out flights: last_fix_at within last 18 hours (for resumption)
    //    - Loads last 10 fixes per aircraft to rebuild in-memory state
    //    - Critical for correct takeoff/landing detection and flight coalescing
    let timeout_duration = chrono::Duration::hours(1);
    match flight_tracker
        .initialize_from_database(timeout_duration)
        .await
    {
        Ok((timed_out, restored)) => {
            info!(
                "Flight tracker initialized: {} flights timed out, {} aircraft states restored",
                timed_out, restored
            );
        }
        Err(e) => {
            warn!("Failed to initialize flight tracker from database: {}", e);
        }
    }

    // Start flight timeout checker (every 60 seconds)
    flight_tracker.start_timeout_checker(60);

    // Start aircraft state cleanup (every hour)
    // Removes aircraft states older than 18 hours
    flight_tracker.start_state_cleanup(3600);

    // Log suppressed APRS types if any
    if !suppress_aprs_types.is_empty() {
        info!(
            "Suppressing APRS types from processing: {:?}",
            suppress_aprs_types
        );
    }

    // Parse and validate aircraft categories to skip
    let parsed_aircraft_categories: Vec<AircraftCategory> = skip_ogn_aircraft_types
        .iter()
        .filter_map(|type_str| {
            type_str
                .parse::<AircraftCategory>()
                .map_err(|e| {
                    warn!("Invalid aircraft category '{}': {}", type_str, e);
                    e
                })
                .ok()
        })
        .collect();

    // Log skipped aircraft categories if any
    if !parsed_aircraft_categories.is_empty() {
        info!(
            "Skipping aircraft categories from processing: {:?}",
            parsed_aircraft_categories
        );
    }

    // Create database fix processor to save all valid fixes to the database
    // Try to create with NATS first, fall back to without NATS if connection fails
    let fix_processor = match FixProcessor::with_flight_tracker_and_nats(
        diesel_pool.clone(),
        aircraft_cache.clone(),
        flight_tracker.clone(),
        &nats_url,
    )
    .await
    {
        Ok(processor_with_nats) => {
            info!("Created FixProcessor with NATS publisher");
            processor_with_nats
                .with_suppressed_aprs_types(suppress_aprs_types.to_vec())
                .with_suppressed_aircraft_categories(parsed_aircraft_categories.clone())
                .with_sync_elevation(flight_tracker.elevation_db().clone())
        }
        Err(e) => {
            warn!(
                "Failed to create FixProcessor with NATS ({}), falling back to processor without NATS",
                e
            );
            FixProcessor::with_flight_tracker(
                diesel_pool.clone(),
                aircraft_cache.clone(),
                flight_tracker.clone(),
            )
            .with_suppressed_aprs_types(suppress_aprs_types.to_vec())
            .with_suppressed_aircraft_categories(parsed_aircraft_categories.clone())
            .with_sync_elevation(flight_tracker.elevation_db().clone())
        }
    };

    // Create server status processor for server messages
    let server_messages_repo = ServerMessagesRepository::new(diesel_pool.clone());
    let server_status_processor = ServerStatusProcessor::new(server_messages_repo);

    // Create repositories
    let receiver_repo = ReceiverRepository::new(diesel_pool.clone());
    let receiver_status_repo = ReceiverStatusRepository::new(diesel_pool.clone());
    let aprs_messages_repo =
        soar::raw_messages_repo::AprsMessagesRepository::new(diesel_pool.clone());

    // Create OgnGenericProcessor for archiving, receiver identification, and APRS message insertion
    let generic_processor = if let Some(archive_path) = archive_dir.clone() {
        let archive_service = soar::ArchiveService::new(archive_path).await?;
        OgnGenericProcessor::new(receiver_repo.clone(), aprs_messages_repo)
            .with_archive_service(archive_service)
    } else {
        OgnGenericProcessor::new(receiver_repo.clone(), aprs_messages_repo)
    };

    // Create receiver status processor for receiver status messages
    let receiver_status_processor =
        ReceiverStatusProcessor::new(receiver_status_repo, receiver_repo.clone());

    // Create receiver position processor for receiver position messages
    let receiver_position_processor = ReceiverPositionProcessor::new(receiver_repo.clone());

    // Create Beast/SBS processing infrastructure (only if ADS-B is enabled)
    // Both Beast (binary ADS-B) and SBS (text CSV) share the same accumulator
    let beast_infrastructure = if !no_adsb {
        let aircraft_repo = AircraftRepository::new(diesel_pool.clone());
        let beast_repo = RawMessagesRepository::new(diesel_pool.clone());

        // Create ADS-B accumulator for combining position/velocity/callsign data
        // Wraps CPR decoder and adds state accumulation across message types
        let adsb_accumulator = Arc::new(AdsbAccumulator::new());

        Some((aircraft_repo, beast_repo, adsb_accumulator))
    } else {
        None
    };

    info!(
        "Setting up APRS processing with {} OGN intake workers - archive directory: {:?}, NATS URL: {}",
        OGN_INTAKE_WORKERS, archive_dir, nats_url
    );

    // OGN intake queue: buffers raw OGN/APRS messages from unix socket
    // Tuple of (received_at timestamp, message string)
    // Only create if APRS is enabled
    let ogn_intake_opt = if !no_aprs {
        let (tx, rx) = flume::bounded::<(DateTime<chrono::Utc>, String)>(OGN_INTAKE_QUEUE_SIZE);
        info!(
            "Created OGN intake queue with capacity {}",
            OGN_INTAKE_QUEUE_SIZE
        );
        Some((tx, rx))
    } else {
        info!("APRS consumer disabled, skipping OGN intake queue creation");
        None
    };

    // Beast intake queue: buffers raw Beast messages from socket server
    // Tuple of (received_at timestamp, raw Beast frame bytes)
    // Only create if ADS-B is enabled
    let beast_intake_opt = if !no_adsb {
        let (tx, rx) = flume::bounded::<(DateTime<chrono::Utc>, Vec<u8>)>(BEAST_INTAKE_QUEUE_SIZE);
        info!(
            "Created Beast intake queue with capacity {}",
            BEAST_INTAKE_QUEUE_SIZE
        );
        Some((tx, rx))
    } else {
        info!("ADS-B consumer disabled, skipping Beast intake queue creation");
        None
    };

    // SBS intake queue: buffers raw SBS CSV messages from socket server
    // Tuple of (received_at timestamp, raw CSV bytes)
    // Only create if ADS-B is enabled (SBS is an alternative ADS-B source)
    let sbs_intake_opt = if !no_adsb {
        let (tx, rx) = flume::bounded::<(DateTime<chrono::Utc>, Vec<u8>)>(SBS_INTAKE_QUEUE_SIZE);
        info!(
            "Created SBS intake queue with capacity {}",
            SBS_INTAKE_QUEUE_SIZE
        );
        Some((tx, rx))
    } else {
        info!("ADS-B consumer disabled, skipping SBS intake queue creation");
        None
    };

    // Create Unix socket server for receiving messages from ingesters
    let socket_path = soar::socket_path();
    let socket_server = soar::socket_server::SocketServer::start(&socket_path)
        .await
        .context("Failed to start socket server")?;
    info!("Socket server listening on {:?}", socket_path);

    // Envelope intake queue: buffers messages from socket server before routing
    let (envelope_tx, envelope_rx) =
        flume::bounded::<soar::protocol::Envelope>(ENVELOPE_INTAKE_QUEUE_SIZE);
    info!(
        "Created envelope intake queue with capacity {}",
        ENVELOPE_INTAKE_QUEUE_SIZE
    );

    // Spawn socket accept loop task
    tokio::spawn(async move {
        socket_server.accept_loop(envelope_tx).await;
    });
    info!("Spawned socket server accept loop");

    // Spawn envelope router task
    let ogn_intake_tx_for_router = ogn_intake_opt.as_ref().map(|(tx, _)| tx.clone());
    let beast_intake_tx_for_router = beast_intake_opt.as_ref().map(|(tx, _)| tx.clone());
    let sbs_intake_tx_for_router = sbs_intake_opt.as_ref().map(|(tx, _)| tx.clone());
    let metrics_envelope_rx = envelope_rx.clone(); // Clone for metrics before moving
    let router_packets_total = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let router_lag_ms = Arc::new(std::sync::atomic::AtomicI64::new(0));
    let router_packets_for_task = router_packets_total.clone();
    let router_lag_for_task = router_lag_ms.clone();
    tokio::spawn(async move {
        info!("Envelope router task started");
        while let Ok(envelope) = envelope_rx.recv_async().await {
            // Convert envelope timestamp to DateTime<Utc>
            let received_at = DateTime::from_timestamp_micros(envelope.timestamp_micros)
                .unwrap_or_else(chrono::Utc::now);

            let lag_millis = (chrono::Utc::now() - received_at).num_milliseconds();
            let lag_seconds = lag_millis as f64 / 1000.0;
            metrics::gauge!("socket.router.lag_seconds").set(lag_seconds);
            router_lag_for_task.store(lag_millis, std::sync::atomic::Ordering::Relaxed);
            router_packets_for_task.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            match envelope.source() {
                soar::protocol::IngestSource::Ogn => {
                    if let Some(ogn_tx) = &ogn_intake_tx_for_router {
                        // Decode bytes to String (OGN messages are UTF-8)
                        match String::from_utf8(envelope.data) {
                            Ok(message) => {
                                if ogn_tx.is_full() {
                                    metrics::counter!("queue.send_blocked_total", "queue" => "ogn_intake").increment(1);
                                }
                                if let Err(e) = ogn_tx.send_async((received_at, message)).await {
                                    error!("Failed to send OGN message to intake queue: {}", e);
                                    metrics::counter!("socket.router.aprs_send_error_total")
                                        .increment(1);
                                } else {
                                    metrics::counter!("socket.router.aprs_routed_total")
                                        .increment(1);
                                }
                            }
                            Err(e) => {
                                error!("Failed to decode OGN message as UTF-8: {}", e);
                                metrics::counter!("socket.router.decode_error_total").increment(1);
                            }
                        }
                    }
                }
                soar::protocol::IngestSource::Beast => {
                    if let Some(beast_tx) = &beast_intake_tx_for_router {
                        if beast_tx.is_full() {
                            metrics::counter!("queue.send_blocked_total", "queue" => "beast_intake").increment(1);
                        }
                        if let Err(e) = beast_tx.send_async((received_at, envelope.data)).await {
                            error!("Failed to send Beast message to intake queue: {}", e);
                            metrics::counter!("socket.router.beast_send_error_total").increment(1);
                        } else {
                            metrics::counter!("socket.router.beast_routed_total").increment(1);
                        }
                    }
                }
                soar::protocol::IngestSource::Sbs => {
                    if let Some(sbs_tx) = &sbs_intake_tx_for_router {
                        if sbs_tx.is_full() {
                            metrics::counter!("queue.send_blocked_total", "queue" => "sbs_intake")
                                .increment(1);
                        }
                        if let Err(e) = sbs_tx.send_async((received_at, envelope.data)).await {
                            error!("Failed to send SBS message to intake queue: {}", e);
                            metrics::counter!("socket.router.sbs_send_error_total").increment(1);
                        } else {
                            metrics::counter!("socket.router.sbs_routed_total").increment(1);
                        }
                    }
                }
            }
        }
        info!("Envelope router task stopped");
    });
    info!("Spawned envelope router task");

    // Spawn OGN intake queue workers (only if APRS is enabled)
    if let Some((_, ogn_intake_rx)) = ogn_intake_opt.as_ref() {
        workers::spawn_ogn_intake_workers(
            ogn_intake_rx.clone(),
            &generic_processor,
            &fix_processor,
            &receiver_status_processor,
            &receiver_position_processor,
            &server_status_processor,
            OGN_INTAKE_WORKERS,
        );
    }

    // Spawn Beast intake queue workers (only if ADS-B is enabled)
    if let (
        Some((beast_aircraft_repo, beast_repo_clone, beast_accumulator)),
        Some((_, beast_intake_rx)),
    ) = (beast_infrastructure.as_ref(), beast_intake_opt.as_ref())
    {
        workers::spawn_beast_intake_workers(
            beast_intake_rx.clone(),
            beast_aircraft_repo,
            beast_repo_clone,
            &fix_processor,
            beast_accumulator,
        );
    }

    // Spawn SBS intake queue workers (only if ADS-B is enabled)
    if let (Some((sbs_aircraft_repo, sbs_repo, sbs_accumulator)), Some((_, sbs_intake_rx))) =
        (beast_infrastructure.as_ref(), sbs_intake_opt.as_ref())
    {
        workers::spawn_sbs_intake_workers(
            sbs_intake_rx.clone(),
            sbs_aircraft_repo,
            sbs_repo,
            &fix_processor,
            sbs_accumulator,
        );
    }

    // Spawn queue depth and system metrics reporter
    monitoring::spawn_metrics_reporter(
        metrics_envelope_rx,
        diesel_pool.clone(),
        router_packets_total,
        router_lag_ms,
    );
    info!("Spawned queue depth metrics reporter (reports every 10 seconds to Prometheus)");

    // Set up graceful shutdown handler
    shutdown::spawn_shutdown_handler(
        ogn_intake_opt.as_ref().map(|(tx, _)| tx.clone()),
        beast_intake_opt.as_ref().map(|(tx, _)| tx.clone()),
    );
    info!("Graceful shutdown handler configured");

    // All processing tasks are now running via socket server and envelope router
    // Just wait for shutdown signal
    info!("Main processing loop started. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;
    info!("Received shutdown signal, exiting...");
    Ok(())
}

#[cfg(test)]
mod tests;
