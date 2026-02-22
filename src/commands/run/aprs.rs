use chrono::DateTime;
use ogn_parser::{AprsData, PositionSourceType};
use soar::fix_processor::FixProcessor;
use soar::ogn::{
    OgnGenericProcessor, ReceiverPositionProcessor, ReceiverStatusProcessor, ServerStatusProcessor,
};
use tracing::{debug, trace};

/// Process a received APRS message by parsing and processing inline.
///
/// This replaces the old PacketRouter approach: instead of routing through multiple
/// intermediate queues, each worker parses, archives, and processes the message directly.
// Note: Intentionally NOT using #[tracing::instrument] here - it causes trace accumulation
// in Tempo because spawned tasks inherit trace context and all messages end up in one huge trace.
pub(crate) async fn process_aprs_message(
    received_at: DateTime<chrono::Utc>,
    message: &str,
    generic_processor: &OgnGenericProcessor,
    fix_processor: &FixProcessor,
    receiver_status_processor: &ReceiverStatusProcessor,
    receiver_position_processor: &ReceiverPositionProcessor,
    server_status_processor: &ServerStatusProcessor,
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
        generic_processor.archive(message).await;
        server_status_processor
            .process_server_message(message, received_at)
            .await;
        return;
    }

    // Try to parse the message using ogn-parser
    match ogn_parser::parse(message) {
        Ok(parsed) => {
            // Track successful parse
            metrics::counter!("aprs.parse.success_total").increment(1);

            // Step 1: Generic processing - archives, identifies receiver, inserts APRS message
            let context = match generic_processor
                .process_packet(&parsed, message, received_at)
                .await
            {
                Some(ctx) => ctx,
                None => {
                    debug!(
                        "Generic processing failed for packet from {}, skipping",
                        parsed.from
                    );
                    metrics::counter!("aprs.router.generic_processor.failed_total").increment(1);
                    return;
                }
            };

            // Step 2: Route to appropriate processor based on packet type
            let position_source = parsed.position_source_type();

            match &parsed.data {
                AprsData::Position(_) => {
                    match position_source {
                        PositionSourceType::Aircraft => {
                            // Process aircraft position inline
                            let raw_message = parsed.raw.clone().unwrap_or_default();
                            fix_processor
                                .process_aprs_packet(parsed, &raw_message, context)
                                .await;
                            metrics::counter!("aprs.messages.processed.aircraft_total")
                                .increment(1);
                        }
                        PositionSourceType::Receiver => {
                            // Process receiver position inline
                            receiver_position_processor
                                .process_receiver_position(&parsed, context)
                                .await;
                            metrics::counter!("aprs.messages.processed.receiver_position_total")
                                .increment(1);
                        }
                        PositionSourceType::WeatherStation => {
                            trace!(
                                "Position from weather station {} - archived only",
                                parsed.from
                            );
                        }
                        source_type => {
                            trace!(
                                "Position from unknown source type {:?} from {} - archived only",
                                source_type, parsed.from
                            );
                        }
                    }
                }
                AprsData::Status(_) => {
                    // Process receiver status inline
                    receiver_status_processor
                        .process_status_packet(&parsed, context)
                        .await;
                    metrics::counter!("aprs.messages.processed.receiver_status_total").increment(1);
                }
                _ => {
                    debug!(
                        "Received packet of type {:?}, no specific handler - archived only",
                        std::mem::discriminant(&parsed.data)
                    );
                }
            }

            metrics::counter!("aprs.messages.processed.total_total").increment(1);
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
