use axum::{Router, http::StatusCode, response::IntoResponse, routing::get};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use pprof::protos::Message;
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn};

static METRICS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Global health state for APRS ingestion service
/// Used by the /health endpoint to determine readiness
#[derive(Clone, Debug, Default)]
pub struct AprsIngestHealth {
    pub aprs_connected: bool,
    pub socket_connected: bool,
    pub last_message_time: Option<Instant>,
}

static APRS_HEALTH: OnceLock<Arc<RwLock<AprsIngestHealth>>> = OnceLock::new();

/// Initialize the global APRS health state
pub fn init_aprs_health() -> Arc<RwLock<AprsIngestHealth>> {
    Arc::new(RwLock::new(AprsIngestHealth::default()))
}

/// Get the global APRS health state
pub fn get_aprs_health() -> Option<Arc<RwLock<AprsIngestHealth>>> {
    APRS_HEALTH.get().cloned()
}

/// Set the global APRS health state (should be called once during initialization)
pub fn set_aprs_health(health: Arc<RwLock<AprsIngestHealth>>) {
    let _ = APRS_HEALTH.set(health);
}

/// Global health state for Beast (ADS-B) ingestion service
/// Used by the /health endpoint to determine readiness
#[derive(Clone, Debug, Default)]
pub struct BeastIngestHealth {
    pub beast_connected: bool,
    pub socket_connected: bool,
    pub last_message_time: Option<Instant>,
    pub last_error: Option<String>,
}

static BEAST_HEALTH: OnceLock<Arc<RwLock<BeastIngestHealth>>> = OnceLock::new();

/// Initialize the global Beast health state
pub fn init_beast_health() -> Arc<RwLock<BeastIngestHealth>> {
    Arc::new(RwLock::new(BeastIngestHealth::default()))
}

/// Get the global Beast health state
pub fn get_beast_health() -> Option<Arc<RwLock<BeastIngestHealth>>> {
    BEAST_HEALTH.get().cloned()
}

/// Set the global Beast health state (should be called once during initialization)
pub fn set_beast_health(health: Arc<RwLock<BeastIngestHealth>>) {
    let _ = BEAST_HEALTH.set(health);
}

/// Initialize Prometheus metrics exporter
/// Returns a handle that can be used to render metrics for scraping
/// Optionally adds a component label to all metrics (e.g., "ingest", "web", "run")
///
/// Note: OTLP metrics export is available via the `telemetry` module's `init_meter_provider`
/// function. The `metrics` crate can only have one recorder (Prometheus in this case), but
/// OpenTelemetry metrics can be instrumented separately using the OpenTelemetry SDK's meter API
/// for dual export to both Prometheus and OTLP endpoints.
pub fn init_metrics(component: Option<&str>) -> PrometheusHandle {
    let mut builder = PrometheusBuilder::new();

    // Add component label if provided
    if let Some(comp) = component {
        builder = builder.add_global_label("component", comp);
    }

    builder
        // Configure HTTP request duration as histogram with appropriate buckets
        // Buckets: 1ms, 5ms, 10ms, 25ms, 50ms, 100ms, 250ms, 500ms, 1s, 2.5s, 5s, 10s
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Full("http_request_duration_seconds".to_string()),
            &[
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ],
        )
        .expect("failed to set buckets for http_request_duration_seconds")
        // Configure NATS publish duration histograms with millisecond buckets
        // Buckets: 0.5ms, 1ms, 2ms, 5ms, 10ms, 25ms, 50ms, 100ms, 250ms, 500ms, 1000ms
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Full("aprs_nats_publish_duration_ms".to_string()),
            &[
                0.5, 1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0,
            ],
        )
        .expect("failed to set buckets for aprs_nats_publish_duration_ms")
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Full(
                "beast_nats_publish_duration_ms".to_string(),
            ),
            &[
                0.5, 1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0,
            ],
        )
        .expect("failed to set buckets for beast_nats_publish_duration_ms")
        // Configure socket client send duration histogram with millisecond buckets
        // Buckets: 0.5ms, 1ms, 2ms, 5ms, 10ms, 25ms, 50ms, 100ms, 250ms, 500ms, 1000ms
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Full(
                "socket_client_send_duration_ms".to_string(),
            ),
            &[
                0.5, 1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0,
            ],
        )
        .expect("failed to set buckets for socket_client_send_duration_ms")
        // Configure coalesce speed histogram with mph buckets
        // Buckets: 0, 5, 10, 20, 30, 50, 100, 200, 500, 1000 mph
        // Covers: stationary gliders, normal glider speeds, high-speed aircraft, anomalies
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Full(
                "flight_tracker.coalesce.speed_mph".to_string(),
            ),
            &[
                0.0, 5.0, 10.0, 20.0, 30.0, 50.0, 100.0, 200.0, 500.0, 1000.0,
            ],
        )
        .expect("failed to set buckets for flight_tracker.coalesce.speed_mph")
        // Configure coalesce distance histograms with km buckets
        // Buckets: 0.1, 0.5, 1, 2, 5, 10, 20, 50, 100, 200, 500 km
        // Covers: close GPS dropouts (100m), nearby coalesces, moderate distances, long-distance resumes
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Full(
                "flight_tracker.coalesce.resumed.distance_km".to_string(),
            ),
            &[
                0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0,
            ],
        )
        .expect("failed to set buckets for flight_tracker.coalesce.resumed.distance_km")
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Full(
                "flight_tracker.coalesce.rejected.distance_km".to_string(),
            ),
            &[
                0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0,
            ],
        )
        .expect("failed to set buckets for flight_tracker.coalesce.rejected.distance_km")
        // Configure coalesce gap duration histogram with hour buckets
        // Buckets: 0.1, 0.5, 1, 2, 4, 6, 12, 18, 24 hours
        // Covers: short gaps (minutes), medium gaps (1-4h), large gaps (4h+), hard limit (18h), edge cases (24h+)
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Full(
                "flight_tracker.coalesce.rejected.gap_hours".to_string(),
            ),
            &[0.1, 0.5, 1.0, 2.0, 4.0, 6.0, 12.0, 18.0, 24.0],
        )
        .expect("failed to set buckets for flight_tracker.coalesce.rejected.gap_hours")
        .install_recorder()
        .expect("failed to install Prometheus recorder")
}

/// CPU profiling handler
/// Returns a flamegraph SVG when profiling is complete
async fn profile_handler() -> impl IntoResponse {
    info!("Starting CPU profiling for 30 seconds");

    // Create a profiler guard
    let guard = match pprof::ProfilerGuardBuilder::default()
        .frequency(99)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
    {
        Ok(g) => g,
        Err(e) => {
            warn!("Failed to create profiler: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to start profiler".to_string(),
            );
        }
    };

    // Profile for 30 seconds
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

    // Generate report
    match guard.report().build() {
        Ok(report) => {
            // Generate flamegraph
            let mut body = Vec::new();
            if let Err(e) = report.flamegraph(&mut body) {
                warn!("Failed to generate flamegraph: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to generate flamegraph".to_string(),
                );
            }

            info!(
                "CPU profiling completed, generated flamegraph ({} bytes)",
                body.len()
            );
            (StatusCode::OK, String::from_utf8_lossy(&body).to_string())
        }
        Err(e) => {
            warn!("Failed to build profiling report: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to build profiling report".to_string(),
            )
        }
    }
}

/// Heap profiling handler
/// Returns profiling data in pprof protobuf format
async fn heap_profile_handler() -> impl IntoResponse {
    info!("Generating heap profile");

    // Create a profiler guard
    let guard = match pprof::ProfilerGuardBuilder::default()
        .frequency(99)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
    {
        Ok(g) => g,
        Err(e) => {
            warn!("Failed to create profiler: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Vec::new());
        }
    };

    // Profile for 10 seconds
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    // Generate pprof report
    match guard.report().build() {
        Ok(report) => {
            let mut body = Vec::new();
            if let Err(e) = report.pprof() {
                warn!("Failed to generate pprof: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, Vec::new());
            }

            if let Ok(profile) = report.pprof() {
                if let Err(e) = profile.write_to_vec(&mut body) {
                    warn!("Failed to serialize pprof: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, Vec::new());
                }
                info!("Heap profile generated ({} bytes)", body.len());
                (StatusCode::OK, body)
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, Vec::new())
            }
        }
        Err(e) => {
            warn!("Failed to build profiling report: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Vec::new())
        }
    }
}

/// Background task to update process metrics
/// Updates uptime and memory usage metrics every 5 seconds
pub async fn process_metrics_task() {
    let start_time = Instant::now();

    loop {
        // Update uptime (in seconds)
        let uptime_seconds = start_time.elapsed().as_secs() as f64;
        metrics::gauge!("process.uptime.seconds").set(uptime_seconds);

        // Set "is up" metric to 1 (binary indicator)
        metrics::gauge!("process.is_up").set(1.0);

        // Get memory usage using procfs (Linux-specific)
        #[cfg(target_os = "linux")]
        {
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        // Parse RSS memory in kB
                        if let Some(kb_str) = line.split_whitespace().nth(1)
                            && let Ok(kb) = kb_str.parse::<f64>()
                        {
                            let bytes = kb * 1024.0;
                            metrics::gauge!("process.memory.bytes").set(bytes);
                        }
                        break;
                    }
                }
            }
        }

        // Sleep for 5 seconds before next update
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

/// Background task to update analytics metrics from database
/// Updates flight counts, device counts, and other analytics gauges every 60 seconds
pub async fn analytics_metrics_task(pool: crate::web::PgPool) {
    use crate::analytics_repo::AnalyticsRepository;

    loop {
        // Sleep first to allow database to warm up on startup
        tokio::time::sleep(Duration::from_secs(60)).await;

        let repo = AnalyticsRepository::new(pool.clone());

        // Get summary data and update gauges
        if let Ok(summary) = repo.get_summary().await {
            metrics::gauge!("analytics.flights.today").set(summary.flights_today as f64);
            metrics::gauge!("analytics.flights.last_7d").set(summary.flights_7d as f64);
            metrics::gauge!("analytics.flights.last_30d").set(summary.flights_30d as f64);
            metrics::gauge!("analytics.aircraft.active_7d").set(summary.active_devices_7d as f64);
            metrics::gauge!("analytics.aircraft.outliers")
                .set(summary.outlier_devices_count as f64);
        }
    }
}

/// Initialize APRS ingest metrics to zero/default values
/// This ensures metrics always appear in Prometheus queries even if no events have occurred
pub fn initialize_aprs_ingest_metrics() {
    // APRS connection metrics
    metrics::counter!("aprs.connection.established_total").absolute(0);
    metrics::counter!("aprs.connection.ended_total").absolute(0);
    metrics::counter!("aprs.connection.failed_total").absolute(0);
    metrics::counter!("aprs.connection.operation_failed_total").absolute(0);
    metrics::gauge!("aprs.connection.connected").set(0.0);

    // APRS keepalive metrics
    metrics::counter!("aprs.keepalive.sent_total").absolute(0);

    // Message type tracking (received from APRS-IS)
    metrics::counter!("aprs.raw_message.received.server_total").absolute(0);
    metrics::counter!("aprs.raw_message.received.aprs_total").absolute(0);
    metrics::counter!("aprs.raw_message.queued.server_total").absolute(0);
    metrics::counter!("aprs.raw_message.queued.aprs_total").absolute(0);

    // Socket send metrics (ingest-ogn → soar-run via Unix socket)
    metrics::counter!("aprs.socket.send_error_total").absolute(0);

    // Connection timeout metric
    metrics::counter!("aprs.connection.timeout_total").absolute(0);
}

/// Initialize Beast ingest metrics to zero/default values
/// This ensures metrics always appear in Prometheus queries even if no events have occurred
pub fn initialize_beast_ingest_metrics() {
    // Beast connection metrics
    metrics::counter!("beast.connection.established_total").absolute(0);
    metrics::counter!("beast.connection.failed_total").absolute(0);
    metrics::counter!("beast.operation.failed_total").absolute(0);
    metrics::counter!("beast.timeout_total").absolute(0);
    metrics::gauge!("beast.connection.connected").set(0.0);

    // Beast data metrics
    metrics::counter!("beast.bytes.received_total").absolute(0);
    metrics::counter!("beast.frames.published_total").absolute(0);
    metrics::counter!("beast.frames.dropped_total").absolute(0);
    metrics::gauge!("beast.message_rate").set(0.0);

    // NATS publishing metrics (ingest-adsb publishes to NATS)
    metrics::counter!("beast.nats.published_total").absolute(0);
    metrics::counter!("beast.nats.publish_error_total").absolute(0);
    metrics::counter!("beast.nats.slow_publish_total").absolute(0);
    metrics::counter!("beast.nats.publish_timeout_total").absolute(0);
    metrics::counter!("beast.nats.connection_failed_total").absolute(0);
    metrics::counter!("beast.nats.stream_setup_failed_total").absolute(0);
    metrics::gauge!("beast.nats.queue_depth").set(0.0);
    metrics::gauge!("beast.nats.in_flight").set(0.0);
    metrics::histogram!("beast.nats.publish_duration_ms").record(0.0);

    // Socket send metrics (ingest-adsb → soar-run via Unix socket)
    metrics::counter!("beast.socket.send_error_total").absolute(0);

    // General ingestion metrics
    metrics::counter!("beast.ingest_failed_total").absolute(0);
}

/// Initialize Beast consumer metrics to zero/default values
/// This ensures metrics always appear in Prometheus queries even if no events have occurred
pub fn initialize_beast_consumer_metrics() {
    // NATS consumer metrics (consume-beast consumes from NATS)
    metrics::counter!("beast.nats.consumed_total").absolute(0);
    metrics::counter!("beast.nats.receive_error_total").absolute(0);
    metrics::counter!("beast.nats.ack_error_total").absolute(0);
    metrics::counter!("beast.nats.invalid_message_total").absolute(0);
    metrics::counter!("beast.nats.connection_failed_total").absolute(0);
    metrics::counter!("beast.nats.consumer_setup_failed_total").absolute(0);

    // Beast consumer processing metrics
    metrics::counter!("beast.consumer.received_total").absolute(0);
    metrics::counter!("beast.consumer.invalid_message_total").absolute(0);
    metrics::counter!("beast.consumer.send_errors_total").absolute(0);
    metrics::counter!("beast.consumer.messages_stored_total").absolute(0);
    metrics::counter!("beast.consumer.write_errors_total").absolute(0);
    metrics::histogram!("beast.consumer.batch_write_ms").record(0.0);

    // Beast decoding metrics
    metrics::counter!("beast.consumer.decoded_total").absolute(0);
    metrics::counter!("beast.consumer.decode_error_total").absolute(0);
    metrics::counter!("beast.consumer.json_error_total").absolute(0);

    // General consumption metrics
    metrics::counter!("beast.consume_failed_total").absolute(0);
}

/// Initialize SOAR run metrics to zero/default values
/// This ensures metrics always appear in Prometheus queries even if no events have occurred
pub fn initialize_run_metrics() {
    // Flight tracker metrics
    metrics::counter!("flight_tracker_timeouts_detected_total").absolute(0);
    metrics::gauge!("flight_tracker_active_aircraft").set(0.0);
    metrics::counter!("flight_tracker.fixes_processed_total").absolute(0);

    // Fast-path flight operations metrics
    metrics::histogram!("flight_tracker.create_flight_fast.latency_ms").record(0.0);
    metrics::histogram!("flight_tracker.complete_flight_fast.latency_ms").record(0.0);
    metrics::histogram!("flight_tracker.enrich_flight_on_creation.latency_ms").record(0.0);
    metrics::histogram!("flight_tracker.enrich_flight_on_completion.latency_ms").record(0.0);

    // Flight coalescing metrics
    metrics::counter!("flight_tracker.coalesce.resumed_total").absolute(0);
    metrics::counter!("flight_tracker.coalesce.callsign_mismatch_total").absolute(0);
    metrics::counter!("flight_tracker.coalesce.no_timeout_flight_total").absolute(0);
    metrics::counter!("flight_tracker.coalesce.rejected.callsign_total").absolute(0);
    metrics::counter!("flight_tracker.coalesce.rejected.probable_landing_total").absolute(0);
    metrics::counter!("flight_tracker.coalesce.rejected.hard_limit_18h_total").absolute(0);
    metrics::counter!("flight_tracker.coalesce.rejected.speed_distance_total").absolute(0);
    metrics::histogram!("flight_tracker.coalesce.rejected.distance_km").record(0.0);
    metrics::histogram!("flight_tracker.coalesce.rejected.gap_hours").record(0.0);
    metrics::histogram!("flight_tracker.coalesce.resumed.distance_km").record(0.0);
    metrics::histogram!("flight_tracker.coalesce.speed_mph").record(0.0);

    // Flight timeout phase tracking
    metrics::counter!("flight_tracker.timeout.phase_total", "phase" => "climbing").absolute(0);
    metrics::counter!("flight_tracker.timeout.phase_total", "phase" => "cruising").absolute(0);
    metrics::counter!("flight_tracker.timeout.phase_total", "phase" => "descending").absolute(0);
    metrics::counter!("flight_tracker.timeout.phase_total", "phase" => "unknown").absolute(0);

    // Flight creation metrics
    metrics::counter!("flight_tracker.flight_created.takeoff_total").absolute(0);
    metrics::counter!("flight_tracker.flight_created.airborne_total").absolute(0);

    // Flight end metrics
    metrics::counter!("flight_tracker.flight_ended.landed_total").absolute(0);
    metrics::counter!("flight_tracker.flight_ended.timed_out_total").absolute(0);

    // Receiver status metrics
    metrics::counter!("receiver_status_updates_total").absolute(0);

    // Elevation cache metrics
    metrics::counter!("elevation_cache_hits_total").absolute(0);
    metrics::counter!("elevation_cache_misses_total").absolute(0);
    metrics::gauge!("elevation_cache_entries").set(0.0);
    metrics::counter!("elevation_tile_cache_hits_total").absolute(0);
    metrics::counter!("elevation_tile_cache_misses_total").absolute(0);
    metrics::gauge!("elevation_tile_cache_entries").set(0.0);

    // Receiver cache metrics
    metrics::counter!("generic_processor.receiver_cache.hit_total").absolute(0);
    metrics::counter!("generic_processor.receiver_cache.miss_total").absolute(0);

    // NATS fix publisher metrics (publishes fixes to NATS for web clients)
    metrics::counter!("nats.fix_publisher.published_total").absolute(0);
    metrics::gauge!("nats.fix_publisher.queue_depth").set(0.0);
    metrics::counter!("nats.fix_publisher.errors_total").absolute(0);

    // Queue drop/close counters
    metrics::counter!("aprs.raw_message_queue.full_total").absolute(0);
    metrics::counter!("aprs.aircraft_queue.full_total").absolute(0);
    metrics::counter!("aprs.aircraft_queue.closed_total").absolute(0);
    metrics::counter!("aprs.receiver_status_queue.full_total").absolute(0);
    metrics::counter!("aprs.receiver_status_queue.closed_total").absolute(0);
    metrics::counter!("aprs.receiver_position_queue.full_total").absolute(0);
    metrics::counter!("aprs.receiver_position_queue.closed_total").absolute(0);
    metrics::counter!("aprs.server_status_queue.full_total").absolute(0);
    metrics::counter!("aprs.server_status_queue.closed_total").absolute(0);

    // APRS intake queue metrics (soar-run receives from socket, not NATS)
    metrics::gauge!("aprs.intake_queue.depth").set(0.0);
    metrics::counter!("aprs.intake.consumed_total").absolute(0);
    metrics::counter!("aprs.intake.process_error_total").absolute(0);
    metrics::counter!("aprs.intake.decode_error_total").absolute(0);
    metrics::counter!("aprs.intake.receive_error_total").absolute(0);
    metrics::counter!("aprs.intake_queue.full_total").absolute(0);

    // Message processing counters by type
    metrics::counter!("aprs.messages.processed.aircraft_total").absolute(0);
    metrics::counter!("aprs.messages.processed.receiver_status_total").absolute(0);
    metrics::counter!("aprs.messages.processed.receiver_position_total").absolute(0);
    metrics::counter!("aprs.messages.processed.server_total").absolute(0);
    metrics::counter!("aprs.messages.processed.total_total").absolute(0);

    // APRS parsing metrics
    metrics::counter!("aprs.parse.success_total").absolute(0);
    metrics::counter!("aprs.parse.failed_total").absolute(0);

    // Fix filtering metrics
    metrics::counter!("aprs.fixes.suppressed_total").absolute(0);
    metrics::counter!("aprs.fixes.skipped_aircraft_type_total").absolute(0);

    // Router metrics
    metrics::counter!("aprs.router_queue.disconnected_total").absolute(0);

    // Socket router metrics
    metrics::counter!("socket.router.aprs_routed_total").absolute(0);
    metrics::counter!("socket.router.aprs_send_error_total").absolute(0);
    metrics::counter!("socket.router.beast_routed_total").absolute(0);
    metrics::counter!("socket.router.beast_send_error_total").absolute(0);
    metrics::counter!("socket.router.decode_error_total").absolute(0);
    metrics::gauge!("socket.envelope_intake_queue.depth").set(0.0);

    // Aircraft position processing latency metrics
    metrics::histogram!("aprs.aircraft.lookup_ms").record(0.0);
    metrics::histogram!("aprs.aircraft.fix_creation_ms").record(0.0);
    metrics::histogram!("aprs.aircraft.process_fix_internal_ms").record(0.0);
    metrics::histogram!("aprs.aircraft.total_processing_ms").record(0.0);
    metrics::histogram!("aprs.aircraft.flight_insert_ms").record(0.0);
    metrics::histogram!("aprs.aircraft.callsign_update_ms").record(0.0);
    metrics::histogram!("aprs.aircraft.elevation_queue_ms").record(0.0);
    metrics::histogram!("aprs.aircraft.nats_publish_ms").record(0.0);

    // Granular flight insert breakdown metrics
    metrics::histogram!("aprs.aircraft.state_transition_ms").record(0.0);
    metrics::histogram!("aprs.aircraft.fix_db_insert_ms").record(0.0);
    metrics::histogram!("aprs.aircraft.device_lookup_ms").record(0.0);
    metrics::histogram!("aprs.aircraft.flight_update_last_fix_ms").record(0.0);

    // Pelias reverse geocoding metrics (Photon is no longer used)
    metrics::counter!("flight_tracker.location.pelias.success_total").absolute(0);
    metrics::counter!("flight_tracker.location.pelias.failure_total").absolute(0);
    metrics::counter!("flight_tracker.location.pelias.no_structured_data_total").absolute(0);
    metrics::histogram!("flight_tracker.location.pelias.latency_ms").record(0.0);
    metrics::counter!("flight_tracker.location.pelias.retry_total", "radius_km" => "exact")
        .absolute(0);
    metrics::counter!("flight_tracker.location.pelias.retry_total", "radius_km" => "1").absolute(0);
    metrics::counter!("flight_tracker.location.pelias.retry_total", "radius_km" => "5").absolute(0);
    metrics::counter!("flight_tracker.location.pelias.retry_total", "radius_km" => "10")
        .absolute(0);

    // Flight location tracking metrics
    metrics::counter!("flight_tracker.location.created_total", "type" => "start_takeoff")
        .absolute(0);
    metrics::counter!("flight_tracker.location.created_total", "type" => "start_airborne")
        .absolute(0);
    metrics::counter!("flight_tracker.location.created_total", "type" => "end_landing").absolute(0);
    metrics::counter!("flight_tracker.location.created_total", "type" => "end_timeout").absolute(0);

    // Beast (ADS-B) processing metrics
    metrics::counter!("beast.run.process_beast_message.called_total").absolute(0);
    metrics::counter!("beast.run.invalid_message_total").absolute(0);
    metrics::counter!("beast.run.decode.success_total").absolute(0);
    metrics::counter!("beast.run.decode.failed_total").absolute(0);
    metrics::counter!("beast.run.icao_extraction_failed_total").absolute(0);
    metrics::counter!("beast.run.aircraft_lookup_failed_total").absolute(0);
    metrics::counter!("beast.run.raw_message_stored_total").absolute(0);
    metrics::counter!("beast.run.raw_message_store_failed_total").absolute(0);
    metrics::counter!("beast.run.adsb_to_fix_failed_total").absolute(0);
    metrics::counter!("beast.run.fixes_processed_total").absolute(0);
    metrics::counter!("beast.run.fix_processing_failed_total").absolute(0);
    metrics::counter!("beast.run.no_fix_created_total").absolute(0);
    metrics::counter!("beast.run.intake.processed_total").absolute(0);
    metrics::counter!("beast.run.nats.consumed_total").absolute(0);
    metrics::counter!("beast.run.nats.connection_failed_total").absolute(0);
    metrics::counter!("beast.run.nats.subscription_failed_total").absolute(0);
    metrics::counter!("beast.run.nats.subscription_ended_total").absolute(0);
    metrics::gauge!("beast.run.nats.lag_seconds").set(0.0);
    metrics::gauge!("beast.run.nats.intake_queue_depth").set(0.0);
    metrics::histogram!("beast.run.message_processing_latency_ms").record(0.0);

    // Queue depth metrics (pipeline order)
    metrics::gauge!("aprs.router_queue.depth").set(0.0);
    metrics::gauge!("aprs.aircraft_queue.depth").set(0.0);
    metrics::gauge!("aprs.receiver_status_queue.depth").set(0.0);
    metrics::gauge!("aprs.receiver_position_queue.depth").set(0.0);
    metrics::gauge!("aprs.server_status_queue.depth").set(0.0);

    // Worker activity gauges (tracks how many workers are actively processing)
    metrics::gauge!("worker.active", "type" => "intake").set(0.0);
    metrics::gauge!("worker.active", "type" => "router").set(0.0);
    metrics::gauge!("worker.active", "type" => "aircraft").set(0.0);
    metrics::gauge!("worker.active", "type" => "receiver_status").set(0.0);
    metrics::gauge!("worker.active", "type" => "receiver_position").set(0.0);
    metrics::gauge!("worker.active", "type" => "server_status").set(0.0);
    metrics::gauge!("worker.active", "type" => "nats_publisher").set(0.0);

    // Processing rate counters (one per worker type)
    metrics::counter!("aprs.intake.processed_total").absolute(0);
    metrics::counter!("aprs.router.processed_total").absolute(0);
    metrics::counter!("aprs.aircraft.processed_total").absolute(0);
    metrics::counter!("aprs.receiver_status.processed_total").absolute(0);
    metrics::counter!("aprs.receiver_position.processed_total").absolute(0);
    metrics::counter!("aprs.server_status.processed_total").absolute(0);

    // Aircraft worker stage tracking (for debugging pipeline jams)
    metrics::counter!("aprs.aircraft.stage_total", "stage" => "entered").absolute(0);
    metrics::counter!("aprs.aircraft.stage_total", "stage" => "fix_created").absolute(0);
    metrics::counter!("aprs.aircraft.stage_total", "stage" => "before_db").absolute(0);
    metrics::counter!("aprs.aircraft.stage_total", "stage" => "after_db").absolute(0);
    metrics::counter!("aprs.aircraft.stage_total", "stage" => "before_flight").absolute(0);
    metrics::counter!("aprs.aircraft.stage_total", "stage" => "after_flight").absolute(0);
    metrics::counter!("aprs.aircraft.stage_total", "stage" => "before_nats").absolute(0);
    metrics::counter!("aprs.aircraft.stage_total", "stage" => "completed").absolute(0);

    // Queue send blocked counters (tracks when send_async has to wait for space)
    metrics::counter!("queue.send_blocked_total", "queue" => "router").absolute(0);
    metrics::counter!("queue.send_blocked_total", "queue" => "aircraft").absolute(0);
    metrics::counter!("queue.send_blocked_total", "queue" => "nats_publisher").absolute(0);
}

/// Initialize analytics metrics to zero/default values
/// This ensures metrics always appear in Prometheus queries even if no events have occurred
pub fn initialize_analytics_metrics() {
    // Analytics cache hit/miss metrics
    metrics::counter!("analytics.cache.hit_total").absolute(0);
    metrics::counter!("analytics.cache.miss_total").absolute(0);
    metrics::gauge!("analytics.cache.size").set(0.0);

    // Analytics query duration metrics (in milliseconds)
    metrics::histogram!("analytics.query.daily_flights_ms").record(0.0);
    metrics::histogram!("analytics.query.hourly_flights_ms").record(0.0);
    metrics::histogram!("analytics.query.duration_distribution_ms").record(0.0);
    metrics::histogram!("analytics.query.aircraft_outliers_ms").record(0.0);
    metrics::histogram!("analytics.query.top_aircraft_ms").record(0.0);
    metrics::histogram!("analytics.query.club_analytics_ms").record(0.0);
    metrics::histogram!("analytics.query.airport_activity_ms").record(0.0);
    metrics::histogram!("analytics.query.summary_ms").record(0.0);

    // Analytics API endpoint request counters
    metrics::counter!("analytics.api.daily_flights.requests_total").absolute(0);
    metrics::counter!("analytics.api.hourly_flights.requests_total").absolute(0);
    metrics::counter!("analytics.api.duration_distribution.requests_total").absolute(0);
    metrics::counter!("analytics.api.aircraft_outliers.requests_total").absolute(0);
    metrics::counter!("analytics.api.top_aircraft.requests_total").absolute(0);
    metrics::counter!("analytics.api.club_analytics.requests_total").absolute(0);
    metrics::counter!("analytics.api.airport_activity.requests_total").absolute(0);
    metrics::counter!("analytics.api.summary.requests_total").absolute(0);

    // Analytics API error counters
    metrics::counter!("analytics.api.errors_total").absolute(0);

    // Analytics data metrics (updated periodically by background task)
    metrics::gauge!("analytics.flights.today").set(0.0);
    metrics::gauge!("analytics.flights.last_7d").set(0.0);
    metrics::gauge!("analytics.flights.last_30d").set(0.0);
    metrics::gauge!("analytics.aircraft.active_7d").set(0.0);
    metrics::gauge!("analytics.aircraft.outliers").set(0.0);
}

/// Initialize airspace-related metrics
pub fn initialize_airspace_metrics() {
    // Airspace sync metrics (for pull-airspaces command)
    metrics::counter!("airspace_sync.total_fetched_total").absolute(0);
    metrics::counter!("airspace_sync.total_inserted_total").absolute(0);
    metrics::gauge!("airspace_sync.last_run_timestamp").set(0.0);
    metrics::gauge!("airspace_sync.success").set(0.0);

    // Airspace API endpoint metrics
    metrics::counter!("api.airspaces.requests_total").absolute(0);
    metrics::counter!("api.airspaces.errors_total").absolute(0);
    metrics::histogram!("api.airspaces.duration_ms").record(0.0);
    metrics::gauge!("api.airspaces.results_count").set(0.0);
}

pub fn initialize_coverage_metrics() {
    // Coverage API metrics
    metrics::counter!("coverage.api.hexes.requests_total").absolute(0);
    metrics::counter!("coverage.api.hexes.success_total").absolute(0);
    metrics::counter!("coverage.api.errors_total").absolute(0);
    metrics::histogram!("coverage.api.hexes.count").record(0.0);
    metrics::histogram!("coverage.query.hexes_ms").record(0.0);
    metrics::counter!("coverage.cache.hit_total").absolute(0);
    metrics::counter!("coverage.cache.miss_total").absolute(0);
}

/// Initialize build info metric with version details as labels
/// This exports a gauge with value 1 and labels containing version, commit, target, etc.
/// Follows the standard "info" metric pattern used by Prometheus exporters.
pub fn initialize_build_info() {
    // Get binary modification time if available
    let binary_modified = get_binary_modified_time().unwrap_or_else(|| "unknown".to_string());

    metrics::gauge!(
        "build_info",
        "version" => env!("VERGEN_GIT_DESCRIBE"),
        "commit" => env!("VERGEN_GIT_SHA"),
        "build_timestamp" => env!("VERGEN_BUILD_TIMESTAMP"),
        "target" => env!("VERGEN_CARGO_TARGET_TRIPLE"),
        "binary_modified" => leak_string(binary_modified),
    )
    .set(1.0);
}

/// Get binary modification time as ISO 8601 string
fn get_binary_modified_time() -> Option<String> {
    let exe_path = std::env::current_exe().ok()?;
    let metadata = std::fs::metadata(&exe_path).ok()?;
    let modified = metadata.modified().ok()?;

    let duration = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
    let datetime =
        chrono::DateTime::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
            .map(|dt| dt.to_rfc3339())?;

    Some(datetime)
}

/// Leak a string to get a &'static str (needed for metric labels)
/// This is safe for build info since it's only called once at startup
fn leak_string(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// Health check handler for APRS ingestion service
/// Returns 200 OK if service is healthy and ready to receive traffic
/// Returns 503 Service Unavailable if not ready
async fn health_check_handler() -> impl IntoResponse {
    if let Some(health) = get_aprs_health() {
        let health_state = health.read().await;

        // Service is healthy if both APRS and socket are connected
        let is_healthy = health_state.aprs_connected && health_state.socket_connected;

        // Check if we've received messages recently (within last 60 seconds)
        let has_recent_messages = health_state
            .last_message_time
            .map(|t| t.elapsed() < Duration::from_secs(60))
            .unwrap_or(false);

        if is_healthy {
            let status_msg = if has_recent_messages {
                "healthy - receiving messages".to_string()
            } else {
                "healthy - connected but no recent messages".to_string()
            };

            info!("Health check: {}", status_msg);
            (StatusCode::OK, status_msg)
        } else {
            let status_msg = format!(
                "unhealthy - aprs_connected: {}, socket_connected: {}",
                health_state.aprs_connected, health_state.socket_connected
            );
            warn!("Health check failed: {}", status_msg);
            (StatusCode::SERVICE_UNAVAILABLE, status_msg)
        }
    } else {
        // Health state not initialized (shouldn't happen in production)
        warn!("Health check failed: health state not initialized");
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "health state not initialized".to_string(),
        )
    }
}

/// Readiness check handler for APRS ingestion service
/// Similar to health check but more strict - requires recent messages
async fn readiness_check_handler() -> impl IntoResponse {
    if let Some(health) = get_aprs_health() {
        let health_state = health.read().await;

        // Service is ready if connected AND has received messages in last 30 seconds
        let has_recent_messages = health_state
            .last_message_time
            .map(|t| t.elapsed() < Duration::from_secs(30))
            .unwrap_or(false);

        let is_ready =
            health_state.aprs_connected && health_state.socket_connected && has_recent_messages;

        if is_ready {
            info!("Readiness check: ready");
            (StatusCode::OK, "ready".to_string())
        } else {
            let status_msg = format!(
                "not ready - aprs: {}, socket: {}, recent_messages: {}",
                health_state.aprs_connected, health_state.socket_connected, has_recent_messages
            );
            warn!("Readiness check failed: {}", status_msg);
            (StatusCode::SERVICE_UNAVAILABLE, status_msg)
        }
    } else {
        warn!("Readiness check failed: health state not initialized");
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "health state not initialized".to_string(),
        )
    }
}

/// Start a standalone metrics server on the specified port
/// This is used by the "run" subcommand to expose metrics independently
/// The component parameter is used to add a global label to all metrics (e.g., "ingest", "web", "run")
pub async fn start_metrics_server(port: u16, component: Option<&str>) {
    let handle = init_metrics(component);
    METRICS_HANDLE
        .set(handle)
        .expect("Metrics handle already initialized");

    // Initialize build info metric with version labels
    initialize_build_info();

    // Start process metrics background task
    tokio::spawn(process_metrics_task());

    let app = Router::new()
        .route(
            "/metrics",
            get(|| async {
                let handle = METRICS_HANDLE
                    .get()
                    .expect("Metrics handle not initialized");
                handle.render()
            }),
        )
        .route("/health", get(health_check_handler))
        .route("/ready", get(readiness_check_handler))
        .route("/debug/pprof/profile", get(profile_handler))
        .route("/debug/pprof/heap", get(heap_profile_handler));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    info!("Starting metrics server on http://{}/metrics", addr);
    if let Some(comp) = component {
        info!("Metrics will be labeled with component={}", comp);
    }
    info!("Health check available at http://{}/health", addr);
    info!("Readiness check available at http://{}/ready", addr);
    info!(
        "CPU profiling available at http://{}/debug/pprof/profile (30s flamegraph)",
        addr
    );
    info!(
        "Heap profiling available at http://{}/debug/pprof/heap (10s pprof)",
        addr
    );

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind metrics server");

    axum::serve(listener, app)
        .await
        .expect("Metrics server failed");
}
