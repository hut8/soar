use crate::packet_processors::generic::PacketContext;
use crate::receiver_repo::ReceiverRepository;
use crate::receiver_status_repo::ReceiverStatusRepository;
use crate::receiver_statuses::NewReceiverStatus;
use metrics::counter;
use ogn_parser::{AprsData, AprsPacket};
use tracing::{error, trace, warn};

/// Processor for handling receiver status packets
#[derive(Clone)]
pub struct ReceiverStatusProcessor {
    /// Repository for storing receiver status data
    status_repo: ReceiverStatusRepository,
    /// Repository for updating receiver timestamps
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
    /// Note: Receiver is guaranteed to exist and APRS message already inserted by GenericProcessor
    #[tracing::instrument(skip(self, packet, context), fields(callsign = %packet.from))]
    pub async fn process_status_packet(&self, packet: &AprsPacket, context: PacketContext) {
        let source_type = packet.position_source_type();
        let callsign = packet.from.to_string();

        trace!(
            "ReceiverStatusProcessor: Processing status packet from {} (source_type: {:?})",
            callsign, source_type
        );

        if let AprsData::Status(status) = &packet.data {
            let status_comment = status.comment.clone();
            // Use the received_at timestamp from context (captured at ingestion time)
            let received_at = context.received_at;

            // Create NewReceiverStatus from status comment
            let mut new_status = NewReceiverStatus::from_status_comment(
                context.receiver_id,
                &status_comment,
                received_at, // packet timestamp
                received_at, // received_at
            );

            // Set the raw_message_id from context
            new_status.raw_message_id = Some(context.raw_message_id);

            // Insert receiver status
            match self.status_repo.insert(&new_status).await {
                Ok(_) => {
                    // Track receiver status update metric
                    counter!("receiver_status_updates_total").increment(1);

                    // Update receiver's latest_packet_at
                    if let Err(e) = self
                        .receiver_repo
                        .update_latest_packet_at(context.receiver_id)
                        .await
                    {
                        error!(
                            "Failed to update latest_packet_at for receiver {}: {}",
                            callsign, e
                        );
                    }
                }
                Err(e) => {
                    error!("Failed to insert receiver status for {}: {}", callsign, e);
                }
            }
        } else {
            warn!(
                "Expected status packet but got different packet type from {}",
                callsign
            );
        }
    }
}
