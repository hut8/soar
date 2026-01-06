use anyhow::Result;
use chrono::Utc;
use tracing::debug;
use uuid::Uuid;

use crate::fixes::Fix;
use crate::sbs::decoder::{DecodedSbsMessage, SbsMessageType};

/// Convert a decoded SBS message to a Fix if it contains useful information
///
/// SBS messages contain rich data directly in CSV format including:
/// - ICAO address → device identifier
/// - Position (lat/lon/alt) - directly available in position messages
/// - Velocity information (ground speed, track, vertical rate) - in velocity messages
/// - Identification (callsign) - in identification messages
/// - Squawk code - in surveillance messages
///
/// Returns None if the message doesn't contain useful fix information.
pub fn sbs_message_to_fix(
    message: &DecodedSbsMessage,
    receiver_id: Uuid,
    aircraft_id: Uuid,
    raw_message_id: Uuid,
) -> Result<Option<Fix>> {
    // We need at least position OR velocity OR identification to create a fix
    let has_position = message.latitude.is_some() && message.longitude.is_some();
    let has_altitude = message.altitude.is_some();
    let has_velocity = message.ground_speed.is_some()
        || message.track.is_some()
        || message.vertical_rate.is_some();
    let has_identification = message.flight_id.is_some() || message.squawk.is_some();

    if !has_position && !has_altitude && !has_velocity && !has_identification {
        debug!(
            "SBS message contains no useful fix data for {}",
            message.icao_address
        );
        return Ok(None);
    }

    // Determine if aircraft is active (ground speed >= 20 knots or airborne)
    let is_active = message.ground_speed.is_none_or(|speed| speed >= 20.0)
        || message.altitude.is_some_and(|alt| alt > 1000); // Consider > 1000ft as airborne

    // Build source_metadata for SBS-specific fields
    let source_metadata = build_sbs_metadata(message);

    // Build the Fix
    let fix = Fix {
        id: Uuid::now_v7(),
        source: message.icao_address.clone(),
        aprs_type: "ADSB".to_string(), // SBS is also ADS-B data
        via: vec![],                   // SBS is direct from aircraft via receiver
        timestamp: message.timestamp,
        latitude: message.latitude.unwrap_or(0.0),
        longitude: message.longitude.unwrap_or(0.0),
        altitude_msl_feet: message.altitude,
        altitude_agl_feet: None, // Will be calculated later
        flight_number: message.flight_id.clone(),
        squawk: message.squawk.clone(),
        ground_speed_knots: message.ground_speed,
        track_degrees: message.track,
        climb_fpm: message.vertical_rate,
        turn_rate_rot: None, // Not provided in SBS format
        source_metadata: Some(source_metadata),
        flight_id: None, // Will be assigned by flight tracker
        aircraft_id,
        received_at: message.timestamp,
        is_active,
        receiver_id,
        raw_message_id,
        altitude_agl_valid: false, // Will be calculated later
        time_gap_seconds: None,    // Will be set by flight tracker if part of a flight
    };

    Ok(Some(fix))
}

/// Check if SBS message type can produce a Fix
pub fn message_can_produce_fix(message_type: &SbsMessageType) -> bool {
    matches!(
        message_type,
        SbsMessageType::Identification
            | SbsMessageType::SurfacePosition
            | SbsMessageType::AirbornePosition
            | SbsMessageType::AirborneVelocity
            | SbsMessageType::SurveillanceAlt
            | SbsMessageType::SurveillanceId
    )
}

/// Build SBS-specific metadata for source_metadata JSONB field
fn build_sbs_metadata(message: &DecodedSbsMessage) -> serde_json::Value {
    let mut metadata = serde_json::Map::new();

    // Add SBS-specific fields
    metadata.insert(
        "sbs_message_type".to_string(),
        serde_json::json!(format!("{:?}", message.message_type)),
    );
    metadata.insert(
        "raw_csv".to_string(),
        serde_json::json!(message.raw_message),
    );

    // Add signal level if available
    if let Some(signal_level) = message.signal_level {
        metadata.insert("signal_level".to_string(), serde_json::json!(signal_level));
    }

    serde_json::Value::Object(metadata)
}

/// Extract ICAO address string from decoded message
pub fn extract_icao_address(message: &DecodedSbsMessage) -> String {
    message.icao_address.clone()
}

/// Extract flight number/callsign from decoded message
pub fn extract_flight_number(message: &DecodedSbsMessage) -> Option<String> {
    message.flight_id.clone()
}

/// Extract altitude from decoded message
pub fn extract_altitude(message: &DecodedSbsMessage) -> Option<i32> {
    message.altitude
}

/// Extract position from decoded message
pub fn extract_position(message: &DecodedSbsMessage) -> Option<(f64, f64)> {
    match (message.latitude, message.longitude) {
        (Some(lat), Some(lon)) => Some((lat, lon)),
        _ => None,
    }
}

/// Extract velocity from decoded message
pub fn extract_velocity(message: &DecodedSbsMessage) -> Option<(f32, f32, i32)> {
    match (message.ground_speed, message.track, message.vertical_rate) {
        (Some(gs), Some(track), vrate) => Some((gs, track, vrate.unwrap_or(0))),
        (Some(gs), None, vrate) => Some((gs, 0.0, vrate.unwrap_or(0))),
        (None, Some(track), vrate) => Some((0.0, track, vrate.unwrap_or(0))),
        (None, None, Some(vrate)) => Some((0.0, 0.0, vrate)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sbs::decoder::decode_sbs_message;

    #[test]
    fn test_sbs_to_fix_position_message() {
        let csv_line = "MSG,3,1,1,4BB268,1,2023/06/01,12:34:56.789,2023/06/01,12:34:56.789,UAL123,35000,,,37.6213,-122.3790,,,0,0,0,0,0";
        let timestamp = Utc::now();
        let receiver_id = Uuid::now_v7();
        let aircraft_id = Uuid::now_v7();
        let raw_message_id = Uuid::now_v7();

        let decoded = decode_sbs_message(csv_line, timestamp).unwrap();
        let fix_result = sbs_message_to_fix(&decoded, receiver_id, aircraft_id, raw_message_id);

        assert!(fix_result.is_ok());
        let fix_opt = fix_result.unwrap();
        assert!(fix_opt.is_some(), "Should create Fix for position message");

        let fix = fix_opt.unwrap();
        assert_eq!(fix.source, "4BB268");
        assert_eq!(fix.flight_number, Some("UAL123".to_string()));
        assert_eq!(fix.altitude_msl_feet, Some(35000));
        assert_eq!(fix.latitude, 37.6213);
        assert_eq!(fix.longitude, -122.3790);
        assert_eq!(fix.aprs_type, "ADSB");
        assert!(fix.is_active);
    }

    #[test]
    fn test_sbs_to_fix_velocity_message() {
        let csv_line = "MSG,4,1,1,4BB268,1,2023/06/01,12:34:56.789,2023/06/01,12:34:56.789,,,450,90,,,,,0,0,0,0,0";
        let timestamp = Utc::now();
        let receiver_id = Uuid::now_v7();
        let aircraft_id = Uuid::now_v7();
        let raw_message_id = Uuid::now_v7();

        let decoded = decode_sbs_message(csv_line, timestamp).unwrap();
        let fix_result = sbs_message_to_fix(&decoded, receiver_id, aircraft_id, raw_message_id);

        assert!(fix_result.is_ok());
        let fix_opt = fix_result.unwrap();
        assert!(fix_opt.is_some(), "Should create Fix for velocity message");

        let fix = fix_opt.unwrap();
        assert_eq!(fix.source, "4BB268");
        assert_eq!(fix.ground_speed_knots, Some(450.0));
        assert_eq!(fix.track_degrees, Some(90.0));
        assert_eq!(fix.altitude_msl_feet, None);
        assert!(fix.is_active); // Ground speed > 20 knots
    }

    #[test]
    fn test_sbs_to_fix_identification_message() {
        let csv_line = "MSG,1,1,1,4BB268,1,2023/06/01,12:34:56.789,2023/06/01,12:34:56.789,UAL123,,,,,,,,,,0,0,0,0,0";
        let timestamp = Utc::now();
        let receiver_id = Uuid::now_v7();
        let aircraft_id = Uuid::now_v7();
        let raw_message_id = Uuid::now_v7();

        let decoded = decode_sbs_message(csv_line, timestamp).unwrap();
        let fix_result = sbs_message_to_fix(&decoded, receiver_id, aircraft_id, raw_message_id);

        assert!(fix_result.is_ok());
        let fix_opt = fix_result.unwrap();
        assert!(
            fix_opt.is_some(),
            "Should create Fix for identification message"
        );

        let fix = fix_opt.unwrap();
        assert_eq!(fix.source, "4BB268");
        assert_eq!(fix.flight_number, Some("UAL123".to_string()));
        assert_eq!(fix.altitude_msl_feet, None);
        assert_eq!(fix.latitude, 0.0);
        assert_eq!(fix.longitude, 0.0);
        assert!(!fix.is_active); // No altitude or high speed
    }

    #[test]
    fn test_sbs_to_fix_surveillance_id_message() {
        let csv_line = "MSG,6,1,1,4BB268,1,2023/06/01,12:34:56.789,2023/06/01,12:34:56.789,,,,,,,1234,0,0,0,0,0";
        let timestamp = Utc::now();
        let receiver_id = Uuid::now_v7();
        let aircraft_id = Uuid::now_v7();
        let raw_message_id = Uuid::now_v7();

        let decoded = decode_sbs_message(csv_line, timestamp).unwrap();
        let fix_result = sbs_message_to_fix(&decoded, receiver_id, aircraft_id, raw_message_id);

        assert!(fix_result.is_ok());
        let fix_opt = fix_result.unwrap();
        assert!(
            fix_opt.is_some(),
            "Should create Fix for surveillance ID message"
        );

        let fix = fix_opt.unwrap();
        assert_eq!(fix.source, "4BB268");
        assert_eq!(fix.squawk, Some("1234".to_string()));
        assert!(!fix.is_active); // No altitude or high speed
    }

    #[test]
    fn test_sbs_to_fix_empty_message() {
        let csv_line =
            "MSG,8,1,1,4BB268,1,2023/06/01,12:34:56.789,2023/06/01,12:34:56.789,,,,,,,,,,,,,,,,0";
        let timestamp = Utc::now();
        let receiver_id = Uuid::now_v7();
        let aircraft_id = Uuid::now_v7();
        let raw_message_id = Uuid::now_v7();

        let decoded = decode_sbs_message(csv_line, timestamp).unwrap();
        let fix_result = sbs_message_to_fix(&decoded, receiver_id, aircraft_id, raw_message_id);

        assert!(fix_result.is_ok());
        let fix_opt = fix_result.unwrap();
        assert!(
            fix_opt.is_none(),
            "Should NOT create Fix for empty AllCall message"
        );
    }

    #[test]
    fn test_message_can_produce_fix() {
        assert!(message_can_produce_fix(&SbsMessageType::Identification));
        assert!(message_can_produce_fix(&SbsMessageType::SurfacePosition));
        assert!(message_can_produce_fix(&SbsMessageType::AirbornePosition));
        assert!(message_can_produce_fix(&SbsMessageType::AirborneVelocity));
        assert!(message_can_produce_fix(&SbsMessageType::SurveillanceAlt));
        assert!(message_can_produce_fix(&SbsMessageType::SurveillanceId));
        assert!(message_can_produce_fix(&SbsMessageType::AirToAir));
        assert!(!message_can_produce_fix(&SbsMessageType::AllCall));
    }

    #[test]
    fn test_extract_icao_address() {
        let csv_line = "MSG,3,1,1,4BB268,1,2023/06/01,12:34:56.789,2023/06/01,12:34:56.789,UAL123,35000,,,37.6213,-122.3790,,,0,0,0,0,0";
        let timestamp = Utc::now();
        let decoded = decode_sbs_message(csv_line, timestamp).unwrap();

        let icao = extract_icao_address(&decoded);
        assert_eq!(icao, "4BB268");
    }

    #[test]
    fn test_extract_flight_number() {
        let csv_line = "MSG,1,1,1,4BB268,1,2023/06/01,12:34:56.789,2023/06/01,12:34:56.789,UAL123,,,,,,,,,,0,0,0,0,0";
        let timestamp = Utc::now();
        let decoded = decode_sbs_message(csv_line, timestamp).unwrap();

        let flight = extract_flight_number(&decoded);
        assert_eq!(flight, Some("UAL123".to_string()));
    }

    #[test]
    fn test_extract_altitude() {
        let csv_line = "MSG,3,1,1,4BB268,1,2023/06/01,12:34:56.789,2023/06/01,12:34:56.789,,35000,,,37.6213,-122.3790,,,0,0,0,0,0";
        let timestamp = Utc::now();
        let decoded = decode_sbs_message(csv_line, timestamp).unwrap();

        let altitude = extract_altitude(&decoded);
        assert_eq!(altitude, Some(35000));
    }

    #[test]
    fn test_extract_position() {
        let csv_line = "MSG,3,1,1,4BB268,1,2023/06/01,12:34:56.789,2023/06/01,12:34:56.789,,35000,,,37.6213,-122.3790,,,0,0,0,0,0";
        let timestamp = Utc::now();
        let decoded = decode_sbs_message(csv_line, timestamp).unwrap();

        let position = extract_position(&decoded);
        assert_eq!(position, Some((37.6213, -122.3790)));
    }

    #[test]
    fn test_extract_velocity() {
        let csv_line = "MSG,4,1,1,4BB268,1,2023/06/01,12:34:56.789,2023/06/01,12:34:56.789,,,450,90,500,,,0,0,0,0,0";
        let timestamp = Utc::now();
        let decoded = decode_sbs_message(csv_line, timestamp).unwrap();

        let velocity = extract_velocity(&decoded);
        assert_eq!(velocity, Some((450.0, 90.0, 500)));
    }

    #[test]
    fn test_sbs_to_fix_comprehensive_message() {
        let csv_line = "MSG,3,1,1,4BB268,1,2023/06/01,12:34:56.789,2023/06/01,12:34:56.789,UAL123,35000,450,90,37.6213,-122.3790,1000,1234,0,0,0,0,0";
        let timestamp = Utc::now();
        let receiver_id = Uuid::now_v7();
        let aircraft_id = Uuid::now_v7();
        let raw_message_id = Uuid::now_v7();

        let decoded = decode_sbs_message(csv_line, timestamp).unwrap();
        let fix_result = sbs_message_to_fix(&decoded, receiver_id, aircraft_id, raw_message_id);

        assert!(fix_result.is_ok());
        let fix_opt = fix_result.unwrap();
        assert!(
            fix_opt.is_some(),
            "Should create Fix for comprehensive message"
        );

        let fix = fix_opt.unwrap();
        assert_eq!(fix.source, "4BB268");
        assert_eq!(fix.flight_number, Some("UAL123".to_string()));
        assert_eq!(fix.altitude_msl_feet, Some(35000));
        assert_eq!(fix.ground_speed_knots, Some(450.0));
        assert_eq!(fix.track_degrees, Some(90.0));
        assert_eq!(fix.climb_fpm, Some(1000));
        assert_eq!(fix.squawk, Some("1234".to_string()));
        assert_eq!(fix.latitude, 37.6213);
        assert_eq!(fix.longitude, -122.3790);
        assert!(fix.is_active);

        // Check metadata
        let metadata = fix.source_metadata.unwrap();
        assert_eq!(
            metadata.get("sbs_message_type").unwrap().as_str().unwrap(),
            "AirbornePosition"
        );
        assert!(metadata.get("raw_csv").is_some());
    }

    #[test]
    fn test_ground_aircraft_not_active() {
        let csv_line = "MSG,2,1,1,4BB268,1,2023/06/01,12:34:56.789,2023/06/01,12:34:56.789,UAL123,0,5,0,37.6213,-122.3790,,,0,0,0,0,0";
        let timestamp = Utc::now();
        let receiver_id = Uuid::now_v7();
        let aircraft_id = Uuid::now_v7();
        let raw_message_id = Uuid::now_v7();

        let decoded = decode_sbs_message(csv_line, timestamp).unwrap();
        let fix_result = sbs_message_to_fix(&decoded, receiver_id, aircraft_id, raw_message_id);

        assert!(fix_result.is_ok());
        let fix_opt = fix_result.unwrap();
        assert!(fix_opt.is_some());

        let fix = fix_opt.unwrap();
        assert_eq!(fix.altitude_msl_feet, Some(0));
        assert_eq!(fix.ground_speed_knots, Some(5.0));
        assert!(
            !fix.is_active,
            "Ground aircraft with low speed should not be active"
        );
    }

    #[test]
    fn test_airborne_aircraft_active() {
        let csv_line = "MSG,3,1,1,4BB268,1,2023/06/01,12:34:56.789,2023/06/01,12:34:56.789,,1500,,,,,,,0,0,0,0,0";
        let timestamp = Utc::now();
        let receiver_id = Uuid::now_v7();
        let aircraft_id = Uuid::now_v7();
        let raw_message_id = Uuid::now_v7();

        let decoded = decode_sbs_message(csv_line, timestamp).unwrap();
        let fix_result = sbs_message_to_fix(&decoded, receiver_id, aircraft_id, raw_message_id);

        assert!(fix_result.is_ok());
        let fix_opt = fix_result.unwrap();
        assert!(fix_opt.is_some());

        let fix = fix_opt.unwrap();
        assert_eq!(fix.altitude_msl_feet, Some(1500));
        assert!(fix.is_active, "Airborne aircraft should be active");
    }
}
