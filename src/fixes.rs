use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use num_traits::AsPrimitive;
use ogn_parser::AprsPacket;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::devices::AddressType;
use crate::ogn_aprs_aircraft::{AdsbEmitterCategory, AircraftType};

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
    pub destination: String,
    pub via: Vec<Option<String>>, // NOT NULL in DB but contains nullable strings

    /// Raw APRS packet for debugging/audit purposes
    pub raw_packet: String,

    /// Timestamp when this fix was received/parsed
    pub timestamp: DateTime<Utc>,

    /// Aircraft position
    pub latitude: f64,
    pub longitude: f64,
    // Note: location field is skipped as it's computed from lat/lng
    pub altitude_feet: Option<i32>,

    /// Aircraft identification - canonically a 24-bit unsigned integer stored as i32 in DB
    pub device_address: i32,
    pub address_type: AddressType,
    pub aircraft_type_ogn: Option<AircraftType>,

    /// Flight information
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

    /// Associations
    pub club_id: Option<Uuid>,
    pub flight_id: Option<Uuid>,
    pub unparsed_data: Option<String>,
    pub device_id: Uuid,

    /// Timestamp when we received/processed the packet
    pub received_at: DateTime<Utc>,

    /// Lag between packet timestamp and when we received it (in milliseconds)
    pub lag: Option<i32>,
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

        // Calculate lag (difference between received_at and timestamp in milliseconds)
        let lag = Some((received_at - timestamp).num_milliseconds() as i32);

        // Extract source, destination, and via from packet header
        let source = packet.from.to_string();
        let destination = packet.to.to_string();
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

                Ok(Some(Fix {
                    id: Uuid::new_v4(),
                    source,
                    destination,
                    via,
                    raw_packet: packet.raw.unwrap_or_default(),
                    timestamp,
                    latitude,
                    longitude,
                    altitude_feet,
                    device_address,
                    address_type,
                    aircraft_type_ogn,
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
                    club_id: None,   // To be set by processors
                    flight_id: None, // Will be set by flight detection processor
                    unparsed_data: pos_packet.comment.unparsed.clone(),
                    device_id,
                    received_at,
                    lag,
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
