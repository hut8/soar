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
    pub jetstream_connected: bool,
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
    pub jetstream_connected: bool,
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
pub fn init_metrics() -> PrometheusHandle {
    PrometheusBuilder::new()
        // Configure HTTP request duration as histogram with appropriate buckets
        // Buckets: 1ms, 5ms, 10ms, 25ms, 50ms, 100ms, 250ms, 500ms, 1s, 2.5s, 5s, 10s
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Full("http_request_duration_seconds".to_string()),
            &[
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ],
        )
        .expect("failed to set buckets for http_request_duration_seconds")
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
            metrics::gauge!("analytics.devices.active_7d").set(summary.active_devices_7d as f64);
            metrics::gauge!("analytics.devices.outliers").set(summary.outlier_devices_count as f64);
            if let Some(score) = summary.data_quality_score {
                metrics::gauge!("analytics.data_quality_score").set(score);
            }
        }
    }
}

/// Initialize APRS ingest metrics to zero/default values
/// This ensures metrics always appear in Prometheus queries even if no events have occurred
pub fn initialize_aprs_ingest_metrics() {
    // APRS connection metrics
    metrics::counter!("aprs.connection.established").absolute(0);
    metrics::counter!("aprs.connection.ended").absolute(0);
    metrics::counter!("aprs.connection.failed").absolute(0);
    metrics::counter!("aprs.connection.operation_failed").absolute(0);
    metrics::gauge!("aprs.connection.connected").set(0.0);

    // APRS keepalive metrics
    metrics::counter!("aprs.keepalive.sent").absolute(0);

    // APRS raw message metrics
    metrics::counter!("aprs.raw_message.processed").absolute(0);
    metrics::counter!("aprs.raw_message_queue.full").absolute(0);
    metrics::gauge!("aprs.raw_message_queue.depth").set(0.0);

    // Message type tracking (received from APRS-IS)
    metrics::counter!("aprs.raw_message.received.server").absolute(0);
    metrics::counter!("aprs.raw_message.received.aprs").absolute(0);
    metrics::counter!("aprs.raw_message.queued.server").absolute(0);
    metrics::counter!("aprs.raw_message.queued.aprs").absolute(0);

    // JetStream publishing metrics (ingest-aprs publishes to JetStream)
    metrics::counter!("aprs.jetstream.published").absolute(0);
    metrics::counter!("aprs.jetstream.publish_error").absolute(0);
    metrics::counter!("aprs.jetstream.slow_publish").absolute(0);
    metrics::counter!("aprs.jetstream.publish_timeout").absolute(0);
    metrics::gauge!("aprs.jetstream.queue_depth").set(0.0);
    metrics::gauge!("aprs.jetstream.in_flight").set(0.0);
    metrics::histogram!("aprs.jetstream.publish_duration_ms").record(0.0);

    // Connection timeout metric
    metrics::counter!("aprs.connection.timeout").absolute(0);

    // Shutdown metrics
    metrics::counter!("aprs.shutdown.queue_depth_at_shutdown").absolute(0);
    metrics::counter!("aprs.shutdown.messages_flushed").absolute(0);
    metrics::histogram!("aprs.shutdown.flush_duration_seconds").record(0.0);
}

/// Initialize Beast ingest metrics to zero/default values
/// This ensures metrics always appear in Prometheus queries even if no events have occurred
pub fn initialize_beast_ingest_metrics() {
    // Beast connection metrics
    metrics::counter!("beast.connection.established").absolute(0);
    metrics::counter!("beast.connection.failed").absolute(0);
    metrics::counter!("beast.operation.failed").absolute(0);
    metrics::counter!("beast.timeout").absolute(0);
    metrics::gauge!("beast.connection.connected").set(0.0);

    // Beast data metrics
    metrics::counter!("beast.bytes.received").absolute(0);
    metrics::counter!("beast.frames.published").absolute(0);
    metrics::counter!("beast.frames.dropped").absolute(0);
    metrics::gauge!("beast.message_rate").set(0.0);

    // JetStream publishing metrics (ingest-beast publishes to JetStream)
    metrics::counter!("beast.jetstream.published").absolute(0);
    metrics::counter!("beast.jetstream.publish_error").absolute(0);
    metrics::counter!("beast.jetstream.slow_publish").absolute(0);
    metrics::counter!("beast.jetstream.publish_timeout").absolute(0);
    metrics::counter!("beast.jetstream.connection_failed").absolute(0);
    metrics::counter!("beast.jetstream.stream_setup_failed").absolute(0);
    metrics::gauge!("beast.jetstream.queue_depth").set(0.0);
    metrics::gauge!("beast.jetstream.in_flight").set(0.0);
    metrics::histogram!("beast.jetstream.publish_duration_ms").record(0.0);

    // General ingestion metrics
    metrics::counter!("beast.ingest_failed").absolute(0);
}

/// Initialize Beast consumer metrics to zero/default values
/// This ensures metrics always appear in Prometheus queries even if no events have occurred
pub fn initialize_beast_consumer_metrics() {
    // JetStream consumer metrics (consume-beast consumes from JetStream)
    metrics::counter!("beast.jetstream.consumed").absolute(0);
    metrics::counter!("beast.jetstream.receive_error").absolute(0);
    metrics::counter!("beast.jetstream.ack_error").absolute(0);
    metrics::counter!("beast.jetstream.invalid_message").absolute(0);
    metrics::counter!("beast.jetstream.connection_failed").absolute(0);
    metrics::counter!("beast.jetstream.consumer_setup_failed").absolute(0);

    // Beast consumer processing metrics
    metrics::counter!("beast.consumer.received").absolute(0);
    metrics::counter!("beast.consumer.invalid_message").absolute(0);
    metrics::counter!("beast.consumer.send_errors").absolute(0);
    metrics::counter!("beast.consumer.messages_stored").absolute(0);
    metrics::counter!("beast.consumer.write_errors").absolute(0);
    metrics::histogram!("beast.consumer.batch_write_ms").record(0.0);

    // General consumption metrics
    metrics::counter!("beast.consume_failed").absolute(0);
}

/// Initialize SOAR run metrics to zero/default values
/// This ensures metrics always appear in Prometheus queries even if no events have occurred
pub fn initialize_run_metrics() {
    // Flight tracker metrics
    metrics::counter!("flight_tracker_timeouts_detected").absolute(0);
    metrics::gauge!("flight_tracker_active_devices").set(0.0);

    // Receiver status metrics
    metrics::counter!("receiver_status_updates_total").absolute(0);

    // AGL backfill metrics
    metrics::counter!("agl_backfill_altitudes_computed_total").absolute(0);
    metrics::counter!("agl_backfill_fixes_processed_total").absolute(0);
    metrics::counter!("agl_backfill_no_elevation_data_total").absolute(0);
    metrics::counter!("agl_backfill_fetch_errors_total").absolute(0);
    metrics::gauge!("agl_backfill_pending_fixes").set(0.0);

    // Elevation cache metrics
    metrics::counter!("elevation_cache_hits").absolute(0);
    metrics::counter!("elevation_cache_misses").absolute(0);
    metrics::gauge!("elevation_cache_entries").set(0.0);
    metrics::counter!("elevation_tile_cache_hits").absolute(0);
    metrics::counter!("elevation_tile_cache_misses").absolute(0);
    metrics::gauge!("elevation_tile_cache_entries").set(0.0);

    // Receiver cache metrics
    metrics::counter!("generic_processor.receiver_cache.hit").absolute(0);
    metrics::counter!("generic_processor.receiver_cache.miss").absolute(0);

    // NATS publisher metrics
    metrics::counter!("nats_publisher_fixes_published").absolute(0);
    metrics::gauge!("nats_publisher_queue_depth").set(0.0);
    metrics::counter!("nats_publisher_errors").absolute(0);

    // Queue drop/close counters
    metrics::counter!("aprs.raw_message_queue.full").absolute(0);
    metrics::counter!("aprs.aircraft_queue.full").absolute(0);
    metrics::counter!("aprs.aircraft_queue.closed").absolute(0);
    metrics::counter!("aprs.receiver_status_queue.full").absolute(0);
    metrics::counter!("aprs.receiver_status_queue.closed").absolute(0);
    metrics::counter!("aprs.receiver_position_queue.full").absolute(0);
    metrics::counter!("aprs.receiver_position_queue.closed").absolute(0);
    metrics::counter!("aprs.server_status_queue.full").absolute(0);
    metrics::counter!("aprs.server_status_queue.closed").absolute(0);

    // JetStream consumer metrics (soar-run consumes from JetStream, doesn't publish)
    metrics::gauge!("aprs.jetstream.intake_queue_depth").set(0.0);
    metrics::counter!("aprs.jetstream.consumed").absolute(0);
    metrics::counter!("aprs.jetstream.process_error").absolute(0);
    metrics::counter!("aprs.jetstream.decode_error").absolute(0);
    metrics::counter!("aprs.jetstream.ack_error").absolute(0);
    metrics::counter!("aprs.jetstream.receive_error").absolute(0);
    metrics::counter!("aprs.jetstream.acked_immediately").absolute(0);
    metrics::counter!("aprs.jetstream.intake_queue_full").absolute(0);

    // Message processing counters by type
    metrics::counter!("aprs.messages.processed.aircraft").absolute(0);
    metrics::counter!("aprs.messages.processed.receiver_status").absolute(0);
    metrics::counter!("aprs.messages.processed.receiver_position").absolute(0);
    metrics::counter!("aprs.messages.processed.server").absolute(0);
    metrics::counter!("aprs.messages.processed.total").absolute(0);

    // Aircraft position processing latency metrics
    metrics::histogram!("aprs.aircraft.device_lookup_ms").record(0.0);
    metrics::histogram!("aprs.aircraft.fix_creation_ms").record(0.0);
    metrics::histogram!("aprs.aircraft.process_fix_internal_ms").record(0.0);
    metrics::histogram!("aprs.aircraft.total_processing_ms").record(0.0);
    metrics::histogram!("aprs.aircraft.flight_insert_ms").record(0.0);
    metrics::histogram!("aprs.aircraft.callsign_update_ms").record(0.0);
    metrics::histogram!("aprs.aircraft.elevation_queue_ms").record(0.0);
    metrics::histogram!("aprs.aircraft.nats_publish_ms").record(0.0);
}

/// Initialize analytics metrics to zero/default values
/// This ensures metrics always appear in Prometheus queries even if no events have occurred
pub fn initialize_analytics_metrics() {
    // Analytics cache hit/miss metrics
    metrics::counter!("analytics.cache.hit").absolute(0);
    metrics::counter!("analytics.cache.miss").absolute(0);
    metrics::gauge!("analytics.cache.size").set(0.0);

    // Analytics query duration metrics (in milliseconds)
    metrics::histogram!("analytics.query.daily_flights_ms").record(0.0);
    metrics::histogram!("analytics.query.hourly_flights_ms").record(0.0);
    metrics::histogram!("analytics.query.duration_distribution_ms").record(0.0);
    metrics::histogram!("analytics.query.device_outliers_ms").record(0.0);
    metrics::histogram!("analytics.query.top_devices_ms").record(0.0);
    metrics::histogram!("analytics.query.club_analytics_ms").record(0.0);
    metrics::histogram!("analytics.query.airport_activity_ms").record(0.0);
    metrics::histogram!("analytics.query.summary_ms").record(0.0);

    // Analytics API endpoint request counters
    metrics::counter!("analytics.api.daily_flights.requests").absolute(0);
    metrics::counter!("analytics.api.hourly_flights.requests").absolute(0);
    metrics::counter!("analytics.api.duration_distribution.requests").absolute(0);
    metrics::counter!("analytics.api.device_outliers.requests").absolute(0);
    metrics::counter!("analytics.api.top_devices.requests").absolute(0);
    metrics::counter!("analytics.api.club_analytics.requests").absolute(0);
    metrics::counter!("analytics.api.airport_activity.requests").absolute(0);
    metrics::counter!("analytics.api.summary.requests").absolute(0);

    // Analytics API error counters
    metrics::counter!("analytics.api.errors").absolute(0);

    // Analytics data metrics (updated periodically by background task)
    metrics::gauge!("analytics.flights.today").set(0.0);
    metrics::gauge!("analytics.flights.last_7d").set(0.0);
    metrics::gauge!("analytics.flights.last_30d").set(0.0);
    metrics::gauge!("analytics.devices.active_7d").set(0.0);
    metrics::gauge!("analytics.devices.outliers").set(0.0);
    metrics::gauge!("analytics.data_quality_score").set(0.0);
}

/// Health check handler for APRS ingestion service
/// Returns 200 OK if service is healthy and ready to receive traffic
/// Returns 503 Service Unavailable if not ready
async fn health_check_handler() -> impl IntoResponse {
    if let Some(health) = get_aprs_health() {
        let health_state = health.read().await;

        // Service is healthy if both APRS and JetStream are connected
        let is_healthy = health_state.aprs_connected && health_state.jetstream_connected;

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
                "unhealthy - aprs_connected: {}, jetstream_connected: {}",
                health_state.aprs_connected, health_state.jetstream_connected
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
            health_state.aprs_connected && health_state.jetstream_connected && has_recent_messages;

        if is_ready {
            info!("Readiness check: ready");
            (StatusCode::OK, "ready".to_string())
        } else {
            let status_msg = format!(
                "not ready - aprs: {}, jetstream: {}, recent_messages: {}",
                health_state.aprs_connected, health_state.jetstream_connected, has_recent_messages
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
pub async fn start_metrics_server(port: u16) {
    let handle = init_metrics();
    METRICS_HANDLE
        .set(handle)
        .expect("Metrics handle already initialized");

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

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Starting metrics server on http://{}/metrics", addr);
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
