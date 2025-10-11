use crate::receiver_repo::ReceiverRepository;
use num_traits::AsPrimitive;
use ogn_parser::{AprsData, AprsPacket};
use tracing::{debug, error, info, warn};

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

            info!(
                "Processing receiver position for {}: lat={}, lon={}",
                callsign, latitude, longitude
            );

            // Update receiver position in database (async)
            tokio::spawn({
                let repo = self.receiver_repo.clone();
                let callsign = callsign.clone();
                async move {
                    // First, get the receiver to obtain its ID
                    match repo.get_receiver_by_callsign(&callsign).await {
                        Ok(Some(receiver)) => {
                            // Update position
                            match repo
                                .update_receiver_position(&callsign, latitude, longitude)
                                .await
                            {
                                Ok(true) => {
                                    debug!(
                                        "Updated receiver position for {}: ({}, {})",
                                        callsign, latitude, longitude
                                    );

                                    // Update latest_packet_at
                                    if let Err(e) = repo.update_latest_packet_at(receiver.id).await
                                    {
                                        error!(
                                            "Failed to update latest_packet_at for receiver {}: {}",
                                            callsign, e
                                        );
                                    }
                                }
                                Ok(false) => {
                                    warn!(
                                        "Receiver {} not found in database, position not updated",
                                        callsign
                                    );
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to update receiver position for {}: {}",
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
                                Ok(receiver_id) => {
                                    info!(
                                        "Auto-inserted receiver {} (id: {}), now updating position",
                                        callsign, receiver_id
                                    );

                                    // Update position
                                    match repo
                                        .update_receiver_position(&callsign, latitude, longitude)
                                        .await
                                    {
                                        Ok(true) => {
                                            debug!(
                                                "Updated receiver position for {}: ({}, {})",
                                                callsign, latitude, longitude
                                            );

                                            // Update latest_packet_at
                                            if let Err(e) =
                                                repo.update_latest_packet_at(receiver_id).await
                                            {
                                                error!(
                                                    "Failed to update latest_packet_at for receiver {}: {}",
                                                    callsign, e
                                                );
                                            }
                                        }
                                        Ok(false) => {
                                            warn!(
                                                "Receiver {} not found in database after auto-insertion, position not updated",
                                                callsign
                                            );
                                        }
                                        Err(e) => {
                                            error!(
                                                "Failed to update receiver position for auto-discovered receiver {}: {}",
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
