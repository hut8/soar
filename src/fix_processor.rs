use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, trace};
use uuid::Uuid;

use crate::aircraft_registrations_repo::AircraftRegistrationsRepository;
use crate::device_repo::DeviceRepository;
use crate::fixes;
use crate::fixes_repo::{AircraftTypeOgn, FixesRepository};
use crate::nats_publisher::NatsFixPublisher;
use crate::{Fix, FixHandler};
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};

/// Database fix processor that saves valid fixes to the database and performs flight tracking
pub struct FixProcessor {
    fixes_repo: FixesRepository,
    device_repo: DeviceRepository,
    aircraft_registrations_repo: AircraftRegistrationsRepository,
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
            aircraft_registrations_repo: AircraftRegistrationsRepository::new(diesel_pool),
            nats_publisher: None,
            tow_plane_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new FixProcessor with NATS publisher
    pub async fn new_with_nats(
        diesel_pool: Pool<ConnectionManager<PgConnection>>,
        nats_url: &str,
    ) -> anyhow::Result<Self> {
        let nats_publisher = NatsFixPublisher::new(nats_url, diesel_pool.clone()).await?;

        Ok(Self {
            fixes_repo: FixesRepository::new(diesel_pool.clone()),
            device_repo: DeviceRepository::new(diesel_pool.clone()),
            aircraft_registrations_repo: AircraftRegistrationsRepository::new(diesel_pool),
            nats_publisher: Some(nats_publisher),
            tow_plane_cache: Arc::new(RwLock::new(HashMap::new())),
        })
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

impl FixHandler for FixProcessor {
    fn process_fix(&self, fix: Fix, raw_message: &str) {
        // Check device_address against devices table first - if not found, skip processing
        if let (Some(device_address), Some(address_type)) = (fix.device_address, fix.address_type) {
            let device_repo = self.device_repo.clone();
            let fixes_repo = self.fixes_repo.clone();
            let aircraft_registrations_repo = self.aircraft_registrations_repo.clone();
            let nats_publisher = self.nats_publisher.clone();
            let tow_plane_cache = self.tow_plane_cache.clone();
            let raw_message = raw_message.to_string();
            let fix_clone = fix.clone();
            tokio::spawn(async move {
                // Check if device exists in database
                match device_repo
                    .get_device_model_by_address(device_address, address_type)
                    .await
                {
                    Ok(Some(device_model)) => {
                        // Device exists, proceed with processing
                        // Convert the position::Fix to a database Fix struct
                        let db_fix =
                            fixes::Fix::from_position_fix(&fix_clone, raw_message.to_string());

                        // Save to database
                        match fixes_repo.insert(&db_fix).await {
                            Ok(_) => {
                                trace!(
                                    "Successfully saved fix to database for aircraft {}",
                                    fix_clone.device_address_hex()
                                );

                                // Publish to NATS if publisher is available
                                if let Some(nats_publisher) = nats_publisher.as_ref() {
                                    nats_publisher.process_fix(fix_clone.clone(), &raw_message);
                                }
                            }
                            Err(e) => {
                                error!(
                                    "Failed to save fix to database for fix: {:?}\ncause:{:?}",
                                    db_fix, e
                                );
                            }
                        }

                        // Update tow plane status based on aircraft type from fix
                        if let Some(foreign_aircraft_type) = fix_clone.aircraft_type {
                            let aircraft_type = AircraftTypeOgn::from(foreign_aircraft_type);
                            let device_id = device_model.id;

                            Self::update_tow_plane_status_static(
                                aircraft_registrations_repo,
                                tow_plane_cache,
                                device_id,
                                aircraft_type,
                            )
                            .await;
                        }
                    }
                    Ok(None) => {
                        trace!(
                            "Device address {} ({:?}) not found in devices table, skipping fix processing",
                            fix_clone.device_address_hex(),
                            address_type
                        );
                    }
                    Err(e) => {
                        error!(
                            "Failed to lookup device address {} ({:?}): {}, skipping fix processing",
                            fix_clone.device_address_hex(),
                            address_type,
                            e
                        );
                    }
                }
            });
        } else {
            trace!(
                "Fix has no device_address or address_type, skipping processing: {:?}",
                fix
            );
        }
    }
}
