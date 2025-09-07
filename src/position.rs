use anyhow::Result;
use chrono::{DateTime, Utc};
use num_traits::AsPrimitive;
use ogn_parser::AprsPacket;
use serde::{Deserialize, Serialize};

use crate::ogn_aprs_aircraft::{AddressType, AdsbEmitterCategory, AircraftType, OgnAprsParameters};

/// A position fix representing an aircraft's location and associated data
/// This is the main domain entity for position updates, agnostic to source (APRS) and destination (database, NATS)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fix {
    /// Source callsign (from APRS packet header)
    pub source: String,
    /// Destination (from APRS packet header)
    pub destination: String,
    /// Via/relay stations (from APRS packet header)
    pub via: Vec<String>,

    /// Timestamp when this fix was received/parsed
    pub timestamp: DateTime<Utc>,

    /// Aircraft position
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_feet: Option<i32>,

    /// Aircraft identification from OGN parameters
    pub aircraft_id: Option<String>, // Hex aircraft ID (e.g., "39D304")
    pub device_id: Option<u32>, // Raw device ID from OGN parameters (numeric)
    pub device_type: Option<AddressType>,
    pub aircraft_type: Option<AircraftType>,

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
    pub bit_errors_corrected: Option<u32>,
    pub freq_offset_khz: Option<f32>,

    /// Club association (to be implemented later)
    pub club_name: Option<String>,
}

impl Fix {
    /// Create a Fix from an APRS packet
    /// Returns Ok(None) if the packet doesn't represent a position fix
    /// Returns Ok(Some(fix)) for valid position fixes
    /// Returns Err for parsing failures
    pub fn from_aprs_packet(packet: AprsPacket) -> Result<Option<Self>> {
        let timestamp = Utc::now();

        // Extract source, destination, and via from packet header
        let source = packet.from.to_string();
        let destination = packet.to.to_string();
        let via = packet.via.iter().map(|v| v.to_string()).collect();

        // Only process position packets
        match packet.data {
            ogn_parser::AprsData::Position(pos_packet) => {
                let latitude = pos_packet.latitude.as_();
                let longitude = pos_packet.longitude.as_();
                let altitude_feet = pos_packet.comment.altitude.map(|a| a as i32);
                let ground_speed_knots = pos_packet.comment.speed.map(|s| s as f32);
                let track_degrees = pos_packet.comment.course
                    .filter(|&c| c < 360)
                    .map(|c| c as f32);

                // Initialize OGN-related fields
                let mut aircraft_id = None;
                let mut device_id = None;
                let mut device_type = None;
                let mut aircraft_type = None;
                let mut flight_number = None;
                let mut emitter_category = None;
                let mut registration = None;
                let mut model = None;
                let mut squawk = None;
                let mut climb_fpm = None;
                let mut turn_rate_rot = None;
                let mut snr_db = None;
                let mut bit_errors_corrected = None;
                let mut freq_offset_khz = None;

                // Try to parse OGN parameters from comment
                if let Some(ref id) = pos_packet.comment.id {
                    // Use pre-parsed ID information
                    aircraft_id = Some(format!("{:06X}", id.address));
                    device_id = Some(id.address);
                    device_type = Some(match id.address_type {
                        0 => AddressType::Unknown,
                        1 => AddressType::Icao,
                        2 => AddressType::Flarm,
                        3 => AddressType::OgnTracker,
                        _ => AddressType::Unknown,
                    });
                    // For now, we'll extract aircraft type from the OGN parameters if available
                    aircraft_type = None;
                } else if let Some(ref comment) = pos_packet.comment.unparsed {
                    // Try to parse OGN parameters from unparsed comment
                    if let Ok(ogn_params) = comment.parse::<OgnAprsParameters>() {
                        aircraft_id = Some(format!("{:06X}", ogn_params.address));
                        device_id = Some(ogn_params.address);
                        device_type = Some(ogn_params.address_type);
                        aircraft_type = Some(ogn_params.aircraft_type);
                        flight_number = ogn_params.flight_number;
                        emitter_category = ogn_params.emitter_category;
                        registration = ogn_params.registration;
                        model = ogn_params.model;
                        squawk = ogn_params.squawk;
                        climb_fpm = ogn_params.climb_fpm;
                        turn_rate_rot = ogn_params.turn_rate_rot;
                        snr_db = ogn_params.snr_db;
                        bit_errors_corrected = ogn_params.bit_errors_corrected;
                        freq_offset_khz = ogn_params.freq_offset_khz;
                    }
                }

                Ok(Some(Fix {
                    source,
                    destination,
                    via,
                    timestamp,
                    latitude,
                    longitude,
                    altitude_feet,
                    aircraft_id,
                    device_id,
                    device_type,
                    aircraft_type,
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
                }))
            }
            _ => {
                // Non-position packets (status, comment, etc.) return None
                Ok(None)
            }
        }
    }

    /// Get a human-readable aircraft identifier
    /// Uses registration if available, otherwise falls back to aircraft ID with type prefix
    pub fn get_aircraft_identifier(&self) -> Option<String> {
        if let Some(ref reg) = self.registration {
            Some(reg.clone())
        } else if let (Some(aircraft_id), Some(dev_type)) = (&self.aircraft_id, &self.device_type)
        {
            let type_prefix = match *dev_type {
                AddressType::Icao => "ICAO",
                AddressType::Flarm => "FLARM",
                AddressType::OgnTracker => "OGN",
                AddressType::Unknown => "Unknown",
            };
            Some(format!("{}-{}", type_prefix, aircraft_id))
        } else {
            None
        }
    }
}
