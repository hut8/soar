//! Packet processors for handling different types of APRS packets
//!
//! This module contains specialized processors for different types of APRS packets:
//! - Aircraft position packets
//! - Receiver position packets
//! - Receiver status packets
//! - Server status messages
//!
//! The `router` module ties everything together with a PacketRouter that dispatches
//! packets to the appropriate processor based on packet type and source.

pub mod aircraft;
pub mod position;
pub mod receiver_position;
pub mod receiver_status;
pub mod router;
pub mod server_status;

// Re-export the main types for convenience
pub use aircraft::AircraftPositionProcessor;
pub use position::PositionPacketProcessor;
pub use receiver_position::ReceiverPositionProcessor;
pub use receiver_status::ReceiverStatusProcessor;
pub use router::PacketRouter;
pub use server_status::ServerStatusProcessor;
