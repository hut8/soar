use anyhow::{Context, Result};
use chrono::DateTime;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use soar::adsb_accumulator::AdsbAccumulator;
use soar::aircraft::AddressType;
use soar::aircraft_repo::AircraftRepository;
use soar::aircraft_types::AircraftCategory;
use soar::beast::decode_beast_frame;
use soar::fix_processor::FixProcessor;
use soar::flight_tracker::FlightTracker;
use soar::instance_lock::InstanceLock;
use soar::packet_processors::{
    AircraftPositionProcessor, GenericProcessor, PacketRouter, ReceiverPositionProcessor,
    ReceiverStatusProcessor, ServerStatusProcessor,
};
use soar::raw_messages_repo::{NewBeastMessage, NewSbsMessage, RawMessagesRepository};
use soar::receiver_repo::ReceiverRepository;
use soar::receiver_status_repo::ReceiverStatusRepository;
use soar::server_messages_repo::ServerMessagesRepository;
use std::env;
use std::sync::Arc;
use tracing::{debug, error, info, trace, warn};
use uuid::Uuid;

// Queue size constants
const OGN_INTAKE_QUEUE_SIZE: usize = 5000;
const BEAST_INTAKE_QUEUE_SIZE: usize = 1000;
const SBS_INTAKE_QUEUE_SIZE: usize = 1000;
const AIRCRAFT_QUEUE_SIZE: usize = 5000;
const RECEIVER_STATUS_QUEUE_SIZE: usize = 50;
const RECEIVER_POSITION_QUEUE_SIZE: usize = 50;
const SERVER_STATUS_QUEUE_SIZE: usize = 50;

fn queue_warning_threshold(queue_size: usize) -> usize {
    queue_size / 2
}

/// Process a received APRS message by parsing and routing through PacketRouter
// Note: Intentionally NOT using #[tracing::instrument] here - it causes trace accumulation
// in Tempo because spawned tasks inherit trace context and all messages end up in one huge trace.
async fn process_aprs_message(
    received_at: DateTime<chrono::Utc>,
    message: &str,
    packet_router: &soar::packet_processors::PacketRouter,
) {
    let start_time = std::time::Instant::now();

    // Track that we're processing a message
    metrics::counter!("aprs.process_aprs_message.called_total").increment(1);

    // Calculate and record lag (difference between now and packet timestamp)
    let now = chrono::Utc::now();
    let lag_seconds = (now - received_at).num_milliseconds() as f64 / 1000.0;
    metrics::gauge!("aprs.lag_seconds").set(lag_seconds);

    // Route server messages (starting with #) differently
    // Server messages don't create PacketContext
    if message.starts_with('#') {
        debug!("Server message: {}", message);
        packet_router
            .process_server_message(message, received_at)
            .await;
        return;
    }

    // Try to parse the message using ogn-parser
    match ogn_parser::parse(message) {
        Ok(parsed) => {
            // Track successful parse
            metrics::counter!("aprs.parse.success_total").increment(1);

            // Call PacketRouter to archive, process, and route to queues
            packet_router
                .process_packet(parsed, message, received_at)
                .await;

            metrics::counter!("aprs.router.process_packet.called_total").increment(1);
        }
        Err(e) => {
            metrics::counter!("aprs.parse.failed_total").increment(1);
            // For OGNFNT sources with invalid lat/lon, log as trace instead of error
            // These are common and expected issues with this data source
            let error_str = e.to_string();
            // For OGNFNT sources with common parsing issues, log as debug/trace instead of info
            // These are expected issues with this data source and not actionable
            if message.contains("OGNFNT")
                && (error_str.contains("Invalid Latitude")
                    || error_str.contains("Invalid Longitude")
                    || error_str.contains("Unsupported Position Format"))
            {
                trace!("Failed to parse APRS message '{message}': {e}");
            } else {
                debug!("Failed to parse APRS message '{message}': {e}");
            }
        }
    }

    // Record processing latency
    let elapsed_micros = start_time.elapsed().as_micros() as f64 / 1000.0; // Convert to milliseconds
    metrics::histogram!("aprs.message_processing_latency_ms").record(elapsed_micros);
}

/// Process a received Beast (ADS-B) message
// Note: Intentionally NOT using #[tracing::instrument] here - it causes trace accumulation
// in Tempo because spawned tasks inherit trace context and all messages end up in one huge trace.
async fn process_beast_message(
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

    // Extract ICAO address from the decoded message for aircraft lookup
    let icao_address = match extract_icao_from_message(&decoded.message) {
        Ok(icao) => icao,
        Err(e) => {
            debug!("Failed to extract ICAO address: {}", e);
            metrics::counter!("beast.run.icao_extraction_failed_total").increment(1);
            return;
        }
    };

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
            metrics::counter!("beast.run.aircraft_lookup_failed_total").increment(1);
            return;
        }
    };

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

    // If we got a partial fix, complete it and process through FixProcessor
    if let Some((partial_fix, trigger)) = fix_result {
        // Determine if aircraft is active (in flight vs on ground)
        // Use on_ground field from ADS-B capability if available, otherwise fall back to ground speed
        let is_active = match partial_fix.on_ground {
            Some(true) => false, // On ground = not active
            Some(false) => true, // Airborne = active
            None => {
                // Fallback: ground speed >= 20 knots indicates active
                partial_fix
                    .ground_speed_knots
                    .is_none_or(|speed| speed >= 20.0)
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

/// Process a received SBS (BaseStation) message
/// SBS format is text-based CSV, unlike Beast which is binary.
// Note: Intentionally NOT using #[tracing::instrument] here - it causes trace accumulation
async fn process_sbs_message(
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
        // Use on_ground field from SBS if available, otherwise fall back to ground speed
        let is_active = match partial_fix.on_ground {
            Some(true) => false, // On ground = not active
            Some(false) => true, // Airborne = active
            None => {
                // Fallback: ground speed >= 20 knots indicates active
                partial_fix
                    .ground_speed_knots
                    .is_none_or(|speed| speed >= 20.0)
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

#[allow(clippy::too_many_arguments)]
// Note: Intentionally NOT using #[tracing::instrument] here because it creates a parent span
// that causes all spawned worker tasks to inherit the same trace context, leading to
// TRACE_TOO_LARGE errors in Tempo as the trace accumulates indefinitely.
pub async fn handle_run(
    archive_dir: Option<String>,
    nats_url: String,
    suppress_aprs_types: &[String],
    skip_ogn_aircraft_types: &[String],
    no_aprs: bool,
    no_adsb: bool,
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> Result<()> {
    // Validate that at least one consumer is enabled
    if no_aprs && no_adsb {
        anyhow::bail!(
            "Cannot disable both APRS and ADS-B consumers. At least one must be enabled."
        );
    }

    // Determine environment
    let soar_env = env::var("SOAR_ENV").unwrap_or_default();
    let is_production = soar_env == "production";
    let is_staging = soar_env == "staging";

    // Log which consumers are enabled
    info!("Starting run command with:");
    info!(
        "  APRS consumer: {}",
        if no_aprs { "DISABLED" } else { "ENABLED" }
    );
    info!(
        "  ADS-B consumer: {}",
        if no_adsb { "DISABLED" } else { "ENABLED" }
    );

    // Initialize the metrics recorder FIRST so the global labels are applied
    // before we initialize any metrics to zero
    if is_production || is_staging {
        info!("Initializing metrics recorder with component=run...");
        soar::metrics::init_metrics_recorder(Some("run"));
    }

    // Initialize all soar-run metrics to zero so they appear in Grafana even before events occur
    // This MUST happen after the recorder is initialized (so global labels apply)
    // and before starting the metrics server (to avoid race conditions where
    // Prometheus scrapes before metrics are initialized)
    info!("Initializing soar-run metrics...");
    soar::metrics::initialize_run_metrics();
    info!("soar-run metrics initialized");

    // Start metrics server in the background (AFTER metrics are initialized)
    if is_production || is_staging {
        // Auto-assign port based on environment: production=9091, staging=9192
        let metrics_port = if is_staging { 9192 } else { 9091 };
        info!("Starting metrics server on port {}", metrics_port);
        tokio::spawn(async move {
            soar::metrics::start_metrics_server(metrics_port, Some("run")).await;
        });
    }

    let lock_name = if is_production {
        "soar-run-production"
    } else {
        "soar-run-dev"
    };
    let _instance_lock = InstanceLock::new(lock_name)?;
    info!(
        "Acquired instance lock: {}",
        _instance_lock.path().display()
    );

    // Log elevation data storage path
    let elevation_path =
        env::var("ELEVATION_DATA_PATH").unwrap_or_else(|_| "/var/lib/soar/elevation".to_string());
    info!("Elevation data path: {}", elevation_path);

    info!(
        "Environment: {}",
        if is_production {
            "production"
        } else if is_staging {
            "staging"
        } else {
            "development"
        }
    );

    // Create FlightTracker
    let flight_tracker = FlightTracker::new(&diesel_pool);

    // Initialize flight tracker from database:
    // 1. Timeout old incomplete flights (last_fix_at older than 1 hour)
    // 2. Restore aircraft states:
    //    - Active flights: last_fix_at within last 1 hour
    //    - Timed-out flights: last_fix_at within last 18 hours (for resumption)
    //    - Loads last 10 fixes per aircraft to rebuild in-memory state
    //    - Critical for correct takeoff/landing detection and flight coalescing
    let timeout_duration = chrono::Duration::hours(1);
    match flight_tracker
        .initialize_from_database(timeout_duration)
        .await
    {
        Ok((timed_out, restored)) => {
            info!(
                "Flight tracker initialized: {} flights timed out, {} aircraft states restored",
                timed_out, restored
            );
        }
        Err(e) => {
            warn!("Failed to initialize flight tracker from database: {}", e);
        }
    }

    // Start flight timeout checker (every 60 seconds)
    flight_tracker.start_timeout_checker(60);

    // Start aircraft state cleanup (every hour)
    // Removes aircraft states older than 18 hours
    flight_tracker.start_state_cleanup(3600);

    // Log suppressed APRS types if any
    if !suppress_aprs_types.is_empty() {
        info!(
            "Suppressing APRS types from processing: {:?}",
            suppress_aprs_types
        );
    }

    // Parse and validate aircraft categories to skip
    let parsed_aircraft_categories: Vec<AircraftCategory> = skip_ogn_aircraft_types
        .iter()
        .filter_map(|type_str| {
            type_str
                .parse::<AircraftCategory>()
                .map_err(|e| {
                    warn!("Invalid aircraft category '{}': {}", type_str, e);
                    e
                })
                .ok()
        })
        .collect();

    // Log skipped aircraft categories if any
    if !parsed_aircraft_categories.is_empty() {
        info!(
            "Skipping aircraft categories from processing: {:?}",
            parsed_aircraft_categories
        );
    }

    // Create database fix processor to save all valid fixes to the database
    // Try to create with NATS first, fall back to without NATS if connection fails
    let fix_processor = match FixProcessor::with_flight_tracker_and_nats(
        diesel_pool.clone(),
        flight_tracker.clone(),
        &nats_url,
    )
    .await
    {
        Ok(processor_with_nats) => {
            info!("Created FixProcessor with NATS publisher");
            processor_with_nats
                .with_suppressed_aprs_types(suppress_aprs_types.to_vec())
                .with_suppressed_aircraft_categories(parsed_aircraft_categories.clone())
                .with_sync_elevation(flight_tracker.elevation_db().clone())
        }
        Err(e) => {
            warn!(
                "Failed to create FixProcessor with NATS ({}), falling back to processor without NATS",
                e
            );
            FixProcessor::with_flight_tracker(diesel_pool.clone(), flight_tracker.clone())
                .with_suppressed_aprs_types(suppress_aprs_types.to_vec())
                .with_suppressed_aircraft_categories(parsed_aircraft_categories.clone())
                .with_sync_elevation(flight_tracker.elevation_db().clone())
        }
    };

    // Create server status processor for server messages
    let server_messages_repo = ServerMessagesRepository::new(diesel_pool.clone());
    let server_status_processor = ServerStatusProcessor::new(server_messages_repo);

    // Create repositories
    let receiver_repo = ReceiverRepository::new(diesel_pool.clone());
    let receiver_status_repo = ReceiverStatusRepository::new(diesel_pool.clone());
    let aprs_messages_repo =
        soar::raw_messages_repo::AprsMessagesRepository::new(diesel_pool.clone());

    // Create GenericProcessor for archiving, receiver identification, and APRS message insertion
    let generic_processor = if let Some(archive_path) = archive_dir.clone() {
        let archive_service = soar::ArchiveService::new(archive_path).await?;
        GenericProcessor::new(receiver_repo.clone(), aprs_messages_repo)
            .with_archive_service(archive_service)
    } else {
        GenericProcessor::new(receiver_repo.clone(), aprs_messages_repo)
    };

    // Create receiver status processor for receiver status messages
    let receiver_status_processor =
        ReceiverStatusProcessor::new(receiver_status_repo, receiver_repo.clone());

    // Create receiver position processor for receiver position messages
    let receiver_position_processor = ReceiverPositionProcessor::new(receiver_repo.clone());

    // Create aircraft position processor
    // Note: FlightDetectionProcessor is now handled inside FixProcessor
    let aircraft_position_processor =
        AircraftPositionProcessor::new().with_fix_processor(fix_processor.clone());

    // Create Beast/SBS processing infrastructure (only if ADS-B is enabled)
    // Both Beast (binary ADS-B) and SBS (text CSV) share the same accumulator
    let beast_infrastructure = if !no_adsb {
        let aircraft_repo = AircraftRepository::new(diesel_pool.clone());
        let beast_repo = RawMessagesRepository::new(diesel_pool.clone());

        // Create ADS-B accumulator for combining position/velocity/callsign data
        // Wraps CPR decoder and adds state accumulation across message types
        let adsb_accumulator = Arc::new(AdsbAccumulator::new());

        Some((aircraft_repo, beast_repo, adsb_accumulator))
    } else {
        None
    };

    info!(
        "Setting up APRS client with PacketRouter - archive directory: {:?}, NATS URL: {}",
        archive_dir, nats_url
    );

    // Create bounded channels for per-processor queues
    // Aircraft positions: highest capacity due to high volume and heavy processing
    let (aircraft_tx, aircraft_rx) = flume::bounded(AIRCRAFT_QUEUE_SIZE);
    info!(
        "Created aircraft position queue with capacity {}",
        AIRCRAFT_QUEUE_SIZE
    );

    // Receiver status: high capacity
    let (receiver_status_tx, receiver_status_rx) = flume::bounded(RECEIVER_STATUS_QUEUE_SIZE);
    info!(
        "Created receiver status queue with capacity {}",
        RECEIVER_STATUS_QUEUE_SIZE
    );

    // Receiver position: medium capacity
    let (receiver_position_tx, receiver_position_rx) = flume::bounded(RECEIVER_POSITION_QUEUE_SIZE);
    info!(
        "Created receiver position queue with capacity {}",
        RECEIVER_POSITION_QUEUE_SIZE
    );

    // Server status: low capacity (rare messages)
    // Channel now includes timestamp: (message, received_at)
    let (server_status_tx, server_status_rx) =
        flume::bounded::<(String, chrono::DateTime<chrono::Utc>)>(SERVER_STATUS_QUEUE_SIZE);
    info!(
        "Created server status queue with capacity {}",
        SERVER_STATUS_QUEUE_SIZE
    );

    // OGN intake queue: buffers raw OGN/APRS messages from unix socket
    // Tuple of (received_at timestamp, message string)
    // Only create if APRS is enabled
    let ogn_intake_opt = if !no_aprs {
        let (tx, rx) = flume::bounded::<(DateTime<chrono::Utc>, String)>(OGN_INTAKE_QUEUE_SIZE);
        info!(
            "Created OGN intake queue with capacity {}",
            OGN_INTAKE_QUEUE_SIZE
        );
        Some((tx, rx))
    } else {
        info!("APRS consumer disabled, skipping OGN intake queue creation");
        None
    };

    // Beast intake queue: buffers raw Beast messages from socket server
    // Tuple of (received_at timestamp, raw Beast frame bytes)
    // Only create if ADS-B is enabled
    let beast_intake_opt = if !no_adsb {
        let (tx, rx) = flume::bounded::<(DateTime<chrono::Utc>, Vec<u8>)>(BEAST_INTAKE_QUEUE_SIZE);
        info!(
            "Created Beast intake queue with capacity {}",
            BEAST_INTAKE_QUEUE_SIZE
        );
        Some((tx, rx))
    } else {
        info!("ADS-B consumer disabled, skipping Beast intake queue creation");
        None
    };

    // SBS intake queue: buffers raw SBS CSV messages from socket server
    // Tuple of (received_at timestamp, raw CSV bytes)
    // Only create if ADS-B is enabled (SBS is an alternative ADS-B source)
    let sbs_intake_opt = if !no_adsb {
        let (tx, rx) = flume::bounded::<(DateTime<chrono::Utc>, Vec<u8>)>(SBS_INTAKE_QUEUE_SIZE);
        info!(
            "Created SBS intake queue with capacity {}",
            SBS_INTAKE_QUEUE_SIZE
        );
        Some((tx, rx))
    } else {
        info!("ADS-B consumer disabled, skipping SBS intake queue creation");
        None
    };

    // Create Unix socket server for receiving messages from ingesters
    let socket_path = soar::socket_path();
    let socket_server = soar::socket_server::SocketServer::start(&socket_path)
        .await
        .context("Failed to start socket server")?;
    info!("Socket server listening on {:?}", socket_path);

    // Envelope intake queue: buffers messages from socket server before routing
    const ENVELOPE_INTAKE_QUEUE_SIZE: usize = 5_000;
    let (envelope_tx, envelope_rx) =
        flume::bounded::<soar::protocol::Envelope>(ENVELOPE_INTAKE_QUEUE_SIZE);
    info!(
        "Created envelope intake queue with capacity {}",
        ENVELOPE_INTAKE_QUEUE_SIZE
    );

    // Spawn socket accept loop task
    tokio::spawn(async move {
        socket_server.accept_loop(envelope_tx).await;
    });
    info!("Spawned socket server accept loop");

    // Spawn envelope router task
    let ogn_intake_tx_for_router = ogn_intake_opt.as_ref().map(|(tx, _)| tx.clone());
    let beast_intake_tx_for_router = beast_intake_opt.as_ref().map(|(tx, _)| tx.clone());
    let sbs_intake_tx_for_router = sbs_intake_opt.as_ref().map(|(tx, _)| tx.clone());
    let metrics_envelope_rx = envelope_rx.clone(); // Clone for metrics before moving
    tokio::spawn(async move {
        info!("Envelope router task started");
        while let Ok(envelope) = envelope_rx.recv_async().await {
            // Convert envelope timestamp to DateTime<Utc>
            let received_at = DateTime::from_timestamp_micros(envelope.timestamp_micros)
                .unwrap_or_else(chrono::Utc::now);

            match envelope.source() {
                soar::protocol::IngestSource::Ogn => {
                    if let Some(ogn_tx) = &ogn_intake_tx_for_router {
                        // Decode bytes to String (OGN messages are UTF-8)
                        match String::from_utf8(envelope.data) {
                            Ok(message) => {
                                if ogn_tx.is_full() {
                                    metrics::counter!("queue.send_blocked_total", "queue" => "ogn_intake").increment(1);
                                }
                                if let Err(e) = ogn_tx.send_async((received_at, message)).await {
                                    error!("Failed to send OGN message to intake queue: {}", e);
                                    metrics::counter!("socket.router.ogn_send_error_total")
                                        .increment(1);
                                } else {
                                    metrics::counter!("socket.router.ogn_routed_total")
                                        .increment(1);
                                }
                            }
                            Err(e) => {
                                error!("Failed to decode OGN message as UTF-8: {}", e);
                                metrics::counter!("socket.router.decode_error_total").increment(1);
                            }
                        }
                    }
                }
                soar::protocol::IngestSource::Beast => {
                    if let Some(beast_tx) = &beast_intake_tx_for_router {
                        if beast_tx.is_full() {
                            metrics::counter!("queue.send_blocked_total", "queue" => "beast_intake").increment(1);
                        }
                        if let Err(e) = beast_tx.send_async((received_at, envelope.data)).await {
                            error!("Failed to send Beast message to intake queue: {}", e);
                            metrics::counter!("socket.router.beast_send_error_total").increment(1);
                        } else {
                            metrics::counter!("socket.router.beast_routed_total").increment(1);
                        }
                    }
                }
                soar::protocol::IngestSource::Sbs => {
                    if let Some(sbs_tx) = &sbs_intake_tx_for_router {
                        if sbs_tx.is_full() {
                            metrics::counter!("queue.send_blocked_total", "queue" => "sbs_intake")
                                .increment(1);
                        }
                        if let Err(e) = sbs_tx.send_async((received_at, envelope.data)).await {
                            error!("Failed to send SBS message to intake queue: {}", e);
                            metrics::counter!("socket.router.sbs_send_error_total").increment(1);
                        } else {
                            metrics::counter!("socket.router.sbs_routed_total").increment(1);
                        }
                    }
                }
            }
        }
        info!("Envelope router task stopped");
    });
    info!("Spawned envelope router task");

    // Create PacketRouter with per-processor queues and internal worker pool
    const PACKET_ROUTER_WORKERS: usize = 50;
    let packet_router = PacketRouter::new(generic_processor)
        .with_aircraft_position_queue(aircraft_tx)
        .with_receiver_status_queue(receiver_status_tx)
        .with_receiver_position_queue(receiver_position_tx)
        .with_server_status_queue(server_status_tx)
        .start(PACKET_ROUTER_WORKERS); // Start workers AFTER configuration

    info!(
        "Created PacketRouter with {} workers and per-processor queues",
        PACKET_ROUTER_WORKERS
    );

    // Spawn intake queue processor (only if APRS is enabled)
    // This task reads raw APRS messages from the intake queue and processes them
    // Separating NATS consumption from processing allows graceful shutdown
    if let Some((_, ogn_intake_rx)) = ogn_intake_opt.as_ref() {
        let intake_router = packet_router.clone();
        let ogn_intake_rx = ogn_intake_rx.clone();
        tokio::spawn(async move {
            info!("Intake queue processor started");
            let mut messages_processed = 0u64;
            while let Ok((received_at, message)) = ogn_intake_rx.recv_async().await {
                // Note: No tracing spans here - they cause trace accumulation in Tempo
                // Use metrics only for observability in the hot path
                metrics::gauge!("worker.active", "type" => "intake").increment(1.0);
                process_aprs_message(received_at, &message, &intake_router).await;
                messages_processed += 1;
                metrics::counter!("aprs.intake.processed_total").increment(1);
                metrics::gauge!("worker.active", "type" => "intake").decrement(1.0);

                // Update intake queue depth metric
                metrics::gauge!("aprs.intake_queue.depth").set(ogn_intake_rx.len() as f64);
            }
            info!(
                "Intake queue processor stopped after processing {} messages",
                messages_processed
            );
        });
        info!("Spawned intake queue processor");
    }

    // Spawn Beast intake queue workers (only if ADS-B is enabled)
    // Multiple workers for parallel processing of Beast (binary) messages
    // Beast message processing involves database operations (aircraft lookup, raw message storage)
    // and state accumulation, so we need multiple workers to handle high traffic volumes.
    // Using 200 workers: ADS-B traffic is ~30,000 msg/sec vs OGN's ~300 msg/sec (100x more)
    // With 200 workers at ~150 msg/sec per worker, we can handle up to 30k msg/sec
    if let (
        Some((beast_aircraft_repo, beast_repo_clone, beast_accumulator)),
        Some((_, beast_intake_rx)),
    ) = (beast_infrastructure.as_ref(), beast_intake_opt.as_ref())
    {
        let num_beast_workers = 200;
        info!("Spawning {} Beast intake queue workers", num_beast_workers);

        for worker_id in 0..num_beast_workers {
            let beast_aircraft_repo = beast_aircraft_repo.clone();
            let beast_repo_clone = beast_repo_clone.clone();
            let beast_fix_processor = fix_processor.clone();
            let beast_accumulator = beast_accumulator.clone();
            let beast_intake_rx = beast_intake_rx.clone();

            tokio::spawn(async move {
                while let Ok((received_at, raw_frame)) = beast_intake_rx.recv_async().await {
                    // Note: No tracing spans here - they cause trace accumulation in Tempo
                    let start_time = std::time::Instant::now();
                    process_beast_message(
                        received_at,
                        &raw_frame,
                        &beast_aircraft_repo,
                        &beast_repo_clone,
                        &beast_fix_processor,
                        &beast_accumulator,
                    )
                    .await;

                    let duration = start_time.elapsed();
                    metrics::histogram!("beast.run.process_message_duration_ms")
                        .record(duration.as_millis() as f64);
                    metrics::counter!("beast.run.intake.processed_total").increment(1);

                    // Update Beast intake queue depth metric (sample from each worker)
                    metrics::gauge!("beast.intake_queue.depth").set(beast_intake_rx.len() as f64);
                }
                info!("Beast intake queue worker {} stopped", worker_id);
            });
        }
        info!("Spawned {} Beast intake queue workers", num_beast_workers);
    }

    // Spawn SBS intake queue workers (only if ADS-B is enabled)
    // SBS (BaseStation) messages are text-based CSV, processed separately from Beast binary
    // SBS typically has lower traffic than Beast, so fewer workers are needed
    // Using 50 workers should be sufficient for most SBS data sources
    // SBS shares the same accumulator with Beast for consistent state tracking
    if let (Some((sbs_aircraft_repo, sbs_repo, sbs_accumulator)), Some((_, sbs_intake_rx))) =
        (beast_infrastructure.as_ref(), sbs_intake_opt.as_ref())
    {
        let num_sbs_workers = 50;
        info!("Spawning {} SBS intake queue workers", num_sbs_workers);

        for worker_id in 0..num_sbs_workers {
            let sbs_aircraft_repo = sbs_aircraft_repo.clone();
            let sbs_repo = sbs_repo.clone();
            let sbs_fix_processor = fix_processor.clone();
            let sbs_accumulator = sbs_accumulator.clone();
            let sbs_intake_rx = sbs_intake_rx.clone();

            tokio::spawn(async move {
                while let Ok((received_at, csv_bytes)) = sbs_intake_rx.recv_async().await {
                    // Note: No tracing spans here - they cause trace accumulation in Tempo
                    let start_time = std::time::Instant::now();
                    process_sbs_message(
                        received_at,
                        &csv_bytes,
                        &sbs_aircraft_repo,
                        &sbs_repo,
                        &sbs_fix_processor,
                        &sbs_accumulator,
                    )
                    .await;

                    let duration = start_time.elapsed();
                    metrics::histogram!("sbs.run.process_message_duration_ms")
                        .record(duration.as_millis() as f64);
                    metrics::counter!("sbs.run.intake.processed_total").increment(1);

                    // Update SBS intake queue depth metric (sample from each worker)
                    metrics::gauge!("sbs.intake_queue.depth").set(sbs_intake_rx.len() as f64);
                }
                info!("SBS intake queue worker {} stopped", worker_id);
            });
        }
        info!("Spawned {} SBS intake queue workers", num_sbs_workers);
    }

    // Spawn dedicated worker pools for each processor type
    // Aircraft position workers (80 workers - heaviest processing due to FixProcessor + flight tracking)
    // Increased from 20 to 80 because aircraft queue was constantly full at 1,000 capacity
    // Most APRS messages (~80-90%) are aircraft positions, so this queue needs the most workers
    let num_aircraft_workers = 80;
    info!(
        "Spawning {} aircraft position workers",
        num_aircraft_workers
    );
    for _worker_id in 0..num_aircraft_workers {
        let worker_rx = aircraft_rx.clone();
        let processor = aircraft_position_processor.clone();
        tokio::spawn(async move {
            while let Ok((packet, context)) = worker_rx.recv_async().await {
                // Note: No tracing spans here - they cause trace accumulation in Tempo
                metrics::gauge!("worker.active", "type" => "aircraft").increment(1.0);
                let start = std::time::Instant::now();
                processor.process_aircraft_position(&packet, context).await;
                let duration = start.elapsed();
                metrics::histogram!("aprs.aircraft.duration_ms")
                    .record(duration.as_millis() as f64);
                metrics::counter!("aprs.aircraft.processed_total").increment(1);
                metrics::counter!("aprs.messages.processed.aircraft_total").increment(1);
                metrics::counter!("aprs.messages.processed.total_total").increment(1);
                metrics::gauge!("worker.active", "type" => "aircraft").decrement(1.0);
            }
        });
    }

    // Receiver status workers (6 workers - medium processing)
    let num_receiver_status_workers = 6;
    info!(
        "Spawning {} receiver status workers",
        num_receiver_status_workers
    );
    for _worker_id in 0..num_receiver_status_workers {
        let worker_rx = receiver_status_rx.clone();
        let processor = receiver_status_processor.clone();
        tokio::spawn(async move {
            while let Ok((packet, context)) = worker_rx.recv_async().await {
                // Note: No tracing spans here - they cause trace accumulation in Tempo
                metrics::gauge!("worker.active", "type" => "receiver_status").increment(1.0);
                let start = std::time::Instant::now();
                processor.process_status_packet(&packet, context).await;
                let duration = start.elapsed();
                metrics::histogram!("aprs.receiver_status.duration_ms")
                    .record(duration.as_millis() as f64);
                metrics::counter!("aprs.receiver_status.processed_total").increment(1);
                metrics::counter!("aprs.messages.processed.receiver_status_total").increment(1);
                metrics::counter!("aprs.messages.processed.total_total").increment(1);
                metrics::gauge!("worker.active", "type" => "receiver_status").decrement(1.0);
            }
        });
    }

    // Receiver position workers (4 workers - light processing)
    let num_receiver_position_workers = 4;
    info!(
        "Spawning {} receiver position workers",
        num_receiver_position_workers
    );
    for _worker_id in 0..num_receiver_position_workers {
        let worker_rx = receiver_position_rx.clone();
        let processor = receiver_position_processor.clone();
        tokio::spawn(async move {
            while let Ok((packet, context)) = worker_rx.recv_async().await {
                // Note: No tracing spans here - they cause trace accumulation in Tempo
                metrics::gauge!("worker.active", "type" => "receiver_position").increment(1.0);
                let start = std::time::Instant::now();
                processor.process_receiver_position(&packet, context).await;
                let duration = start.elapsed();
                metrics::histogram!("aprs.receiver_position.duration_ms")
                    .record(duration.as_millis() as f64);
                metrics::counter!("aprs.receiver_position.processed_total").increment(1);
                metrics::counter!("aprs.messages.processed.receiver_position_total").increment(1);
                metrics::counter!("aprs.messages.processed.total_total").increment(1);
                metrics::gauge!("worker.active", "type" => "receiver_position").decrement(1.0);
            }
        });
    }

    // Server status workers (2 workers - very light processing)
    info!("Spawning 2 server status workers");
    for _worker_id in 0..2 {
        let worker_rx = server_status_rx.clone();
        let processor = server_status_processor.clone();
        tokio::spawn(async move {
            while let Ok((message, received_at)) = worker_rx.recv_async().await {
                // Note: No tracing spans here - they cause trace accumulation in Tempo
                metrics::gauge!("worker.active", "type" => "server_status").increment(1.0);
                let start = std::time::Instant::now();
                processor
                    .process_server_message(&message, received_at)
                    .await;
                let duration = start.elapsed();
                metrics::histogram!("aprs.server_status.duration_ms")
                    .record(duration.as_millis() as f64);
                metrics::counter!("aprs.server_status.processed_total").increment(1);
                metrics::counter!("aprs.messages.processed.server_total").increment(1);
                metrics::counter!("aprs.messages.processed.total_total").increment(1);
                metrics::gauge!("worker.active", "type" => "server_status").decrement(1.0);
            }
        });
    }

    // Spawn queue depth and system metrics reporter
    // Reports the depth of all processing queues and DB pool state to Prometheus every 10 seconds
    let metrics_packet_router = packet_router.clone();
    let metrics_aircraft_rx = aircraft_rx.clone();
    let metrics_receiver_status_rx = receiver_status_rx.clone();
    let metrics_receiver_position_rx = receiver_position_rx.clone();
    let metrics_server_status_rx = server_status_rx.clone();
    let metrics_db_pool = diesel_pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        interval.tick().await; // First tick completes immediately

        loop {
            interval.tick().await;

            // Sample queue depths (lock-free with flume!)
            let envelope_intake_depth = metrics_envelope_rx.len();
            let internal_queue_depth = metrics_packet_router.internal_queue_depth();
            let aircraft_depth = metrics_aircraft_rx.len();
            let receiver_status_depth = metrics_receiver_status_rx.len();
            let receiver_position_depth = metrics_receiver_position_rx.len();
            let server_status_depth = metrics_server_status_rx.len();

            // Get database pool state
            let pool_state = metrics_db_pool.state();
            let active_connections = pool_state.connections - pool_state.idle_connections;

            // Report queue depths to Prometheus
            metrics::gauge!("socket.envelope_intake_queue.depth").set(envelope_intake_depth as f64);
            metrics::gauge!("aprs.router_queue.depth").set(internal_queue_depth as f64);
            metrics::gauge!("aprs.aircraft_queue.depth").set(aircraft_depth as f64);
            metrics::gauge!("aprs.receiver_status_queue.depth").set(receiver_status_depth as f64);
            metrics::gauge!("aprs.receiver_position_queue.depth")
                .set(receiver_position_depth as f64);
            metrics::gauge!("aprs.server_status_queue.depth").set(server_status_depth as f64);

            // Report database pool state to Prometheus
            metrics::gauge!("aprs.db_pool.total_connections").set(pool_state.connections as f64);
            metrics::gauge!("aprs.db_pool.active_connections").set(active_connections as f64);
            metrics::gauge!("aprs.db_pool.idle_connections")
                .set(pool_state.idle_connections as f64);

            // Warn if queues are building up
            // Envelope intake queue: 80% threshold (critical - first point of backpressure)
            if envelope_intake_depth > (ENVELOPE_INTAKE_QUEUE_SIZE * 80 / 100) {
                let percent = (envelope_intake_depth as f64 / ENVELOPE_INTAKE_QUEUE_SIZE as f64
                    * 100.0) as usize;
                warn!(
                    "Envelope intake queue building up: {}/{} messages ({}% full) - socket reads may slow down",
                    envelope_intake_depth, ENVELOPE_INTAKE_QUEUE_SIZE, percent
                );
            }

            // Internal router queue: 50% threshold
            use soar::packet_processors::router::INTERNAL_QUEUE_CAPACITY;
            if internal_queue_depth > queue_warning_threshold(INTERNAL_QUEUE_CAPACITY) {
                let percent =
                    (internal_queue_depth as f64 / INTERNAL_QUEUE_CAPACITY as f64 * 100.0) as usize;
                warn!(
                    "PacketRouter internal queue building up: {}/{} messages ({}% full)",
                    internal_queue_depth, INTERNAL_QUEUE_CAPACITY, percent
                );
            }

            // Aircraft position queue: 50% threshold
            if aircraft_depth > queue_warning_threshold(AIRCRAFT_QUEUE_SIZE) {
                let percent = (aircraft_depth as f64 / AIRCRAFT_QUEUE_SIZE as f64 * 100.0) as usize;
                warn!(
                    "Aircraft position queue building up: {}/{} messages ({}% full)",
                    aircraft_depth, AIRCRAFT_QUEUE_SIZE, percent
                );
            }
            if receiver_status_depth > queue_warning_threshold(RECEIVER_STATUS_QUEUE_SIZE) {
                let percent = (receiver_status_depth as f64 / RECEIVER_STATUS_QUEUE_SIZE as f64
                    * 100.0) as usize;
                warn!(
                    "Receiver status queue building up: {}/{} messages ({}% full)",
                    receiver_status_depth, RECEIVER_STATUS_QUEUE_SIZE, percent
                );
            }
            if receiver_position_depth > queue_warning_threshold(RECEIVER_POSITION_QUEUE_SIZE) {
                let percent = (receiver_position_depth as f64 / RECEIVER_POSITION_QUEUE_SIZE as f64
                    * 100.0) as usize;
                warn!(
                    "Receiver position queue building up: {}/{} messages ({}% full)",
                    receiver_position_depth, RECEIVER_POSITION_QUEUE_SIZE, percent
                );
            }
        }
    });
    info!("Spawned queue depth metrics reporter (reports every 10 seconds to Prometheus)");

    // Set up graceful shutdown handler
    let shutdown_aircraft_rx = aircraft_rx.clone();
    let shutdown_receiver_status_rx = receiver_status_rx.clone();
    let shutdown_receiver_position_rx = receiver_position_rx.clone();
    let shutdown_server_status_rx = server_status_rx.clone();
    let shutdown_ogn_intake_opt = ogn_intake_opt.as_ref().map(|(tx, _)| tx.clone());
    let shutdown_beast_intake_opt = beast_intake_opt.as_ref().map(|(tx, _)| tx.clone());

    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("Received shutdown signal (Ctrl+C), initiating graceful shutdown...");
                info!("Socket server will stop accepting connections, allowing queues to drain...");

                // Wait for queues to drain (check every second, max 10 minutes)
                for i in 1..=600 {
                    let intake_depth = shutdown_ogn_intake_opt.as_ref().map_or(0, |tx| tx.len());
                    let beast_intake_depth =
                        shutdown_beast_intake_opt.as_ref().map_or(0, |tx| tx.len());
                    let aircraft_depth = shutdown_aircraft_rx.len();
                    let receiver_status_depth = shutdown_receiver_status_rx.len();
                    let receiver_position_depth = shutdown_receiver_position_rx.len();
                    let server_status_depth = shutdown_server_status_rx.len();

                    let total_queued = intake_depth
                        + beast_intake_depth
                        + aircraft_depth
                        + receiver_status_depth
                        + receiver_position_depth
                        + server_status_depth;

                    if total_queued == 0 {
                        info!("All queues drained, shutting down now");
                        break;
                    }

                    info!(
                        "Waiting for queues to drain ({}/600s): {} intake, {} beast_intake, {} aircraft, {} rx_status, {} rx_pos, {} server",
                        i,
                        intake_depth,
                        beast_intake_depth,
                        aircraft_depth,
                        receiver_status_depth,
                        receiver_position_depth,
                        server_status_depth
                    );

                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }

                info!("Graceful shutdown complete");
                std::process::exit(0);
            }
            Err(err) => {
                eprintln!("Unable to listen for shutdown signal: {}", err);
            }
        }
    });
    info!("Graceful shutdown handler configured");

    // All processing tasks are now running via socket server and envelope router
    // Just wait for shutdown signal
    info!("Main processing loop started. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;
    info!("Received shutdown signal, exiting...");
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_timestamp_parsing_valid() {
        // Test that a valid ISO-8601 timestamp is correctly parsed
        let message = "2025-01-15T12:34:56.789Z FLRDDA5BA>APRS,qAS,LFNX:/160829h4902.45N/00531.30E'342/049/A=001322";

        // We can't directly test process_aprs_message since it's async and requires PacketRouter,
        // but we can test the parsing logic
        let (timestamp_str, rest) = message.split_once(' ').unwrap();
        let parsed = chrono::DateTime::parse_from_rfc3339(timestamp_str);

        assert!(parsed.is_ok());
        let timestamp = parsed.unwrap().with_timezone(&chrono::Utc);
        assert_eq!(timestamp.year(), 2025);
        assert_eq!(timestamp.month(), 1);
        assert_eq!(timestamp.day(), 15);
        assert_eq!(timestamp.hour(), 12);
        assert_eq!(timestamp.minute(), 34);
        assert_eq!(timestamp.second(), 56);
        assert_eq!(
            rest,
            "FLRDDA5BA>APRS,qAS,LFNX:/160829h4902.45N/00531.30E'342/049/A=001322"
        );
    }

    #[test]
    fn test_timestamp_parsing_invalid() {
        // Test that an invalid timestamp doesn't crash
        let message = "INVALID-TIMESTAMP FLRDDA5BA>APRS,qAS,LFNX:/160829h4902.45N/00531.30E";

        let (timestamp_str, _rest) = message.split_once(' ').unwrap();
        let parsed = chrono::DateTime::parse_from_rfc3339(timestamp_str);

        assert!(parsed.is_err());
    }

    #[test]
    fn test_timestamp_parsing_missing() {
        // Test that a message without a space (no timestamp) is handled
        let message = "FLRDDA5BA>APRS,qAS,LFNX:/160829h4902.45N/00531.30E'342/049/A=001322";

        let result = message.split_once(' ');
        assert!(result.is_none());
    }

    #[test]
    fn test_timestamp_parsing_server_message() {
        // Test that server messages with timestamps are handled correctly
        let message = "2025-01-15T12:34:56.789Z # aprsc 2.1.15-gc67551b 22 Sep 2025 21:51:55 GMT GLIDERN1 51.178.19.212:10152";

        let (timestamp_str, rest) = message.split_once(' ').unwrap();
        let parsed = chrono::DateTime::parse_from_rfc3339(timestamp_str);

        assert!(parsed.is_ok());
        assert!(rest.starts_with('#'));
        assert_eq!(
            rest,
            "# aprsc 2.1.15-gc67551b 22 Sep 2025 21:51:55 GMT GLIDERN1 51.178.19.212:10152"
        );
    }

    #[test]
    fn test_timestamp_format_rfc3339() {
        // Test that Utc::now().to_rfc3339() produces a parseable timestamp
        let now = chrono::Utc::now();
        let timestamp_str = now.to_rfc3339();

        let parsed = chrono::DateTime::parse_from_rfc3339(&timestamp_str);
        assert!(parsed.is_ok());

        let parsed_utc = parsed.unwrap().with_timezone(&chrono::Utc);
        // Should be within 1 second (to account for processing time)
        let diff = (now - parsed_utc).num_milliseconds().abs();
        assert!(diff < 1000);
    }
}
