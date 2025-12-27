use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use num_traits::AsPrimitive;
use ogn_parser::AprsPacket;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// Custom serializer for via field to convert Vec to comma-separated string
fn serialize_via<S>(via: &[Option<String>], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let via_string = via
        .iter()
        .map(|opt| opt.as_deref().unwrap_or(""))
        .collect::<Vec<&str>>()
        .join(",");
    serializer.serialize_str(&via_string)
}

/// Custom deserializer for via field to convert comma-separated string to Vec
fn deserialize_via<'de, D>(deserializer: D) -> Result<Vec<Option<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(s.split(',')
        .map(|s| {
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        })
        .collect())
}

/// A position fix representing an aircraft's location and associated data
/// This is the unified domain entity for position updates and database storage
#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::fixes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(rename_all = "camelCase")]
pub struct Fix {
    /// Unique identifier for this fix
    pub id: Uuid,

    /// APRS packet header information
    pub source: String,
    pub aprs_type: String,
    #[serde(serialize_with = "serialize_via", deserialize_with = "deserialize_via")]
    pub via: Vec<Option<String>>, // NOT NULL in DB but contains nullable strings

    /// Timestamp when this fix was received/parsed
    pub timestamp: DateTime<Utc>,

    /// Aircraft position
    pub latitude: f64,
    pub longitude: f64,
    // Note: location and geom fields are skipped as they're computed from lat/lng
    pub altitude_msl_feet: Option<i32>,
    pub altitude_agl_feet: Option<i32>,

    /// Flight information
    pub flight_number: Option<String>,
    pub squawk: Option<String>,

    /// Performance data
    pub ground_speed_knots: Option<f32>,
    pub track_degrees: Option<f32>,
    pub climb_fpm: Option<i32>,
    pub turn_rate_rot: Option<f32>,

    /// Protocol-specific metadata stored as JSONB
    /// For APRS: snr_db, bit_errors_corrected, freq_offset_khz, gnss_*_resolution
    /// For ADS-B: nic, nac_p, nac_v, sil, emergency_status, on_ground, etc.
    pub source_metadata: Option<serde_json::Value>,

    /// Associations
    pub flight_id: Option<Uuid>,
    pub aircraft_id: Uuid,

    /// Timestamp when we received/processed the packet
    pub received_at: DateTime<Utc>,

    /// Whether the aircraft is considered active (ground_speed >= 25 knots)
    #[serde(rename = "active")]
    pub is_active: bool,

    /// Receiver that reported this fix (from via array)
    pub receiver_id: Uuid,

    /// Reference to the raw message that contains the raw packet data
    pub raw_message_id: Uuid,

    /// Whether altitude_agl_feet has been looked up (true even if NULL due to no data)
    pub altitude_agl_valid: bool,

    /// Number of seconds elapsed since the previous fix within the same flight
    /// NULL for the first fix in a flight or for fixes without a flight_id
    pub time_gap_seconds: Option<i32>,
}

/// Extended Fix struct that includes raw packet data from aprs_messages table
/// Used for API responses where raw packet data is needed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixWithRawPacket {
    #[serde(flatten)]
    pub fix: Fix,

    /// Raw APRS packet data (joined from aprs_messages table)
    pub raw_packet: Option<String>,
}

/// Extended Fix struct that includes both raw packet and aircraft information
/// Used for receiver fixes API where aircraft details need to be displayed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixWithAircraftInfo {
    #[serde(flatten)]
    pub fix: Fix,

    /// Raw APRS packet data (joined from aprs_messages table)
    pub raw_packet: Option<String>,

    /// Full aircraft information (joined from aircraft table)
    pub aircraft: Option<crate::actions::views::AircraftView>,
}

/// Extended Fix struct that includes flight metadata for WebSocket streaming
/// Used when streaming fixes to include current flight information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixWithFlightInfo {
    #[serde(flatten)]
    pub fix: Fix,

    /// Current flight information (if fix is part of an active flight)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flight: Option<crate::flights::Flight>,
}

impl FixWithRawPacket {
    /// Create a FixWithRawPacket from a Fix and optional raw packet string
    pub fn new(fix: Fix, raw_packet: Option<String>) -> Self {
        Self { fix, raw_packet }
    }
}

impl FixWithAircraftInfo {
    /// Create a FixWithAircraftInfo from a Fix, raw packet, and aircraft information
    pub fn new(
        fix: Fix,
        raw_packet: Option<String>,
        aircraft: Option<crate::actions::views::AircraftView>,
    ) -> Self {
        Self {
            fix,
            raw_packet,
            aircraft,
        }
    }
}

impl std::ops::Deref for FixWithRawPacket {
    type Target = Fix;

    fn deref(&self) -> &Self::Target {
        &self.fix
    }
}

impl std::ops::DerefMut for FixWithRawPacket {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fix
    }
}

impl std::ops::Deref for FixWithAircraftInfo {
    type Target = Fix;

    fn deref(&self) -> &Self::Target {
        &self.fix
    }
}

impl std::ops::DerefMut for FixWithAircraftInfo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fix
    }
}

impl FixWithFlightInfo {
    /// Create a FixWithFlightInfo from a Fix and optional Flight
    pub fn new(fix: Fix, flight: Option<crate::flights::Flight>) -> Self {
        Self { fix, flight }
    }
}

impl std::ops::Deref for FixWithFlightInfo {
    type Target = Fix;

    fn deref(&self) -> &Self::Target {
        &self.fix
    }
}

impl std::ops::DerefMut for FixWithFlightInfo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fix
    }
}

impl Fix {
    /// Create a Fix from an APRS packet
    /// Returns Ok(None) if the packet doesn't represent a position fix
    /// Returns Ok(Some(fix)) for valid position fixes
    /// Returns Err for parsing failures
    /// Note: device_id, receiver_id, and raw_message_id are all required as they should be
    /// determined before Fix creation
    pub fn from_aprs_packet(
        packet: AprsPacket,
        received_at: DateTime<Utc>,
        aircraft_id: Uuid,
        receiver_id: Uuid,
        raw_message_id: Uuid,
    ) -> Result<Option<Self>> {
        // For now, use received_at as the packet timestamp
        let timestamp = received_at;

        // Extract source, aprs_type, and via from packet header
        let source = packet.from.to_string();
        let aprs_type = packet.to.to_string();
        let via = packet.via.iter().map(|v| Some(v.to_string())).collect();

        // Only process position packets
        match packet.data {
            ogn_parser::AprsData::Position(pos_packet) => {
                let latitude = pos_packet.latitude.as_();
                let longitude = pos_packet.longitude.as_();
                let altitude_feet = pos_packet.comment.altitude;
                let ground_speed_knots = pos_packet.comment.speed.map(|s| s as f32);
                let track_degrees = pos_packet
                    .comment
                    .course
                    .filter(|&c| c < 360)
                    .map(|c| c as f32);

                let flight_number = pos_packet.comment.flight_number.clone();
                let squawk = pos_packet.comment.squawk.clone();
                let climb_fpm = pos_packet.comment.climb_rate.map(|c| c as i32);
                let turn_rate_rot = pos_packet
                    .comment
                    .turn_rate
                    .and_then(|t| t.to_string().parse::<f32>().ok());
                let snr_db = pos_packet
                    .comment
                    .signal_quality
                    .and_then(|s| s.to_string().parse::<f32>().ok());
                let bit_errors_corrected = pos_packet.comment.error.map(|e| e as i32);
                let freq_offset_khz = pos_packet
                    .comment
                    .frequency_offset
                    .and_then(|f| f.to_string().parse::<f32>().ok());

                // Note: OGN parameters (device_address, address_type, aircraft_type) are now
                // parsed and stored on the Aircraft record by fix_processor, not on individual fixes

                // Parse GPS quality field (format: "AxB" where A=horizontal_resolution, B=vertical_resolution, in meters)
                let (gnss_horizontal_resolution, gnss_vertical_resolution) =
                    if let Some(ref gps_quality) = pos_packet.comment.gps_quality {
                        // Parse "AxB" format
                        if let Some((horiz_str, vert_str)) = gps_quality.split_once('x') {
                            let horiz = horiz_str.parse::<i16>().ok();
                            let vert = vert_str.parse::<i16>().ok();
                            (horiz, vert)
                        } else {
                            (None, None)
                        }
                    } else {
                        (None, None)
                    };

                // Determine if aircraft is active based on ground speed
                let is_active = ground_speed_knots.is_none_or(|speed| speed >= 20.0);

                // Build source_metadata JSONB for APRS-specific fields
                let source_metadata = {
                    let mut metadata = serde_json::Map::new();
                    metadata.insert("protocol".to_string(), serde_json::json!("aprs"));

                    if let Some(snr) = snr_db {
                        metadata.insert("snr_db".to_string(), serde_json::json!(snr));
                    }
                    if let Some(errors) = bit_errors_corrected {
                        metadata.insert(
                            "bit_errors_corrected".to_string(),
                            serde_json::json!(errors),
                        );
                    }
                    if let Some(offset) = freq_offset_khz {
                        metadata.insert("freq_offset_khz".to_string(), serde_json::json!(offset));
                    }
                    if let Some(horiz) = gnss_horizontal_resolution {
                        metadata.insert(
                            "gnss_horizontal_resolution".to_string(),
                            serde_json::json!(horiz),
                        );
                    }
                    if let Some(vert) = gnss_vertical_resolution {
                        metadata.insert(
                            "gnss_vertical_resolution".to_string(),
                            serde_json::json!(vert),
                        );
                    }

                    Some(serde_json::Value::Object(metadata))
                };

                Ok(Some(Fix {
                    id: Uuid::now_v7(),
                    source,
                    aprs_type,
                    via,
                    timestamp,
                    latitude,
                    longitude,
                    altitude_msl_feet: altitude_feet,
                    altitude_agl_feet: None, // Will be calculated by processors
                    flight_number,
                    squawk,
                    ground_speed_knots,
                    track_degrees,
                    climb_fpm,
                    turn_rate_rot,
                    source_metadata,
                    flight_id: None, // Will be set by flight detection processor
                    aircraft_id,
                    received_at,
                    is_active,
                    receiver_id,
                    raw_message_id,
                    altitude_agl_valid: false, // Will be set to true when elevation is looked up
                    time_gap_seconds: None,    // Will be set by flight tracker if part of a flight
                }))
            }
            _ => {
                // Non-position packets (status, comment, etc.) return None
                Ok(None)
            }
        }
    }
}
