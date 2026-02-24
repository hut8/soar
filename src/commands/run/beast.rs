use anyhow::Result;
use chrono::DateTime;
use rs1090::decode::DF;
use rs1090::decode::adsb::ME;
use soar::adsb_accumulator::AdsbAccumulator;
use soar::aircraft::AddressType;
use soar::aircraft_repo::AircraftRepository;
use soar::beast::decode_beast_frame;
use soar::fix_processor::FixProcessor;
use soar::raw_messages_repo::{NewBeastMessage, RawMessagesRepository};
use std::sync::Arc;
use tracing::{debug, warn};
use uuid::Uuid;

/// Process a received Beast (ADS-B) message
// Note: Intentionally NOT using #[tracing::instrument] here - it causes trace accumulation
// in Tempo because spawned tasks inherit trace context and all messages end up in one huge trace.
pub(crate) async fn process_beast_message(
    received_at: DateTime<chrono::Utc>,
    raw_frame: &[u8],
    aircraft_repo: &AircraftRepository,
    beast_repo: &RawMessagesRepository,
    fix_processor: &FixProcessor,
    accumulator: &Arc<AdsbAccumulator>,
) {
    let start_time = std::time::Instant::now();

    // Track that we're processing a message
    metrics::counter!("beast.run.process_beast_message.called_total").increment(1);

    // Validate minimum message length
    // Beast frame minimum: 1 (0x1A) + 1 (type) + 6 (timestamp) + 1 (signal) + 2 (Mode A/C payload) = 11 bytes
    if raw_frame.len() < 11 {
        warn!(
            "Invalid Beast message: too short ({} bytes, expected at least 11)",
            raw_frame.len()
        );
        metrics::counter!("beast.run.invalid_message_total").increment(1);
        return;
    }

    // Calculate and record lag (difference between now and packet timestamp)
    let now = chrono::Utc::now();
    let lag_seconds = (now - received_at).num_milliseconds() as f64 / 1000.0;
    metrics::gauge!("beast.run.lag_seconds").set(lag_seconds);

    // Decode the Beast frame using rs1090
    let decoded = match decode_beast_frame(raw_frame, received_at) {
        Ok(decoded) => {
            metrics::counter!("beast.run.decode.success_total").increment(1);
            decoded
        }
        Err(e) => {
            debug!("Failed to decode Beast frame: {}", e);
            metrics::counter!("beast.run.decode.failed_total").increment(1);
            return;
        }
    };

    // Track downlink format (DF) distribution
    let df_label = df_type_label(&decoded.message.df);
    metrics::counter!("beast.run.df_type_total", "df" => df_label).increment(1);

    // For ADS-B (DF17), also track the message entity (ME/BDS) subtype
    if let DF::ExtendedSquitterADSB(ref adsb) = decoded.message.df {
        let me_label = me_type_label(&adsb.message);
        metrics::counter!("beast.run.adsb_bds_type_total", "bds" => me_label).increment(1);
    }

    // Extract ICAO address from the decoded message for aircraft lookup
    let icao_address = match extract_icao_from_message(&decoded.message) {
        Ok(icao) => icao,
        Err(e) => {
            debug!("Failed to extract ICAO address: {}", e);
            metrics::counter!("beast.run.icao_extraction_failed_total").increment(1);
            return;
        }
    };

    // Skip zero ICAO addresses - invalid data
    if icao_address == 0 {
        metrics::counter!("beast.run.skipped_zero_address_total").increment(1);
        return;
    }

    // Store raw Beast message in database
    // ADS-B/Beast messages don't have a receiver concept, so receiver_id is None
    let raw_message_id = match beast_repo
        .insert_beast(NewBeastMessage::new(
            raw_frame.to_vec(),
            received_at,
            None, // receiver_id - ADS-B has no receiver concept
            None, // unparsed field (could add decoded JSON if needed)
        ))
        .await
    {
        Ok(id) => {
            metrics::counter!("beast.run.raw_message_stored_total").increment(1);
            id
        }
        Err(e) => {
            warn!("Failed to store raw Beast message: {}", e);
            metrics::counter!("beast.run.raw_message_store_failed_total").increment(1);
            return;
        }
    };

    // Process ADS-B message through accumulator (combines position/velocity/callsign)
    let fix_result = match accumulator.process_adsb_message(
        &decoded.message,
        raw_frame,
        received_at,
        icao_address,
    ) {
        Ok(result) => result,
        Err(e) => {
            debug!("Failed to process ADS-B message: {}", e);
            metrics::counter!("beast.run.adsb_to_fix_failed_total").increment(1);
            return;
        }
    };

    // If we got a partial fix, look up the aircraft and process through FixProcessor
    if let Some((partial_fix, trigger)) = fix_result {
        // Get or create aircraft by ICAO address (deferred until we have a fix to avoid
        // redundant lookups on non-fix-producing frames like velocity/squawk updates)
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
                metrics::counter!("beast.run.aircraft_lookup_failed_total").increment(1);
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
                    "Accumulator emitted ADS-B fix without on_ground; dropping fix for icao_hex={}",
                    partial_fix.icao_hex
                );
                metrics::counter!("beast.run.fix_missing_on_ground_total").increment(1);
                return;
            }
        };

        // Build source metadata with ADS-B-specific fields and trigger
        let mut metadata = serde_json::Map::new();
        metadata.insert("protocol".to_string(), serde_json::json!("adsb"));
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
            turn_rate_rot: None, // ADS-B doesn't provide turn rate
            source_metadata: Some(serde_json::Value::Object(metadata)),
            flight_id: None, // Will be assigned by flight tracker
            aircraft_id: aircraft.id,
            received_at,
            is_active,
            receiver_id: None, // ADS-B doesn't have a receiver concept
            raw_message_id,
            altitude_agl_valid: false, // Will be calculated later
            time_gap_seconds: None,    // Will be set by flight tracker
        };

        match fix_processor.process_fix(fix).await {
            Ok(_) => {
                metrics::counter!("beast.run.fixes_processed_total").increment(1);
            }
            Err(e) => {
                warn!("Failed to process Beast fix: {}", e);
                metrics::counter!("beast.run.fix_processing_failed_total").increment(1);
            }
        }
    } else {
        // No fix created (insufficient data - need valid position)
        debug!("ADS-B message did not produce a fix (insufficient data for valid fix)");
        metrics::counter!("beast.run.no_fix_created_total").increment(1);
    }

    // Record processing latency
    let elapsed_ms = start_time.elapsed().as_millis() as f64;
    metrics::histogram!("beast.run.message_processing_latency_ms").record(elapsed_ms);
}

/// Return a human-readable label for the Mode S Downlink Format
fn df_type_label(df: &DF) -> &'static str {
    match df {
        DF::ShortAirAirSurveillance { .. } => "DF0 Short Air-Air",
        DF::SurveillanceAltitudeReply { .. } => "DF4 Surveillance Alt",
        DF::SurveillanceIdentityReply { .. } => "DF5 Surveillance ID",
        DF::AllCallReply { .. } => "DF11 All Call Reply",
        DF::LongAirAirSurveillance { .. } => "DF16 Long Air-Air",
        DF::ExtendedSquitterADSB(_) => "DF17 ADS-B",
        DF::ExtendedSquitterTisB { .. } => "DF18 TIS-B",
        DF::ExtendedSquitterMilitary { .. } => "DF19 Military",
        DF::CommBAltitudeReply { .. } => "DF20 Comm-B Alt",
        DF::CommBIdentityReply { .. } => "DF21 Comm-B ID",
        DF::CommDExtended { .. } => "DF24 Comm-D",
    }
}

/// Return a human-readable label for the ADS-B Message Entity (ME/BDS) subtype
fn me_type_label(me: &ME) -> &'static str {
    match me {
        ME::NoPosition(_) => "No Position",
        ME::BDS08 { .. } => "BDS08 Identification",
        ME::BDS06 { .. } => "BDS06 Surface Position",
        ME::BDS05 { .. } => "BDS05 Airborne Position",
        ME::BDS09(_) => "BDS09 Airborne Velocity",
        ME::BDS61(_) => "BDS61 Aircraft Status",
        ME::BDS62(_) => "BDS62 Target State",
        ME::BDS65(_) => "BDS65 Operation Status",
        ME::Reserved0(_) | ME::Reserved1 { .. } => "Reserved",
        ME::SurfaceSystemStatus(_) => "Surface System Status",
        ME::AircraftOperationalCoordination(_) => "Operational Coordination",
    }
}

/// Extract ICAO address from decoded ADS-B message
fn extract_icao_from_message(message: &rs1090::prelude::Message) -> Result<u32> {
    // Serialize to JSON to access icao24 field
    let json = serde_json::to_value(message)?;

    if let Some(icao_str) = json.get("icao24").and_then(|v| v.as_str()) {
        // Parse hex string to u32
        u32::from_str_radix(icao_str, 16)
            .map_err(|e| anyhow::anyhow!("Failed to parse ICAO address '{}': {}", icao_str, e))
    } else {
        // Fallback to CRC for non-ADS-B messages
        debug!("No icao24 field in message, using CRC: {}", message.crc);
        Ok(message.crc)
    }
}
