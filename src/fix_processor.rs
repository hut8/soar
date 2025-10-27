use num_traits::AsPrimitive;
use tracing::Instrument;
use tracing::{debug, error, trace, warn};

use crate::Fix;
use crate::device_repo::DeviceRepository;
use crate::elevation::ElevationTask;
use crate::fixes_repo::FixesRepository;
use crate::flight_tracker::FlightTracker;
use crate::nats_publisher::NatsFixPublisher;
use crate::packet_processors::generic::PacketContext;
use crate::receiver_repo::ReceiverRepository;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use ogn_parser::AprsPacket;
use tokio::sync::mpsc;

/// Database fix processor that saves valid fixes to the database and performs flight tracking
#[derive(Clone)]
pub struct FixProcessor {
    fixes_repo: FixesRepository,
    device_repo: DeviceRepository,
    receiver_repo: ReceiverRepository,
    flight_detection_processor: FlightTracker,
    nats_publisher: Option<NatsFixPublisher>,
    /// Sender for elevation processing tasks (bounded channel to prevent queue overflow)
    elevation_tx: Option<mpsc::Sender<ElevationTask>>,
}

impl FixProcessor {
    pub fn new(diesel_pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self {
            fixes_repo: FixesRepository::new(diesel_pool.clone()),
            device_repo: DeviceRepository::new(diesel_pool.clone()),
            receiver_repo: ReceiverRepository::new(diesel_pool.clone()),
            flight_detection_processor: FlightTracker::new(&diesel_pool),
            nats_publisher: None,
            elevation_tx: None,
        }
    }

    /// Add elevation channel sender to the processor
    pub fn with_elevation_channel(mut self, elevation_tx: mpsc::Sender<ElevationTask>) -> Self {
        self.elevation_tx = Some(elevation_tx);
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
            elevation_tx: None,
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
            elevation_tx: None,
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
            elevation_tx: None,
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
        let received_at = chrono::Utc::now();

        // Try to create a fix from the packet
        match packet.data {
            ogn_parser::AprsData::Position(ref pos_packet) => {
                let mut device_address = 0i32;
                let mut address_type = crate::devices::AddressType::Unknown;

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
                }

                // When creating devices spontaneously (not from DDB), determine address_type from tracker_device_type
                // tracker_device_type is the packet destination (e.g., "OGFLR", "OGADSB")
                let tracker_device_type = packet.to.to_string();
                let spontaneous_address_type = match tracker_device_type.as_str() {
                    "OGFLR" => crate::devices::AddressType::Flarm,
                    "OGADSB" => crate::devices::AddressType::Icao,
                    _ => crate::devices::AddressType::Unknown,
                };

                // Look up or create device based on device_address
                // When creating spontaneously, use address_type derived from tracker_device_type
                match self
                    .device_repo
                    .get_or_insert_device_by_address(device_address, spontaneous_address_type)
                    .await
                {
                    Ok(device_model) => {
                        // Extract ICAO model code and ADS-B emitter category from packet for device update
                        let icao_model_code: Option<String> = pos_packet
                            .comment
                            .model
                            .as_ref()
                            .map(|model| model.to_string());
                        let adsb_emitter_category = pos_packet
                            .comment
                            .adsb_emitter_category
                            .and_then(|cat| cat.to_string().parse().ok());

                        // Check if we have new/different information to update
                        let icao_changed = icao_model_code.is_some()
                            && icao_model_code != device_model.icao_model_code;
                        let adsb_changed = adsb_emitter_category.is_some()
                            && adsb_emitter_category != device_model.adsb_emitter_category;
                        let tracker_type_changed =
                            Some(&tracker_device_type) != device_model.tracker_device_type.as_ref();

                        // Update device fields only if we have new/different information
                        if icao_changed || adsb_changed || tracker_type_changed {
                            let device_repo = self.device_repo.clone();
                            let device_id = device_model.id;
                            let tracker_type_to_update = Some(tracker_device_type.clone());
                            tokio::spawn(
                                async move {
                                    if let Err(e) = device_repo
                                        .update_adsb_fields(
                                            device_id,
                                            icao_model_code,
                                            adsb_emitter_category,
                                            tracker_type_to_update,
                                        )
                                        .await
                                    {
                                        error!(
                                            "Failed to update ADS-B fields for device {}: {}",
                                            device_id, e
                                        );
                                    }
                                }
                                .instrument(tracing::debug_span!("update_device_adsb_fields", device_id = %device_id))
                            );
                        }

                        // Device exists or was just created, create fix with proper device_id
                        match Fix::from_aprs_packet(packet, received_at, device_model.id) {
                            Ok(Some(mut fix)) => {
                                // Set the aprs_message_id and receiver_id from context
                                fix.aprs_message_id = Some(context.aprs_message_id);
                                fix.receiver_id = Some(context.receiver_id);
                                self.process_fix_internal(fix, raw_message).await;
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
    async fn process_fix_internal(&self, fix: Fix, raw_message: &str) {
        // Step 1 & 2: Process through flight detection AND save to database
        // This is done atomically while holding a per-device lock to prevent race conditions
        let updated_fix = match self
            .flight_detection_processor
            .process_and_insert_fix(fix, &self.fixes_repo)
            .await
        {
            Some(fix) => fix,
            None => return, // Fix was discarded (duplicate, error, etc.)
        };

        // Step 3: Update receiver's latest_packet_at if this fix has a receiver_id
        if let Some(receiver_id) = updated_fix.receiver_id {
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
        }

        // Step 4: Update device cached fields (aircraft_type_ogn and last_fix_at)
        let device_repo = self.device_repo.clone();
        let device_id = updated_fix.device_id;
        let aircraft_type = updated_fix.aircraft_type_ogn;
        let fix_timestamp = updated_fix.timestamp;
        tokio::spawn(
            async move {
                if let Err(e) = device_repo
                    .update_cached_fields(device_id, aircraft_type, fix_timestamp)
                    .await
                {
                    error!(
                        "Failed to update cached fields for device {}: {}",
                        device_id, e
                    );
                }
            }
            .instrument(
                tracing::debug_span!("update_device_cached_fields", device_id = %device_id),
            ),
        );

        // Step 2.5: Calculate and update altitude_agl via dedicated elevation channel
        // This prevents slow elevation lookups from blocking the main processing queue
        if let Some(elevation_tx) = &self.elevation_tx {
            let task = ElevationTask {
                fix_id: updated_fix.id,
                fix: updated_fix.clone(),
                fixes_repo: self.fixes_repo.clone(),
            };

            // Try to send with timeout to detect channel backlog
            match elevation_tx.try_send(task) {
                Ok(()) => {
                    // Successfully queued for elevation processing
                    metrics::counter!("aprs.elevation.queued").increment(1);
                }
                Err(mpsc::error::TrySendError::Full(_)) => {
                    // Channel is full - elevation processing is backed up
                    warn!(
                        "Elevation processing queue is FULL (1000 tasks buffered) - dropping elevation calculation for fix {}. \
                         This indicates elevation lookups are slower than incoming fix rate.",
                        updated_fix.id
                    );
                    metrics::counter!("aprs.elevation.dropped").increment(1);
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    error!(
                        "Elevation processing channel is closed - cannot queue elevation calculation"
                    );
                    metrics::counter!("aprs.elevation.channel_closed").increment(1);
                }
            }
        }

        // Step 3: Publish to NATS with updated fix (including flight_id and flight info)
        if let Some(nats_publisher) = self.nats_publisher.as_ref() {
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
            nats_publisher.process_fix(fix_with_flight, raw_message);
        }
    }
}
