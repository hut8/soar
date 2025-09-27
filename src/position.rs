use anyhow::Result;
use chrono::{DateTime, Utc};
use num_traits::AsPrimitive;
use ogn_parser::AprsPacket;
use serde::{Deserialize, Serialize};

use crate::devices::AddressType;
use crate::ogn_aprs_aircraft::{AdsbEmitterCategory, AircraftType};

/// A position fix representing an aircraft's location and associated data
/// This is the main domain entity for position updates, agnostic to source (APRS) and destination (database, NATS)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fix {
    /// Unique identifier for this fix
    pub id: uuid::Uuid,
    /// Raw APRS packet content
    pub raw_packet: String,
    /// Source callsign (from APRS packet header)
    pub source: String,
    /// Destination (from APRS packet header)
    pub destination: String,
    /// Via/relay stations (from APRS packet header)
    pub via: Vec<String>,

    /// Timestamp when this fix was received/parsed
    pub timestamp: DateTime<Utc>,

    /// Timestamp when we received/processed the packet
    pub received_at: DateTime<Utc>,

    /// Lag between packet timestamp and when we received it (in milliseconds)
    pub lag: Option<i32>,

    /// Aircraft position
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_feet: Option<i32>,

    /// Aircraft identification from OGN parameters
    pub device_address: Option<u32>, // Raw device address from OGN parameters (numeric)
    pub device_address_hex: Option<String>, // Hex string representation of device address
    pub address_type: Option<AddressType>,
    pub aircraft_type: Option<AircraftType>,

    /// Flight information
    pub flight_id: Option<uuid::Uuid>,
    pub flight_number: Option<String>,
    pub emitter_category: Option<AdsbEmitterCategory>,
    pub registration: Option<String>,
    pub model: Option<String>,
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

    /// Club association (to be implemented later)
    pub club_name: Option<String>,
    pub club_id: Option<uuid::Uuid>,

    /// Unparsed portion of the packet (if any)
    pub unparsed_data: Option<String>,
}

impl Fix {
    /// Create a Fix from an APRS packet
    /// Returns Ok(None) if the packet doesn't represent a position fix
    /// Returns Ok(Some(fix)) for valid position fixes
    /// Returns Err for parsing failures
    pub fn from_aprs_packet(
        packet: AprsPacket,
        received_at: DateTime<Utc>,
    ) -> Result<Option<Self>> {
        // For now, use received_at as the packet timestamp
        let timestamp = received_at;

        // Calculate lag (difference between received_at and timestamp in milliseconds)
        let lag = Some((received_at - timestamp).num_milliseconds() as i32);

        // Extract source, destination, and via from packet header
        let source = packet.from.to_string();
        let destination = packet.to.to_string();
        let via = packet.via.iter().map(|v| v.to_string()).collect();

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
                let mut device_address = None;
                let mut address_type = None;
                let mut aircraft_type = None;
                let flight_number = None;
                let emitter_category = None;
                let registration = None;
                let model = None;
                let squawk = None;
                let climb_fpm = None;
                let turn_rate_rot = None;
                let snr_db = None;
                let bit_errors_corrected = None;
                let freq_offset_khz = None;

                // Try to parse OGN parameters from comment
                if let Some(ref id) = pos_packet.comment.id {
                    // Use pre-parsed ID information
                    device_address = Some(id.address);
                    address_type = Some(match id.address_type {
                        0 => AddressType::Unknown,
                        1 => AddressType::Icao,
                        2 => AddressType::Flarm,
                        3 => AddressType::Ogn,
                        _ => AddressType::Unknown,
                    });
                    // Extract aircraft type from the OGN parameters
                    aircraft_type = Some(AircraftType::from(id.aircraft_type));
                }

                Ok(Some(Fix {
                    id: uuid::Uuid::new_v4(),
                    raw_packet: packet.raw.unwrap_or_default(),
                    source,
                    destination,
                    via,
                    timestamp,
                    received_at,
                    lag,
                    latitude,
                    longitude,
                    altitude_feet,
                    device_address,
                    device_address_hex: device_address.map(|addr| format!("{:06X}", addr)),
                    address_type,
                    aircraft_type,
                    flight_id: None, // Will be set by flight detection processor
                    flight_number,
                    emitter_category,
                    registration,
                    model,
                    squawk,
                    ground_speed_knots,
                    track_degrees,
                    climb_fpm,
                    turn_rate_rot,
                    snr_db,
                    bit_errors_corrected,
                    freq_offset_khz,
                    club_name: None, // To be implemented later
                    club_id: None,   // To be set by processors
                    unparsed_data: pos_packet.comment.unparsed.clone(),
                }))
            }
            _ => {
                // Non-position packets (status, comment, etc.) return None
                Ok(None)
            }
        }
    }

    /// Convert device address to 6-character uppercase hex string
    pub fn device_address_hex(&self) -> String {
        match self.device_address {
            Some(addr) => format!("{:06X}", addr),
            None => "000000".to_string(),
        }
    }

    /// Get a human-readable aircraft identifier
    /// Uses registration if available, otherwise falls back to aircraft ID with type prefix
    pub fn get_aircraft_identifier(&self) -> Option<String> {
        if let Some(ref reg) = self.registration {
            Some(reg.clone())
        } else if let (Some(_device_address), Some(addr_type)) =
            (&self.device_address, &self.address_type)
        {
            let type_prefix = match *addr_type {
                AddressType::Icao => "ICAO",
                AddressType::Flarm => "FLARM",
                AddressType::Ogn => "OGN",
                AddressType::Unknown => "Unknown",
            };
            Some(format!("{}-{}", type_prefix, self.device_address_hex()))
        } else {
            None
        }
    }
}
