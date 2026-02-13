use chrono::DateTime;
use tracing::info;

/// Spawn graceful shutdown handler that waits for queues to drain on Ctrl+C.
pub(crate) fn spawn_shutdown_handler(
    shutdown_aircraft_rx: flume::Receiver<(
        ogn_parser::AprsPacket,
        soar::packet_processors::PacketContext,
    )>,
    shutdown_receiver_status_rx: flume::Receiver<(
        ogn_parser::AprsPacket,
        soar::packet_processors::PacketContext,
    )>,
    shutdown_receiver_position_rx: flume::Receiver<(
        ogn_parser::AprsPacket,
        soar::packet_processors::PacketContext,
    )>,
    shutdown_server_status_rx: flume::Receiver<(String, DateTime<chrono::Utc>)>,
    shutdown_ogn_intake_opt: Option<flume::Sender<(DateTime<chrono::Utc>, String)>>,
    shutdown_beast_intake_opt: Option<flume::Sender<(DateTime<chrono::Utc>, Vec<u8>)>>,
) {
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("Received shutdown signal (Ctrl+C), initiating graceful shutdown...");
                info!("Socket server will stop accepting connections, allowing queues to drain...");

                // Wait for queues to drain (check every second, max 10 minutes)
                for i in 1..=600 {
                    let intake_depth = shutdown_ogn_intake_opt.as_ref().map_or(0, |tx| tx.len());
                    let beast_intake_depth =
                        shutdown_beast_intake_opt.as_ref().map_or(0, |tx| tx.len());
                    let aircraft_depth = shutdown_aircraft_rx.len();
                    let receiver_status_depth = shutdown_receiver_status_rx.len();
                    let receiver_position_depth = shutdown_receiver_position_rx.len();
                    let server_status_depth = shutdown_server_status_rx.len();

                    let total_queued = intake_depth
                        + beast_intake_depth
                        + aircraft_depth
                        + receiver_status_depth
                        + receiver_position_depth
                        + server_status_depth;

                    if total_queued == 0 {
                        info!("All queues drained, shutting down now");
                        break;
                    }

                    info!(
                        "Waiting for queues to drain ({}/600s): {} intake, {} beast_intake, {} aircraft, {} rx_status, {} rx_pos, {} server",
                        i,
                        intake_depth,
                        beast_intake_depth,
                        aircraft_depth,
                        receiver_status_depth,
                        receiver_position_depth,
                        server_status_depth
                    );

                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }

                info!("Graceful shutdown complete");
                std::process::exit(0);
            }
            Err(err) => {
                eprintln!("Unable to listen for shutdown signal: {}", err);
            }
        }
    });
}
