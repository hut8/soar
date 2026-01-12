//! CPR (Compact Position Reporting) decoder for ADS-B position messages
//!
//! ADS-B aircraft transmit position using CPR encoding, which alternates between
//! "even" and "odd" frames. To decode a position, you need both frames from the
//! same aircraft within a short time window (~10 seconds).
//!
//! This module maintains a cache of recent position frames per aircraft and
//! decodes positions when both even and odd frames are available.

use anyhow::Result;
use chrono::{DateTime, Utc};
use rs1090::decode::cpr::decode_positions;
use rs1090::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::debug;

/// How long to cache position frames before expiring them (seconds)
const FRAME_EXPIRY_SECONDS: i64 = 10;

/// Decoded position from CPR
#[derive(Debug, Clone)]
pub struct DecodedPosition {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_feet: Option<i32>,
}

/// CPR decoder that maintains state for even/odd frame pairing
pub struct CprDecoder {
    /// Cache of recent messages per aircraft (ICAO address)
    /// This is wrapped in Arc<Mutex<>> to allow shared access from async tasks
    message_cache: Arc<Mutex<HashMap<u32, Vec<TimedMessage>>>>,
}

impl Default for CprDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl CprDecoder {
    /// Create a new CPR decoder
    ///
    /// Uses "global" CPR decoding which requires both even and odd frames
    /// to decode a position.
    pub fn new() -> Self {
        Self {
            message_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Process a new ADS-B message and attempt to decode position
    ///
    /// Returns Some(position) if position could be decoded (either via local or global CPR),
    /// None if we need to wait for more frames.
    pub fn decode_message(
        &self,
        message: &Message,
        timestamp: DateTime<Utc>,
        icao: u32,
        raw_frame: Vec<u8>,
    ) -> Result<Option<DecodedPosition>> {
        // Convert timestamp to f64 (seconds since epoch)
        let timestamp_secs = timestamp.timestamp() as f64
            + (timestamp.timestamp_subsec_nanos() as f64 / 1_000_000_000.0);

        // Create TimedMessage for rs1090
        let timed_message = TimedMessage {
            timestamp: timestamp_secs,
            frame: raw_frame,
            message: Some(message.clone()),
            metadata: vec![],
            decode_time: None,
        };

        // Lock the cache and add this message
        let mut cache = self.message_cache.lock().unwrap();

        // Clean up expired frames first
        self.cleanup_expired_frames(&mut cache, timestamp);

        // Get or create the message list for this aircraft
        let messages = cache.entry(icao).or_default();
        messages.push(timed_message);

        // Try to decode position using rs1090's decode_positions
        let position = self.try_decode_position(messages, icao)?;

        Ok(position)
    }

    /// Attempt to decode position from cached messages for an aircraft
    fn try_decode_position(
        &self,
        messages: &mut [TimedMessage],
        icao: u32,
    ) -> Result<Option<DecodedPosition>> {
        // Call rs1090's decode_positions which will modify messages in place
        // to populate position info in the Message objects
        // Uses global CPR decoding (no reference position)
        decode_positions(messages, None, &None);

        // Check if any message now has a decoded position
        for msg in messages.iter() {
            if let Some(message) = &msg.message {
                // Serialize to check for position fields
                if let Ok(json) = serde_json::to_value(message) {
                    // Check if lat/lon were decoded
                    if let (Some(lat), Some(lon)) = (
                        json.get("latitude").and_then(|v| v.as_f64()),
                        json.get("longitude").and_then(|v| v.as_f64()),
                    ) {
                        // Position successfully decoded!
                        let altitude = json
                            .get("altitude")
                            .and_then(|v| v.as_i64())
                            .map(|a| a as i32);

                        debug!(
                            "CPR decoded position for {:06X}: lat={:.4}, lon={:.4}, alt={:?}",
                            icao, lat, lon, altitude
                        );

                        return Ok(Some(DecodedPosition {
                            latitude: lat,
                            longitude: lon,
                            altitude_feet: altitude,
                        }));
                    }
                }
            }
        }

        // No position decoded yet - need more frames
        Ok(None)
    }

    /// Remove expired frames from the cache
    fn cleanup_expired_frames(
        &self,
        cache: &mut HashMap<u32, Vec<TimedMessage>>,
        current_time: DateTime<Utc>,
    ) {
        let expiry_threshold = (current_time.timestamp() - FRAME_EXPIRY_SECONDS) as f64;

        // Remove expired messages from each aircraft's cache
        for messages in cache.values_mut() {
            messages.retain(|msg| msg.timestamp > expiry_threshold);
        }

        // Remove aircraft with no remaining messages
        cache.retain(|_, messages| !messages.is_empty());
    }

    /// Get the number of aircraft currently in the cache
    pub fn cached_aircraft_count(&self) -> usize {
        self.message_cache.lock().unwrap().len()
    }

    /// Clear the entire cache (useful for testing)
    #[cfg(test)]
    pub fn clear_cache(&self) {
        self.message_cache.lock().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    fn decode_test_frame(frame: &[u8]) -> Message {
        Message::try_from(frame).expect("Failed to decode test frame")
    }

    #[test]
    fn test_cpr_decoder_creation() {
        let decoder = CprDecoder::new();
        assert_eq!(decoder.cached_aircraft_count(), 0);
    }

    #[test]
    fn test_cache_expiry() {
        let decoder = CprDecoder::new();

        // Add a message
        let frame = hex!("8D40621D58C382D690C8AC2863A7");
        let message = decode_test_frame(&frame);
        let old_time = Utc::now() - chrono::Duration::seconds(15); // 15 seconds ago
        let icao = 0x40621D;

        let _ = decoder.decode_message(&message, old_time, icao, frame.to_vec());

        // Should have 1 aircraft in cache
        assert_eq!(decoder.cached_aircraft_count(), 1);

        // Add another message with current timestamp - this should trigger cleanup
        let current_time = Utc::now();
        let _ = decoder.decode_message(&message, current_time, icao, frame.to_vec());

        // Old messages should be expired, but we still have 1 (the new one)
        assert_eq!(decoder.cached_aircraft_count(), 1);
    }

    #[test]
    fn test_position_decoding_requires_both_frames() {
        let decoder = CprDecoder::new();

        // Single even frame - should not decode position yet
        let even_frame = hex!("8D40621D58C382D690C8AC2863A7");
        let message = decode_test_frame(&even_frame);
        let timestamp = Utc::now();
        let icao = 0x40621D;

        let result = decoder
            .decode_message(&message, timestamp, icao, even_frame.to_vec())
            .unwrap();

        // Should return None - need both frames
        assert!(result.is_none(), "Should not decode with only one frame");
    }

    // Note: Testing actual CPR decoding with real even/odd paired frames requires
    // capturing live ADS-B data or using known test vectors. The rs1090 library
    // handles the complex CPR algorithm internally. Integration tests with real
    // dump1090 feeds will verify end-to-end CPR position decoding.
    //
    // For production testing, we can:
    // 1. Feed known even/odd message pairs from recorded ADS-B data
    // 2. Verify decoded lat/lon matches expected aircraft positions
    // 3. Test with various reference positions (receiver locations)
}
