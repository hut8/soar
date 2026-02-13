use chrono::DateTime;
use tracing::{debug, trace};

/// Process a received APRS message by parsing and routing through PacketRouter
// Note: Intentionally NOT using #[tracing::instrument] here - it causes trace accumulation
// in Tempo because spawned tasks inherit trace context and all messages end up in one huge trace.
pub(crate) async fn process_aprs_message(
    received_at: DateTime<chrono::Utc>,
    message: &str,
    packet_router: &soar::packet_processors::PacketRouter,
) {
    let start_time = std::time::Instant::now();

    // Track that we're processing a message
    metrics::counter!("aprs.process_aprs_message.called_total").increment(1);

    // Calculate and record lag (difference between now and packet timestamp)
    let now = chrono::Utc::now();
    let lag_seconds = (now - received_at).num_milliseconds() as f64 / 1000.0;
    metrics::gauge!("aprs.lag_seconds").set(lag_seconds);

    // Route server messages (starting with #) differently
    // Server messages don't create PacketContext
    if message.starts_with('#') {
        debug!("Server message: {}", message);
        packet_router
            .process_server_message(message, received_at)
            .await;
        return;
    }

    // Try to parse the message using ogn-parser
    match ogn_parser::parse(message) {
        Ok(parsed) => {
            // Track successful parse
            metrics::counter!("aprs.parse.success_total").increment(1);

            // Call PacketRouter to archive, process, and route to queues
            packet_router
                .process_packet(parsed, message, received_at)
                .await;

            metrics::counter!("aprs.router.process_packet.called_total").increment(1);
        }
        Err(e) => {
            metrics::counter!("aprs.parse.failed_total").increment(1);
            // For OGNFNT sources with invalid lat/lon, log as trace instead of error
            // These are common and expected issues with this data source
            let error_str = e.to_string();
            // For OGNFNT sources with common parsing issues, log as debug/trace instead of info
            // These are expected issues with this data source and not actionable
            if message.contains("OGNFNT")
                && (error_str.contains("Invalid Latitude")
                    || error_str.contains("Invalid Longitude")
                    || error_str.contains("Unsupported Position Format"))
            {
                trace!("Failed to parse APRS message '{message}': {e}");
            } else {
                debug!("Failed to parse APRS message '{message}': {e}");
            }
        }
    }

    // Record processing latency
    let elapsed_micros = start_time.elapsed().as_micros() as f64 / 1000.0; // Convert to milliseconds
    metrics::histogram!("aprs.message_processing_latency_ms").record(elapsed_micros);
}
