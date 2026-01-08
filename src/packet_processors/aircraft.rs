use crate::fix_processor::FixProcessor;
use crate::flight_tracker::FlightTracker;
use crate::packet_processors::generic::PacketContext;
use ogn_parser::AprsPacket;
use std::sync::Arc;
use tracing::warn;

/// Processor for handling aircraft position packets
#[derive(Clone)]
pub struct AircraftPositionProcessor {
    /// Fix processor for database storage
    fix_processor: Option<FixProcessor>,
    /// Flight detection processor for flight tracking
    flight_detection_processor: Option<Arc<FlightTracker>>,
}

impl Default for AircraftPositionProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl AircraftPositionProcessor {
    /// Create a new AircraftPositionProcessor
    pub fn new() -> Self {
        Self {
            fix_processor: None,
            flight_detection_processor: None,
        }
    }

    /// Add a fix processor for database storage
    pub fn with_fix_processor(mut self, processor: FixProcessor) -> Self {
        self.fix_processor = Some(processor);
        self
    }

    /// Add a flight detection processor for flight tracking
    pub fn with_flight_detection_processor(mut self, processor: Arc<FlightTracker>) -> Self {
        self.flight_detection_processor = Some(processor);
        self
    }

    /// Process an aircraft position packet
    /// Note: Receiver is guaranteed to exist and APRS message already inserted by GenericProcessor
    #[tracing::instrument(skip(self, packet, context), fields(packet_from = %packet.from))]
    pub async fn process_aircraft_position(&self, packet: &AprsPacket, context: PacketContext) {
        let raw_message = packet.raw.clone().unwrap_or_default();

        // Convert to Fix and process with fix processor if available
        if let Some(ref fix_proc) = self.fix_processor {
            fix_proc
                .process_aprs_packet(packet.clone(), &raw_message, context)
                .await;
        } else {
            warn!("No fix processor configured, skipping aircraft position");
        }

        // Note: The flight detection processor is now handled inside FixProcessor
        // so we don't need to call it separately here anymore
    }
}
