use super::generic::{GenericProcessor, PacketContext};
use ogn_parser::{AprsData, AprsPacket, PositionSourceType};
use tokio::sync::mpsc;
use tracing::{debug, trace, warn};

/// PacketRouter routes packets to appropriate specialized processor queues
/// This is the main router that the AprsClient calls directly (no queue between them)
#[derive(Clone)]
pub struct PacketRouter {
    /// Generic processor for archiving, receiver identification, and APRS message insertion (required, runs inline)
    generic_processor: GenericProcessor,
    /// Optional channel sender for aircraft position packets
    aircraft_position_tx: Option<mpsc::Sender<(AprsPacket, PacketContext)>>,
    /// Optional channel sender for receiver status packets
    receiver_status_tx: Option<mpsc::Sender<(AprsPacket, PacketContext)>>,
    /// Optional channel sender for receiver position packets
    receiver_position_tx: Option<mpsc::Sender<(AprsPacket, PacketContext)>>,
    /// Optional channel sender for server status messages
    server_status_tx: Option<mpsc::Sender<String>>,
}

impl PacketRouter {
    /// Create a new PacketRouter with a generic processor (required)
    pub fn new(generic_processor: GenericProcessor) -> Self {
        Self {
            generic_processor,
            aircraft_position_tx: None,
            receiver_status_tx: None,
            receiver_position_tx: None,
            server_status_tx: None,
        }
    }

    /// Add aircraft position queue sender
    pub fn with_aircraft_position_queue(
        mut self,
        sender: mpsc::Sender<(AprsPacket, PacketContext)>,
    ) -> Self {
        self.aircraft_position_tx = Some(sender);
        self
    }

    /// Add receiver status queue sender
    pub fn with_receiver_status_queue(
        mut self,
        sender: mpsc::Sender<(AprsPacket, PacketContext)>,
    ) -> Self {
        self.receiver_status_tx = Some(sender);
        self
    }

    /// Add receiver position queue sender
    pub fn with_receiver_position_queue(
        mut self,
        sender: mpsc::Sender<(AprsPacket, PacketContext)>,
    ) -> Self {
        self.receiver_position_tx = Some(sender);
        self
    }

    /// Add server status queue sender
    pub fn with_server_status_queue(mut self, sender: mpsc::Sender<String>) -> Self {
        self.server_status_tx = Some(sender);
        self
    }
}

impl PacketRouter {
    /// Process a server message (line starting with #)
    /// 1. GenericProcessor archives it
    /// 2. Route to server status queue (or drop if full)
    pub fn process_server_message(&self, raw_message: &str) {
        // Step 1: Archive via GenericProcessor
        self.generic_processor.process_server_message(raw_message);

        // Step 2: Route to server status queue if configured
        if let Some(tx) = &self.server_status_tx {
            match tx.try_send(raw_message.to_string()) {
                Ok(()) => {
                    trace!("Routed server message to queue");
                }
                Err(mpsc::error::TrySendError::Full(_)) => {
                    warn!("Server status queue FULL - dropping server message");
                    metrics::counter!("aprs.server_status_queue.full").increment(1);
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    warn!("Server status queue CLOSED - cannot route server message");
                    metrics::counter!("aprs.server_status_queue.closed").increment(1);
                }
            }
        } else {
            trace!("No server status queue configured, server message archived only");
        }
    }

    /// Process an APRS packet through the complete pipeline
    /// 1. GenericProcessor archives and inserts to database (inline)
    /// 2. Route to appropriate queue based on packet type (or drop if full)
    pub async fn process_packet(&self, packet: AprsPacket, raw_message: &str) {
        // Step 1: Generic processing (inline) - archives, identifies receiver, inserts APRS message
        let context = match self
            .generic_processor
            .process_packet(&packet, raw_message)
            .await
        {
            Some(ctx) => ctx,
            None => {
                warn!(
                    "Generic processing failed for packet from {}, skipping routing",
                    packet.from
                );
                return;
            }
        };

        // Step 2: Route to appropriate queue based on packet type
        // Capture packet.from before moving packet (needed for error messages)
        let packet_from = packet.from.clone();
        let position_source = packet.position_source_type();

        match &packet.data {
            AprsData::Position(_) => {
                // Route based on position source type
                match position_source {
                    PositionSourceType::Aircraft => {
                        // Route to aircraft position queue
                        if let Some(tx) = &self.aircraft_position_tx {
                            match tx.try_send((packet, context)) {
                                Ok(()) => {
                                    trace!("Routed aircraft position to queue");
                                }
                                Err(mpsc::error::TrySendError::Full(_)) => {
                                    warn!(
                                        "Aircraft position queue FULL - dropping packet from {}",
                                        packet_from
                                    );
                                    metrics::counter!("aprs.aircraft_queue.full").increment(1);
                                }
                                Err(mpsc::error::TrySendError::Closed(_)) => {
                                    warn!("Aircraft position queue CLOSED - cannot route packet");
                                    metrics::counter!("aprs.aircraft_queue.closed").increment(1);
                                }
                            }
                        } else {
                            trace!("No aircraft position queue configured, packet archived only");
                        }
                    }
                    PositionSourceType::Receiver => {
                        // Route to receiver position queue
                        if let Some(tx) = &self.receiver_position_tx {
                            match tx.try_send((packet, context)) {
                                Ok(()) => {
                                    trace!("Routed receiver position to queue");
                                }
                                Err(mpsc::error::TrySendError::Full(_)) => {
                                    warn!(
                                        "Receiver position queue FULL - dropping packet from {}",
                                        packet_from
                                    );
                                    metrics::counter!("aprs.receiver_position_queue.full")
                                        .increment(1);
                                }
                                Err(mpsc::error::TrySendError::Closed(_)) => {
                                    warn!("Receiver position queue CLOSED - cannot route packet");
                                    metrics::counter!("aprs.receiver_position_queue.closed")
                                        .increment(1);
                                }
                            }
                        } else {
                            trace!("No receiver position queue configured, packet archived only");
                        }
                    }
                    PositionSourceType::WeatherStation => {
                        trace!(
                            "Position from weather station {} - archived only",
                            packet_from
                        );
                    }
                    source_type => {
                        trace!(
                            "Position from unknown source type {:?} from {} - archived only",
                            source_type, packet_from
                        );
                    }
                }
            }
            AprsData::Status(_) => {
                // Route to receiver status queue
                trace!(
                    "Received status packet from {} (source type: {:?})",
                    packet_from, position_source
                );
                if let Some(tx) = &self.receiver_status_tx {
                    match tx.try_send((packet, context)) {
                        Ok(()) => {
                            trace!("Routed receiver status to queue");
                        }
                        Err(mpsc::error::TrySendError::Full(_)) => {
                            warn!(
                                "Receiver status queue FULL - dropping packet from {}",
                                packet_from
                            );
                            metrics::counter!("aprs.receiver_status_queue.full").increment(1);
                        }
                        Err(mpsc::error::TrySendError::Closed(_)) => {
                            warn!("Receiver status queue CLOSED - cannot route packet");
                            metrics::counter!("aprs.receiver_status_queue.closed").increment(1);
                        }
                    }
                } else {
                    trace!("No receiver status queue configured, packet archived only");
                }
            }
            _ => {
                debug!(
                    "Received packet of type {:?}, no specific handler - archived only",
                    std::mem::discriminant(&packet.data)
                );
            }
        }
    }
}
