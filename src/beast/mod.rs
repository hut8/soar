pub mod client;
pub mod decoder;

pub use client::{BeastClient, BeastClientConfig};
pub use decoder::{DecodedBeastMessage, decode_beast_frame};
