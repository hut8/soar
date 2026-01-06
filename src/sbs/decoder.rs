use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::str::FromStr;

/// Decoded SBS message with metadata
#[derive(Debug, Clone)]
pub struct DecodedSbsMessage {
    pub timestamp: DateTime<Utc>,
    pub raw_message: String,
    pub message_type: SbsMessageType,
    pub icao_address: String,
    pub flight_id: Option<String>,
    pub altitude: Option<i32>,
    pub ground_speed: Option<f32>,
    pub track: Option<f32>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub vertical_rate: Option<i32>,
    pub squawk: Option<String>,
    pub signal_level: Option<f32>,
}

/// SBS message types according to BaseStation format
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SbsMessageType {
    /// ES Identification and Category
    Identification,
    /// ES Surface Position Message
    SurfacePosition,
    /// ES Airborne Position Message
    AirbornePosition,
    /// ES Airborne Velocity Message
    AirborneVelocity,
    /// Surveillance Alt Message
    SurveillanceAlt,
    /// Surveillance ID Message
    SurveillanceId,
    /// Air To Air Message
    AirToAir,
    /// All Call Reply
    AllCall,
}

impl FromStr for SbsMessageType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "1" => Ok(SbsMessageType::Identification),
            "2" => Ok(SbsMessageType::SurfacePosition),
            "3" => Ok(SbsMessageType::AirbornePosition),
            "4" => Ok(SbsMessageType::AirborneVelocity),
            "5" => Ok(SbsMessageType::SurveillanceAlt),
            "6" => Ok(SbsMessageType::SurveillanceId),
            "7" => Ok(SbsMessageType::AirToAir),
            "8" => Ok(SbsMessageType::AllCall),
            _ => Err(anyhow::anyhow!("Invalid SBS message type: {}", s)),
        }
    }
}

// Raw SBS message structure for CSV parsing (unused but kept for reference)
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct SbsField {
    message_type: String,
    transmission_type: String,
    session_id: String,
    aircraft_id: String,
    flight_id: String,
    generated_date: String,
    generated_time: String,
    logged_date: String,
    logged_time: String,
    callsign: String,
    altitude: String,
    ground_speed: String,
    track: String,
    latitude: String,
    longitude: String,
    vertical_rate: String,
    squawk: String,
    alert: String,
    emergency: String,
    spi: String,
    on_ground: String,
}

/// Decode an SBS CSV message line into a structured message
///
/// SBS messages are CSV format with the following fields:
/// MSG,type,session,hex_id,flight_id,gen_date,gen_time,log_date,log_time,callsign,
/// altitude,ground_speed,track,lat,lon,vertical_rate,squawk,alert,emergency,spi,on_ground
///
/// Example: "MSG,3,,,A1B2C3,,,,,,,35000,,,45.123,-122.456,,,1200,,,0,0,0,0"
pub fn decode_sbs_message(csv_line: &str, received_at: DateTime<Utc>) -> Result<DecodedSbsMessage> {
    // Skip empty lines
    if csv_line.trim().is_empty() {
        return Err(anyhow::anyhow!("Empty SBS message"));
    }

    // Parse CSV fields
    let fields: Vec<&str> = csv_line.split(',').collect();
    if fields.len() < 22 {
        return Err(anyhow::anyhow!(
            "Invalid SBS message: expected at least 22 fields, got {}",
            fields.len()
        ));
    }

    // Verify it's an MSG type
    if fields[0] != "MSG" {
        return Err(anyhow::anyhow!(
            "Invalid SBS message: expected 'MSG' prefix"
        ));
    }

    // Parse message type
    let message_type = fields[1]
        .parse::<SbsMessageType>()
        .context("Invalid SBS message type")?;

    // Extract ICAO address (field 3, zero-indexed)
    let icao_address = fields[3].trim().to_string();
    if icao_address.is_empty() {
        return Err(anyhow::anyhow!("SBS message missing ICAO address"));
    }

    // Parse flight ID (field 4)
    let flight_id = if fields[4].trim().is_empty() {
        None
    } else {
        Some(fields[4].trim().to_string())
    };

    // Parse altitude (field 10)
    let altitude = if fields[10].trim().is_empty() {
        None
    } else {
        fields[10]
            .trim()
            .parse::<i32>()
            .map(Some)
            .with_context(|| format!("Failed to parse altitude: {}", fields[10]))?
    };

    // Parse ground speed (field 11)
    let ground_speed = if fields[11].trim().is_empty() {
        None
    } else {
        fields[11]
            .trim()
            .parse::<f32>()
            .map(Some)
            .with_context(|| format!("Failed to parse ground speed: {}", fields[11]))?
    };

    // Parse track (field 12)
    let track = if fields[12].trim().is_empty() {
        None
    } else {
        fields[12]
            .trim()
            .parse::<f32>()
            .map(Some)
            .with_context(|| format!("Failed to parse track: {}", fields[12]))?
    };

    // Parse latitude (field 13)
    let latitude = if fields[13].trim().is_empty() {
        None
    } else {
        fields[13]
            .trim()
            .parse::<f64>()
            .map(Some)
            .with_context(|| format!("Failed to parse latitude: {}", fields[13]))?
    };

    // Parse longitude (field 14)
    let longitude = if fields[14].trim().is_empty() {
        None
    } else {
        fields[14]
            .trim()
            .parse::<f64>()
            .map(Some)
            .with_context(|| format!("Failed to parse longitude: {}", fields[14]))?
    };

    // Parse vertical rate (field 15)
    let vertical_rate = if fields[15].trim().is_empty() {
        None
    } else {
        fields[15]
            .trim()
            .parse::<i32>()
            .map(Some)
            .with_context(|| format!("Failed to parse vertical rate: {}", fields[15]))?
    };

    // Parse squawk (field 16)
    let squawk = if fields[16].trim().is_empty() {
        None
    } else {
        Some(fields[16].trim().to_string())
    };

    // Signal level is not available in SBS format, but we can derive from on_ground flag
    let signal_level = if fields.len() > 21 && !fields[21].trim().is_empty() {
        fields[21].trim().parse::<u8>().ok().map(|_| 1.0) // Just indicate signal received
    } else {
        None
    };

    Ok(DecodedSbsMessage {
        timestamp: received_at,
        raw_message: csv_line.to_string(),
        message_type,
        icao_address,
        flight_id,
        altitude,
        ground_speed,
        track,
        latitude,
        longitude,
        vertical_rate,
        squawk,
        signal_level,
    })
}

/// Convert decoded SBS message to JSON for storage
pub fn message_to_json(msg: &DecodedSbsMessage) -> Result<serde_json::Value> {
    let mut json = serde_json::Map::new();

    json.insert(
        "message_type".to_string(),
        serde_json::json!(format!("{:?}", msg.message_type)),
    );
    json.insert(
        "icao_address".to_string(),
        serde_json::json!(msg.icao_address),
    );

    if let Some(ref flight_id) = msg.flight_id {
        json.insert("flight_id".to_string(), serde_json::json!(flight_id));
    }

    if let Some(altitude) = msg.altitude {
        json.insert("altitude".to_string(), serde_json::json!(altitude));
    }

    if let Some(ground_speed) = msg.ground_speed {
        json.insert("ground_speed".to_string(), serde_json::json!(ground_speed));
    }

    if let Some(track) = msg.track {
        json.insert("track".to_string(), serde_json::json!(track));
    }

    if let Some(latitude) = msg.latitude {
        json.insert("latitude".to_string(), serde_json::json!(latitude));
    }

    if let Some(longitude) = msg.longitude {
        json.insert("longitude".to_string(), serde_json::json!(longitude));
    }

    if let Some(vertical_rate) = msg.vertical_rate {
        json.insert(
            "vertical_rate".to_string(),
            serde_json::json!(vertical_rate),
        );
    }

    if let Some(ref squawk) = msg.squawk {
        json.insert("squawk".to_string(), serde_json::json!(squawk));
    }

    if let Some(signal_level) = msg.signal_level {
        json.insert("signal_level".to_string(), serde_json::json!(signal_level));
    }

    Ok(serde_json::Value::Object(json))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_sbs_identification_message() {
        let csv_line = "MSG,1,1,1,738065,1,2008/11/28,23:48:18.611,2008/11/28,23:53:19.161,RYR1427,,,,,,,0,,0,0,0,0";
        let timestamp = Utc::now();

        let result = decode_sbs_message(csv_line, timestamp);
        assert!(result.is_ok());

        let decoded = result.unwrap();
        assert_eq!(decoded.message_type, SbsMessageType::Identification);
        assert_eq!(decoded.icao_address, "738065");
        assert_eq!(decoded.flight_id, Some("RYR1427".to_string()));
        assert_eq!(decoded.altitude, None);
        assert_eq!(decoded.ground_speed, None);
        assert_eq!(decoded.track, None);
    }

    #[test]
    fn test_decode_sbs_position_message() {
        let csv_line = "MSG,3,1,1,738065,1,2008/11/28,23:48:18.611,2008/11/28,23:53:19.161,,36000,,,51.45735,1.02826,,,0,0,0,0";
        let timestamp = Utc::now();

        let result = decode_sbs_message(csv_line, timestamp);
        assert!(result.is_ok());

        let decoded = result.unwrap();
        assert_eq!(decoded.message_type, SbsMessageType::AirbornePosition);
        assert_eq!(decoded.icao_address, "738065");
        assert_eq!(decoded.altitude, Some(36000));
        assert_eq!(decoded.latitude, Some(51.45735));
        assert_eq!(decoded.longitude, Some(1.02826));
        assert_eq!(decoded.ground_speed, None);
        assert_eq!(decoded.track, None);
    }

    #[test]
    fn test_decode_sbs_velocity_message() {
        let csv_line = "MSG,4,1,1,738065,1,2008/11/28,23:48:18.611,2008/11/28,23:53:19.161,,,420,179,,,0,0,0,0";
        let timestamp = Utc::now();

        let result = decode_sbs_message(csv_line, timestamp);
        assert!(result.is_ok());

        let decoded = result.unwrap();
        assert_eq!(decoded.message_type, SbsMessageType::AirborneVelocity);
        assert_eq!(decoded.icao_address, "738065");
        assert_eq!(decoded.ground_speed, Some(420.0));
        assert_eq!(decoded.track, Some(179.0));
        assert_eq!(decoded.altitude, None);
    }

    #[test]
    fn test_decode_sbs_surveillance_id_message() {
        let csv_line =
            "MSG,6,1,1,738065,1,2008/11/28,23:48:18.611,2008/11/28,23:53:19.161,,,,,,,7541,0,0,0,0";
        let timestamp = Utc::now();

        let result = decode_sbs_message(csv_line, timestamp);
        assert!(result.is_ok());

        let decoded = result.unwrap();
        assert_eq!(decoded.message_type, SbsMessageType::SurveillanceId);
        assert_eq!(decoded.icao_address, "738065");
        assert_eq!(decoded.squawk, Some("7541".to_string()));
    }

    #[test]
    fn test_decode_invalid_message() {
        let csv_line = "INVALID,1,2,3";
        let timestamp = Utc::now();

        let result = decode_sbs_message(csv_line, timestamp);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_empty_message() {
        let csv_line = "";
        let timestamp = Utc::now();

        let result = decode_sbs_message(csv_line, timestamp);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_message_missing_fields() {
        let csv_line = "MSG,3,1,1,738065";
        let timestamp = Utc::now();

        let result = decode_sbs_message(csv_line, timestamp);
        assert!(result.is_err());
    }

    #[test]
    fn test_message_to_json() {
        let csv_line = "MSG,3,1,1,738065,1,2008/11/28,23:48:18.611,2008/11/28,23:53:19.161,,36000,,,51.45735,1.02826,,,0,0,0,0";
        let timestamp = Utc::now();

        let decoded = decode_sbs_message(csv_line, timestamp).unwrap();
        let json = message_to_json(&decoded).unwrap();

        assert_eq!(
            json.get("icao_address").unwrap().as_str().unwrap(),
            "738065"
        );
        assert_eq!(
            json.get("message_type").unwrap().as_str().unwrap(),
            "AirbornePosition"
        );
        assert_eq!(json.get("altitude").unwrap().as_i64().unwrap(), 36000);
        assert_eq!(json.get("latitude").unwrap().as_f64().unwrap(), 51.45735);
        assert_eq!(json.get("longitude").unwrap().as_f64().unwrap(), 1.02826);
    }

    #[test]
    fn test_sbs_message_type_parsing() {
        assert_eq!(
            "1".parse::<SbsMessageType>().unwrap(),
            SbsMessageType::Identification
        );
        assert_eq!(
            "3".parse::<SbsMessageType>().unwrap(),
            SbsMessageType::AirbornePosition
        );
        assert_eq!(
            "4".parse::<SbsMessageType>().unwrap(),
            SbsMessageType::AirborneVelocity
        );

        let result = "99".parse::<SbsMessageType>();
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_realistic_message_with_callsign() {
        let csv_line = "MSG,1,,,A1B2C3,,,,,,,RYR-123,,,,,,,0,0,0,0";
        let timestamp = Utc::now();

        let result = decode_sbs_message(csv_line, timestamp);
        assert!(result.is_ok());

        let decoded = result.unwrap();
        assert_eq!(decoded.icao_address, "A1B2C3");
        assert_eq!(decoded.flight_id, Some("RYR-123".to_string()));
    }

    #[test]
    fn test_decode_comprehensive_message() {
        let csv_line = "MSG,3,1,1,4BB268,1,2023/06/01,12:34:56.789,2023/06/01,12:34:56.789,UAL123,35000,450,90,37.6213,-122.3790,1000,1234,0,0,0,0,0";
        let timestamp = Utc::now();

        let result = decode_sbs_message(csv_line, timestamp);
        assert!(result.is_ok());

        let decoded = result.unwrap();
        assert_eq!(decoded.message_type, SbsMessageType::AirbornePosition);
        assert_eq!(decoded.icao_address, "4BB268");
        assert_eq!(decoded.flight_id, Some("UAL123".to_string()));
        assert_eq!(decoded.altitude, Some(35000));
        assert_eq!(decoded.ground_speed, Some(450.0));
        assert_eq!(decoded.track, Some(90.0));
        assert_eq!(decoded.latitude, Some(37.6213));
        assert_eq!(decoded.longitude, Some(-122.3790));
        assert_eq!(decoded.vertical_rate, Some(1000));
        assert_eq!(decoded.squawk, Some("1234".to_string()));
    }
}
