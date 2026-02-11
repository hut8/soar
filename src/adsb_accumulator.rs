//! ADS-B/SBS message accumulator for combining data from multiple message types
//!
//! ADS-B and SBS protocols send different data in separate message types:
//! - Position messages (ADS-B TC 9-18, SBS MSG,3): lat/lon/altitude
//! - Velocity messages (ADS-B TC 19, SBS MSG,4): speed, track, climb rate
//! - Identification messages (ADS-B TC 1-4, SBS MSG,1): callsign
//!
//! This accumulator maintains per-aircraft state and emits fixes whenever
//! enough data is available (at minimum: valid position).

use anyhow::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use rs1090::decode::adsb::{ADSB, ME};
use rs1090::decode::bds::bds09::AirborneVelocitySubType;
use rs1090::decode::{Capability, DF};
use rs1090::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, error, trace};

use crate::beast::cpr_decoder::CprDecoder;
use crate::sbs::parser::{SbsMessage, SbsMessageType};

/// How long to cache aircraft state before expiring (seconds)
/// Matches CPR decoder frame expiry
const STATE_EXPIRY_SECONDS: i64 = 10;

/// Which message type triggered the fix creation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixTrigger {
    /// Triggered by position update (ADS-B TC 9-18, SBS MSG,3)
    Position,
    /// Triggered by velocity update (ADS-B TC 19, SBS MSG,4)
    Velocity,
    /// Triggered by altitude-only update (SBS MSG,5, MSG,7)
    Altitude,
    /// Triggered by identification update (ADS-B TC 1-4, SBS MSG,1)
    Identification,
    /// Triggered by squawk update (SBS MSG,6)
    Squawk,
}

impl std::fmt::Display for FixTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FixTrigger::Position => write!(f, "position"),
            FixTrigger::Velocity => write!(f, "velocity"),
            FixTrigger::Altitude => write!(f, "altitude"),
            FixTrigger::Identification => write!(f, "identification"),
            FixTrigger::Squawk => write!(f, "squawk"),
        }
    }
}

/// Position data with timestamp for expiry tracking
#[derive(Debug, Clone)]
pub struct PositionData {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_feet: Option<i32>,
    pub timestamp: DateTime<Utc>,
}

impl PositionData {
    /// Check if position coordinates are valid (not 0,0 and within valid ranges)
    pub fn is_valid(&self) -> bool {
        // Reject (0, 0) which indicates missing/incomplete position data
        if self.latitude.abs() < 0.001 && self.longitude.abs() < 0.001 {
            return false;
        }
        // Validate coordinate ranges
        (-90.0..=90.0).contains(&self.latitude) && (-180.0..=180.0).contains(&self.longitude)
    }

    /// Check if position is expired based on given current time
    pub fn is_expired(&self, current_time: DateTime<Utc>) -> bool {
        let age = current_time.signed_duration_since(self.timestamp);
        age.num_seconds() > STATE_EXPIRY_SECONDS
    }
}

/// Velocity data with timestamp for expiry tracking
#[derive(Debug, Clone)]
pub struct VelocityData {
    pub ground_speed_knots: Option<f32>,
    pub track_degrees: Option<f32>,
    pub vertical_rate_fpm: Option<i32>,
    pub timestamp: DateTime<Utc>,
}

impl VelocityData {
    /// Check if velocity is expired based on given current time
    pub fn is_expired(&self, current_time: DateTime<Utc>) -> bool {
        let age = current_time.signed_duration_since(self.timestamp);
        age.num_seconds() > STATE_EXPIRY_SECONDS
    }
}

/// Accumulated state for a single aircraft
#[derive(Debug, Clone)]
pub struct AccumulatedAircraftState {
    /// Last decoded position (from CPR decoder or SBS MSG,3)
    pub position: Option<PositionData>,

    /// Last velocity data (from ADS-B TC 19 or SBS MSG,4)
    pub velocity: Option<VelocityData>,

    /// Last known callsign (from ADS-B TC 1-4 or SBS MSG,1)
    pub callsign: Option<String>,

    /// Last known squawk code
    pub squawk: Option<String>,

    /// Whether aircraft is on ground (from ADS-B capability or SBS on_ground)
    pub on_ground: Option<bool>,

    /// Timestamp of last update (for entry expiry)
    pub last_update: DateTime<Utc>,

    /// Count of consecutive messages processed without emitting a fix
    pub consecutive_no_fix: u32,

    /// Whether we've already logged a warning for consecutive no-fix messages
    pub warned_no_fix: bool,
}

impl AccumulatedAircraftState {
    /// Create a new empty state
    pub fn new() -> Self {
        Self {
            position: None,
            velocity: None,
            callsign: None,
            squawk: None,
            on_ground: None,
            last_update: Utc::now(),
            consecutive_no_fix: 0,
            warned_no_fix: false,
        }
    }

    /// Get valid (non-expired) position if available
    pub fn valid_position(&self, current_time: DateTime<Utc>) -> Option<&PositionData> {
        self.position
            .as_ref()
            .filter(|p| p.is_valid() && !p.is_expired(current_time))
    }

    /// Get valid (non-expired) velocity if available
    pub fn valid_velocity(&self, current_time: DateTime<Utc>) -> Option<&VelocityData> {
        self.velocity
            .as_ref()
            .filter(|v| !v.is_expired(current_time))
    }

    /// Check if this state is completely expired and can be removed
    pub fn is_expired(&self, current_time: DateTime<Utc>) -> bool {
        let age = current_time.signed_duration_since(self.last_update);
        age.num_seconds() > STATE_EXPIRY_SECONDS
    }
}

impl Default for AccumulatedAircraftState {
    fn default() -> Self {
        Self::new()
    }
}

/// Partial fix data before aircraft lookup
/// Contains all the accumulated data, caller completes with aircraft_id, raw_message_id, etc.
#[derive(Debug, Clone)]
pub struct PartialFix {
    /// ICAO address as hex string (e.g., "AB1234")
    pub icao_hex: String,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_feet: Option<i32>,
    pub ground_speed_knots: Option<f32>,
    pub track_degrees: Option<f32>,
    pub vertical_rate_fpm: Option<i32>,
    pub callsign: Option<String>,
    pub squawk: Option<String>,
    pub timestamp: DateTime<Utc>,
    /// Age of position data in milliseconds (how old the cached position is)
    pub position_age_ms: i64,
    /// Whether aircraft is on the ground (from ADS-B capability or SBS on_ground field)
    /// true = on ground, false = airborne, None = unknown
    pub on_ground: Option<bool>,
}

/// How often to run cleanup (every N messages)
const CLEANUP_INTERVAL: u64 = 1000;

/// Number of consecutive messages without a fix before logging a warning.
/// This helps detect situations where an aircraft is sending messages but
/// we can never emit a fix (e.g., missing on_ground status, CPR decoding issues).
const NO_FIX_WARNING_THRESHOLD: u32 = 10;

/// Thread-safe accumulator for combining ADS-B/SBS message data
pub struct AdsbAccumulator {
    /// Per-aircraft state map (ICAO address -> state)
    /// Using DashMap for concurrent per-key access (no global lock)
    states: Arc<DashMap<u32, AccumulatedAircraftState>>,

    /// CPR decoder for ADS-B position decoding
    cpr_decoder: CprDecoder,

    /// Message counter for periodic cleanup (avoids cleanup on every message)
    message_count: AtomicU64,
}

impl Default for AdsbAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl AdsbAccumulator {
    /// Create a new accumulator
    ///
    /// Uses global CPR decoding which requires both even and odd frames
    /// to decode a position.
    pub fn new() -> Self {
        Self {
            states: Arc::new(DashMap::new()),
            cpr_decoder: CprDecoder::new(),
            message_count: AtomicU64::new(0),
        }
    }

    /// Process an ADS-B message and potentially emit a fix
    ///
    /// Returns `Some((PartialFix, FixTrigger))` if we have enough data to emit a fix,
    /// `None` otherwise.
    pub fn process_adsb_message(
        &self,
        message: &Message,
        raw_frame: &[u8],
        timestamp: DateTime<Utc>,
        icao_address: u32,
    ) -> Result<Option<(PartialFix, FixTrigger)>> {
        // Determine what type of data this message contains
        let message_data = self.extract_adsb_data(message, raw_frame, timestamp, icao_address)?;

        if message_data.is_empty() {
            // Message type we don't handle (e.g., surface position, all call reply)
            return Ok(None);
        }

        // Update state and determine trigger
        let trigger = self.update_state(icao_address, timestamp, &message_data);

        // Try to emit a fix
        let result = self.try_emit_fix(icao_address, timestamp, trigger);

        // Track consecutive messages without fix emission
        self.track_no_fix(icao_address, result.is_some());

        // Periodic cleanup of expired states (every CLEANUP_INTERVAL messages)
        let count = self.message_count.fetch_add(1, Ordering::Relaxed);
        if count.is_multiple_of(CLEANUP_INTERVAL) {
            self.cleanup_expired(timestamp);
        }

        // Update metrics
        metrics::counter!("adsb_accumulator.adsb_processed_total").increment(1);

        Ok(result)
    }

    /// Process an SBS message and potentially emit a fix
    ///
    /// Returns `Some((PartialFix, FixTrigger))` if we have enough data to emit a fix,
    /// `None` otherwise.
    pub fn process_sbs_message(
        &self,
        sbs_msg: &SbsMessage,
        timestamp: DateTime<Utc>,
    ) -> Result<Option<(PartialFix, FixTrigger)>> {
        let icao_address = sbs_msg
            .icao_address()
            .ok_or_else(|| anyhow::anyhow!("Invalid ICAO address in SBS message"))?;

        // Extract data from SBS message
        let message_data = self.extract_sbs_data(sbs_msg, timestamp);

        if message_data.is_empty() {
            // Message type we don't handle (e.g., MSG,8 All Call Reply)
            return Ok(None);
        }

        // Update state and determine trigger
        let trigger = self.update_state(icao_address, timestamp, &message_data);

        // Try to emit a fix
        let result = self.try_emit_fix(icao_address, timestamp, trigger);

        // Track consecutive messages without fix emission
        self.track_no_fix(icao_address, result.is_some());

        // Periodic cleanup - shares counter with ADS-B processing
        // (already incremented in process_adsb_message, but SBS also increments)
        let count = self.message_count.fetch_add(1, Ordering::Relaxed);
        if count.is_multiple_of(CLEANUP_INTERVAL) {
            self.cleanup_expired(timestamp);
        }

        // Update metrics
        metrics::counter!("adsb_accumulator.sbs_processed_total").increment(1);

        Ok(result)
    }

    /// Get the number of aircraft currently tracked
    pub fn tracked_aircraft_count(&self) -> usize {
        self.states.len()
    }

    /// Extract data from an ADS-B message
    fn extract_adsb_data(
        &self,
        message: &Message,
        raw_frame: &[u8],
        timestamp: DateTime<Utc>,
        icao_address: u32,
    ) -> Result<MessageData> {
        let mut data = MessageData::default();

        // Try CPR position decoding first
        if let Ok(Some(decoded_pos)) =
            self.cpr_decoder
                .decode_message(message, timestamp, icao_address, raw_frame.to_vec())
        {
            // Got a valid position from CPR decoder
            if is_valid_position(decoded_pos.latitude, decoded_pos.longitude) {
                data.position = Some(PositionData {
                    latitude: decoded_pos.latitude,
                    longitude: decoded_pos.longitude,
                    altitude_feet: decoded_pos.altitude_feet,
                    timestamp,
                });
            }
        }

        // Extract velocity from TC 19 messages
        if let Some(velocity) = self.extract_adsb_velocity(message, timestamp) {
            data.velocity = Some(velocity);
        }

        // Extract callsign from TC 1-4 messages
        if let Some(callsign) = self.extract_adsb_callsign(message) {
            data.callsign = Some(callsign);
        }

        // Extract altitude from position messages (even if CPR decoding incomplete)
        if data.position.is_none()
            && let Some(altitude) = self.extract_adsb_altitude(message)
        {
            data.altitude_only = Some(altitude);
        }

        // Extract on_ground status from capability field or vertical status
        if let Some(on_ground) = self.extract_adsb_on_ground(message) {
            data.on_ground = Some(on_ground);
        }

        Ok(data)
    }

    /// Extract the ADSB struct from a DF17 message
    fn get_adsb<'a>(&self, message: &'a Message) -> Option<&'a ADSB> {
        match &message.df {
            DF::ExtendedSquitterADSB(adsb) => Some(adsb),
            _ => None,
        }
    }

    /// Extract velocity data from ADS-B velocity message (BDS09, TC 19)
    fn extract_adsb_velocity(
        &self,
        message: &Message,
        timestamp: DateTime<Utc>,
    ) -> Option<VelocityData> {
        let adsb = self.get_adsb(message)?;

        // BDS09 contains velocity data
        if let ME::BDS09(velocity) = &adsb.message {
            // Extract ground speed and track from GroundSpeedDecoding subtype
            let (ground_speed_knots, track_degrees) = match &velocity.velocity {
                AirborneVelocitySubType::GroundSpeedDecoding(gsd) => {
                    (Some(gsd.groundspeed as f32), Some(gsd.track as f32))
                }
                AirborneVelocitySubType::AirspeedSubsonic(asd) => {
                    // Airspeed messages have heading instead of track, and airspeed instead of groundspeed
                    (
                        asd.airspeed.map(|a| a as f32),
                        asd.heading.map(|h| h as f32),
                    )
                }
                AirborneVelocitySubType::AirspeedSupersonic(asd) => {
                    (asd.airspeed.map(|a| a as f32), asd.heading)
                }
                _ => (None, None),
            };

            // Vertical rate is in ft/min
            let vertical_rate_fpm = velocity.vertical_rate.map(|v| v as i32);

            return Some(VelocityData {
                ground_speed_knots,
                track_degrees,
                vertical_rate_fpm,
                timestamp,
            });
        }

        None
    }

    /// Extract callsign from ADS-B identification message (BDS08, TC 1-4)
    fn extract_adsb_callsign(&self, message: &Message) -> Option<String> {
        let adsb = self.get_adsb(message)?;

        // BDS08 contains aircraft identification (callsign)
        if let ME::BDS08 { inner, .. } = &adsb.message {
            let callsign = inner.callsign.trim().to_string();
            if !callsign.is_empty() {
                return Some(callsign);
            }
        }

        None
    }

    /// Extract altitude from ADS-B position message (BDS05, without full CPR decoding)
    fn extract_adsb_altitude(&self, message: &Message) -> Option<i32> {
        let adsb = self.get_adsb(message)?;

        // BDS05 contains airborne position with altitude
        if let ME::BDS05 { inner, .. } = &adsb.message {
            return inner.alt;
        }

        None
    }

    /// Extract on_ground status from ADS-B capability field
    ///
    /// The capability field in DF17 messages indicates ground/airborne status:
    /// - AG_GROUND: on ground
    /// - AG_AIRBORNE: airborne
    /// - Others: unknown
    fn extract_adsb_on_ground(&self, message: &Message) -> Option<bool> {
        let adsb = self.get_adsb(message)?;

        match adsb.capability {
            Capability::AG_GROUND => Some(true),
            Capability::AG_AIRBORNE => Some(false),
            // AG_LEVEL1, AG_RESERVED, AG_GROUND_AIRBORNE, AG_DR0 don't give definitive status
            _ => None,
        }
    }

    /// Extract data from an SBS message
    fn extract_sbs_data(&self, sbs_msg: &SbsMessage, timestamp: DateTime<Utc>) -> MessageData {
        let mut data = MessageData::default();

        match sbs_msg.message_type {
            SbsMessageType::EsIdentification => {
                // MSG,1: Callsign only
                if let Some(ref callsign) = sbs_msg.callsign {
                    data.callsign = Some(callsign.clone());
                }
            }
            SbsMessageType::EsSurfacePosition => {
                // MSG,2: Surface position - we might want to handle this differently
                // For now, treat like airborne position
                if sbs_msg.has_position()
                    && let (Some(lat), Some(lon)) = (sbs_msg.latitude, sbs_msg.longitude)
                    && is_valid_position(lat, lon)
                {
                    data.position = Some(PositionData {
                        latitude: lat,
                        longitude: lon,
                        altitude_feet: sbs_msg.altitude,
                        timestamp,
                    });
                }
            }
            SbsMessageType::EsAirbornePosition => {
                // MSG,3: Full position with altitude
                if sbs_msg.has_position()
                    && let (Some(lat), Some(lon)) = (sbs_msg.latitude, sbs_msg.longitude)
                    && is_valid_position(lat, lon)
                {
                    data.position = Some(PositionData {
                        latitude: lat,
                        longitude: lon,
                        altitude_feet: sbs_msg.altitude,
                        timestamp,
                    });
                }
                // MSG,3 can also have velocity
                if sbs_msg.has_velocity() {
                    data.velocity = Some(VelocityData {
                        ground_speed_knots: sbs_msg.ground_speed,
                        track_degrees: sbs_msg.track,
                        vertical_rate_fpm: sbs_msg.vertical_rate,
                        timestamp,
                    });
                }
            }
            SbsMessageType::EsAirborneVelocity => {
                // MSG,4: Velocity only
                data.velocity = Some(VelocityData {
                    ground_speed_knots: sbs_msg.ground_speed,
                    track_degrees: sbs_msg.track,
                    vertical_rate_fpm: sbs_msg.vertical_rate,
                    timestamp,
                });
            }
            SbsMessageType::SurveillanceAlt | SbsMessageType::AirToAir => {
                // MSG,5 and MSG,7: Altitude only
                if let Some(alt) = sbs_msg.altitude {
                    data.altitude_only = Some(alt);
                }
            }
            SbsMessageType::SurveillanceId => {
                // MSG,6: Squawk
                if let Some(ref squawk) = sbs_msg.squawk {
                    data.squawk = Some(squawk.clone());
                }
            }
            SbsMessageType::AllCallReply => {
                // MSG,8: No useful data
            }
        }

        // on_ground can be present in any SBS message type
        if let Some(on_ground) = sbs_msg.on_ground {
            data.on_ground = Some(on_ground);
        }

        // For surface position messages (MSG,2), aircraft is definitely on ground
        if sbs_msg.message_type == SbsMessageType::EsSurfacePosition {
            data.on_ground = Some(true);
        }

        data
    }

    /// Update aircraft state with new data and return the trigger type
    fn update_state(
        &self,
        icao_address: u32,
        timestamp: DateTime<Utc>,
        data: &MessageData,
    ) -> FixTrigger {
        let mut trigger = FixTrigger::Position; // Default, will be overwritten

        // Get or create state entry
        let mut entry = self.states.entry(icao_address).or_default();
        let state = entry.value_mut();

        // Update state with new data
        if let Some(ref position) = data.position {
            state.position = Some(position.clone());
            trigger = FixTrigger::Position;
        }

        if let Some(ref velocity) = data.velocity {
            state.velocity = Some(velocity.clone());
            if data.position.is_none() {
                trigger = FixTrigger::Velocity;
            }
        }

        if let Some(ref callsign) = data.callsign {
            state.callsign = Some(callsign.clone());
            if data.position.is_none() && data.velocity.is_none() {
                trigger = FixTrigger::Identification;
            }
        }

        if let Some(ref squawk) = data.squawk {
            state.squawk = Some(squawk.clone());
            if data.position.is_none() && data.velocity.is_none() && data.callsign.is_none() {
                trigger = FixTrigger::Squawk;
            }
        }

        // Handle altitude-only updates
        if let Some(altitude) = data.altitude_only {
            // Update altitude in existing position if we have one
            if let Some(ref mut pos) = state.position {
                pos.altitude_feet = Some(altitude);
            }
            if data.position.is_none()
                && data.velocity.is_none()
                && data.callsign.is_none()
                && data.squawk.is_none()
            {
                trigger = FixTrigger::Altitude;
            }
        }

        // Update on_ground status (from ADS-B capability or SBS on_ground field)
        if let Some(on_ground) = data.on_ground {
            state.on_ground = Some(on_ground);
        }

        state.last_update = timestamp;
        trigger
    }

    /// Try to emit a fix if we have enough data
    ///
    /// Requires both a valid position AND a known on_ground status.
    /// ADS-B transponders authoritatively report air/ground status; without it
    /// we cannot determine is_active and would create spurious flights.
    fn try_emit_fix(
        &self,
        icao_address: u32,
        timestamp: DateTime<Utc>,
        trigger: FixTrigger,
    ) -> Option<(PartialFix, FixTrigger)> {
        let state = self.states.get(&icao_address)?;

        // Must have valid position to emit a fix
        let position = state.valid_position(timestamp)?;

        // Must have on_ground status to determine is_active
        if state.on_ground.is_none() {
            metrics::counter!("adsb_accumulator.fix_skipped_no_on_ground_total").increment(1);
            return None;
        }

        // Get velocity if available (not expired)
        let velocity = state.valid_velocity(timestamp);

        let position_age_ms = timestamp
            .signed_duration_since(position.timestamp)
            .num_milliseconds();

        let partial_fix = PartialFix {
            icao_hex: format!("{:06X}", icao_address),
            latitude: position.latitude,
            longitude: position.longitude,
            altitude_feet: position.altitude_feet,
            ground_speed_knots: velocity.and_then(|v| v.ground_speed_knots),
            track_degrees: velocity.and_then(|v| v.track_degrees),
            vertical_rate_fpm: velocity.and_then(|v| v.vertical_rate_fpm),
            callsign: state.callsign.clone(),
            squawk: state.squawk.clone(),
            timestamp,
            position_age_ms,
            on_ground: state.on_ground,
        };

        trace!(
            "Emitting fix for {:06X} triggered by {}: lat={:.4}, lon={:.4}, alt={:?}, speed={:?}",
            icao_address,
            trigger,
            partial_fix.latitude,
            partial_fix.longitude,
            partial_fix.altitude_feet,
            partial_fix.ground_speed_knots
        );

        // Update metrics by trigger type
        metrics::counter!("adsb_accumulator.fix_emitted_total", "trigger" => trigger.to_string())
            .increment(1);

        Some((partial_fix, trigger))
    }

    /// Track consecutive messages without fix emission for an aircraft.
    /// Logs an error if too many consecutive messages fail to produce a fix.
    fn track_no_fix(&self, icao_address: u32, fix_emitted: bool) {
        if let Some(mut entry) = self.states.get_mut(&icao_address) {
            let state = entry.value_mut();
            if fix_emitted {
                state.consecutive_no_fix = 0;
                state.warned_no_fix = false;
            } else {
                state.consecutive_no_fix += 1;
                if state.consecutive_no_fix > NO_FIX_WARNING_THRESHOLD && !state.warned_no_fix {
                    error!(
                        "Aircraft {:06X}: {} consecutive messages without fix (has_position={}, has_velocity={}, has_callsign={}, on_ground={:?})",
                        icao_address,
                        state.consecutive_no_fix,
                        state.position.is_some(),
                        state.velocity.is_some(),
                        state.callsign.is_some(),
                        state.on_ground,
                    );
                    state.warned_no_fix = true;
                }
            }
        }
    }

    /// Clean up expired state entries
    fn cleanup_expired(&self, current_time: DateTime<Utc>) {
        let before_count = self.states.len();

        self.states.retain(|_icao, state| {
            let keep = !state.is_expired(current_time);
            if !keep {
                metrics::counter!("adsb_accumulator.expiry_total").increment(1);
            }
            keep
        });

        let removed = before_count - self.states.len();
        if removed > 0 {
            debug!("Cleaned up {} expired aircraft states", removed);
        }

        // Update gauge
        metrics::gauge!("adsb_accumulator.state_count").set(self.states.len() as f64);
    }
}

/// Temporary struct to hold extracted message data
#[derive(Default)]
struct MessageData {
    position: Option<PositionData>,
    velocity: Option<VelocityData>,
    callsign: Option<String>,
    squawk: Option<String>,
    altitude_only: Option<i32>,
    on_ground: Option<bool>,
}

impl MessageData {
    fn is_empty(&self) -> bool {
        self.position.is_none()
            && self.velocity.is_none()
            && self.callsign.is_none()
            && self.squawk.is_none()
            && self.altitude_only.is_none()
            && self.on_ground.is_none()
    }
}

/// Check if position coordinates are valid (not 0,0 and within valid ranges)
fn is_valid_position(latitude: f64, longitude: f64) -> bool {
    // Reject (0, 0) which indicates missing/incomplete position data
    if latitude.abs() < 0.001 && longitude.abs() < 0.001 {
        return false;
    }
    // Validate coordinate ranges
    (-90.0..=90.0).contains(&latitude) && (-180.0..=180.0).contains(&longitude)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_trigger_display() {
        assert_eq!(FixTrigger::Position.to_string(), "position");
        assert_eq!(FixTrigger::Velocity.to_string(), "velocity");
        assert_eq!(FixTrigger::Altitude.to_string(), "altitude");
        assert_eq!(FixTrigger::Identification.to_string(), "identification");
        assert_eq!(FixTrigger::Squawk.to_string(), "squawk");
    }

    #[test]
    fn test_position_data_validity() {
        let valid_pos = PositionData {
            latitude: 37.7749,
            longitude: -122.4194,
            altitude_feet: Some(35000),
            timestamp: Utc::now(),
        };
        assert!(valid_pos.is_valid());

        // (0, 0) is invalid
        let zero_pos = PositionData {
            latitude: 0.0,
            longitude: 0.0,
            altitude_feet: Some(35000),
            timestamp: Utc::now(),
        };
        assert!(!zero_pos.is_valid());

        // Out of range is invalid
        let out_of_range = PositionData {
            latitude: 91.0,
            longitude: 0.0,
            altitude_feet: None,
            timestamp: Utc::now(),
        };
        assert!(!out_of_range.is_valid());
    }

    #[test]
    fn test_position_expiry() {
        let now = Utc::now();
        let old_timestamp = now - chrono::Duration::seconds(15);

        let expired_pos = PositionData {
            latitude: 37.7749,
            longitude: -122.4194,
            altitude_feet: Some(35000),
            timestamp: old_timestamp,
        };
        assert!(expired_pos.is_expired(now));

        let fresh_pos = PositionData {
            latitude: 37.7749,
            longitude: -122.4194,
            altitude_feet: Some(35000),
            timestamp: now - chrono::Duration::seconds(5),
        };
        assert!(!fresh_pos.is_expired(now));
    }

    #[test]
    fn test_accumulator_creation() {
        let acc = AdsbAccumulator::new();
        assert_eq!(acc.tracked_aircraft_count(), 0);
    }

    #[test]
    fn test_sbs_position_extraction() {
        let acc = AdsbAccumulator::new();
        let now = Utc::now();

        let sbs_msg = SbsMessage {
            message_type: SbsMessageType::EsAirbornePosition,
            transmission_type: None,
            session_id: None,
            aircraft_id: "AB1234".to_string(),
            is_military: None,
            callsign: None,
            altitude: Some(35000),
            ground_speed: Some(450.0),
            track: Some(90.0),
            latitude: Some(37.7749),
            longitude: Some(-122.4194),
            vertical_rate: Some(-500),
            squawk: None,
            alert: None,
            emergency: None,
            spi: None,
            on_ground: None,
        };

        let data = acc.extract_sbs_data(&sbs_msg, now);
        assert!(data.position.is_some());
        assert!(data.velocity.is_some());

        let pos = data.position.unwrap();
        assert!((pos.latitude - 37.7749).abs() < 0.0001);
        assert!((pos.longitude - (-122.4194)).abs() < 0.0001);
        assert_eq!(pos.altitude_feet, Some(35000));
    }

    #[test]
    fn test_sbs_velocity_only() {
        let acc = AdsbAccumulator::new();
        let now = Utc::now();

        let sbs_msg = SbsMessage {
            message_type: SbsMessageType::EsAirborneVelocity,
            transmission_type: None,
            session_id: None,
            aircraft_id: "AB1234".to_string(),
            is_military: None,
            callsign: None,
            altitude: None,
            ground_speed: Some(450.0),
            track: Some(90.0),
            latitude: None,
            longitude: None,
            vertical_rate: Some(-500),
            squawk: None,
            alert: None,
            emergency: None,
            spi: None,
            on_ground: None,
        };

        let data = acc.extract_sbs_data(&sbs_msg, now);
        assert!(data.position.is_none());
        assert!(data.velocity.is_some());

        let vel = data.velocity.unwrap();
        assert_eq!(vel.ground_speed_knots, Some(450.0));
        assert_eq!(vel.track_degrees, Some(90.0));
        assert_eq!(vel.vertical_rate_fpm, Some(-500));
    }

    #[test]
    fn test_velocity_with_cached_position_emits_fix() {
        let acc = AdsbAccumulator::new();
        let now = Utc::now();

        // First, add a position via MSG,3 with on_ground status
        let pos_msg = SbsMessage {
            message_type: SbsMessageType::EsAirbornePosition,
            transmission_type: None,
            session_id: None,
            aircraft_id: "AB1234".to_string(),
            is_military: None,
            callsign: None,
            altitude: Some(35000),
            ground_speed: None,
            track: None,
            latitude: Some(37.7749),
            longitude: Some(-122.4194),
            vertical_rate: None,
            squawk: None,
            alert: None,
            emergency: None,
            spi: None,
            on_ground: Some(false),
        };

        let result = acc.process_sbs_message(&pos_msg, now).unwrap();
        assert!(result.is_some());
        let (fix, trigger) = result.unwrap();
        assert_eq!(trigger, FixTrigger::Position);
        assert!((fix.latitude - 37.7749).abs() < 0.0001);

        // Now send velocity-only MSG,4
        let vel_msg = SbsMessage {
            message_type: SbsMessageType::EsAirborneVelocity,
            transmission_type: None,
            session_id: None,
            aircraft_id: "AB1234".to_string(),
            is_military: None,
            callsign: None,
            altitude: None,
            ground_speed: Some(450.0),
            track: Some(90.0),
            latitude: None,
            longitude: None,
            vertical_rate: Some(-500),
            squawk: None,
            alert: None,
            emergency: None,
            spi: None,
            on_ground: None,
        };

        let result = acc.process_sbs_message(&vel_msg, now).unwrap();
        assert!(result.is_some(), "Should emit fix with cached position");

        let (fix, trigger) = result.unwrap();
        assert_eq!(trigger, FixTrigger::Velocity);
        // Should have both position (from cache) and velocity (from this message)
        assert!((fix.latitude - 37.7749).abs() < 0.0001);
        assert_eq!(fix.ground_speed_knots, Some(450.0));
        assert_eq!(fix.track_degrees, Some(90.0));
        assert_eq!(fix.vertical_rate_fpm, Some(-500));
    }

    #[test]
    fn test_velocity_without_position_no_fix() {
        let acc = AdsbAccumulator::new();
        let now = Utc::now();

        // Send velocity-only message for aircraft with no cached position
        // Using a valid hex ICAO address that hasn't been seen before
        let vel_msg = SbsMessage {
            message_type: SbsMessageType::EsAirborneVelocity,
            transmission_type: None,
            session_id: None,
            aircraft_id: "CAFE00".to_string(), // Valid hex ICAO, but never seen before
            is_military: None,
            callsign: None,
            altitude: None,
            ground_speed: Some(450.0),
            track: Some(90.0),
            latitude: None,
            longitude: None,
            vertical_rate: Some(-500),
            squawk: None,
            alert: None,
            emergency: None,
            spi: None,
            on_ground: None,
        };

        let result = acc.process_sbs_message(&vel_msg, now).unwrap();
        assert!(
            result.is_none(),
            "Should not emit fix without cached position"
        );
    }

    #[test]
    fn test_expired_position_no_fix() {
        let acc = AdsbAccumulator::new();
        let old_time = Utc::now() - chrono::Duration::seconds(15);
        let now = Utc::now();

        // Add position at old time with on_ground status
        let pos_msg = SbsMessage {
            message_type: SbsMessageType::EsAirbornePosition,
            transmission_type: None,
            session_id: None,
            aircraft_id: "AB1234".to_string(),
            is_military: None,
            callsign: None,
            altitude: Some(35000),
            ground_speed: None,
            track: None,
            latitude: Some(37.7749),
            longitude: Some(-122.4194),
            vertical_rate: None,
            squawk: None,
            alert: None,
            emergency: None,
            spi: None,
            on_ground: Some(false),
        };

        let _ = acc.process_sbs_message(&pos_msg, old_time).unwrap();

        // Send velocity at current time - position should be expired
        let vel_msg = SbsMessage {
            message_type: SbsMessageType::EsAirborneVelocity,
            transmission_type: None,
            session_id: None,
            aircraft_id: "AB1234".to_string(),
            is_military: None,
            callsign: None,
            altitude: None,
            ground_speed: Some(450.0),
            track: Some(90.0),
            latitude: None,
            longitude: None,
            vertical_rate: Some(-500),
            squawk: None,
            alert: None,
            emergency: None,
            spi: None,
            on_ground: None,
        };

        let result = acc.process_sbs_message(&vel_msg, now).unwrap();
        assert!(
            result.is_none(),
            "Should not emit fix with expired position"
        );
    }

    #[test]
    fn test_zero_position_rejected() {
        let acc = AdsbAccumulator::new();
        let now = Utc::now();

        // Try to add (0, 0) position
        let pos_msg = SbsMessage {
            message_type: SbsMessageType::EsAirbornePosition,
            transmission_type: None,
            session_id: None,
            aircraft_id: "AB1234".to_string(),
            is_military: None,
            callsign: None,
            altitude: Some(35000),
            ground_speed: None,
            track: None,
            latitude: Some(0.0),
            longitude: Some(0.0),
            vertical_rate: None,
            squawk: None,
            alert: None,
            emergency: None,
            spi: None,
            on_ground: None,
        };

        let result = acc.process_sbs_message(&pos_msg, now).unwrap();
        assert!(
            result.is_none(),
            "Should not emit fix with (0, 0) coordinates"
        );
    }

    #[test]
    fn test_position_without_on_ground_no_fix() {
        let acc = AdsbAccumulator::new();
        let now = Utc::now();

        // Valid position but no on_ground status
        let pos_msg = SbsMessage {
            message_type: SbsMessageType::EsAirbornePosition,
            transmission_type: None,
            session_id: None,
            aircraft_id: "AB1234".to_string(),
            is_military: None,
            callsign: None,
            altitude: Some(35000),
            ground_speed: None,
            track: None,
            latitude: Some(37.7749),
            longitude: Some(-122.4194),
            vertical_rate: None,
            squawk: None,
            alert: None,
            emergency: None,
            spi: None,
            on_ground: None,
        };

        let result = acc.process_sbs_message(&pos_msg, now).unwrap();
        assert!(
            result.is_none(),
            "Should not emit fix without on_ground status"
        );
    }

    #[test]
    fn test_position_with_on_ground_emits_fix() {
        let acc = AdsbAccumulator::new();
        let now = Utc::now();

        // Valid position with on_ground status
        let pos_msg = SbsMessage {
            message_type: SbsMessageType::EsAirbornePosition,
            transmission_type: None,
            session_id: None,
            aircraft_id: "AB1234".to_string(),
            is_military: None,
            callsign: None,
            altitude: Some(35000),
            ground_speed: None,
            track: None,
            latitude: Some(37.7749),
            longitude: Some(-122.4194),
            vertical_rate: None,
            squawk: None,
            alert: None,
            emergency: None,
            spi: None,
            on_ground: Some(false),
        };

        let result = acc.process_sbs_message(&pos_msg, now).unwrap();
        assert!(result.is_some(), "Should emit fix with on_ground status");
        let (fix, _) = result.unwrap();
        assert_eq!(fix.on_ground, Some(false));
    }

    #[test]
    fn test_on_ground_true_emits_fix() {
        let acc = AdsbAccumulator::new();
        let now = Utc::now();

        // Surface position (on ground)
        let pos_msg = SbsMessage {
            message_type: SbsMessageType::EsSurfacePosition,
            transmission_type: None,
            session_id: None,
            aircraft_id: "AB1234".to_string(),
            is_military: None,
            callsign: None,
            altitude: None,
            ground_speed: Some(5.0),
            track: Some(180.0),
            latitude: Some(37.7749),
            longitude: Some(-122.4194),
            vertical_rate: None,
            squawk: None,
            alert: None,
            emergency: None,
            spi: None,
            on_ground: None, // MSG,2 sets on_ground=true automatically
        };

        let result = acc.process_sbs_message(&pos_msg, now).unwrap();
        assert!(
            result.is_some(),
            "Should emit fix for surface position (on_ground=true)"
        );
        let (fix, _) = result.unwrap();
        assert_eq!(fix.on_ground, Some(true));
    }
}
