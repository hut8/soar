use tracing::{error, trace, warn};

use crate::device_repo::DeviceRepository;
use crate::fixes;
use crate::fixes_repo::FixesRepository;
use crate::{Fix, FixProcessor};
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};

/// Database fix processor that saves valid fixes to the database
pub struct DatabaseFixProcessor {
    fixes_repo: FixesRepository,
    device_repo: DeviceRepository,
}

impl DatabaseFixProcessor {
    pub fn new(diesel_pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self {
            fixes_repo: FixesRepository::new(diesel_pool.clone()),
            device_repo: DeviceRepository::new(diesel_pool),
        }
    }
}

impl FixProcessor for DatabaseFixProcessor {
    fn process_fix(&self, fix: Fix, raw_message: &str) {
        // Check device_id against devices table first - if not found, skip processing
        if let Some(device_id) = fix.device_id {
            let device_repo = self.device_repo.clone();
            let fixes_repo = self.fixes_repo.clone();
            let raw_message = raw_message.to_string();
            tokio::spawn(async move {
                // Check if device exists in database
                match device_repo.get_device_by_id(device_id).await {
                    Ok(Some(_device)) => {
                        // Device exists, proceed with processing
                        // Convert the position::Fix to a database Fix struct
                        let db_fix = fixes::Fix::from_position_fix(&fix, raw_message.to_string());

                        // Save to database
                        match fixes_repo.insert(&db_fix).await {
                            Ok(_) => {
                                trace!(
                                    "Successfully saved fix to database for aircraft {:?}",
                                    db_fix.aircraft_id
                                );
                            }
                            Err(e) => {
                                error!(
                                    "Failed to save fix to database for fix: {:?}\ncause:{:?}",
                                    db_fix, e
                                );
                            }
                        }
                    }
                    Ok(None) => {
                        warn!(
                            "Device ID {} not found in devices table, skipping fix processing",
                            device_id
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to lookup device ID {}: {}, skipping fix processing",
                            device_id, e
                        );
                    }
                }
            });
        } else {
            warn!("Fix has no device_id, skipping processing: {:?}", fix);
        }
    }
}
