use crate::receiver_repo::ReceiverRepository;
use num_traits::AsPrimitive;
use ogn_parser::{AprsData, AprsPacket};
use tracing::{debug, error, info, trace, warn};

/// Processor for handling receiver position packets
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
    pub fn process_receiver_position(&self, packet: &AprsPacket) {
        // Extract position data from packet
        if let AprsData::Position(position) = &packet.data {
            let callsign = packet.from.to_string();
            let latitude = position.latitude.as_();
            let longitude = position.longitude.as_();

            // Update receiver position in database (async)
            tokio::spawn({
                let repo = self.receiver_repo.clone();
                let callsign = callsign.clone();
                async move {
                    // First, get the receiver to obtain its ID
                    match repo.get_receiver_by_callsign(&callsign).await {
                        Ok(Some(_receiver)) => {
                            // Update receiver location directly
                            match repo
                                .update_receiver_position(&callsign, latitude, longitude)
                                .await
                            {
                                Ok(_) => {
                                    trace!(
                                        "Successfully updated position for receiver {}: ({}, {})",
                                        callsign, latitude, longitude
                                    );
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to update position for receiver {}: {}",
                                        callsign, e
                                    );
                                }
                            }
                        }
                        Ok(None) => {
                            debug!(
                                "Position packet from {} is not a known receiver, auto-inserting",
                                callsign
                            );

                            // Auto-insert minimal receiver
                            match repo.insert_minimal_receiver(&callsign).await {
                                Ok(_receiver_id) => {
                                    info!("Auto-inserted receiver {}", callsign);

                                    // Update receiver location directly
                                    match repo
                                        .update_receiver_position(&callsign, latitude, longitude)
                                        .await
                                    {
                                        Ok(_) => {
                                            debug!(
                                                "Successfully updated position for auto-inserted receiver {}: ({}, {})",
                                                callsign, latitude, longitude
                                            );
                                        }
                                        Err(e) => {
                                            error!(
                                                "Failed to update position for auto-inserted receiver {}: {}",
                                                callsign, e
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to auto-insert receiver {}: {}", callsign, e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to look up receiver {}: {}", callsign, e);
                        }
                    }
                }
            });
        } else {
            warn!(
                "Expected position packet from receiver {} but got different packet type",
                packet.from
            );
        }
    }
}
