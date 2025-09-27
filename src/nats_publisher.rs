use anyhow::Result;
use async_nats::Client;
use serde_json;
use std::sync::Arc;
use tracing::{debug, error, info};
use uuid;

use crate::Fix;
use crate::aprs_client::FixHandler;
use crate::device_repo::DeviceRepository;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};

/// Get device UUID for a given device address and type
/// This maps OGN/FLARM device addresses to device UUIDs for NATS subjects
async fn get_device_uuid_by_address(
    device_repo: &DeviceRepository,
    device_address: u32,
    address_type: crate::devices::AddressType,
) -> Option<uuid::Uuid> {
    match device_repo
        .get_device_by_address(device_address, address_type)
        .await
    {
        Ok(Some(device)) => {
            if let Some(device_id) = device.id {
                Some(device_id)
            } else {
                debug!(
                    "Device {:06X} ({:?}) found but has no UUID",
                    device_address, address_type
                );
                None
            }
        }
        Ok(None) => {
            debug!(
                "No device found for address {:06X} ({:?})",
                device_address, address_type
            );
            None
        }
        Err(e) => {
            error!(
                "Failed to lookup device {:06X} ({:?}): {}",
                device_address, address_type, e
            );
            None
        }
    }
}

/// Publish Fix to NATS
async fn publish_to_nats(nats_client: &Client, device_id: &str, fix: &Fix) -> Result<()> {
    let subject = format!("aircraft.fix.{}", device_id);

    // Serialize the Fix to JSON
    let payload = serde_json::to_vec(fix)?;

    nats_client.publish(subject, payload.into()).await?;
    debug!("Published fix for {} to NATS", device_id);

    Ok(())
}

/// NATS publisher for aircraft position fixes
pub struct NatsFixPublisher {
    nats_client: Arc<Client>,
    device_repo: DeviceRepository,
}

impl NatsFixPublisher {
    /// Create a new NATS publisher for position fixes
    pub async fn new(
        nats_url: &str,
        diesel_pool: Pool<ConnectionManager<PgConnection>>,
    ) -> Result<Self> {
        info!("Connecting to NATS server at {}", nats_url);
        let nats_client = async_nats::connect(nats_url).await?;

        Ok(Self {
            nats_client: Arc::new(nats_client),
            device_repo: DeviceRepository::new(diesel_pool),
        })
    }
}

impl FixHandler for NatsFixPublisher {
    fn process_fix(&self, fix: Fix, _raw_message: &str) {
        // Clone the client and device repo for the async task
        let nats_client = Arc::clone(&self.nats_client);
        let device_repo = self.device_repo.clone();

        tokio::spawn(async move {
            // Extract values we need to avoid partial moves
            let device_address = fix.device_address;
            let address_type = fix.address_type;
            let source = fix.source.clone();

            // Get device UUID for NATS subject
            let device_uuid = if let (Some(device_address), Some(address_type)) =
                (device_address, address_type)
            {
                // Look up device UUID from device database
                get_device_uuid_by_address(&device_repo, device_address, address_type).await
            } else {
                None
            };

            if let Some(device_uuid) = device_uuid {
                // Use device UUID as the device_id in NATS subject
                let device_id = device_uuid.to_string();

                if let Err(e) = publish_to_nats(&nats_client, &device_id, &fix).await {
                    error!("Failed to publish fix for device {}: {}", device_id, e);
                } else {
                    info!("Published fix for device {}", device_id);
                }
            } else {
                // No device UUID found - log warning and skip publishing
                debug!(
                    "Skipping NATS publish for fix from {} - no device UUID found (address: {:?}, type: {:?})",
                    source, device_address, address_type
                );
            }
        });
    }
}
