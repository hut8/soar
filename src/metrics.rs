use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

/// Initialize Prometheus metrics exporter
/// Returns a handle that can be used to render metrics for scraping
pub fn init_metrics() -> PrometheusHandle {
    PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install Prometheus recorder")
}
