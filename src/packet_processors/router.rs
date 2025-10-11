use super::position::PositionPacketProcessor;
use super::receiver_status::ReceiverStatusProcessor;
use super::server_status::ServerStatusProcessor;
use crate::aprs_client::{ArchiveService, PacketHandler};
use anyhow::Result;
use ogn_parser::{AprsData, AprsPacket};
use tracing::{debug, trace, warn};

/// PacketRouter implements PacketProcessor and routes packets to appropriate specialized processors
/// This is the main router that the AprsClient should use
pub struct PacketRouter {
    /// Optional archive service for message archival
    archive_service: Option<ArchiveService>,
    /// Position packet processor for handling position data
    position_processor: Option<PositionPacketProcessor>,
    /// Receiver status processor for handling status data from receivers
    receiver_status_processor: Option<ReceiverStatusProcessor>,
    /// Server status processor for handling server comment messages
    server_status_processor: Option<ServerStatusProcessor>,
}

impl Default for PacketRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl PacketRouter {
    /// Create a new PacketRouter without archival
    pub fn new() -> Self {
        Self {
            archive_service: None,
            position_processor: None,
            receiver_status_processor: None,
            server_status_processor: None,
        }
    }

    /// Create a new PacketRouter with archival enabled
    pub async fn with_archive(base_dir: String) -> Result<Self> {
        let archive_service = ArchiveService::new(base_dir).await?;
        Ok(Self {
            archive_service: Some(archive_service),
            position_processor: None,
            receiver_status_processor: None,
            server_status_processor: None,
        })
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
}

impl PacketHandler for PacketRouter {
    fn process_raw_message(&self, raw_message: &str) {
        // Handle archival if configured
        if let Some(archive) = &self.archive_service {
            archive.archive(raw_message);
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn process_packet(&self, packet: AprsPacket) {
        match &packet.data {
            AprsData::Position(_) => {
                if let Some(pos_proc) = &self.position_processor {
                    pos_proc.process_position_packet(&packet);
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
                    status_proc.process_status_packet(&packet);
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
