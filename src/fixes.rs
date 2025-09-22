use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::devices::AddressType;
use crate::ogn_aprs_aircraft::{AdsbEmitterCategory, AircraftType};

/// A position fix stored in the database
/// This represents position data that has been persisted from APRS packets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fix {
    /// Unique identifier for this fix
    pub id: Uuid,

    /// APRS packet header information
    pub source: String,
    pub destination: String,
    pub via: Vec<String>,

    /// Raw APRS packet for debugging/audit purposes
    pub raw_packet: String,

    /// Timestamp when this fix was received/parsed
    pub timestamp: DateTime<Utc>,

    /// Aircraft position
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_feet: Option<i32>,

    /// Aircraft identification
    pub device_address_hex: Option<String>, // Hex device address (e.g., "39D304")
    pub address_type: Option<AddressType>,
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

    /// Club association
    pub club_id: Option<Uuid>,

    /// Unparsed portion of the packet (if any)
    pub unparsed_data: Option<String>,
}

impl Fix {
    /// Create a new Fix from a position fix with raw packet data
    pub fn from_position_fix(position_fix: &crate::position::Fix, raw_packet: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            source: position_fix.source.clone(),
            destination: position_fix.destination.clone(),
            via: position_fix.via.clone(),
            raw_packet,
            timestamp: position_fix.timestamp,
            latitude: position_fix.latitude,
            longitude: position_fix.longitude,
            altitude_feet: position_fix.altitude_feet,
            device_address_hex: position_fix.device_address.map(|addr| format!("{:06X}", addr)),
            address_type: position_fix.address_type,
            aircraft_type: position_fix.aircraft_type,
            flight_number: position_fix.flight_number.clone(),
            emitter_category: position_fix.emitter_category,
            registration: position_fix.registration.clone(),
            model: position_fix.model.clone(),
            squawk: position_fix.squawk.clone(),
            ground_speed_knots: position_fix.ground_speed_knots,
            track_degrees: position_fix.track_degrees,
            climb_fpm: position_fix.climb_fpm,
            turn_rate_rot: position_fix.turn_rate_rot,
            snr_db: position_fix.snr_db,
            bit_errors_corrected: position_fix.bit_errors_corrected,
            freq_offset_khz: position_fix.freq_offset_khz,
            club_id: None, // Will be set by repository based on aircraft registration
            unparsed_data: position_fix.unparsed_data.clone(),
        }
    }

    /// Get a human-readable aircraft identifier
    /// Uses registration if available, otherwise falls back to aircraft ID with type prefix
    pub fn get_aircraft_identifier(&self) -> Option<String> {
        if let Some(ref reg) = self.registration {
            Some(reg.clone())
        } else if let (Some(device_address), Some(addr_type)) =
            (&self.device_address_hex, &self.address_type)
        {
            let type_prefix = match *addr_type {
                AddressType::Icao => "ICAO",
                AddressType::Flarm => "FLARM",
                AddressType::Ogn => "OGN",
                AddressType::Unknown => "Unknown",
            };
            Some(format!("{}-{}", type_prefix, device_address))
        } else {
            None
        }
    }
}
