use num_traits::AsPrimitive;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, trace};
use uuid::Uuid;

use crate::Fix;
use crate::aircraft_registrations_repo::AircraftRegistrationsRepository;
use crate::device_repo::DeviceRepository;
use crate::fixes_repo::{AircraftTypeOgn, FixesRepository};
use crate::flight_tracker::FlightTracker;
use crate::nats_publisher::NatsFixPublisher;
use crate::receiver_repo::ReceiverRepository;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use ogn_parser::AprsPacket;

/// Database fix processor that saves valid fixes to the database and performs flight tracking
#[derive(Clone)]
pub struct FixProcessor {
    fixes_repo: FixesRepository,
    device_repo: DeviceRepository,
    aircraft_registrations_repo: AircraftRegistrationsRepository,
    receiver_repo: ReceiverRepository,
    flight_detection_processor: FlightTracker,
    nats_publisher: Option<NatsFixPublisher>,
    /// Cache to track tow plane status updates to avoid unnecessary database calls
    /// Maps device_id -> (aircraft_type, is_tow_plane_in_db)
    tow_plane_cache: Arc<RwLock<HashMap<Uuid, (AircraftTypeOgn, bool)>>>,
}

impl FixProcessor {
    pub fn new(diesel_pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self {
            fixes_repo: FixesRepository::new(diesel_pool.clone()),
            device_repo: DeviceRepository::new(diesel_pool.clone()),
            aircraft_registrations_repo: AircraftRegistrationsRepository::new(diesel_pool.clone()),
            receiver_repo: ReceiverRepository::new(diesel_pool.clone()),
            flight_detection_processor: FlightTracker::new(&diesel_pool),
            nats_publisher: None,
            tow_plane_cache: Arc::new(RwLock::new(HashMap::new())),
        }
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
            aircraft_registrations_repo: AircraftRegistrationsRepository::new(diesel_pool.clone()),
            receiver_repo: ReceiverRepository::new(diesel_pool.clone()),
            flight_detection_processor: FlightTracker::new(&diesel_pool),
            nats_publisher: Some(nats_publisher),
            tow_plane_cache: Arc::new(RwLock::new(HashMap::new())),
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
            aircraft_registrations_repo: AircraftRegistrationsRepository::new(diesel_pool.clone()),
            receiver_repo: ReceiverRepository::new(diesel_pool.clone()),
            flight_detection_processor: flight_tracker,
            nats_publisher: None,
            tow_plane_cache: Arc::new(RwLock::new(HashMap::new())),
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
            aircraft_registrations_repo: AircraftRegistrationsRepository::new(diesel_pool.clone()),
            receiver_repo: ReceiverRepository::new(diesel_pool.clone()),
            flight_detection_processor: flight_tracker,
            nats_publisher: Some(nats_publisher),
            tow_plane_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get a reference to the flight tracker for state management
    pub fn flight_tracker(&self) -> &FlightTracker {
        &self.flight_detection_processor
    }

    /// Update tow plane status based on aircraft type from fix (static version for use in async spawned task)
    async fn update_tow_plane_status_static(
        aircraft_registrations_repo: AircraftRegistrationsRepository,
        tow_plane_cache: Arc<RwLock<HashMap<Uuid, (AircraftTypeOgn, bool)>>>,
        device_id: Uuid,
        aircraft_type: AircraftTypeOgn,
    ) {
        let should_be_tow_plane = aircraft_type == AircraftTypeOgn::TowTug;

        // Check cache first
        {
            let cache = tow_plane_cache.read().await;
            if let Some(&(cached_type, cached_is_tow_plane)) = cache.get(&device_id) {
                // If the aircraft type hasn't changed and we know the current DB state, skip
                if cached_type == aircraft_type && cached_is_tow_plane == should_be_tow_plane {
                    return;
                }
            }
        }

        // Update the database
        match aircraft_registrations_repo
            .update_tow_plane_status_by_device_id(device_id, should_be_tow_plane)
            .await
        {
            Ok(was_updated) => {
                if was_updated {
                    debug!(
                        "Updated tow plane status for device {} to {} (aircraft type: {:?})",
                        device_id, should_be_tow_plane, aircraft_type
                    );
                } else {
                    trace!(
                        "Tow plane status already correct for device {} (aircraft type: {:?})",
                        device_id, aircraft_type
                    );
                }

                // Update cache with the new state
                let mut cache = tow_plane_cache.write().await;
                cache.insert(device_id, (aircraft_type, should_be_tow_plane));
            }
            Err(e) => {
                error!(
                    "Failed to update tow plane status for device {}: {}",
                    device_id, e
                );
            }
        }
    }
}

impl FixProcessor {
    /// Process an APRS packet by looking up device and creating a Fix
    /// This is the main entry point that orchestrates the entire pipeline
    pub fn process_aprs_packet(&self, packet: AprsPacket, raw_message: &str) {
        let device_repo = self.device_repo.clone();
        let self_clone = self.clone();
        let raw_message = raw_message.to_string();

        tokio::spawn(async move {
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

                    // Look up or create device based on device_address and address_type
                    match device_repo
                        .get_or_insert_device_by_address(device_address, address_type)
                        .await
                    {
                        Ok(device_model) => {
                            // Device exists or was just created, create fix with proper device_id
                            match Fix::from_aprs_packet(packet, received_at, device_model.id) {
                                Ok(Some(fix)) => {
                                    self_clone.process_fix_internal(fix, &raw_message).await;
                                }
                                Ok(None) => {
                                    trace!("No position fix in APRS position packet");
                                }
                                Err(e) => {
                                    debug!(
                                        "Failed to extract fix from APRS position packet: {}",
                                        e
                                    );
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
        });
    }

    /// Internal method to process a fix through the complete pipeline
    async fn process_fix_internal(&self, fix: Fix, raw_message: &str) {
        // Step 1: Process through flight detection (adds flight_id)
        let updated_fix = match self.flight_detection_processor.process_fix(fix).await {
            Some(fix) => fix,
            None => return, // Fix was discarded (duplicate, etc.)
        };

        // Step 2: Save to database
        match self.fixes_repo.insert(&updated_fix).await {
            Ok(_) => {
                trace!(
                    "Successfully saved fix to database for aircraft {}",
                    updated_fix.device_address_hex()
                );

                // Update receiver's latest_packet_at if this fix has a receiver_id
                if let Some(receiver_id) = updated_fix.receiver_id {
                    let receiver_repo = self.receiver_repo.clone();
                    tokio::spawn(async move {
                        if let Err(e) = receiver_repo.update_latest_packet_at(receiver_id).await {
                            error!(
                                "Failed to update latest_packet_at for receiver {}: {}",
                                receiver_id, e
                            );
                        }
                    });
                }

                // Update device cached fields (aircraft_type_ogn and last_fix_at)
                let device_repo = self.device_repo.clone();
                let device_id = updated_fix.device_id;
                let aircraft_type = updated_fix.aircraft_type_ogn;
                let fix_timestamp = updated_fix.timestamp;
                tokio::spawn(async move {
                    if let Err(e) = device_repo
                        .update_cached_fields(device_id, aircraft_type, fix_timestamp)
                        .await
                    {
                        error!(
                            "Failed to update cached fields for device {}: {}",
                            device_id, e
                        );
                    }
                });
            }
            Err(e) => {
                error!(
                    "Failed to save fix to database for fix: {:?}\ncause:{:?}",
                    updated_fix, e
                );
                return; // Don't continue if we can't save
            }
        }

        // Step 2.5: Calculate and update altitude_agl asynchronously (non-blocking)
        let flight_tracker = self.flight_detection_processor.clone();
        let fixes_repo = self.fixes_repo.clone();
        let fix_id = updated_fix.id;
        let fix_for_agl = updated_fix.clone();

        tokio::spawn(async move {
            flight_tracker
                .calculate_and_update_agl_async(fix_id, &fix_for_agl, fixes_repo)
                .await;
        });

        // Step 3: Publish to NATS with updated fix (including flight_id)
        if let Some(nats_publisher) = self.nats_publisher.as_ref() {
            nats_publisher.process_fix(updated_fix.clone(), raw_message);
        }

        // Step 4: Update tow plane status based on aircraft type from fix
        if let Some(foreign_aircraft_type) = updated_fix.aircraft_type_ogn {
            let aircraft_type = AircraftTypeOgn::from(foreign_aircraft_type);
            let device_id = updated_fix.device_id;

            let aircraft_registrations_repo = self.aircraft_registrations_repo.clone();
            let tow_plane_cache = self.tow_plane_cache.clone();

            Self::update_tow_plane_status_static(
                aircraft_registrations_repo,
                tow_plane_cache,
                device_id,
                aircraft_type,
            )
            .await;
        }
    }
}
