use anyhow::Result;
use chrono::{DateTime, Utc};
use rs1090::prelude::*;
use tracing::debug;
use uuid::Uuid;

use crate::fixes::Fix;

/// Convert a decoded ADS-B message to a Fix if it contains position information
///
/// This is a simplified converter that extracts what it can from individual messages.
/// Full position accuracy requires CPR (Compact Position Reporting) decoding which
/// needs to track message pairs and maintain state. That will be implemented in Phase 5.
///
/// For now, this extracts:
/// - ICAO address → device identifier
/// - Velocity information (ground speed, track, vertical rate)
/// - Identification (callsign)
/// - Altitude (from position messages)
///
/// Returns None if the message doesn't contain useful fix information.
pub fn adsb_message_to_fix(
    message: &Message,
    timestamp: DateTime<Utc>,
    receiver_id: Uuid,
    device_id: Uuid,
    raw_message_id: Uuid,
) -> Result<Option<Fix>> {
    // Extract ICAO address for the source field
    let icao_address = extract_icao_address(message)?;

    // Try to extract position information
    // Note: For ADS-B, positions are encoded using CPR which requires
    // pairing even/odd frames. For now, we'll extract what we can.
    let position_info = extract_position_info(message);

    // Extract velocity information
    let velocity_info = extract_velocity_info(message);

    // Extract identification
    let callsign = extract_callsign(message);

    // We need at least position OR velocity to create a fix
    if position_info.is_none() && velocity_info.is_none() {
        return Ok(None);
    }

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
        aprs_type: "ADSB".to_string(), // ADS-B messages don't have APRS-style "to" field
        via: vec![],                   // ADS-B is direct from aircraft
        timestamp,
        latitude: position_info.as_ref().map(|p| p.latitude).unwrap_or(0.0),
        longitude: position_info.as_ref().map(|p| p.longitude).unwrap_or(0.0),
        altitude_msl_feet: position_info.and_then(|p| p.altitude_feet),
        altitude_agl_feet: None, // Will be calculated later
        flight_number: callsign,
        squawk: extract_squawk(message),
        ground_speed_knots: velocity_info.as_ref().and_then(|v| v.ground_speed_knots),
        track_degrees: velocity_info.as_ref().and_then(|v| v.track_degrees),
        climb_fpm: velocity_info.and_then(|v| v.vertical_rate_fpm),
        turn_rate_rot: None, // Not provided in ADS-B velocity messages
        source_metadata: Some(source_metadata),
        flight_id: None, // Will be assigned by flight tracker
        device_id,
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
/// Note: This is a placeholder for Phase 4. Full CPR decoding requires
/// tracking even/odd message pairs and maintaining position state.
/// That will be implemented in Phase 5 using rs1090's decode_positions() function.
fn extract_position_info(_message: &Message) -> Option<PositionInfo> {
    // CPR decoding not yet implemented
    // This will be added in Phase 5
    None
}

/// Velocity information extracted from ADS-B message
#[derive(Debug, Clone)]
struct VelocityInfo {
    ground_speed_knots: Option<f32>,
    track_degrees: Option<f32>,
    vertical_rate_fpm: Option<i32>,
}

/// Extract velocity information from airborne velocity messages
fn extract_velocity_info(_message: &Message) -> Option<VelocityInfo> {
    // Velocity extraction not yet implemented
    // This will be added after we parse the DF enum properly
    None
}

/// Extract callsign from identification messages
fn extract_callsign(_message: &Message) -> Option<String> {
    // Callsign extraction not yet implemented
    // This will be added after we parse the DF enum properly
    None
}

/// Extract squawk code if available
fn extract_squawk(_message: &Message) -> Option<String> {
    // Squawk extraction not yet implemented
    None
}

/// Build ADS-B-specific metadata for source_metadata JSONB field
fn build_adsb_metadata(message: &Message) -> serde_json::Value {
    let mut metadata = serde_json::Map::new();
    metadata.insert("protocol".to_string(), serde_json::json!("adsb"));

    // Add downlink format
    // This will be expanded as we extract more information
    metadata.insert("crc".to_string(), serde_json::json!(message.crc));

    serde_json::Value::Object(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::beast::decoder::decode_beast_frame;
    use hex_literal::hex;

    #[test]
    fn test_adsb_to_fix_basic() {
        // Example ADS-B message with ICAO 4BB463
        let frame = hex!("8d4bb463003d10000000001b5bec");
        let timestamp = Utc::now();
        let receiver_id = Uuid::now_v7();
        let device_id = Uuid::now_v7();
        let raw_message_id = Uuid::now_v7();

        let decoded = decode_beast_frame(&frame, timestamp).unwrap();
        let fix_result = adsb_message_to_fix(
            &decoded.message,
            timestamp,
            receiver_id,
            device_id,
            raw_message_id,
        );

        // For now, this will return None since we haven't implemented
        // position/velocity extraction yet
        assert!(fix_result.is_ok());
        assert!(
            fix_result.unwrap().is_none(),
            "Should return None when no position/velocity data"
        );
    }

    #[test]
    fn test_extract_icao_address() {
        // ADS-B message with ICAO 4BB463 (hex)
        let frame = hex!("8d4bb463003d10000000001b5bec");
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
}
