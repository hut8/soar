use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rs1090::prelude::*;

/// Decoded Beast message with metadata
#[derive(Debug, Clone)]
pub struct DecodedBeastMessage {
    pub timestamp: DateTime<Utc>,
    pub raw_frame: Vec<u8>,
    pub message: Message,
}

/// Decode a Beast binary frame into an ADS-B message
///
/// Beast frames are Mode S messages (7 or 14 bytes) with additional metadata.
/// The frame format from dump1090 is:
/// - 1 byte: frame type (0x1A = Mode S short, 0x1A = Mode S long)
/// - 6 bytes: timestamp (48-bit counter at 12 MHz)
/// - 1 byte: signal level
/// - N bytes: Mode S message (7 or 14 bytes)
///
/// However, we receive just the raw Mode S bytes after the Beast client strips
/// the framing, so we only need to decode the Mode S message itself.
pub fn decode_beast_frame(
    raw_frame: &[u8],
    received_at: DateTime<Utc>,
) -> Result<DecodedBeastMessage> {
    // Validate frame length (Mode S short = 7 bytes, Mode S long = 14 bytes)
    if raw_frame.len() != 7 && raw_frame.len() != 14 {
        return Err(anyhow::anyhow!(
            "Invalid Beast frame length: {} bytes (expected 7 or 14)",
            raw_frame.len()
        ));
    }

    // Decode the Mode S message using rs1090
    let message = Message::try_from(raw_frame).context("Failed to decode Mode S message")?;

    Ok(DecodedBeastMessage {
        timestamp: received_at,
        raw_frame: raw_frame.to_vec(),
        message,
    })
}

/// Convert decoded message to JSON for storage
///
/// This serializes the rs1090 Message structure to JSON which contains
/// all the decoded ADS-B information (positions, velocities, identifications, etc.)
pub fn message_to_json(msg: &Message) -> Result<serde_json::Value> {
    serde_json::to_value(msg).context("Failed to serialize message to JSON")
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn test_decode_valid_adsb_message() {
        // Example ADS-B message from rs1090 tests
        // DF=17 (Extended Squitter), ICAO=4BB463
        let frame = hex!("8d4bb463003d10000000001b5bec");
        let timestamp = Utc::now();

        let result = decode_beast_frame(&frame, timestamp);
        assert!(result.is_ok());

        let decoded = result.unwrap();
        assert_eq!(decoded.raw_frame, frame);
        // Message was decoded successfully
        assert_eq!(decoded.message.crc, 0); // Valid ADS-B should have CRC = 0
    }

    #[test]
    fn test_decode_invalid_length() {
        let frame = hex!("8d4bb463"); // Too short
        let timestamp = Utc::now();

        let result = decode_beast_frame(&frame, timestamp);
        assert!(result.is_err());
    }

    #[test]
    fn test_message_to_json() {
        let frame = hex!("8d4bb463003d10000000001b5bec");
        let timestamp = Utc::now();

        let decoded = decode_beast_frame(&frame, timestamp).unwrap();
        let json_result = message_to_json(&decoded.message);
        assert!(json_result.is_ok());

        let json = json_result.unwrap();
        // The JSON should have a "df" field for downlink format
        assert!(json.get("df").is_some());
    }
}
