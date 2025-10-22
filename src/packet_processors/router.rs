use super::generic::GenericProcessor;
use super::position::PositionPacketProcessor;
use super::receiver_status::ReceiverStatusProcessor;
use super::server_status::ServerStatusProcessor;
use crate::aprs_client::ArchiveService;
use anyhow::Result;
use ogn_parser::{AprsData, AprsPacket};
use tracing::{debug, trace, warn};

/// PacketRouter routes packets to appropriate specialized processors
/// This is the main router that the AprsClient should use
pub struct PacketRouter {
    /// Optional archive service for message archival
    archive_service: Option<ArchiveService>,
    /// Generic processor for receiver identification and APRS message insertion (required)
    generic_processor: GenericProcessor,
    /// Position packet processor for handling position data
    position_processor: Option<PositionPacketProcessor>,
    /// Receiver status processor for handling status data from receivers
    receiver_status_processor: Option<ReceiverStatusProcessor>,
    /// Server status processor for handling server comment messages
    server_status_processor: Option<ServerStatusProcessor>,
}

impl PacketRouter {
    /// Create a new PacketRouter with a generic processor (required)
    pub fn new(generic_processor: GenericProcessor) -> Self {
        Self {
            archive_service: None,
            generic_processor,
            position_processor: None,
            receiver_status_processor: None,
            server_status_processor: None,
        }
    }

    /// Create a new PacketRouter with archival enabled
    pub async fn with_archive(
        generic_processor: GenericProcessor,
        base_dir: String,
    ) -> Result<Self> {
        let archive_service = ArchiveService::new(base_dir).await?;
        Ok(Self {
            archive_service: Some(archive_service),
            generic_processor,
            position_processor: None,
            receiver_status_processor: None,
            server_status_processor: None,
        })
    }

    /// Add archive service to the router
    pub fn with_archive_service(mut self, archive_service: ArchiveService) -> Self {
        self.archive_service = Some(archive_service);
        self
    }

    /// Add a position processor to the router
    pub fn with_position_processor(mut self, processor: PositionPacketProcessor) -> Self {
        self.position_processor = Some(processor);
        self
    }

    /// Add a receiver status processor to the router
    pub fn with_receiver_status_processor(mut self, processor: ReceiverStatusProcessor) -> Self {
        self.receiver_status_processor = Some(processor);
        self
    }

    /// Add a server status processor to the router
    pub fn with_server_status_processor(mut self, processor: ServerStatusProcessor) -> Self {
        self.server_status_processor = Some(processor);
        self
    }
}

impl PacketRouter {
    /// Process a server message (line starting with #)
    pub async fn process_server_message(&self, raw_message: &str) {
        if let Some(server_proc) = &self.server_status_processor {
            trace!("PacketRouter processing server message with ServerStatusProcessor");
            server_proc.process_server_message(raw_message).await;
        } else {
            trace!(
                "No server status processor configured, logging server message: {}",
                raw_message
            );
        }
    }

    /// Process an APRS packet through the complete pipeline
    /// 1. Archive (if configured)
    /// 2. Generic processing (identify receiver, insert APRS message)
    /// 3. Route to type-specific processor with context
    pub async fn process_packet(&self, packet: AprsPacket, raw_message: &str) {
        // Step 1: Archive if configured
        if let Some(archive) = &self.archive_service {
            archive.archive(raw_message);
        }

        // Step 2: Generic processing - identify receiver and insert APRS message
        let context = match self
            .generic_processor
            .process_packet(&packet, raw_message)
            .await
        {
            Some(ctx) => ctx,
            None => {
                warn!(
                    "Generic processing failed for packet from {}, skipping type-specific processing",
                    packet.from
                );
                return;
            }
        };

        // Step 3: Route to type-specific processor with context
        match &packet.data {
            AprsData::Position(_) => {
                if let Some(pos_proc) = &self.position_processor {
                    pos_proc.process_position_packet(&packet, context).await;
                } else {
                    trace!("No position processor configured, skipping position packet");
                }
            }
            AprsData::Status(_) => {
                trace!(
                    "Received status packet from {} (source type: {:?})",
                    packet.from,
                    packet.position_source_type()
                );
                if let Some(status_proc) = &self.receiver_status_processor {
                    status_proc.process_status_packet(&packet, context).await;
                } else {
                    warn!(
                        "No receiver status processor configured, skipping status packet from {}",
                        packet.from
                    );
                }
            }
            _ => {
                debug!(
                    "Received packet of type {:?}, no specific handler",
                    std::mem::discriminant(&packet.data)
                );
            }
        }
    }
}
