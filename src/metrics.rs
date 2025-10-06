use axum::{Router, routing::get};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::net::SocketAddr;
use std::sync::OnceLock;
use tracing::info;

static METRICS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Initialize Prometheus metrics exporter
/// Returns a handle that can be used to render metrics for scraping
pub fn init_metrics() -> PrometheusHandle {
    PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install Prometheus recorder")
}

/// Start a standalone metrics server on the specified port
/// This is used by the "run" subcommand to expose metrics independently
pub async fn start_metrics_server(port: u16) {
    let handle = init_metrics();
    METRICS_HANDLE
        .set(handle)
        .expect("Metrics handle already initialized");

    let app = Router::new().route(
        "/metrics",
        get(|| async {
            let handle = METRICS_HANDLE
                .get()
                .expect("Metrics handle not initialized");
            handle.render()
        }),
    );

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Starting metrics server on http://{}/metrics", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind metrics server");

    axum::serve(listener, app)
        .await
        .expect("Metrics server failed");
}
