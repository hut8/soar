use super::constants::*;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use soar::packet_processors::PacketRouter;
use tracing::warn;

/// Spawn queue depth and system metrics reporter.
/// Reports the depth of all processing queues and DB pool state to Prometheus every 10 seconds.
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
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        interval.tick().await; // First tick completes immediately

        loop {
            interval.tick().await;

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

            // Warn if queues are building up
            // Envelope intake queue: 80% threshold (critical - first point of backpressure)
            if envelope_intake_depth > (ENVELOPE_INTAKE_QUEUE_SIZE * 80 / 100) {
                let percent = (envelope_intake_depth as f64 / ENVELOPE_INTAKE_QUEUE_SIZE as f64
                    * 100.0) as usize;
                warn!(
                    "Envelope intake queue building up: {}/{} messages ({}% full) - socket reads may slow down",
                    envelope_intake_depth, ENVELOPE_INTAKE_QUEUE_SIZE, percent
                );
            }

            // Internal router queue: 50% threshold
            use soar::packet_processors::router::INTERNAL_QUEUE_CAPACITY;
            if internal_queue_depth > queue_warning_threshold(INTERNAL_QUEUE_CAPACITY) {
                let percent =
                    (internal_queue_depth as f64 / INTERNAL_QUEUE_CAPACITY as f64 * 100.0) as usize;
                warn!(
                    "PacketRouter internal queue building up: {}/{} messages ({}% full)",
                    internal_queue_depth, INTERNAL_QUEUE_CAPACITY, percent
                );
            }

            // Aircraft position queue: 50% threshold
            if aircraft_depth > queue_warning_threshold(AIRCRAFT_QUEUE_SIZE) {
                let percent = (aircraft_depth as f64 / AIRCRAFT_QUEUE_SIZE as f64 * 100.0) as usize;
                warn!(
                    "Aircraft position queue building up: {}/{} messages ({}% full)",
                    aircraft_depth, AIRCRAFT_QUEUE_SIZE, percent
                );
            }
            if receiver_status_depth > queue_warning_threshold(RECEIVER_STATUS_QUEUE_SIZE) {
                let percent = (receiver_status_depth as f64 / RECEIVER_STATUS_QUEUE_SIZE as f64
                    * 100.0) as usize;
                warn!(
                    "Receiver status queue building up: {}/{} messages ({}% full)",
                    receiver_status_depth, RECEIVER_STATUS_QUEUE_SIZE, percent
                );
            }
            if receiver_position_depth > queue_warning_threshold(RECEIVER_POSITION_QUEUE_SIZE) {
                let percent = (receiver_position_depth as f64 / RECEIVER_POSITION_QUEUE_SIZE as f64
                    * 100.0) as usize;
                warn!(
                    "Receiver position queue building up: {}/{} messages ({}% full)",
                    receiver_position_depth, RECEIVER_POSITION_QUEUE_SIZE, percent
                );
            }
        }
    });
}
