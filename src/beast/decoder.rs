use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rs1090::prelude::*;
use tracing::debug;

/// Decoded Beast message with metadata
#[derive(Debug, Clone)]
pub struct DecodedBeastMessage {
    pub timestamp: DateTime<Utc>,
    pub raw_frame: Vec<u8>,
    pub message: Message,
    pub message_type: u8, // Beast message type byte
    pub signal_level: u8, // Signal level from Beast frame
}

/// Decode a Beast binary frame into an ADS-B message
///
/// Beast frames are Mode S messages with additional metadata.
/// The frame format from dump1090 is:
/// - 1 byte: 0x1A (frame start marker)
/// - 1 byte: message type
/// - 6 bytes: timestamp (48-bit counter at 12 MHz)
/// - 1 byte: signal level
/// - N bytes: Mode S message (2 for Mode A/C, 7 for short, 14 for long)
///
/// This function strips the Beast framing and extracts the Mode S payload.
pub fn decode_beast_frame(
    raw_frame: &[u8],
    received_at: DateTime<Utc>,
) -> Result<DecodedBeastMessage> {
    // Validate minimum frame length: 1 (0x1A) + 1 (type) + 6 (timestamp) + 1 (signal) + 2 (Mode A/C) = 11 bytes
    if raw_frame.len() < 11 {
        return Err(anyhow::anyhow!(
            "Invalid Beast frame length: {} bytes (expected at least 11)",
            raw_frame.len()
        ));
    }

    // Verify frame starts with 0x1A
    if raw_frame[0] != 0x1A {
        return Err(anyhow::anyhow!(
            "Invalid Beast frame: expected 0x1A start marker, got 0x{:02X}",
            raw_frame[0]
        ));
    }

    // Extract Beast metadata
    let message_type = raw_frame[1];
    // Skip 6-byte timestamp at raw_frame[2..8]
    let signal_level = raw_frame[8];

    // Extract Mode S payload (everything after the 9-byte header)
    let mode_s_payload = &raw_frame[9..];

    // Validate Mode S payload length (2 for Mode A/C, 7 for short, 14 for long)
    let payload_len = mode_s_payload.len();
    if payload_len != 2 && payload_len != 7 && payload_len != 14 {
        return Err(anyhow::anyhow!(
            "Invalid Mode S payload length: {} bytes (expected 2, 7, or 14)",
            payload_len
        ));
    }

    // Decode the Mode S message using rs1090
    let message = Message::try_from(mode_s_payload).context("Failed to decode Mode S message")?;

    // Log the full decoded message for debugging
    debug!("Decoded ADS-B frame: {:?}", message);

    Ok(DecodedBeastMessage {
        timestamp: received_at,
        raw_frame: mode_s_payload.to_vec(),
        message,
        message_type,
        signal_level,
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
        let mode_s_payload = hex!("8d4bb463003d10000000001b5bec");

        // Wrap in Beast framing: 0x1A + type + 6-byte timestamp + signal + payload
        let mut frame = vec![0x1A, 0x33]; // 0x33 = Mode S long (14 bytes)
        frame.extend_from_slice(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05]); // timestamp
        frame.push(0x80); // signal level
        frame.extend_from_slice(&mode_s_payload);

        let timestamp = Utc::now();

        let result = decode_beast_frame(&frame, timestamp);
        assert!(result.is_ok());

        let decoded = result.unwrap();
        assert_eq!(decoded.raw_frame, mode_s_payload);
        assert_eq!(decoded.message_type, 0x33);
        assert_eq!(decoded.signal_level, 0x80);
        // Message was decoded successfully
        assert_eq!(decoded.message.crc, 0); // Valid ADS-B should have CRC = 0
    }

    #[test]
    fn test_decode_invalid_length() {
        let frame = hex!("8d4bb463"); // Too short (no Beast framing)
        let timestamp = Utc::now();

        let result = decode_beast_frame(&frame, timestamp);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_missing_start_marker() {
        // Frame without 0x1A start marker
        let mut frame = vec![0xFF, 0x33]; // Wrong start byte
        frame.extend_from_slice(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05]); // timestamp
        frame.push(0x80); // signal
        frame.extend_from_slice(&hex!("8d4bb463003d10000000001b5bec"));

        let timestamp = Utc::now();
        let result = decode_beast_frame(&frame, timestamp);
        assert!(result.is_err());
    }

    #[test]
    fn test_message_to_json() {
        let mode_s_payload = hex!("8d4bb463003d10000000001b5bec");

        // Wrap in Beast framing
        let mut frame = vec![0x1A, 0x33];
        frame.extend_from_slice(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05]);
        frame.push(0x80);
        frame.extend_from_slice(&mode_s_payload);

        let timestamp = Utc::now();

        let decoded = decode_beast_frame(&frame, timestamp).unwrap();
        let json_result = message_to_json(&decoded.message);
        assert!(json_result.is_ok());

        let json = json_result.unwrap();
        // The JSON should have a "df" field for downlink format
        assert!(json.get("df").is_some());
    }
}
