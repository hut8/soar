use num_traits::AsPrimitive;
use tracing::Instrument;
use tracing::{debug, error, trace, warn};

use crate::Fix;
use crate::device_repo::{DevicePacketFields, DeviceRepository};
use crate::elevation::{ElevationService, ElevationTask};
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
    Sync { elevation_db: ElevationService },
    /// Asynchronous: Queue elevation tasks for processing by dedicated workers
    Async {
        channel: flume::Sender<ElevationTask>,
    },
}

/// Database fix processor that saves valid fixes to the database and performs flight tracking
#[derive(Clone)]
pub struct FixProcessor {
    fixes_repo: FixesRepository,
    device_repo: DeviceRepository,
    receiver_repo: ReceiverRepository,
    flight_detection_processor: FlightTracker,
    nats_publisher: Option<NatsFixPublisher>,
    /// Elevation processing mode (sync or async)
    elevation_mode: Option<ElevationMode>,
    /// APRS types to suppress from processing (e.g., OGADSB, OGFLR)
    suppressed_aprs_types: Vec<String>,
}

impl FixProcessor {
    pub fn new(diesel_pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self {
            fixes_repo: FixesRepository::new(diesel_pool.clone()),
            device_repo: DeviceRepository::new(diesel_pool.clone()),
            receiver_repo: ReceiverRepository::new(diesel_pool.clone()),
            flight_detection_processor: FlightTracker::new(&diesel_pool),
            nats_publisher: None,
            elevation_mode: None,
            suppressed_aprs_types: Vec::new(),
        }
    }

    /// Configure synchronous elevation processing
    pub fn with_sync_elevation(mut self, elevation_db: ElevationService) -> Self {
        self.elevation_mode = Some(ElevationMode::Sync { elevation_db });
        self
    }

    /// Configure asynchronous elevation processing (legacy mode)
    pub fn with_async_elevation(mut self, elevation_tx: flume::Sender<ElevationTask>) -> Self {
        self.elevation_mode = Some(ElevationMode::Async {
            channel: elevation_tx,
        });
        self
    }

    /// Set APRS types to suppress from processing
    pub fn with_suppressed_aprs_types(mut self, types: Vec<String>) -> Self {
        self.suppressed_aprs_types = types;
        self
    }

    /// Create a new FixProcessor with NATS publisher
    pub async fn new_with_nats(
        diesel_pool: Pool<ConnectionManager<PgConnection>>,
        nats_url: &str,
    ) -> anyhow::Result<Self> {
        let nats_publisher = NatsFixPublisher::new(nats_url).await?;

        Ok(Self {
            fixes_repo: FixesRepository::new(diesel_pool.clone()),
            device_repo: DeviceRepository::new(diesel_pool.clone()),
            receiver_repo: ReceiverRepository::new(diesel_pool.clone()),
            flight_detection_processor: FlightTracker::new(&diesel_pool),
            nats_publisher: Some(nats_publisher),
            elevation_mode: None,
            suppressed_aprs_types: Vec::new(),
        })
    }

    /// Create a new FixProcessor with a custom FlightTracker (for state persistence)
    pub fn with_flight_tracker(
        diesel_pool: Pool<ConnectionManager<PgConnection>>,
        flight_tracker: FlightTracker,
    ) -> Self {
        Self {
            fixes_repo: FixesRepository::new(diesel_pool.clone()),
            device_repo: DeviceRepository::new(diesel_pool.clone()),
            receiver_repo: ReceiverRepository::new(diesel_pool.clone()),
            flight_detection_processor: flight_tracker,
            nats_publisher: None,
            elevation_mode: None,
            suppressed_aprs_types: Vec::new(),
        }
    }

    /// Create a new FixProcessor with custom FlightTracker and NATS publisher
    pub async fn with_flight_tracker_and_nats(
        diesel_pool: Pool<ConnectionManager<PgConnection>>,
        flight_tracker: FlightTracker,
        nats_url: &str,
    ) -> anyhow::Result<Self> {
        let nats_publisher = NatsFixPublisher::new(nats_url).await?;

        Ok(Self {
            fixes_repo: FixesRepository::new(diesel_pool.clone()),
            device_repo: DeviceRepository::new(diesel_pool.clone()),
            receiver_repo: ReceiverRepository::new(diesel_pool.clone()),
            flight_detection_processor: flight_tracker,
            nats_publisher: Some(nats_publisher),
            elevation_mode: None,
            suppressed_aprs_types: Vec::new(),
        })
    }

    /// Get a reference to the flight tracker for state management
    pub fn flight_tracker(&self) -> &FlightTracker {
        &self.flight_detection_processor
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
        let total_start = std::time::Instant::now();

        // Use the received_at timestamp from context (captured at ingestion time)
        // This ensures accurate timestamps even if messages queue up during processing
        let received_at = context.received_at;

        // Try to create a fix from the packet
        match packet.data {
            ogn_parser::AprsData::Position(ref pos_packet) => {
                let mut device_address = 0i32;
                let mut address_type = crate::devices::AddressType::Unknown;
                let mut aircraft_type = None;

                // Extract device info from OGN parameters
                if let Some(ref id) = pos_packet.comment.id {
                    device_address = id.address.as_();
                    address_type = match id.address_type {
                        0 => crate::devices::AddressType::Unknown,
                        1 => crate::devices::AddressType::Icao,
                        2 => crate::devices::AddressType::Flarm,
                        3 => crate::devices::AddressType::Ogn,
                        _ => crate::devices::AddressType::Unknown,
                    };
                    // Extract aircraft type from OGN parameters
                    aircraft_type = Some(crate::ogn_aprs_aircraft::AircraftType::from(
                        id.aircraft_type,
                    ));
                }

                // When creating devices spontaneously (not from DDB), determine address_type from tracker_device_type
                // tracker_device_type is the packet destination (e.g., "OGFLR", "OGADSB")
                let tracker_device_type = packet.to.to_string();

                // Check if this APRS type is suppressed
                if self
                    .suppressed_aprs_types
                    .iter()
                    .any(|t| t == &tracker_device_type)
                {
                    trace!("Suppressing fix from APRS type: {}", tracker_device_type);
                    metrics::counter!("aprs.fixes.suppressed").increment(1);
                    return;
                }

                let spontaneous_address_type = match tracker_device_type.as_str() {
                    "OGFLR" => crate::devices::AddressType::Flarm,
                    "OGADSB" => crate::devices::AddressType::Icao,
                    _ => crate::devices::AddressType::Unknown,
                };

                // Extract all available fields from packet for device creation/update
                let icao_model_code: Option<String> = pos_packet
                    .comment
                    .model
                    .as_ref()
                    .map(|model| model.to_string());
                let adsb_emitter_category = pos_packet
                    .comment
                    .adsb_emitter_category
                    .and_then(|cat| cat.to_string().parse().ok());
                let registration: Option<String> = pos_packet
                    .comment
                    .registration
                    .as_ref()
                    .map(|reg| reg.to_string());

                let packet_fields = DevicePacketFields {
                    aircraft_type,
                    icao_model_code: icao_model_code.clone(),
                    adsb_emitter_category,
                    tracker_device_type: Some(tracker_device_type.clone()),
                    registration: registration.clone(),
                };

                // Look up or create device based on device_address
                // When creating spontaneously, use address_type derived from tracker_device_type
                // Use device_for_fix to atomically update last_fix_at and insert all available fields
                let device_lookup_start = std::time::Instant::now();
                match self
                    .device_repo
                    .device_for_fix(
                        device_address,
                        spontaneous_address_type,
                        received_at,
                        packet_fields,
                    )
                    .await
                {
                    Ok(device_model) => {
                        metrics::histogram!("aprs.aircraft.device_upsert_ms")
                            .record(device_lookup_start.elapsed().as_micros() as f64 / 1000.0);

                        // All device fields (including ICAO, ADS-B, tracker type, registration)
                        // are now updated atomically in device_for_fix - no separate update needed

                        // Device exists or was just created, create fix with proper device_id
                        let fix_creation_start = std::time::Instant::now();
                        match Fix::from_aprs_packet(
                            packet,
                            received_at,
                            device_model.id,
                            context.receiver_id,
                            context.aprs_message_id,
                        ) {
                            Ok(Some(fix)) => {
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
                            "Failed to get or insert device address {:06X} ({:?}): {}, skipping fix processing",
                            device_address, address_type, e
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
    #[tracing::instrument(skip(self, fix, raw_message), fields(device_id = %fix.device_id))]
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

            metrics::histogram!("aprs.elevation.sync_duration_ms")
                .record(elevation_start.elapsed().as_micros() as f64 / 1000.0);
            metrics::counter!("aprs.elevation.sync_processed").increment(1);
        }

        // Step 1: Process through flight detection AND save to database
        // This is done atomically while holding a per-device lock to prevent race conditions
        let flight_insert_start = std::time::Instant::now();
        let updated_fix = match self
            .flight_detection_processor
            .process_and_insert_fix(fix, &self.fixes_repo)
            .await
        {
            Some(fix) => fix,
            None => return, // Fix was discarded (duplicate, error, etc.)
        };
        metrics::histogram!("aprs.aircraft.flight_insert_ms")
            .record(flight_insert_start.elapsed().as_micros() as f64 / 1000.0);

        // Step 2: Update receiver's latest_packet_at
        let receiver_id = updated_fix.receiver_id;
        let receiver_repo = self.receiver_repo.clone();
        tokio::spawn(
            async move {
                if let Err(e) = receiver_repo.update_latest_packet_at(receiver_id).await {
                    error!(
                        "Failed to update latest_packet_at for receiver {}: {}",
                        receiver_id, e
                    );
                }
            }
            .instrument(
                tracing::debug_span!("update_receiver_timestamp", receiver_id = %receiver_id),
            ),
        );

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
                    if flight.callsign.is_none() {
                        if let Err(e) = self
                            .flight_detection_processor
                            .update_flight_callsign(flight_id, Some(flight_number.clone()))
                            .await
                        {
                            debug!("Failed to update callsign for flight {}: {}", flight_id, e);
                        }
                    } else if flight.callsign.as_deref() != Some(flight_number) {
                        // Callsign changed from one value to another - this should not happen
                        // because flight tracker should have created a new flight
                        warn!(
                            "Flight {} callsign mismatch: has '{}' but fix has '{}' - this indicates a bug in flight coalescing",
                            flight_id,
                            flight.callsign.as_ref().unwrap(),
                            flight_number
                        );
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

        // Step 4: Calculate and update altitude_agl via dedicated elevation channel (async mode only)
        // In sync mode, elevation was already calculated before database insert (Step 0)
        // This prevents slow elevation lookups from blocking the main processing queue in async mode
        if let Some(ElevationMode::Async { channel }) = &self.elevation_mode {
            let elevation_queue_start = std::time::Instant::now();
            let task = ElevationTask {
                fix_id: updated_fix.id,
                fix: updated_fix.clone(),
            };

            // Send to elevation queue with blocking send_async
            // Never drops elevation tasks - applies backpressure if queue fills
            // Large queue (100K) prevents backpressure under normal conditions
            match channel.send_async(task).await {
                Ok(_) => {
                    metrics::counter!("aprs.elevation.queued").increment(1);
                }
                Err(_) => {
                    error!("Elevation processing channel is closed");
                    metrics::counter!("aprs.elevation.channel_closed").increment(1);
                }
            }
            metrics::histogram!("aprs.aircraft.elevation_queue_ms")
                .record(elevation_queue_start.elapsed().as_micros() as f64 / 1000.0);
        }

        // Step 5: Publish to NATS with updated fix (including flight_id and flight info)
        if let Some(nats_publisher) = self.nats_publisher.as_ref() {
            let nats_publish_start = std::time::Instant::now();
            // Look up flight information if this fix is part of a flight
            let flight = if let Some(flight_id) = updated_fix.flight_id {
                match self
                    .flight_detection_processor
                    .get_flight_by_id(flight_id)
                    .await
                {
                    Ok(Some(flight)) => Some(flight),
                    Ok(None) => {
                        warn!(
                            "Fix {} has flight_id {} but flight not found",
                            updated_fix.id, flight_id
                        );
                        None
                    }
                    Err(e) => {
                        error!(
                            "Failed to fetch flight {} for fix {}: {}",
                            flight_id, updated_fix.id, e
                        );
                        None
                    }
                }
            } else {
                None
            };

            let fix_with_flight = crate::fixes::FixWithFlightInfo::new(updated_fix.clone(), flight);
            nats_publisher
                .process_fix(fix_with_flight, raw_message)
                .await;
            metrics::histogram!("aprs.aircraft.nats_publish_ms")
                .record(nats_publish_start.elapsed().as_micros() as f64 / 1000.0);
        }
    }
}
