//! OGN/APRS packet processing
//!
//! This module contains processors for handling OGN/APRS packets:
//! - Generic processing (receiver identification, APRS message insertion)
//! - Receiver position packets
//! - Receiver status packets
//! - Server status messages

pub mod generic_processor;
pub mod packet_context;
pub mod receiver_position;
pub mod receiver_status;
pub mod server_status;

// Re-export the main types for convenience
pub use generic_processor::OgnGenericProcessor;
pub use packet_context::PacketContext;
pub use receiver_position::ReceiverPositionProcessor;
pub use receiver_status::ReceiverStatusProcessor;
pub use server_status::ServerStatusProcessor;
