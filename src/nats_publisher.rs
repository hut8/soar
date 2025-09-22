use anyhow::Result;
use async_nats::Client;
use serde_json;
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::Fix;
use crate::aprs_client::FixProcessor;
use crate::device_repo::DeviceRepository;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};

/// Get registration number for a given device address and type
/// This maps OGN/FLARM device addresses to aircraft registration numbers
async fn get_registration_for_device(
    device_repo: &DeviceRepository,
    device_address: u32,
    address_type: crate::devices::AddressType,
) -> Option<String> {
    match device_repo
        .get_device_by_address(device_address, address_type)
        .await
    {
        Ok(Some(device)) => {
            if !device.registration.is_empty() {
                Some(device.registration)
            } else {
                debug!(
                    "Device {:06X} ({:?}) found but has no registration",
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
async fn publish_to_nats(nats_client: &Client, aircraft_id: &str, fix: &Fix) -> Result<()> {
    let subject = format!("aircraft.fix.{}", aircraft_id);

    // Serialize the Fix to JSON
    let payload = serde_json::to_vec(fix)?;

    nats_client.publish(subject, payload.into()).await?;
    debug!("Published fix for {} to NATS", aircraft_id);

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

impl FixProcessor for NatsFixPublisher {
    fn process_fix(&self, fix: Fix, _raw_message: &str) {
        // Clone the client and device repo for the async task
        let nats_client = Arc::clone(&self.nats_client);
        let device_repo = self.device_repo.clone();

        tokio::spawn(async move {
            // Get aircraft identifier from the fix
            let aircraft_id = if let Some(registration) = &fix.registration {
                // Use registration if available
                registration.clone()
            } else if let (Some(device_address), Some(address_type)) =
                (fix.device_address, fix.address_type)
            {
                // Try to look up registration from device database
                if let Some(registration) =
                    get_registration_for_device(&device_repo, device_address, address_type).await
                {
                    registration
                } else {
                    // Use aircraft ID with type prefix if no registration found
                    fix.get_aircraft_identifier()
                        .unwrap_or_else(|| format!("UNKNOWN-{}", fix.source))
                }
            } else {
                // Fallback to source callsign
                fix.source.clone()
            };

            if let Err(e) = publish_to_nats(&nats_client, &aircraft_id, &fix).await {
                error!("Failed to publish fix for {}: {}", aircraft_id, e);
            } else {
                info!("Published fix for aircraft {}", aircraft_id);
            }
        });
    }
}
