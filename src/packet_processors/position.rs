use super::aircraft::AircraftPositionProcessor;
use super::receiver_position::ReceiverPositionProcessor;
use ogn_parser::{AprsPacket, PositionSourceType};
use tracing::{trace, warn};

/// Processor for handling position packets from various sources
pub struct PositionPacketProcessor {
    /// Aircraft position processor for handling aircraft-specific logic
    aircraft_processor: Option<AircraftPositionProcessor>,
    /// Receiver position processor for handling receiver-specific logic
    receiver_processor: Option<ReceiverPositionProcessor>,
}

impl Default for PositionPacketProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl PositionPacketProcessor {
    /// Create a new PositionPacketProcessor
    pub fn new() -> Self {
        Self {
            aircraft_processor: None,
            receiver_processor: None,
        }
    }

    /// Add an aircraft position processor
    pub fn with_aircraft_processor(mut self, processor: AircraftPositionProcessor) -> Self {
        self.aircraft_processor = Some(processor);
        self
    }

    /// Add a receiver position processor
    pub fn with_receiver_processor(mut self, processor: ReceiverPositionProcessor) -> Self {
        self.receiver_processor = Some(processor);
        self
    }

    /// Process a position packet, routing based on source type
    pub fn process_position_packet(&self, packet: &AprsPacket) {
        match packet.position_source_type() {
            PositionSourceType::Aircraft => {
                if let Some(aircraft_proc) = &self.aircraft_processor {
                    aircraft_proc.process_aircraft_position(packet);
                } else {
                    warn!("No aircraft processor configured, skipping aircraft position");
                }
            }
            PositionSourceType::Receiver => {
                if let Some(receiver_proc) = &self.receiver_processor {
                    receiver_proc.process_receiver_position(packet);
                } else {
                    warn!(
                        "No receiver processor configured, skipping receiver position from {}",
                        packet.from
                    );
                }
            }
            PositionSourceType::WeatherStation => {
                trace!(
                    "Position from weather station {} - logging and ignoring",
                    packet.from
                );
            }
            source_type => {
                trace!(
                    "Position from unknown source type {:?} from {} - ignoring",
                    source_type, packet.from
                );
            }
        }
    }
}
