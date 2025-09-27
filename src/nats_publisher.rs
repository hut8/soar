use anyhow::Result;
use async_nats::Client;
use serde_json;
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::Fix;
use crate::aprs_client::FixHandler;

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
#[derive(Clone)]
pub struct NatsFixPublisher {
    nats_client: Arc<Client>,
}

impl NatsFixPublisher {
    /// Create a new NATS publisher for position fixes
    pub async fn new(
        nats_url: &str,
    ) -> Result<Self> {
        info!("Connecting to NATS server at {}", nats_url);
        let nats_client = async_nats::ConnectOptions::new()
            .name("soar-run")
            .connect(nats_url)
            .await?;

        Ok(Self {
            nats_client: Arc::new(nats_client),
        })
    }
}

impl FixHandler for NatsFixPublisher {
    fn process_fix(&self, fix: Fix, _raw_message: &str) {
        // Clone the client and device repo for the async task
        let nats_client = Arc::clone(&self.nats_client);


        tokio::spawn(async move {
            // Use device UUID as the device_id in NATS subject
            if let Err(e) = publish_to_nats(&nats_client, &fix.device_id.to_string(), &fix).await {
                error!("Failed to publish fix for device {}: {}", fix.device_id, e);
            } else {
                info!("Published fix for device {}", fix.device_id);
            }
        });
    }
}
