use chrono::DateTime;
use soar::adsb_accumulator::AdsbAccumulator;
use soar::aircraft::AddressType;
use soar::aircraft_repo::AircraftRepository;
use soar::fix_processor::FixProcessor;
use soar::raw_messages_repo::{NewSbsMessage, RawMessagesRepository};
use std::sync::Arc;
use tracing::{debug, warn};
use uuid::Uuid;

/// Process a received SBS (BaseStation) message
/// SBS format is text-based CSV, unlike Beast which is binary.
// Note: Intentionally NOT using #[tracing::instrument] here - it causes trace accumulation
pub(crate) async fn process_sbs_message(
    received_at: DateTime<chrono::Utc>,
    csv_bytes: &[u8],
    aircraft_repo: &AircraftRepository,
    sbs_repo: &RawMessagesRepository,
    fix_processor: &FixProcessor,
    accumulator: &Arc<AdsbAccumulator>,
) {
    let start_time = std::time::Instant::now();

    // Track that we're processing a message
    metrics::counter!("sbs.run.process_sbs_message.called_total").increment(1);

    // Validate minimum message length
    // Minimum SBS CSV: "MSG,1,,,A,,,,,," = 11 chars
    if csv_bytes.len() < 11 {
        warn!(
            "Invalid SBS message: too short ({} bytes, expected at least 11)",
            csv_bytes.len()
        );
        metrics::counter!("sbs.run.invalid_message_total").increment(1);
        return;
    }

    // Calculate and record lag (difference between now and packet timestamp)
    let now = chrono::Utc::now();
    let lag_seconds = (now - received_at).num_milliseconds() as f64 / 1000.0;
    metrics::gauge!("sbs.run.lag_seconds").set(lag_seconds);

    // Decode CSV line as UTF-8
    let csv_line = match std::str::from_utf8(csv_bytes) {
        Ok(line) => line.trim(),
        Err(e) => {
            debug!("Failed to decode SBS message as UTF-8: {}", e);
            metrics::counter!("sbs.run.decode.utf8_failed_total").increment(1);
            return;
        }
    };

    // Parse the SBS CSV message
    let sbs_msg = match soar::sbs::parse_sbs_message(csv_line) {
        Ok(msg) => {
            metrics::counter!("sbs.run.decode.success_total").increment(1);
            msg
        }
        Err(e) => {
            debug!("Failed to parse SBS message '{}': {}", csv_line, e);
            metrics::counter!("sbs.run.decode.failed_total").increment(1);
            return;
        }
    };

    // Extract ICAO address from the aircraft_id field (hex string)
    let icao_address = match sbs_msg.icao_address() {
        Some(icao) => icao,
        None => {
            debug!(
                "Failed to parse ICAO address from SBS aircraft_id: '{}'",
                sbs_msg.aircraft_id
            );
            metrics::counter!("sbs.run.icao_extraction_failed_total").increment(1);
            return;
        }
    };

    if icao_address == 0 {
        metrics::counter!("sbs.run.skipped_zero_address_total").increment(1);
        return;
    }

    // Store raw SBS message in database (stored as UTF-8 bytes with 'sbs' source)
    let raw_message_id = match sbs_repo
        .insert_sbs(NewSbsMessage::new(
            csv_bytes.to_vec(),
            received_at,
            None, // receiver_id - SBS has no receiver concept
            None, // unparsed field
        ))
        .await
    {
        Ok(id) => {
            metrics::counter!("sbs.run.raw_message_stored_total").increment(1);
            id
        }
        Err(e) => {
            warn!("Failed to store raw SBS message: {}", e);
            metrics::counter!("sbs.run.raw_message_store_failed_total").increment(1);
            return;
        }
    };

    // Process SBS message through accumulator (combines position/velocity/callsign)
    let fix_result = match accumulator.process_sbs_message(&sbs_msg, received_at) {
        Ok(result) => result,
        Err(e) => {
            debug!("Failed to process SBS message: {}", e);
            metrics::counter!("sbs.run.accumulator_failed_total").increment(1);
            return;
        }
    };

    // If we got a partial fix, we need to look up the aircraft and complete the fix
    if let Some((partial_fix, trigger)) = fix_result {
        // Get or create aircraft by ICAO address
        let aircraft = match aircraft_repo
            .get_or_insert_aircraft_by_address(icao_address as i32, AddressType::Icao)
            .await
        {
            Ok(aircraft) => aircraft,
            Err(e) => {
                warn!(
                    "Failed to get/create aircraft for ICAO {:06X}: {}",
                    icao_address, e
                );
                metrics::counter!("sbs.run.aircraft_lookup_failed_total").increment(1);
                return;
            }
        };

        // Determine if aircraft is active (in flight vs on ground)
        // The accumulator guarantees on_ground is set before emitting a fix,
        // but handle missing values defensively to avoid panics.
        let is_active = match partial_fix.on_ground {
            Some(on_ground) => !on_ground,
            None => {
                warn!(
                    "Accumulator emitted SBS fix without on_ground; dropping fix for icao_hex={}",
                    partial_fix.icao_hex
                );
                metrics::counter!("sbs.run.fix_missing_on_ground_total").increment(1);
                return;
            }
        };

        // Build source metadata for SBS-specific fields with trigger
        let mut metadata = serde_json::Map::new();
        metadata.insert("protocol".to_string(), serde_json::json!("sbs"));
        metadata.insert(
            "sbs_message_type".to_string(),
            serde_json::json!(sbs_msg.message_type as u8),
        );
        metadata.insert(
            "trigger".to_string(),
            serde_json::json!(trigger.to_string()),
        );
        if partial_fix.position_age_ms > 0 {
            metadata.insert(
                "position_age_ms".to_string(),
                serde_json::json!(partial_fix.position_age_ms),
            );
        }
        if let Some(on_ground) = sbs_msg.on_ground {
            metadata.insert("on_ground".to_string(), serde_json::json!(on_ground));
        }
        if let Some(alert) = sbs_msg.alert {
            metadata.insert("alert".to_string(), serde_json::json!(alert));
        }
        if let Some(emergency) = sbs_msg.emergency {
            metadata.insert("emergency".to_string(), serde_json::json!(emergency));
        }
        if let Some(spi) = sbs_msg.spi {
            metadata.insert("spi".to_string(), serde_json::json!(spi));
        }

        // Build Fix from partial fix
        // Note: SBS/ADS-B protocols don't have a receiver concept, so receiver_id is None
        let fix = soar::Fix {
            id: Uuid::now_v7(),
            source: partial_fix.icao_hex,
            latitude: partial_fix.latitude,
            longitude: partial_fix.longitude,
            altitude_msl_feet: partial_fix.altitude_feet,
            altitude_agl_feet: None, // Will be calculated later
            flight_number: partial_fix.callsign,
            squawk: partial_fix.squawk,
            ground_speed_knots: partial_fix.ground_speed_knots,
            track_degrees: partial_fix.track_degrees,
            climb_fpm: partial_fix.vertical_rate_fpm,
            turn_rate_rot: None, // SBS doesn't provide turn rate
            source_metadata: Some(serde_json::Value::Object(metadata)),
            flight_id: None, // Will be assigned by flight tracker
            aircraft_id: aircraft.id,
            received_at,
            is_active,
            receiver_id: None,         // SBS doesn't have receiver data
            raw_message_id,            // FK to raw_messages table
            altitude_agl_valid: false, // Will be calculated later
            time_gap_seconds: None,    // Will be set by flight tracker
        };

        // Process the fix through FixProcessor
        match fix_processor.process_fix(fix).await {
            Ok(_) => {
                metrics::counter!("sbs.run.fixes_processed_total").increment(1);
            }
            Err(e) => {
                warn!("Failed to process SBS fix: {}", e);
                metrics::counter!("sbs.run.fix_processing_failed_total").increment(1);
            }
        }
    } else {
        // No fix created (insufficient data - need valid position)
        debug!(
            "SBS message type {:?} did not produce a fix (insufficient data)",
            sbs_msg.message_type
        );
        metrics::counter!("sbs.run.no_fix_created_total").increment(1);
    }

    // Record processing latency
    let elapsed_ms = start_time.elapsed().as_millis() as f64;
    metrics::histogram!("sbs.run.message_processing_latency_ms").record(elapsed_ms);
}
