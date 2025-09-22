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
        // Check device_address against devices table first - if not found, skip processing
        if let (Some(device_address), Some(address_type)) = (fix.device_address, fix.address_type) {
            let device_repo = self.device_repo.clone();
            let fixes_repo = self.fixes_repo.clone();
            let raw_message = raw_message.to_string();
            let fix_clone = fix.clone();
            tokio::spawn(async move {
                // Check if device exists in database
                match device_repo
                    .get_device_by_address(device_address, address_type)
                    .await
                {
                    Ok(Some(_device)) => {
                        // Device exists, proceed with processing
                        // Convert the position::Fix to a database Fix struct
                        let db_fix = fixes::Fix::from_position_fix(&fix_clone, raw_message.to_string());

                        // Save to database
                        match fixes_repo.insert(&db_fix).await {
                            Ok(_) => {
                                trace!(
                                    "Successfully saved fix to database for aircraft {}",
                                    fix_clone.device_address_hex()
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
            warn!(
                "Fix has no device_address or address_type, skipping processing: {:?}",
                fix
            );
        }
    }
}
