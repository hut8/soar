use crate::receiver_repo::ReceiverRepository;
use crate::receiver_status_repo::ReceiverStatusRepository;
use crate::receiver_statuses::NewReceiverStatus;
use metrics::counter;
use ogn_parser::{AprsData, AprsPacket};
use tracing::{debug, error, info, trace, warn};

/// Processor for handling receiver status packets
pub struct ReceiverStatusProcessor {
    /// Repository for storing receiver status data
    status_repo: ReceiverStatusRepository,
    /// Repository for looking up receiver IDs by callsign
    receiver_repo: ReceiverRepository,
}

impl ReceiverStatusProcessor {
    /// Create a new ReceiverStatusProcessor
    pub fn new(status_repo: ReceiverStatusRepository, receiver_repo: ReceiverRepository) -> Self {
        Self {
            status_repo,
            receiver_repo,
        }
    }

    /// Process a status packet from a receiver
    pub fn process_status_packet(&self, packet: &AprsPacket) {
        let source_type = packet.position_source_type();
        let callsign = packet.from.to_string();

        trace!(
            "ReceiverStatusProcessor: Processing status packet from {} (source_type: {:?})",
            callsign, source_type
        );

        // Process all status packets, not just those identified as receivers
        // The receiver lookup will determine if it's actually a receiver
        if let AprsData::Status(status) = &packet.data {
            let status_comment = status.comment.clone();
            let received_at = chrono::Utc::now();
            let raw_data = packet.raw.clone().unwrap_or_default();

            // Look up receiver ID asynchronously
            tokio::spawn({
                let receiver_repo = self.receiver_repo.clone();
                let status_repo = self.status_repo.clone();
                let raw_data = raw_data.clone();

                async move {
                    // Look up receiver by callsign
                    match receiver_repo.get_receiver_by_callsign(&callsign).await {
                        Ok(Some(receiver)) => {
                            // Create NewReceiverStatus from status comment
                            let new_status = NewReceiverStatus::from_status_comment(
                                receiver.id,
                                &status_comment,
                                received_at, // packet timestamp
                                received_at, // received_at
                                raw_data.clone(),
                            );

                            // Insert receiver status
                            match status_repo.insert(&new_status).await {
                                Ok(_) => {
                                    // Track receiver status update metric
                                    counter!("receiver_status_updates_total").increment(1);

                                    // Update receiver's latest_packet_at
                                    if let Err(e) =
                                        receiver_repo.update_latest_packet_at(receiver.id).await
                                    {
                                        error!(
                                            "Failed to update latest_packet_at for receiver {}: {}",
                                            callsign, e
                                        );
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to insert receiver status for {}: {}",
                                        callsign, e
                                    );
                                }
                            }
                        }
                        Ok(None) => {
                            debug!(
                                "Status packet from {} is not a known receiver, auto-inserting",
                                callsign
                            );

                            // Auto-insert minimal receiver
                            match receiver_repo.insert_minimal_receiver(&callsign).await {
                                Ok(receiver_id) => {
                                    info!(
                                        "Auto-inserted receiver {} (id: {}), now inserting status",
                                        callsign, receiver_id
                                    );

                                    // Create NewReceiverStatus from status comment
                                    let new_status = NewReceiverStatus::from_status_comment(
                                        receiver_id,
                                        &status_comment,
                                        received_at, // packet timestamp
                                        received_at, // received_at
                                        raw_data.clone(),
                                    );

                                    // Insert receiver status
                                    match status_repo.insert(&new_status).await {
                                        Ok(_) => {
                                            // Track receiver status update metric
                                            counter!("receiver_status_updates_total").increment(1);

                                            // Update receiver's latest_packet_at
                                            if let Err(e) = receiver_repo
                                                .update_latest_packet_at(receiver_id)
                                                .await
                                            {
                                                error!(
                                                    "Failed to update latest_packet_at for receiver {}: {}",
                                                    callsign, e
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            error!(
                                                "Failed to insert receiver status for auto-discovered receiver {}: {}",
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
                            error!("Failed to look up receiver by callsign {}: {}", callsign, e);
                        }
                    }
                }
            });
        } else {
            warn!(
                "Expected status packet but got different packet type from {}",
                callsign
            );
        }
    }
}
