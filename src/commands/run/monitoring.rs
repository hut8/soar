use super::constants::*;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use tracing::info;

/// Spawn queue depth and system metrics reporter.
/// Reports the depth of the envelope queue and DB pool state to Prometheus every 10 seconds.
/// Logs a periodic stats summary every 30 seconds with incoming packet rate, lag, and queue depth.
pub(crate) fn spawn_metrics_reporter(
    metrics_envelope_rx: flume::Receiver<soar::protocol::Envelope>,
    metrics_db_pool: Pool<ConnectionManager<PgConnection>>,
    router_packets_total: Arc<AtomicU64>,
    router_lag_ms: Arc<AtomicI64>,
) {
    tokio::spawn(async move {
        const METRICS_INTERVAL_SECS: u64 = 10;
        const STATS_INTERVAL_SECS: u64 = 30;

        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(METRICS_INTERVAL_SECS));
        interval.tick().await; // First tick completes immediately

        let mut prev_lag_secs: Option<f64> = None;
        let mut ticks_since_stats: u64 = 0;
        let mut last_stats_time = std::time::Instant::now();

        loop {
            interval.tick().await;
            ticks_since_stats += 1;

            // Sample queue depths (lock-free with flume!)
            let envelope_intake_depth = metrics_envelope_rx.len();

            // Get database pool state
            let pool_state = metrics_db_pool.state();
            let active_connections = pool_state.connections - pool_state.idle_connections;

            // Report queue depths to Prometheus
            metrics::gauge!("socket.envelope_intake_queue.depth").set(envelope_intake_depth as f64);

            // Report database pool state to Prometheus
            metrics::gauge!("aprs.db_pool.total_connections").set(pool_state.connections as f64);
            metrics::gauge!("aprs.db_pool.active_connections").set(active_connections as f64);
            metrics::gauge!("aprs.db_pool.idle_connections")
                .set(pool_state.idle_connections as f64);

            // Compute processing rate ratio and catchup ETA from lag derivative.
            let lag_secs = router_lag_ms.load(Ordering::Relaxed) as f64 / 1000.0;
            if let Some(prev) = prev_lag_secs {
                let dt = METRICS_INTERVAL_SECS as f64;
                let dl_dt = (lag_secs - prev) / dt; // negative when catching up

                // R = 1 - dL/dt: if lag decreases by 1s per second, R = 2
                let rate_ratio = 1.0 - dl_dt;
                metrics::gauge!("socket.router.processing_rate_ratio").set(rate_ratio);

                if rate_ratio > 1.0 && lag_secs > 1.0 {
                    let eta_secs = lag_secs / (rate_ratio - 1.0);
                    metrics::gauge!("socket.router.catchup_eta_seconds").set(eta_secs);
                } else if lag_secs <= 1.0 {
                    metrics::gauge!("socket.router.catchup_eta_seconds").set(0.0);
                } else {
                    metrics::gauge!("socket.router.catchup_eta_seconds").set(-1.0);
                }
            }
            prev_lag_secs = Some(lag_secs);

            // Log periodic stats every 30 seconds
            let stats_ticks = STATS_INTERVAL_SECS / METRICS_INTERVAL_SECS;
            if ticks_since_stats >= stats_ticks {
                ticks_since_stats = 0;

                // Get and reset packet counter, measure actual elapsed time
                let packets = router_packets_total.swap(0, Ordering::Relaxed);
                let elapsed = last_stats_time.elapsed().as_secs_f64();
                last_stats_time = std::time::Instant::now();
                let rate = if elapsed > 0.0 {
                    packets as f64 / elapsed
                } else {
                    0.0
                };

                // Get latest lag
                let lag_ms = router_lag_ms.load(Ordering::Relaxed);
                let lag_str = if lag_ms < 1000 {
                    format!("{}ms", lag_ms)
                } else {
                    format!("{:.1}s", lag_ms as f64 / 1000.0)
                };

                info!(
                    "stats: incoming={{rate:{:.1}/s}} lag={} envelope_queue={}/{}",
                    rate, lag_str, envelope_intake_depth, ENVELOPE_INTAKE_QUEUE_SIZE,
                );
            }
        }
    });
}
