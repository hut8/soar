use super::constants::*;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use soar::packet_processors::PacketRouter;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use tracing::info;

/// Spawn queue depth and system metrics reporter.
/// Reports the depth of all processing queues and DB pool state to Prometheus every 10 seconds.
/// Logs a periodic stats summary every 30 seconds with incoming packet rate, lag, and queue depth.
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_metrics_reporter(
    metrics_envelope_rx: flume::Receiver<soar::protocol::Envelope>,
    metrics_packet_router: PacketRouter,
    metrics_aircraft_rx: flume::Receiver<(
        ogn_parser::AprsPacket,
        soar::packet_processors::PacketContext,
    )>,
    metrics_receiver_status_rx: flume::Receiver<(
        ogn_parser::AprsPacket,
        soar::packet_processors::PacketContext,
    )>,
    metrics_receiver_position_rx: flume::Receiver<(
        ogn_parser::AprsPacket,
        soar::packet_processors::PacketContext,
    )>,
    metrics_server_status_rx: flume::Receiver<(String, chrono::DateTime<chrono::Utc>)>,
    metrics_db_pool: Pool<ConnectionManager<PgConnection>>,
    router_packets_total: Arc<AtomicU64>,
    router_lag_ms: Arc<AtomicI64>,
) {
    tokio::spawn(async move {
        const METRICS_INTERVAL_SECS: u64 = 10;
        const STATS_INTERVAL_SECS: f64 = 30.0;
        const HALF_LIFE_SECS: f64 = 15.0 * 60.0; // 15-minute half-life

        // Half-life for the lag derivative EWMA (5 minutes) — shorter than the
        // incoming-rate EWMA so the catchup ETA responds to changes reasonably
        // quickly while still being smooth enough to be useful.
        const LAG_DERIVATIVE_HALF_LIFE_SECS: f64 = 5.0 * 60.0;

        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(METRICS_INTERVAL_SECS));
        interval.tick().await; // First tick completes immediately

        let mut ewma_incoming = soar::metrics::Ewma::new(HALF_LIFE_SECS);
        let mut ewma_lag_derivative = soar::metrics::Ewma::new(LAG_DERIVATIVE_HALF_LIFE_SECS);
        let mut prev_lag_secs: Option<f64> = None;
        let mut ticks_since_stats: u64 = 0;

        loop {
            interval.tick().await;
            ticks_since_stats += 1;

            // Sample queue depths (lock-free with flume!)
            let envelope_intake_depth = metrics_envelope_rx.len();
            let internal_queue_depth = metrics_packet_router.internal_queue_depth();
            let aircraft_depth = metrics_aircraft_rx.len();
            let receiver_status_depth = metrics_receiver_status_rx.len();
            let receiver_position_depth = metrics_receiver_position_rx.len();
            let server_status_depth = metrics_server_status_rx.len();

            // Get database pool state
            let pool_state = metrics_db_pool.state();
            let active_connections = pool_state.connections - pool_state.idle_connections;

            // Report queue depths to Prometheus
            metrics::gauge!("socket.envelope_intake_queue.depth").set(envelope_intake_depth as f64);
            metrics::gauge!("aprs.router_queue.depth").set(internal_queue_depth as f64);
            metrics::gauge!("aprs.aircraft_queue.depth").set(aircraft_depth as f64);
            metrics::gauge!("aprs.receiver_status_queue.depth").set(receiver_status_depth as f64);
            metrics::gauge!("aprs.receiver_position_queue.depth")
                .set(receiver_position_depth as f64);
            metrics::gauge!("aprs.server_status_queue.depth").set(server_status_depth as f64);

            // Report database pool state to Prometheus
            metrics::gauge!("aprs.db_pool.total_connections").set(pool_state.connections as f64);
            metrics::gauge!("aprs.db_pool.active_connections").set(active_connections as f64);
            metrics::gauge!("aprs.db_pool.idle_connections")
                .set(pool_state.idle_connections as f64);

            // Compute processing rate ratio and catchup ETA from lag derivative.
            //
            // The processing rate ratio R measures how many seconds of data we
            // process per wall-clock second:
            //   R = 1 - dL/dt   (where L = lag, dL/dt = rate of change of lag)
            //
            // When R > 1 we're catching up; the net gain is (R - 1) seconds of
            // backlog reduced per wall-clock second.  The ETA to be fully caught
            // up, accounting for new data arriving during the catchup period:
            //   ETA = L / (R - 1)
            //
            // When R <= 1 we're falling behind or treading water: ETA = -1
            // (sentinel for "never").
            let lag_secs = router_lag_ms.load(Ordering::Relaxed) as f64 / 1000.0;
            if let Some(prev) = prev_lag_secs {
                let dt = METRICS_INTERVAL_SECS as f64;
                let dl_dt = (lag_secs - prev) / dt; // negative when catching up
                ewma_lag_derivative.update(dl_dt, dt);

                let smoothed_dl_dt = ewma_lag_derivative.value();
                // R = 1 - dL/dt: if lag decreases by 1s per second, R = 2
                let rate_ratio = 1.0 - smoothed_dl_dt;
                metrics::gauge!("socket.router.processing_rate_ratio").set(rate_ratio);

                if rate_ratio > 1.0 && lag_secs > 1.0 {
                    let eta_secs = lag_secs / (rate_ratio - 1.0);
                    metrics::gauge!("socket.router.catchup_eta_seconds").set(eta_secs);
                } else if lag_secs <= 1.0 {
                    // Already caught up (within 1 second)
                    metrics::gauge!("socket.router.catchup_eta_seconds").set(0.0);
                } else {
                    // Falling behind or treading water
                    metrics::gauge!("socket.router.catchup_eta_seconds").set(-1.0);
                }
            }
            prev_lag_secs = Some(lag_secs);

            // Log periodic stats every 30 seconds (every 3rd tick at 10s interval)
            let stats_ticks = (STATS_INTERVAL_SECS / METRICS_INTERVAL_SECS as f64) as u64;
            if ticks_since_stats >= stats_ticks {
                ticks_since_stats = 0;

                // Get and reset packet counter
                let packets = router_packets_total.swap(0, Ordering::Relaxed);
                let instant_rate = packets as f64 / STATS_INTERVAL_SECS;
                ewma_incoming.update(instant_rate, STATS_INTERVAL_SECS);

                // Get latest lag
                let lag_ms = router_lag_ms.load(Ordering::Relaxed);
                let lag_str = if lag_ms < 1000 {
                    format!("{}ms", lag_ms)
                } else {
                    format!("{:.1}s", lag_ms as f64 / 1000.0)
                };

                info!(
                    "stats: incoming={{rate:{:.1}/s}} lag={} envelope_queue={}/{}",
                    ewma_incoming.value(),
                    lag_str,
                    envelope_intake_depth,
                    ENVELOPE_INTAKE_QUEUE_SIZE,
                );
            }
        }
    });
}
