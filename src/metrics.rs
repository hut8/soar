use axum::{Router, http::StatusCode, response::IntoResponse, routing::get};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use pprof::protos::Message;
use std::net::SocketAddr;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tracing::{info, warn};

static METRICS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

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
        .route("/debug/pprof/profile", get(profile_handler))
        .route("/debug/pprof/heap", get(heap_profile_handler));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Starting metrics server on http://{}/metrics", addr);
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
