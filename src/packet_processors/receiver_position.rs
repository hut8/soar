use crate::packet_processors::generic::PacketContext;
use crate::receiver_repo::ReceiverRepository;
use num_traits::AsPrimitive;
use ogn_parser::{AprsData, AprsPacket};
use tracing::{error, trace, warn};

/// Processor for handling receiver position packets
#[derive(Clone)]
pub struct ReceiverPositionProcessor {
    /// Repository for updating receiver locations
    receiver_repo: ReceiverRepository,
}

impl ReceiverPositionProcessor {
    /// Create a new ReceiverPositionProcessor
    pub fn new(receiver_repo: ReceiverRepository) -> Self {
        Self { receiver_repo }
    }

    /// Process a receiver position packet and update its location
    /// Note: Receiver is guaranteed to exist in database (created by GenericProcessor)
    /// Note: No #[instrument] here - this is called for every receiver position packet
    /// and would cause trace accumulation beyond Tempo's 5MB limit
    pub async fn process_receiver_position(&self, packet: &AprsPacket, _context: PacketContext) {
        // Extract position data from packet
        if let AprsData::Position(position) = &packet.data {
            let callsign = packet.from.to_string();
            let latitude = position.latitude.as_();
            let longitude = position.longitude.as_();

            // Update receiver position in database
            match self
                .receiver_repo
                .update_receiver_position(&callsign, latitude, longitude)
                .await
            {
                Ok(updated) => {
                    if updated {
                        trace!(
                            "Successfully updated position for receiver {}: ({}, {})",
                            callsign, latitude, longitude
                        );
                    } else {
                        // This shouldn't happen since GenericProcessor ensures receiver exists
                        warn!(
                            "Receiver {} not found when updating position - this indicates a race condition or database issue",
                            callsign
                        );
                    }
                }
                Err(e) => {
                    error!("Failed to update position for receiver {}: {}", callsign, e);
                }
            }
        } else {
            warn!(
                "Expected position packet from receiver {} but got different packet type",
                packet.from
            );
        }
    }
}
