use crate::aprs_client::ArchiveService;
use crate::aprs_messages_repo::{AprsMessagesRepository, NewAprsMessage};
use crate::receiver_repo::ReceiverRepository;
use moka::sync::Cache;
use ogn_parser::{AprsData, AprsPacket};
use std::sync::Arc;
use tracing::{debug, error, trace, warn};
use uuid::Uuid;

/// Context containing IDs from generic packet processing
/// This is passed to specific packet processors so they don't need to duplicate receiver/message logic
#[derive(Debug, Clone)]
pub struct PacketContext {
    /// ID of the APRS message record created for this packet
    pub aprs_message_id: Uuid,
    /// ID of the receiver that sent/relayed this packet
    pub receiver_id: Uuid,
    /// Timestamp when the message was received from APRS-IS
    /// This is captured at ingestion time to prevent clock skew from queue processing delays
    pub received_at: chrono::DateTime<chrono::Utc>,
    /// JetStream message handle for ACKing after processing
    /// This is None for non-JetStream sources (e.g., archive replay)
    pub jetstream_msg: Option<Arc<async_nats::jetstream::Message>>,
}

/// Generic processor that handles archiving, receiver identification, and APRS message insertion
/// This runs before packet-type-specific processing to ensure all packets are properly recorded
#[derive(Clone)]
pub struct GenericProcessor {
    receiver_repo: ReceiverRepository,
    aprs_messages_repo: AprsMessagesRepository,
    archive_service: Option<ArchiveService>,
    /// Cache mapping receiver callsign to receiver ID
    /// This avoids repeated database lookups for the same receiver
    receiver_cache: Arc<Cache<String, Uuid>>,
}

impl GenericProcessor {
    /// Create a new GenericProcessor
    pub fn new(
        receiver_repo: ReceiverRepository,
        aprs_messages_repo: AprsMessagesRepository,
    ) -> Self {
        // Create a cache with 10,000 entry capacity and 1 hour TTL
        // This should cover most receivers we see in a typical session
        let receiver_cache = Cache::builder()
            .max_capacity(10_000)
            .time_to_live(std::time::Duration::from_secs(3600))
            .build();

        Self {
            receiver_repo,
            aprs_messages_repo,
            archive_service: None,
            receiver_cache: Arc::new(receiver_cache),
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
        received_at: chrono::DateTime<chrono::Utc>,
        jetstream_msg: Option<Arc<async_nats::jetstream::Message>>,
    ) -> Option<PacketContext> {
        // Step 1: Archive the raw message if archiving is enabled
        if let Some(archive) = &self.archive_service {
            archive.archive(raw_message);
        }

        // Step 2: Identify the receiver callsign
        let receiver_callsign = self.identify_receiver(packet);

        // Step 3: Get receiver ID from cache or database
        let receiver_id = if let Some(cached_id) = self.receiver_cache.get(&receiver_callsign) {
            // Cache hit - use cached receiver ID
            trace!("Receiver {} found in cache", receiver_callsign);
            metrics::counter!("generic_processor.receiver_cache.hit").increment(1);
            cached_id
        } else {
            // Cache miss - lookup/insert in database
            debug!(
                "Receiver {} not in cache, querying database",
                receiver_callsign
            );
            metrics::counter!("generic_processor.receiver_cache.miss").increment(1);

            match self
                .receiver_repo
                .insert_minimal_receiver(&receiver_callsign)
                .await
            {
                Ok(id) => {
                    // Store in cache for future lookups
                    self.receiver_cache.insert(receiver_callsign.clone(), id);
                    debug!("Cached receiver {} with ID {}", receiver_callsign, id);
                    id
                }
                Err(e) => {
                    error!(
                        "Failed to insert/lookup receiver {}: {}",
                        receiver_callsign, e
                    );
                    return None;
                }
            }
        };

        // Step 4: Extract unparsed data from packet
        let unparsed = match &packet.data {
            AprsData::Position(pos) => pos.comment.unparsed.clone(),
            AprsData::Status(status) => status.comment.unparsed.clone(),
            _ => None,
        };

        // Step 5: Insert APRS message
        let new_aprs_message =
            NewAprsMessage::new(raw_message.to_string(), received_at, receiver_id, unparsed);

        let received_at_timestamp = new_aprs_message.received_at;
        match self.aprs_messages_repo.insert(new_aprs_message).await {
            Ok(id) => {
                trace!(
                    "Inserted APRS message {} for receiver {}",
                    id, receiver_callsign
                );
                Some(PacketContext {
                    aprs_message_id: id,
                    receiver_id,
                    received_at: received_at_timestamp,
                    jetstream_msg,
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
