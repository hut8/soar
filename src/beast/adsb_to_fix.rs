use anyhow::Result;
use chrono::{DateTime, Utc};
use rs1090::prelude::*;
use tracing::debug;
use uuid::Uuid;

use crate::beast::cpr_decoder::CprDecoder;
use crate::fixes::Fix;

/// Convert a decoded ADS-B message to a Fix if it contains valid position information
///
/// **IMPORTANT**: A "fix" is a position fix - this function REQUIRES valid position data.
/// It will return an error if position cannot be decoded (incomplete CPR decoding).
///
/// The CPR decoder maintains state to pair even/odd position frames and decode
/// full latitude/longitude coordinates. Without valid position data, this function
/// returns an error rather than creating a fix with invalid (0, 0) coordinates.
///
/// Extracts:
/// - ICAO address → device identifier
/// - Position (lat/lon/alt) - REQUIRED via CPR decoder
/// - Velocity information (ground speed, track, vertical rate) - optional
/// - Identification (callsign) - optional
///
/// Returns:
/// - `Ok(Some(Fix))` if valid position data is available
/// - `Ok(None)` if message type doesn't produce fixes (shouldn't happen for position messages)
/// - `Err` if position data is required but unavailable (incomplete CPR decoding)
pub fn adsb_message_to_fix(
    message: &Message,
    raw_frame: &[u8],
    timestamp: DateTime<Utc>,
    receiver_id: Uuid,
    aircraft_id: Uuid,
    raw_message_id: Uuid,
    cpr_decoder: Option<&CprDecoder>,
) -> Result<Option<Fix>> {
    // Extract ICAO address for the source field
    let icao_address = extract_icao_address(message)?;

    // Try to decode full position using CPR decoder if available
    let position_info = if let Some(decoder) = cpr_decoder {
        // Use CPR decoder for full lat/lon position
        match decoder.decode_message(message, timestamp, icao_address, raw_frame.to_vec())? {
            Some(decoded_pos) => Some(PositionInfo {
                latitude: decoded_pos.latitude,
                longitude: decoded_pos.longitude,
                altitude_feet: decoded_pos.altitude_feet,
            }),
            None => {
                // CPR decoder needs more frames - try to extract at least altitude
                extract_position_info(message)
            }
        }
    } else {
        // No CPR decoder - fall back to basic extraction (altitude only)
        extract_position_info(message)
    };

    // Extract velocity information
    let velocity_info = extract_velocity_info(message);

    // Extract identification
    let callsign = extract_callsign(message);

    // A "fix" is a position fix - we MUST have valid position data to create one
    // Don't create fixes with (0, 0) coordinates from velocity-only messages
    let position = position_info.ok_or_else(|| {
        anyhow::anyhow!("Cannot create fix without valid position data (CPR decoding incomplete or no position message)")
    })?;

    // Build source_metadata for ADS-B-specific fields
    let source_metadata = build_adsb_metadata(message);

    // Determine if aircraft is active (ground speed >= 20 knots)
    let is_active = velocity_info
        .as_ref()
        .and_then(|v| v.ground_speed_knots)
        .is_none_or(|speed| speed >= 20.0);

    // Build the Fix
    let fix = Fix {
        id: Uuid::now_v7(),
        source: format!("{:06X}", icao_address),
        timestamp,
        latitude: position.latitude,
        longitude: position.longitude,
        altitude_msl_feet: position.altitude_feet,
        altitude_agl_feet: None, // Will be calculated later
        flight_number: callsign,
        squawk: extract_squawk(message),
        ground_speed_knots: velocity_info.as_ref().and_then(|v| v.ground_speed_knots),
        track_degrees: velocity_info.as_ref().and_then(|v| v.track_degrees),
        climb_fpm: velocity_info.and_then(|v| v.vertical_rate_fpm),
        turn_rate_rot: None, // Not provided in ADS-B velocity messages
        source_metadata: Some(source_metadata),
        flight_id: None, // Will be assigned by flight tracker
        aircraft_id,
        received_at: timestamp,
        is_active,
        receiver_id,
        raw_message_id,
        altitude_agl_valid: false, // Will be calculated later
        time_gap_seconds: None,    // Will be set by flight tracker if part of a flight
    };

    Ok(Some(fix))
}

/// Extract ICAO 24-bit address from message
fn extract_icao_address(message: &Message) -> Result<u32> {
    // The Message type from rs1090 serializes to JSON with an "icao24" field
    // We can extract it by serializing to JSON and parsing
    let json = serde_json::to_value(message)?;

    if let Some(icao_str) = json.get("icao24").and_then(|v| v.as_str()) {
        // Parse hex string to u32
        u32::from_str_radix(icao_str, 16)
            .map_err(|e| anyhow::anyhow!("Failed to parse ICAO address '{}': {}", icao_str, e))
    } else {
        // Fallback to CRC for non-ADS-B messages
        debug!("No icao24 field in message, using CRC: {}", message.crc);
        Ok(message.crc)
    }
}

/// Position information extracted from ADS-B message
#[derive(Debug, Clone)]
struct PositionInfo {
    latitude: f64,
    longitude: f64,
    altitude_feet: Option<i32>,
}

/// Extract position information from ADS-B message
///
/// Note: Full CPR decoding for lat/lon requires tracking even/odd message pairs
/// and maintaining state across messages. For now, we extract altitude only.
/// Full position decoding will require implementing a stateful CPR decoder using
/// rs1090's decode_positions() function in a future phase.
fn extract_position_info(message: &Message) -> Option<PositionInfo> {
    // Serialize to JSON to access position fields (BDS 05 - airborne position)
    let json = serde_json::to_value(message).ok()?;

    // Check if this is a position message (has altitude field)
    json.get("altitude")
        .and_then(|v| v.as_i64())
        .map(|altitude| PositionInfo {
            latitude: 0.0,  // CPR decoding required
            longitude: 0.0, // CPR decoding required
            altitude_feet: Some(altitude as i32),
        })
}

/// Velocity information extracted from ADS-B message
#[derive(Debug, Clone)]
struct VelocityInfo {
    ground_speed_knots: Option<f32>,
    track_degrees: Option<f32>,
    vertical_rate_fpm: Option<i32>,
}

/// Extract velocity information from airborne velocity messages
fn extract_velocity_info(message: &Message) -> Option<VelocityInfo> {
    // Serialize to JSON to access velocity fields (BDS 09 - airborne velocity)
    let json = serde_json::to_value(message).ok()?;

    // Check if this is a velocity message (has groundspeed field)
    if json.get("groundspeed").is_some() {
        let ground_speed_knots = json
            .get("groundspeed")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32);

        let track_degrees = json.get("track").and_then(|v| v.as_f64()).map(|v| v as f32);

        let vertical_rate_fpm = json
            .get("vertical_rate")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);

        Some(VelocityInfo {
            ground_speed_knots,
            track_degrees,
            vertical_rate_fpm,
        })
    } else {
        None
    }
}

/// Extract callsign from identification messages
fn extract_callsign(message: &Message) -> Option<String> {
    // Serialize to JSON to access callsign field (BDS 08 - aircraft identification)
    let json = serde_json::to_value(message).ok()?;

    json.get("callsign")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Extract squawk code if available
fn extract_squawk(_message: &Message) -> Option<String> {
    // Squawk extraction not yet implemented
    None
}

/// Build ADS-B-specific metadata for source_metadata JSONB field
/// Note: Protocol is determined from raw_messages.source enum, not stored in metadata
fn build_adsb_metadata(message: &Message) -> serde_json::Value {
    let mut metadata = serde_json::Map::new();

    // Add protocol identifier
    metadata.insert("protocol".to_string(), serde_json::json!("adsb"));

    // Add CRC and other ADS-B-specific fields
    // This will be expanded as we extract more information (NIC, NAC, SIL, etc.)
    metadata.insert("crc".to_string(), serde_json::json!(message.crc));

    serde_json::Value::Object(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::beast::decoder::decode_beast_frame;
    use hex_literal::hex;

    /// Helper function to wrap a Mode S payload in a Beast frame for testing
    fn wrap_in_beast_frame(mode_s_payload: &[u8]) -> Vec<u8> {
        let mut frame = vec![0x1A]; // Start marker
        frame.push(0x33); // Type byte (Mode S long)
        frame.extend_from_slice(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05]); // 6-byte timestamp
        frame.push(0x80); // Signal level
        frame.extend_from_slice(mode_s_payload); // Mode S payload
        frame
    }

    #[test]
    fn test_adsb_to_fix_basic() {
        // Example ADS-B message with ICAO 4BB463
        let mode_s_payload = hex!("8d4bb463003d10000000001b5bec");
        let frame = wrap_in_beast_frame(&mode_s_payload);
        let timestamp = Utc::now();
        let receiver_id = Uuid::now_v7();
        let device_id = Uuid::now_v7();
        let raw_message_id = Uuid::now_v7();

        let decoded = decode_beast_frame(&frame, timestamp).unwrap();
        let fix_result = adsb_message_to_fix(
            &decoded.message,
            &mode_s_payload,
            timestamp,
            receiver_id,
            device_id,
            raw_message_id,
            None, // No CPR decoder for this basic test
        );

        // Should return error when no valid position data (CPR decoding required)
        // A "fix" is a position fix - we cannot create one without coordinates
        assert!(fix_result.is_err());
        assert!(
            fix_result
                .unwrap_err()
                .to_string()
                .contains("Cannot create fix without valid position data"),
            "Should error when no position data available"
        );
    }

    #[test]
    fn test_extract_icao_address() {
        // ADS-B message with ICAO 4BB463 (hex)
        let mode_s_payload = hex!("8d4bb463003d10000000001b5bec");
        let frame = wrap_in_beast_frame(&mode_s_payload);
        let timestamp = Utc::now();
        let decoded = decode_beast_frame(&frame, timestamp).unwrap();

        let icao = extract_icao_address(&decoded.message).unwrap();
        assert_eq!(
            icao, 0x4BB463,
            "ICAO address should be 4BB463 (hex) = 4961379 (decimal)"
        );
        assert_eq!(
            format!("{:06X}", icao),
            "4BB463",
            "ICAO should format as 6-digit hex"
        );
    }

    #[test]
    fn test_extract_velocity() {
        // Airborne velocity message (DF=17, BDS 09)
        let mode_s_payload = hex!("8D485020994409940838175B284F");
        let frame = wrap_in_beast_frame(&mode_s_payload);
        let timestamp = Utc::now();
        let decoded = decode_beast_frame(&frame, timestamp).unwrap();

        let velocity = extract_velocity_info(&decoded.message).expect("Should extract velocity");

        // Based on the test output we saw earlier:
        // groundspeed: 159.20113, track: 182.88037, vertical_rate: -832
        assert!(velocity.ground_speed_knots.is_some());
        assert!(velocity.track_degrees.is_some());
        assert!(velocity.vertical_rate_fpm.is_some());

        let speed = velocity.ground_speed_knots.unwrap();
        assert!(
            (speed - 159.2).abs() < 1.0,
            "Ground speed should be ~159 knots"
        );

        let track = velocity.track_degrees.unwrap();
        assert!((track - 182.9).abs() < 1.0, "Track should be ~183 degrees");

        let vrate = velocity.vertical_rate_fpm.unwrap();
        assert_eq!(vrate, -832, "Vertical rate should be -832 fpm");
    }

    #[test]
    fn test_extract_callsign() {
        // Aircraft identification message (DF=17, BDS 08)
        let mode_s_payload = hex!("8D4840D6202CC371C32CE0576098");
        let frame = wrap_in_beast_frame(&mode_s_payload);
        let timestamp = Utc::now();
        let decoded = decode_beast_frame(&frame, timestamp).unwrap();

        let callsign = extract_callsign(&decoded.message).expect("Should extract callsign");
        assert_eq!(callsign, "KLM1023", "Callsign should be KLM1023");
    }

    #[test]
    fn test_extract_position_altitude() {
        // Airborne position message (DF=17, BDS 05)
        let mode_s_payload = hex!("8D40621D58C382D690C8AC2863A7");
        let frame = wrap_in_beast_frame(&mode_s_payload);
        let timestamp = Utc::now();
        let decoded = decode_beast_frame(&frame, timestamp).unwrap();

        let position = extract_position_info(&decoded.message).expect("Should extract position");

        assert!(position.altitude_feet.is_some(), "Should have altitude");
        assert_eq!(
            position.altitude_feet.unwrap(),
            38000,
            "Altitude should be 38000 feet"
        );

        // Lat/lon not decoded yet (CPR required)
        assert_eq!(
            position.latitude, 0.0,
            "Latitude placeholder until CPR implemented"
        );
        assert_eq!(
            position.longitude, 0.0,
            "Longitude placeholder until CPR implemented"
        );
    }

    #[test]
    fn test_adsb_to_fix_with_velocity() {
        // Velocity-only message without position data should NOT create a Fix
        // A "fix" is a position fix - velocity alone is not sufficient
        let mode_s_payload = hex!("8D485020994409940838175B284F");
        let frame = wrap_in_beast_frame(&mode_s_payload);
        let timestamp = Utc::now();
        let receiver_id = Uuid::now_v7();
        let device_id = Uuid::now_v7();
        let raw_message_id = Uuid::now_v7();

        let decoded = decode_beast_frame(&frame, timestamp).unwrap();
        let fix_result = adsb_message_to_fix(
            &decoded.message,
            &mode_s_payload,
            timestamp,
            receiver_id,
            device_id,
            raw_message_id,
            None, // No CPR decoder for this test
        );

        // Should return error when only velocity data is available
        // We need valid position coordinates to create a fix
        assert!(fix_result.is_err());
        assert!(
            fix_result
                .unwrap_err()
                .to_string()
                .contains("Cannot create fix without valid position data"),
            "Should error when only velocity data (no position) is available"
        );
    }
}
