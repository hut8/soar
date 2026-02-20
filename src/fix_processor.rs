use num_traits::AsPrimitive;
use tracing::Instrument;
use tracing::{debug, error, trace, warn};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::Fix;
use crate::aircraft_repo::{AircraftCache, AircraftPacketFields};
use crate::elevation::ElevationService;
use crate::fixes_repo::FixesRepository;
use crate::flight_tracker::FlightTracker;
use crate::nats_publisher::NatsFixPublisher;
use crate::packet_processors::generic::PacketContext;
use crate::receiver_repo::ReceiverRepository;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use ogn_parser::AprsPacket;

/// Elevation processing mode configuration
#[derive(Clone)]
pub enum ElevationMode {
    /// Synchronous: Calculate elevation inline before database insert
    Sync { elevation_db: Box<ElevationService> },
}

/// Database fix processor that saves valid fixes to the database and performs flight tracking
#[derive(Clone)]
pub struct FixProcessor {
    fixes_repo: FixesRepository,
    aircraft_cache: AircraftCache,
    receiver_repo: ReceiverRepository,
    flight_detection_processor: FlightTracker,
    nats_publisher: Option<NatsFixPublisher>,
    /// Elevation processing mode (sync or async)
    elevation_mode: Option<ElevationMode>,
    /// APRS types to suppress from processing (e.g., OGADSB, OGFLR)
    suppressed_aprs_types: Vec<String>,
    /// Aircraft categories to skip from processing (e.g., Landplane for jets)
    suppressed_aircraft_categories: Vec<crate::aircraft_types::AircraftCategory>,
}

impl FixProcessor {
    pub fn new(
        diesel_pool: Pool<ConnectionManager<PgConnection>>,
        aircraft_cache: AircraftCache,
    ) -> Self {
        Self {
            fixes_repo: FixesRepository::new(diesel_pool.clone()),
            aircraft_cache: aircraft_cache.clone(),
            receiver_repo: ReceiverRepository::new(diesel_pool.clone()),
            flight_detection_processor: FlightTracker::new(&diesel_pool, aircraft_cache),
            nats_publisher: None,
            elevation_mode: None,
            suppressed_aprs_types: Vec::new(),
            suppressed_aircraft_categories: Vec::new(),
        }
    }

    /// Configure synchronous elevation processing
    pub fn with_sync_elevation(mut self, elevation_db: ElevationService) -> Self {
        self.elevation_mode = Some(ElevationMode::Sync {
            elevation_db: Box::new(elevation_db),
        });
        self
    }

    /// Set APRS types to suppress from processing
    pub fn with_suppressed_aprs_types(mut self, types: Vec<String>) -> Self {
        self.suppressed_aprs_types = types;
        self
    }

    /// Set aircraft categories to skip from processing
    pub fn with_suppressed_aircraft_categories(
        mut self,
        categories: Vec<crate::aircraft_types::AircraftCategory>,
    ) -> Self {
        self.suppressed_aircraft_categories = categories;
        self
    }

    /// Create a new FixProcessor with NATS publisher
    pub async fn new_with_nats(
        diesel_pool: Pool<ConnectionManager<PgConnection>>,
        aircraft_cache: AircraftCache,
        nats_url: &str,
    ) -> anyhow::Result<Self> {
        let nats_publisher = NatsFixPublisher::new(nats_url).await?;

        Ok(Self {
            fixes_repo: FixesRepository::new(diesel_pool.clone()),
            aircraft_cache: aircraft_cache.clone(),
            receiver_repo: ReceiverRepository::new(diesel_pool.clone()),
            flight_detection_processor: FlightTracker::new(&diesel_pool, aircraft_cache),
            nats_publisher: Some(nats_publisher),
            elevation_mode: None,
            suppressed_aprs_types: Vec::new(),
            suppressed_aircraft_categories: Vec::new(),
        })
    }

    /// Create a new FixProcessor with a custom FlightTracker (for state persistence)
    pub fn with_flight_tracker(
        diesel_pool: Pool<ConnectionManager<PgConnection>>,
        aircraft_cache: AircraftCache,
        flight_tracker: FlightTracker,
    ) -> Self {
        Self {
            fixes_repo: FixesRepository::new(diesel_pool.clone()),
            aircraft_cache,
            receiver_repo: ReceiverRepository::new(diesel_pool.clone()),
            flight_detection_processor: flight_tracker,
            nats_publisher: None,
            elevation_mode: None,
            suppressed_aprs_types: Vec::new(),
            suppressed_aircraft_categories: Vec::new(),
        }
    }

    /// Create a new FixProcessor with custom FlightTracker and NATS publisher
    pub async fn with_flight_tracker_and_nats(
        diesel_pool: Pool<ConnectionManager<PgConnection>>,
        aircraft_cache: AircraftCache,
        flight_tracker: FlightTracker,
        nats_url: &str,
    ) -> anyhow::Result<Self> {
        let nats_publisher = NatsFixPublisher::new(nats_url).await?;

        Ok(Self {
            fixes_repo: FixesRepository::new(diesel_pool.clone()),
            aircraft_cache,
            receiver_repo: ReceiverRepository::new(diesel_pool.clone()),
            flight_detection_processor: flight_tracker,
            nats_publisher: Some(nats_publisher),
            elevation_mode: None,
            suppressed_aprs_types: Vec::new(),
            suppressed_aircraft_categories: Vec::new(),
        })
    }

    /// Get a reference to the flight tracker for state management
    pub fn flight_tracker(&self) -> &FlightTracker {
        &self.flight_detection_processor
    }

    /// Process a pre-created Fix (e.g., from ADS-B)
    /// This is used when the fix has already been created from a non-APRS source
    #[tracing::instrument(skip(self, fix), fields(aircraft_id = %fix.aircraft_id))]
    pub async fn process_fix(&self, fix: Fix) -> anyhow::Result<()> {
        // Use an empty raw message for non-APRS fixes
        self.process_fix_internal(fix, "").await;
        Ok(())
    }
}

impl FixProcessor {
    /// Process an APRS packet by looking up device and creating a Fix
    /// This is the main entry point that orchestrates the entire pipeline
    /// Note: Receiver is guaranteed to exist and APRS message already inserted by GenericProcessor
    #[tracing::instrument(skip(self, packet, raw_message, context))]
    pub async fn process_aprs_packet(
        &self,
        packet: AprsPacket,
        raw_message: &str,
        context: PacketContext,
    ) {
        metrics::counter!("aprs.aircraft.stage_total", "stage" => "entered").increment(1);
        let total_start = std::time::Instant::now();

        // Use the received_at timestamp from context (captured at ingestion time)
        // This ensures accurate timestamps even if messages queue up during processing
        let received_at = context.received_at;

        // Try to create a fix from the packet
        match packet.data {
            ogn_parser::AprsData::Position(ref pos_packet) => {
                let mut device_address = 0i32;
                let mut address_type = crate::aircraft::AddressType::Unknown;
                let mut aircraft_category = None;

                // Extract device info from OGN parameters
                // Note: The ID field supports two formats:
                // 1. Standard: idXXYYYYYY (8 hex after "id") - XX=metadata, YYYYYY=6-digit address
                // 2. NAVITER: idXXXXYYYYYY (10 hex after "id") - XXXX=metadata, YYYYYY=6-digit address
                // The address is always the last 6 hex digits (24-bit), which fits in i32.
                // See docs/FANET_ID_FORMAT.md for details.
                if let Some(ref id) = pos_packet.comment.id {
                    device_address = id.address.as_();
                    address_type = match id.address_type {
                        0 => crate::aircraft::AddressType::Unknown,
                        1 => crate::aircraft::AddressType::Icao,
                        2 => crate::aircraft::AddressType::Flarm,
                        3 => crate::aircraft::AddressType::Ogn,
                        _ => crate::aircraft::AddressType::Unknown,
                    };
                    // Extract aircraft category from OGN parameters
                    aircraft_category = Some(
                        crate::aircraft_types::AircraftCategory::from_ogn_byte(id.aircraft_type),
                    );
                }

                // When creating devices spontaneously (not from DDB), determine address_type from tracker_device_type
                // tracker_device_type is the packet destination (e.g., "OGFLR", "OGADSB")
                let tracker_device_type = packet.to.to_string();

                // Track APRS type before filtering
                metrics::counter!("aprs.type.received_total", "type" => tracker_device_type.clone())
                    .increment(1);

                // Check if this APRS type is suppressed
                if self
                    .suppressed_aprs_types
                    .iter()
                    .any(|t| t == &tracker_device_type)
                {
                    trace!("Suppressing fix from APRS type: {}", tracker_device_type);
                    metrics::counter!("aprs.fixes.suppressed_total").increment(1);
                    return;
                }

                // Check if this aircraft category should be skipped
                if let Some(ref ac_category) = aircraft_category
                    && self
                        .suppressed_aircraft_categories
                        .iter()
                        .any(|c| c == ac_category)
                {
                    trace!("Skipping fix from aircraft category: {:?}", ac_category);
                    metrics::counter!("aprs.fixes.skipped_aircraft_category_total", "category" => format!("{:?}", ac_category))
                        .increment(1);
                    return;
                }

                // Skip fixes with zero address - these are invalid/garbage data
                if device_address == 0 {
                    trace!("Skipping fix with zero device address");
                    metrics::counter!("aprs.fixes.skipped_zero_address_total").increment(1);
                    return;
                }

                let spontaneous_address_type = match tracker_device_type.as_str() {
                    "OGFLR" => crate::aircraft::AddressType::Flarm,
                    "OGADSB" => crate::aircraft::AddressType::Icao,
                    _ => crate::aircraft::AddressType::Unknown,
                };

                // Extract all available fields from packet for device creation/update
                // The model field can be either a 3-4 character ICAO code or a full model name
                let model_string = pos_packet.comment.model.as_ref().map(|m| m.to_string());

                // Use as aircraft_model unconditionally
                let aircraft_model = model_string.clone();

                // Only use as icao_model_code if it's exactly 3 or 4 characters (per ICAO Doc 8643)
                let icao_model_code = model_string.filter(|model| {
                    let len = model.len();
                    len == 3 || len == 4
                });

                let adsb_emitter_category = pos_packet
                    .comment
                    .adsb_emitter_category
                    .and_then(|cat| cat.to_string().parse().ok());
                let registration: Option<String> = pos_packet
                    .comment
                    .registration
                    .as_ref()
                    .map(|reg| reg.to_string());

                let packet_fields = AircraftPacketFields {
                    aircraft_category,
                    aircraft_model,
                    icao_model_code,
                    adsb_emitter_category,
                    tracker_device_type: Some(tracker_device_type.clone()),
                    registration: registration.clone(),
                };

                // Look up or create aircraft based on device_address
                // When creating spontaneously, use address_type derived from tracker_device_type
                let aircraft_lookup_start = std::time::Instant::now();
                metrics::counter!("aprs.aircraft.stage_total", "stage" => "before_db").increment(1);
                match self
                    .aircraft_cache
                    .get_or_upsert(device_address, spontaneous_address_type, packet_fields)
                    .await
                {
                    Ok(aircraft) => {
                        metrics::counter!("aprs.aircraft.stage_total", "stage" => "after_db")
                            .increment(1);
                        metrics::histogram!("aprs.aircraft.upsert_ms")
                            .record(aircraft_lookup_start.elapsed().as_micros() as f64 / 1000.0);

                        // Aircraft exists or was just created, create fix with proper aircraft_id
                        let aircraft_id = aircraft.id.expect("aircraft must have id");
                        let fix_creation_start = std::time::Instant::now();
                        match Fix::from_aprs_packet(
                            packet,
                            received_at,
                            aircraft_id,
                            Some(context.receiver_id),
                            context.raw_message_id,
                        ) {
                            Ok(Some(fix)) => {
                                metrics::counter!("aprs.aircraft.stage_total", "stage" => "fix_created").increment(1);
                                metrics::histogram!("aprs.aircraft.fix_creation_ms").record(
                                    fix_creation_start.elapsed().as_micros() as f64 / 1000.0,
                                );

                                let process_internal_start = std::time::Instant::now();
                                self.process_fix_internal(fix, raw_message).await;
                                metrics::histogram!("aprs.aircraft.process_fix_internal_ms")
                                    .record(
                                        process_internal_start.elapsed().as_micros() as f64
                                            / 1000.0,
                                    );

                                // Record total processing time
                                metrics::histogram!("aprs.aircraft.total_processing_ms")
                                    .record(total_start.elapsed().as_micros() as f64 / 1000.0);
                            }
                            Ok(None) => {
                                trace!("No position fix in APRS position packet");
                            }
                            Err(e) => {
                                debug!("Failed to extract fix from APRS position packet: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            "Failed to get or insert device address {:06X} ({:?}): {}, skipping fix processing (packet_registration={:?})",
                            device_address, address_type, e, registration
                        );
                    }
                }
            }
            _ => {
                // Non-position packets return without processing
                trace!("Non-position packet, no fix to process");
            }
        }
    }

    /// Internal method to process a fix through the complete pipeline
    #[tracing::instrument(skip(self, fix, raw_message), fields(device_id = %fix.aircraft_id))]
    async fn process_fix_internal(&self, mut fix: Fix, raw_message: &str) {
        // Step 0: Calculate elevation synchronously if in sync mode (before database insert)
        // This ensures the fix is inserted with complete data including AGL altitude
        if let Some(ElevationMode::Sync { elevation_db }) = &self.elevation_mode
            && fix.altitude_msl_feet.is_some()
        {
            let elevation_start = std::time::Instant::now();

            // Calculate AGL altitude synchronously
            let agl =
                crate::flight_tracker::altitude::calculate_altitude_agl(elevation_db, &fix).await;

            // Update fix with AGL data before database insert
            fix.altitude_agl_feet = agl;
            fix.altitude_agl_valid = true; // Mark as valid even if agl is None (no elevation data available)

            // Recalculate is_active now that we have AGL data.
            // ADS-B fixes use the transponder's on_ground field directly —
            // skip the speed/AGL heuristic which is designed for APRS.
            if !fix.has_transponder_data() {
                fix.is_active = crate::flight_tracker::should_be_active(&fix);
            }

            // Warn when ADS-B reports on-ground but AGL suggests otherwise
            if fix.has_transponder_data()
                && !fix.is_active
                && let Some(agl) = fix.altitude_agl_feet
                && agl.abs() >= 100
            {
                warn!(
                    "ADS-B on-ground but AGL={} ft for aircraft {} at ({:.4}, {:.4}), MSL={:?} ft — possible altitude reference mismatch",
                    agl, fix.aircraft_id, fix.latitude, fix.longitude, fix.altitude_msl_feet
                );
            }

            metrics::histogram!("aprs.elevation.sync_duration_ms")
                .record(elevation_start.elapsed().as_micros() as f64 / 1000.0);
            metrics::counter!("aprs.elevation.sync_processed_total").increment(1);
        }

        // Step 1: Process through flight detection AND save to database
        // This is done atomically while holding a per-device lock to prevent race conditions
        metrics::counter!("aprs.aircraft.stage_total", "stage" => "before_flight").increment(1);
        let flight_insert_start = std::time::Instant::now();
        let updated_fix = match self
            .flight_detection_processor
            .process_and_insert_fix(fix, &self.fixes_repo)
            .await
        {
            Some(fix) => fix,
            None => return, // Fix was discarded (duplicate, error, etc.)
        };
        metrics::counter!("aprs.aircraft.stage_total", "stage" => "after_flight").increment(1);
        metrics::histogram!("aprs.aircraft.flight_insert_ms")
            .record(flight_insert_start.elapsed().as_micros() as f64 / 1000.0);

        // Step 2: Update receiver's latest_packet_at (only for fixes with receiver_id)
        if let Some(receiver_id) = updated_fix.receiver_id {
            let receiver_repo = self.receiver_repo.clone();
            let receiver_span = tracing::debug_span!(parent: None, "update_receiver_timestamp", receiver_id = %receiver_id);
            let _ = receiver_span.set_parent(opentelemetry::Context::new());
            tokio::spawn(
                async move {
                    if let Err(e) = receiver_repo.update_latest_packet_at(receiver_id).await {
                        error!(
                            "Failed to update latest_packet_at for receiver {}: {}",
                            receiver_id, e
                        );
                    }
                }
                .instrument(receiver_span),
            );
        }

        // Step 3: Update flight callsign if this fix has a flight_id and flight_number
        // IMPORTANT: Only update from NULL to a value, never from one non-null value to another.
        // If callsign changes from one value to another, the flight tracker will create a new flight.
        if let (Some(flight_id), Some(flight_number)) =
            (updated_fix.flight_id, &updated_fix.flight_number)
            && !flight_number.is_empty()
        {
            let callsign_update_start = std::time::Instant::now();
            // Get the current flight to check if callsign needs updating
            match self
                .flight_detection_processor
                .get_flight_by_id(flight_id)
                .await
            {
                Ok(Some(flight)) => {
                    // Only update if current callsign is None (NULL -> value update)
                    // Do NOT update if callsign already has a different value (that would indicate a bug)
                    match &flight.callsign {
                        None => {
                            if let Err(e) = self
                                .flight_detection_processor
                                .update_flight_callsign(
                                    flight_id,
                                    updated_fix.aircraft_id,
                                    Some(flight_number.clone()),
                                    updated_fix.received_at,
                                )
                                .await
                            {
                                debug!("Failed to update callsign for flight {}: {}", flight_id, e);
                            }
                        }
                        Some(existing) if existing != flight_number => {
                            // Callsign changed from one value to another - this should not happen
                            // because flight tracker should have created a new flight
                            error!(
                                "Flight {} callsign mismatch: has '{}' but fix has '{}' - this indicates a bug in flight coalescing",
                                flight_id, existing, flight_number
                            );
                        }
                        Some(_) => {} // Callsign already matches, nothing to do
                    }
                }
                Ok(None) => {
                    trace!(
                        "Flight {} not found when trying to update callsign",
                        flight_id
                    );
                }
                Err(e) => {
                    debug!(
                        "Failed to fetch flight {} for callsign update: {}",
                        flight_id, e
                    );
                }
            }
            metrics::histogram!("aprs.aircraft.callsign_update_ms")
                .record(callsign_update_start.elapsed().as_micros() as f64 / 1000.0);
        }

        // Step 4: Publish to NATS with updated fix (includes flight_id for clients to fetch if needed)
        if let Some(nats_publisher) = self.nats_publisher.as_ref() {
            let nats_publish_start = std::time::Instant::now();
            metrics::counter!("aprs.aircraft.stage_total", "stage" => "before_nats").increment(1);
            nats_publisher
                .process_fix(updated_fix.clone(), raw_message)
                .await;
            metrics::histogram!("aprs.aircraft.nats_publish_ms")
                .record(nats_publish_start.elapsed().as_micros() as f64 / 1000.0);
        }

        metrics::counter!("aprs.aircraft.stage_total", "stage" => "completed").increment(1);
    }
}
