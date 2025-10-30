use crate::aprs_client::ArchiveService;
use crate::aprs_messages_repo::{AprsMessagesRepository, NewAprsMessage};
use crate::receiver_repo::ReceiverRepository;
use ogn_parser::{AprsData, AprsPacket};
use tracing::{error, trace, warn};
use uuid::Uuid;

/// Context containing IDs from generic packet processing
/// This is passed to specific packet processors so they don't need to duplicate receiver/message logic
#[derive(Debug, Clone, Copy)]
pub struct PacketContext {
    /// ID of the APRS message record created for this packet
    pub aprs_message_id: Uuid,
    /// ID of the receiver that sent/relayed this packet
    pub receiver_id: Uuid,
}

/// Generic processor that handles archiving, receiver identification, and APRS message insertion
/// This runs before packet-type-specific processing to ensure all packets are properly recorded
#[derive(Clone)]
pub struct GenericProcessor {
    receiver_repo: ReceiverRepository,
    aprs_messages_repo: AprsMessagesRepository,
    archive_service: Option<ArchiveService>,
}

impl GenericProcessor {
    /// Create a new GenericProcessor
    pub fn new(
        receiver_repo: ReceiverRepository,
        aprs_messages_repo: AprsMessagesRepository,
    ) -> Self {
        Self {
            receiver_repo,
            aprs_messages_repo,
            archive_service: None,
        }
    }

    /// Create a new GenericProcessor with archiving enabled
    pub fn with_archive_service(mut self, archive_service: ArchiveService) -> Self {
        self.archive_service = Some(archive_service);
        self
    }

    /// Process a packet generically: archive, identify receiver, insert APRS message, return context
    pub async fn process_packet(
        &self,
        packet: &AprsPacket,
        raw_message: &str,
    ) -> Option<PacketContext> {
        // Step 1: Archive the raw message if archiving is enabled
        if let Some(archive) = &self.archive_service {
            archive.archive(raw_message);
        }

        // Step 2: Identify the receiver callsign
        let receiver_callsign = self.identify_receiver(packet);

        // Step 3: Ensure receiver exists in database (insert if needed)
        let receiver_id = match self
            .receiver_repo
            .insert_minimal_receiver(&receiver_callsign)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                error!(
                    "Failed to insert/lookup receiver {}: {}",
                    receiver_callsign, e
                );
                return None;
            }
        };

        // Step 4: Extract unparsed data from packet
        let unparsed = match &packet.data {
            AprsData::Position(pos) => pos.comment.unparsed.clone(),
            AprsData::Status(status) => status.comment.unparsed.clone(),
            _ => None,
        };

        // Step 5: Insert APRS message
        let aprs_message_id = Uuid::now_v7();
        let new_aprs_message = NewAprsMessage {
            id: aprs_message_id,
            raw_message: raw_message.to_string(),
            received_at: chrono::Utc::now(),
            receiver_id,
            unparsed,
        };

        match self.aprs_messages_repo.insert(new_aprs_message).await {
            Ok(id) => {
                trace!(
                    "Inserted APRS message {} for receiver {}",
                    id, receiver_callsign
                );
                Some(PacketContext {
                    aprs_message_id: id,
                    receiver_id,
                })
            }
            Err(e) => {
                error!(
                    "Failed to insert APRS message for receiver {}: {}",
                    receiver_callsign, e
                );
                None
            }
        }
    }

    /// Identify the receiver callsign from an APRS packet
    ///
    /// Rules:
    /// - If "via" contains "TCPIP*" → use "from" field as callsign
    /// - Otherwise → use last element of "via" as callsign
    /// - If callsign matches GLIDERNx pattern → log error but proceed
    fn identify_receiver(&self, packet: &AprsPacket) -> String {
        // Check if via contains "TCPIP*"
        let has_tcpip = packet
            .via
            .iter()
            .any(|v| v.0.eq_ignore_ascii_case("TCPIP*"));

        let callsign = if has_tcpip {
            // Use "from" field for TCPIP packets
            packet.from.to_string()
        } else {
            // Use last element of "via" for RF packets
            packet
                .via
                .last()
                .map(|v| v.0.to_string())
                .unwrap_or_else(|| {
                    // Fallback to "from" if via is empty (shouldn't happen in practice)
                    warn!(
                        "Packet from {} has empty via field, using from as receiver",
                        packet.from
                    );
                    packet.from.to_string()
                })
        };

        // Check for GLIDERN server callsigns (GLIDERN1-5)
        if callsign.starts_with("GLIDERN")
            && callsign.len() == 8
            && let Some(ch) = callsign.chars().nth(7)
            && ch.is_ascii_digit()
        {
            error!(
                "Receiver callsign {} matches GLIDERN server pattern - this indicates the message came directly from a server, not a receiver station",
                callsign
            );
        }

        callsign
    }

    /// Process a server message (lines starting with #) - archives only, no database insertion
    pub fn process_server_message(&self, raw_message: &str) {
        // Archive the server message if archiving is enabled
        if let Some(archive) = &self.archive_service {
            archive.archive(raw_message);
        }
        // Server messages are just archived, not processed further in GenericProcessor
        // They will be routed to ServerStatusProcessor for database insertion
    }
}
