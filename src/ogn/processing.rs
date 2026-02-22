use chrono::{DateTime, Utc};
use ogn_parser::{AprsData, PositionSourceType};
use tracing::{debug, info, trace};

use crate::fix_processor::FixProcessor;
use crate::ogn::{
    OgnGenericProcessor, ReceiverPositionProcessor, ReceiverStatusProcessor, ServerStatusProcessor,
};

/// Process a raw OGN/APRS message through the full pipeline.
///
/// This is the shared core logic used by both the production OGN intake workers
/// and the test/replay path (`process_messages_from_source`).
///
/// Steps:
/// 1. Server messages (starting with `#`): archive and process server status
/// 2. Parse with `ogn_parser::parse()`
/// 3. Generic processing: archive, receiver upsert, raw message insert
/// 4. Route by packet type: aircraft position, receiver position, receiver status
///
/// Returns `true` if the message was successfully processed (parsed and routed),
/// `false` if parsing failed or generic processing failed.
pub async fn process_ogn_message(
    received_at: DateTime<Utc>,
    message: &str,
    generic_processor: &OgnGenericProcessor,
    fix_processor: &FixProcessor,
    receiver_status_processor: &ReceiverStatusProcessor,
    receiver_position_processor: &ReceiverPositionProcessor,
    server_status_processor: &ServerStatusProcessor,
) -> bool {
    // Server messages (starting with #) don't create PacketContext
    if message.starts_with('#') {
        debug!("Server message: {}", message);
        generic_processor.archive(message).await;
        server_status_processor
            .process_server_message(message, received_at)
            .await;
        return true;
    }

    // Try to parse the message using ogn-parser
    match ogn_parser::parse(message) {
        Ok(parsed) => {
            // Generic processing: archive, identify receiver, insert APRS message
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
                    return false;
                }
            };

            // Route to appropriate processor based on packet type
            let position_source = parsed.position_source_type();
            match &parsed.data {
                AprsData::Position(_) => match position_source {
                    PositionSourceType::Aircraft => {
                        let raw_message = parsed.raw.clone().unwrap_or_default();
                        fix_processor
                            .process_aprs_packet(parsed, &raw_message, context)
                            .await;
                    }
                    PositionSourceType::Receiver => {
                        receiver_position_processor
                            .process_receiver_position(&parsed, context)
                            .await;
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
                },
                AprsData::Status(_) => {
                    receiver_status_processor
                        .process_status_packet(&parsed, context)
                        .await;
                }
                _ => {
                    debug!(
                        "Packet type {:?} from {} - archived only",
                        std::mem::discriminant(&parsed.data),
                        parsed.from
                    );
                }
            }
            true
        }
        Err(e) => {
            let error_str = e.to_string();
            if message.contains("OGNFNT")
                && (error_str.contains("Invalid Latitude")
                    || error_str.contains("Invalid Longitude")
                    || error_str.contains("Unsupported Position Format"))
            {
                trace!("Failed to parse APRS message '{message}': {e}");
            } else {
                info!("Failed to parse APRS message '{message}': {e}");
            }
            false
        }
    }
}
