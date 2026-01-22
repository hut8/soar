// Protocol definitions for queue communication
//
// This module contains the generated protobuf code and helper functions
// for serializing/deserializing messages between ingesters and soar-run.

// Include generated protobuf code
#[allow(clippy::all)]
mod generated {
    include!("generated/soar.protocol.rs");
}

// Re-export protocol types
pub use generated::{Envelope, IngestSource};

use anyhow::{Context, Result};
use prost::Message;

/// Serialize an envelope to bytes using protobuf
pub fn serialize_envelope(envelope: &Envelope) -> Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(envelope.encoded_len());
    envelope
        .encode(&mut buf)
        .context("Failed to serialize envelope")?;
    Ok(buf)
}

/// Deserialize an envelope from bytes using protobuf
pub fn deserialize_envelope(data: &[u8]) -> Result<Envelope> {
    Envelope::decode(data).context("Failed to deserialize envelope")
}

/// Create a new envelope from source and data, timestamped NOW
///
/// Use this when creating an envelope at the moment of message receipt.
/// For queued messages, use `new_envelope_with_timestamp` to preserve
/// the original receive time.
pub fn new_envelope(source: IngestSource, data: Vec<u8>) -> Envelope {
    Envelope {
        source: source as i32,
        timestamp_micros: chrono::Utc::now().timestamp_micros(),
        data,
    }
}

/// Create a new envelope with a specific timestamp
///
/// Use this when you have a pre-recorded receive time (e.g., from a queue).
pub fn new_envelope_with_timestamp(
    source: IngestSource,
    data: Vec<u8>,
    timestamp_micros: i64,
) -> Envelope {
    Envelope {
        source: source as i32,
        timestamp_micros,
        data,
    }
}

/// Create and serialize an envelope in one step, timestamped NOW
///
/// This is the preferred method for ingest clients that want to store
/// serialized envelopes in a queue. The timestamp is captured at the
/// moment this function is called, preserving the true receive time.
pub fn create_serialized_envelope(source: IngestSource, data: Vec<u8>) -> Result<Vec<u8>> {
    let envelope = new_envelope(source, data);
    serialize_envelope(&envelope)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize() {
        let envelope = new_envelope(IngestSource::Ogn, b"test message".to_vec());

        let serialized = serialize_envelope(&envelope).unwrap();
        let deserialized = deserialize_envelope(&serialized).unwrap();

        assert_eq!(deserialized.source, IngestSource::Ogn as i32);
        assert_eq!(deserialized.data, b"test message");
        // Timestamp should be close (within a second)
        assert!((deserialized.timestamp_micros - envelope.timestamp_micros).abs() < 1_000_000);
    }

    #[test]
    fn test_ingest_source_variants() {
        assert_eq!(IngestSource::Ogn as i32, 0);
        assert_eq!(IngestSource::Beast as i32, 1);
        assert_eq!(IngestSource::Sbs as i32, 2);
    }
}
