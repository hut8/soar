use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use num_traits::AsPrimitive;
use ogn_parser::AprsPacket;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use crate::devices::AddressType;
use crate::ogn_aprs_aircraft::AircraftType;

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
    // Note: location field is skipped as it's computed from lat/lng
    #[serde(rename = "altitude_msl_feet")]
    pub altitude_msl_feet: Option<i32>,
    #[serde(rename = "altitude_agl_feet")]
    pub altitude_agl_feet: Option<i32>,

    /// Aircraft identification - canonically a 24-bit unsigned integer stored as i32 in DB
    pub device_address: i32,
    pub address_type: AddressType,
    pub aircraft_type_ogn: Option<AircraftType>,

    /// Flight information
    pub flight_number: Option<String>,
    pub registration: Option<String>,
    pub squawk: Option<String>,

    /// Performance data
    pub ground_speed_knots: Option<f32>,
    pub track_degrees: Option<f32>,
    pub climb_fpm: Option<i32>,
    pub turn_rate_rot: Option<f32>,

    /// Signal quality
    pub snr_db: Option<f32>,
    pub bit_errors_corrected: Option<i32>,
    pub freq_offset_khz: Option<f32>,

    /// GNSS resolution (units unknown)
    pub gnss_horizontal_resolution: Option<i16>,
    pub gnss_vertical_resolution: Option<i16>,

    /// Associations
    pub flight_id: Option<Uuid>,
    pub device_id: Uuid,

    /// Timestamp when we received/processed the packet
    pub received_at: DateTime<Utc>,

    /// Whether the aircraft is considered active (ground_speed >= 25 knots)
    #[serde(rename = "active")]
    pub is_active: bool,

    /// Receiver that reported this fix (from via array)
    pub receiver_id: Option<Uuid>,

    /// Reference to the APRS message that contains the raw packet data
    pub aprs_message_id: Option<Uuid>,

    /// Whether altitude_agl_feet has been looked up (true even if NULL due to no data)
    pub altitude_agl_valid: bool,
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

/// Extended Fix struct that includes flight metadata for WebSocket streaming
/// Used when streaming fixes to include current flight information
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Note: device_id must be provided as it should be looked up from device_address/address_type
    pub fn from_aprs_packet(
        packet: AprsPacket,
        received_at: DateTime<Utc>,
        device_id: Uuid,
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

                // Initialize OGN-related fields
                let mut device_address = 0i32;
                let mut address_type = AddressType::Unknown;
                let mut aircraft_type_ogn = None;
                let flight_number = pos_packet.comment.flight_number.clone();
                let registration = None;
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

                // Try to parse OGN parameters from comment
                if let Some(ref id) = pos_packet.comment.id {
                    // Use pre-parsed ID information
                    device_address = id.address.as_();
                    address_type = match id.address_type {
                        0 => AddressType::Unknown,
                        1 => AddressType::Icao,
                        2 => AddressType::Flarm,
                        3 => AddressType::Ogn,
                        _ => AddressType::Unknown,
                    };
                    // Extract aircraft type from the OGN parameters
                    aircraft_type_ogn = Some(AircraftType::from(id.aircraft_type));
                }

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

                Ok(Some(Fix {
                    id: Uuid::new_v4(),
                    source,
                    aprs_type,
                    via,
                    timestamp,
                    latitude,
                    longitude,
                    altitude_msl_feet: altitude_feet,
                    altitude_agl_feet: None, // Will be calculated by processors
                    device_address,
                    address_type,
                    aircraft_type_ogn,
                    flight_number,
                    registration,
                    squawk,
                    ground_speed_knots,
                    track_degrees,
                    climb_fpm,
                    turn_rate_rot,
                    snr_db,
                    bit_errors_corrected,
                    freq_offset_khz,
                    gnss_horizontal_resolution,
                    gnss_vertical_resolution,
                    flight_id: None, // Will be set by flight detection processor
                    device_id,
                    received_at,
                    is_active,
                    receiver_id: None,         // Will be set during fix insertion
                    aprs_message_id: None,     // Will be populated during fix processing
                    altitude_agl_valid: false, // Will be set to true when elevation is looked up
                }))
            }
            _ => {
                // Non-position packets (status, comment, etc.) return None
                Ok(None)
            }
        }
    }

    /// Convert device address string to canonical 6-character uppercase hex format
    /// The device address is canonically a 24-bit unsigned integer, but stored as hex string
    pub fn device_address_hex(&self) -> String {
        format!("{:06X}", self.device_address)
    }

    /// Get a human-readable aircraft identifier
    /// Uses registration if available, otherwise falls back to aircraft ID with type prefix
    pub fn get_aircraft_identifier(&self) -> Option<String> {
        if let Some(ref reg) = self.registration {
            Some(reg.clone())
        } else {
            let type_prefix = match self.address_type {
                AddressType::Icao => "ICAO",
                AddressType::Flarm => "FLARM",
                AddressType::Ogn => "OGN",
                AddressType::Unknown => "Unknown",
            };
            Some(format!("{}-{:06X}", type_prefix, self.device_address))
        }
    }
}
