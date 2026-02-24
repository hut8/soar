use anyhow::{Context, Result};

/// SBS message types (MSG,1 through MSG,8)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SbsMessageType {
    /// MSG,1: ES Identification and Category (callsign)
    EsIdentification = 1,
    /// MSG,2: ES Surface Position Message (surface position)
    EsSurfacePosition = 2,
    /// MSG,3: ES Airborne Position Message (altitude, lat/lon)
    EsAirbornePosition = 3,
    /// MSG,4: ES Airborne Velocity Message (speed, track, vertical rate)
    EsAirborneVelocity = 4,
    /// MSG,5: Surveillance Alt Message (altitude only)
    SurveillanceAlt = 5,
    /// MSG,6: Surveillance ID Message (squawk)
    SurveillanceId = 6,
    /// MSG,7: Air To Air Message (altitude)
    AirToAir = 7,
    /// MSG,8: All Call Reply (no data)
    AllCallReply = 8,
}

impl SbsMessageType {
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::EsIdentification),
            2 => Some(Self::EsSurfacePosition),
            3 => Some(Self::EsAirbornePosition),
            4 => Some(Self::EsAirborneVelocity),
            5 => Some(Self::SurveillanceAlt),
            6 => Some(Self::SurveillanceId),
            7 => Some(Self::AirToAir),
            8 => Some(Self::AllCallReply),
            _ => None,
        }
    }
}

/// Parsed SBS message containing all fields from the CSV format
#[derive(Debug, Clone)]
pub struct SbsMessage {
    /// Message type (1-8)
    pub message_type: SbsMessageType,
    /// Transmission type
    pub transmission_type: Option<u8>,
    /// Session ID
    pub session_id: Option<String>,
    /// Aircraft ID (ICAO hex address as string, e.g., "738065" or "AB1234")
    pub aircraft_id: String,
    /// Is military aircraft
    pub is_military: Option<bool>,
    /// Callsign (from MSG,1)
    pub callsign: Option<String>,
    /// Altitude in feet (from MSG,3,5,7)
    pub altitude: Option<i32>,
    /// Ground speed in knots (from MSG,3,4)
    pub ground_speed: Option<f32>,
    /// Track/heading in degrees (from MSG,3,4)
    pub track: Option<f32>,
    /// Latitude (from MSG,3)
    pub latitude: Option<f64>,
    /// Longitude (from MSG,3)
    pub longitude: Option<f64>,
    /// Vertical rate in feet/minute (from MSG,4)
    pub vertical_rate: Option<i32>,
    /// Squawk code (from MSG,6)
    pub squawk: Option<String>,
    /// Alert flag
    pub alert: Option<bool>,
    /// Emergency flag
    pub emergency: Option<bool>,
    /// SPI (Ident) flag
    pub spi: Option<bool>,
    /// On ground flag
    pub on_ground: Option<bool>,
}

impl SbsMessage {
    /// Parse the aircraft ID as a hexadecimal ICAO address
    pub fn icao_address(&self) -> Option<u32> {
        u32::from_str_radix(&self.aircraft_id, 16).ok()
    }

    /// Check if this message contains position data
    pub fn has_position(&self) -> bool {
        self.latitude.is_some() && self.longitude.is_some()
    }

    /// Check if this message contains velocity data
    pub fn has_velocity(&self) -> bool {
        self.ground_speed.is_some() || self.vertical_rate.is_some()
    }
}

/// Parse an SBS CSV line into an SbsMessage
///
/// SBS format: MSG,<type>,<transmission_type>,<session_id>,<aircraft_id>,<is_military>,
///             <date_gen>,<time_gen>,<date_log>,<time_log>,<callsign>,<altitude>,
///             <ground_speed>,<track>,<latitude>,<longitude>,<vertical_rate>,<squawk>,
///             <alert>,<emergency>,<spi>,<on_ground>
pub fn parse_sbs_message(line: &str) -> Result<SbsMessage> {
    let fields: Vec<&str> = line.split(',').collect();

    // Minimum: MSG,<type>,,,<aircraft_id> = at least 5 fields
    if fields.len() < 5 {
        anyhow::bail!(
            "SBS message too short: expected at least 5 fields, got {}",
            fields.len()
        );
    }

    // Field 0: Must be "MSG"
    if fields[0] != "MSG" {
        anyhow::bail!("SBS message must start with MSG, got '{}'", fields[0]);
    }

    // Field 1: Message type (1-8)
    let type_num: u8 = fields[1]
        .parse()
        .with_context(|| format!("Invalid message type: '{}'", fields[1]))?;

    let message_type = SbsMessageType::from_u8(type_num)
        .ok_or_else(|| anyhow::anyhow!("Unknown message type: {}", type_num))?;

    // Field 2: Transmission type (optional)
    let transmission_type = parse_optional_u8(fields.get(2).copied());

    // Field 3: Session ID (optional)
    let session_id = parse_optional_string(fields.get(3).copied());

    // Field 4: Aircraft ID (required, hex ICAO address)
    let aircraft_id = fields[4].to_string();
    if aircraft_id.is_empty() {
        anyhow::bail!("Aircraft ID is required");
    }

    // Field 5: Is military (optional, 0 or 1)
    let is_military = parse_optional_bool(fields.get(5).copied());

    // Fields 6-9: Date/time (we skip these, using our own timestamp)

    // Field 10: Callsign (optional)
    let callsign = parse_optional_string(fields.get(10).copied())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // Field 11: Altitude (optional, feet)
    let altitude = parse_optional_i32(fields.get(11).copied());

    // Field 12: Ground speed (optional, knots)
    let ground_speed = parse_optional_f32(fields.get(12).copied());

    // Field 13: Track (optional, degrees)
    let track = parse_optional_f32(fields.get(13).copied());

    // Field 14: Latitude (optional)
    let latitude = parse_optional_f64(fields.get(14).copied());

    // Field 15: Longitude (optional)
    let longitude = parse_optional_f64(fields.get(15).copied());

    // Field 16: Vertical rate (optional, feet/min)
    let vertical_rate = parse_optional_i32(fields.get(16).copied());

    // Field 17: Squawk (optional)
    let squawk = parse_optional_string(fields.get(17).copied()).filter(|s| !s.is_empty());

    // Field 18: Alert (optional, 0 or 1)
    let alert = parse_optional_bool(fields.get(18).copied());

    // Field 19: Emergency (optional, 0 or 1)
    let emergency = parse_optional_bool(fields.get(19).copied());

    // Field 20: SPI (optional, 0 or 1)
    let spi = parse_optional_bool(fields.get(20).copied());

    // Field 21: On ground (optional, 0 or 1)
    let on_ground = parse_optional_bool(fields.get(21).copied());

    Ok(SbsMessage {
        message_type,
        transmission_type,
        session_id,
        aircraft_id,
        is_military,
        callsign,
        altitude,
        ground_speed,
        track,
        latitude,
        longitude,
        vertical_rate,
        squawk,
        alert,
        emergency,
        spi,
        on_ground,
    })
}

fn parse_optional_string(field: Option<&str>) -> Option<String> {
    field.filter(|s| !s.is_empty()).map(|s| s.to_string())
}

fn parse_optional_u8(field: Option<&str>) -> Option<u8> {
    field.filter(|s| !s.is_empty()).and_then(|s| s.parse().ok())
}

fn parse_optional_i32(field: Option<&str>) -> Option<i32> {
    field.filter(|s| !s.is_empty()).and_then(|s| s.parse().ok())
}

fn parse_optional_f32(field: Option<&str>) -> Option<f32> {
    field.filter(|s| !s.is_empty()).and_then(|s| s.parse().ok())
}

fn parse_optional_f64(field: Option<&str>) -> Option<f64> {
    field.filter(|s| !s.is_empty()).and_then(|s| s.parse().ok())
}

fn parse_optional_bool(field: Option<&str>) -> Option<bool> {
    field.filter(|s| !s.is_empty()).and_then(|s| match s {
        "0" | "-1" => Some(false),
        "1" => Some(true),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use std::io::{BufRead, BufReader};

    #[test]
    fn test_parse_msg_1_identification() {
        let line = "MSG,1,1,1,738065,1,2008/11/28,23:48:18.611,2008/11/28,23:53:19.161,RYR1427,,,,,,,0,,0,0";
        let msg = parse_sbs_message(line).unwrap();

        assert_eq!(msg.message_type, SbsMessageType::EsIdentification);
        assert_eq!(msg.aircraft_id, "738065");
        assert_eq!(msg.callsign, Some("RYR1427".to_string()));
        assert!(msg.altitude.is_none());
        assert!(msg.latitude.is_none());
    }

    #[test]
    fn test_parse_msg_3_position() {
        let line = "MSG,3,1,1,738065,1,2008/11/28,23:48:18.611,2008/11/28,23:53:19.161,,36000,,,51.45735,1.02826,,,0,0,0,0";
        let msg = parse_sbs_message(line).unwrap();

        assert_eq!(msg.message_type, SbsMessageType::EsAirbornePosition);
        assert_eq!(msg.aircraft_id, "738065");
        assert_eq!(msg.altitude, Some(36000));
        assert!((msg.latitude.unwrap() - 51.45735).abs() < 0.0001);
        assert!((msg.longitude.unwrap() - 1.02826).abs() < 0.0001);
        assert!(msg.has_position());
    }

    #[test]
    fn test_parse_msg_4_velocity() {
        let line = "MSG,4,1,1,738065,1,2008/11/28,23:48:18.611,2008/11/28,23:53:19.161,,,420,179,,,0,0,0,0";
        let msg = parse_sbs_message(line).unwrap();

        assert_eq!(msg.message_type, SbsMessageType::EsAirborneVelocity);
        assert_eq!(msg.aircraft_id, "738065");
        assert_eq!(msg.ground_speed, Some(420.0));
        assert_eq!(msg.track, Some(179.0));
        assert!(msg.has_velocity());
        assert!(!msg.has_position());
    }

    #[test]
    fn test_parse_msg_6_squawk() {
        // SBS format: positions 10-16 are callsign,altitude,groundspeed,track,lat,lon,verticalrate
        // Position 17 is squawk, so we need 7 empty fields after timestamp before squawk
        let line = "MSG,6,1,1,738065,1,2008/11/28,23:48:18.611,2008/11/28,23:53:19.161,,,,,,,,7541,0,0,0,0";
        let msg = parse_sbs_message(line).unwrap();

        assert_eq!(msg.message_type, SbsMessageType::SurveillanceId);
        assert_eq!(msg.squawk, Some("7541".to_string()));
    }

    #[test]
    fn test_parse_short_format() {
        // Some SBS implementations use shorter messages
        let line = "MSG,3,,,AB1234,,,,,,,5000,,,51.5074,-0.1278,,,0,0,0,0";
        let msg = parse_sbs_message(line).unwrap();

        assert_eq!(msg.message_type, SbsMessageType::EsAirbornePosition);
        assert_eq!(msg.aircraft_id, "AB1234");
        assert_eq!(msg.altitude, Some(5000));
        assert!(msg.has_position());
    }

    #[test]
    fn test_icao_address_parsing() {
        let line = "MSG,3,,,AB1234,,,,,,,5000,,,51.5074,-0.1278,,,0,0,0,0";
        let msg = parse_sbs_message(line).unwrap();

        // AB1234 in hex = 11219508 in decimal
        assert_eq!(msg.icao_address(), Some(0xAB1234));
    }

    #[test]
    fn test_invalid_message_type() {
        let line = "MSG,9,,,AB1234,,,,,,,5000,,,51.5074,-0.1278,,,0,0,0,0";
        let result = parse_sbs_message(line);
        assert!(result.is_err());
    }

    #[test]
    fn test_non_msg_prefix() {
        let line = "STA,3,,,AB1234,,,,,,,5000,,,51.5074,-0.1278,,,0,0,0,0";
        let result = parse_sbs_message(line);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_aircraft_id() {
        let line = "MSG,3,,,,,,,,,,5000,,,51.5074,-0.1278,,,0,0,0,0";
        let result = parse_sbs_message(line);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_real_sbs_file() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data/sbs/adsbhub-sbs-sample-20260224.txt.gz");
        let file = std::fs::File::open(&path)
            .unwrap_or_else(|e| panic!("Failed to open {}: {}", path.display(), e));
        let decoder = flate2::read::GzDecoder::new(file);
        let reader = BufReader::new(decoder);

        let mut total = 0u64;
        let mut failures = 0u64;
        let mut type_counts: HashMap<u8, u64> = HashMap::new();
        let mut unique_aircraft: HashSet<String> = HashSet::new();

        // Spot-check counters
        let mut msg1_with_callsign = 0u64;
        let mut msg1_total = 0u64;
        let mut msg3_with_position = 0u64;
        let mut msg3_total = 0u64;
        let mut msg4_with_velocity = 0u64;
        let mut msg4_total = 0u64;
        let mut failure_samples: Vec<String> = Vec::new();
        const MAX_FAILURE_SAMPLES: usize = 10;

        for line in reader.lines() {
            let line = line.expect("Failed to read line");
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            total += 1;

            match parse_sbs_message(line) {
                Ok(msg) => {
                    let type_num = msg.message_type as u8;
                    *type_counts.entry(type_num).or_default() += 1;
                    unique_aircraft.insert(msg.aircraft_id.clone());

                    match msg.message_type {
                        SbsMessageType::EsIdentification => {
                            msg1_total += 1;
                            if msg.callsign.is_some() {
                                msg1_with_callsign += 1;
                            }
                        }
                        SbsMessageType::EsAirbornePosition => {
                            msg3_total += 1;
                            if msg.has_position() && msg.altitude.is_some() {
                                msg3_with_position += 1;
                            }
                        }
                        SbsMessageType::EsAirborneVelocity => {
                            msg4_total += 1;
                            if msg.has_velocity() {
                                msg4_with_velocity += 1;
                            }
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    failures += 1;
                    if failure_samples.len() < MAX_FAILURE_SAMPLES {
                        failure_samples.push(format!("  line {total}: {e} — {line}"));
                    }
                }
            }
        }

        // Print summary for manual review
        eprintln!("\n=== SBS Parse Summary ===");
        eprintln!("Total lines:      {total}");
        eprintln!("Failures:         {failures}");
        eprintln!("Unique aircraft:  {}", unique_aircraft.len());
        let mut sorted_types: Vec<_> = type_counts.iter().collect();
        sorted_types.sort_by_key(|(k, _)| *k);
        for (t, count) in &sorted_types {
            eprintln!("  MSG,{t}: {count}");
        }
        eprintln!("MSG,1 with callsign:           {msg1_with_callsign}/{msg1_total}");
        eprintln!("MSG,3 with position+altitude:  {msg3_with_position}/{msg3_total}");
        eprintln!("MSG,4 with velocity:           {msg4_with_velocity}/{msg4_total}");

        if !failure_samples.is_empty() {
            eprintln!("First {} failure(s):", failure_samples.len());
            for sample in &failure_samples {
                eprintln!("{sample}");
            }
            if failures > MAX_FAILURE_SAMPLES as u64 {
                eprintln!(
                    "  ... and {} more failures suppressed",
                    failures - MAX_FAILURE_SAMPLES as u64
                );
            }
        }

        // All lines must parse successfully
        assert_eq!(
            failures, 0,
            "{failures} out of {total} lines failed to parse"
        );

        // Spot-checks: the majority of typed messages should have their expected fields
        assert!(msg1_total > 0, "Expected some MSG,1 messages in the sample");
        assert!(
            msg1_with_callsign as f64 / msg1_total as f64 > 0.9,
            "Expected >90% of MSG,1 to have callsign, got {msg1_with_callsign}/{msg1_total}"
        );

        assert!(msg3_total > 0, "Expected some MSG,3 messages in the sample");
        assert!(
            msg3_with_position as f64 / msg3_total as f64 > 0.9,
            "Expected >90% of MSG,3 to have position+altitude, got {msg3_with_position}/{msg3_total}"
        );

        assert!(msg4_total > 0, "Expected some MSG,4 messages in the sample");
        assert!(
            msg4_with_velocity as f64 / msg4_total as f64 > 0.9,
            "Expected >90% of MSG,4 to have velocity, got {msg4_with_velocity}/{msg4_total}"
        );
    }
}
