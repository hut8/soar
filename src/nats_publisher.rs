use anyhow::Result;
use async_nats::Client;
use serde_json;
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::Fix;

/// Get the topic prefix based on the environment
fn get_topic_prefix() -> &'static str {
    match std::env::var("SOAR_ENV") {
        Ok(env) if env == "production" => "aircraft",
        _ => "staging.aircraft",
    }
}

/// Publish Fix to NATS (both device and area topics)
async fn publish_to_nats(nats_client: &Client, device_id: &str, fix: &Fix) -> Result<()> {
    let topic_prefix = get_topic_prefix();

    // Serialize the Fix to JSON once
    let payload = serde_json::to_vec(fix)?;

    // 1. Publish by device ID (existing functionality)
    let device_subject = format!("{}.fix.{}", topic_prefix, device_id);
    nats_client
        .publish(device_subject.clone(), payload.clone().into())
        .await?;
    debug!(
        "Published fix for {} to NATS device subject: {}",
        device_id, device_subject
    );

    // 2. Publish by area (new functionality)
    let area_subject = get_area_subject(topic_prefix, fix.latitude, fix.longitude);
    nats_client
        .publish(area_subject.clone(), payload.into())
        .await?;
    debug!(
        "Published fix for {} to NATS area subject: {}",
        device_id, area_subject
    );

    Ok(())
}

/// Get the area subject for a given latitude and longitude
fn get_area_subject(topic_prefix: &str, latitude: f64, longitude: f64) -> String {
    let lat_floor = latitude.floor() as i32;
    let lon_floor = longitude.floor() as i32;
    format!("{}.area.{}.{}", topic_prefix, lat_floor, lon_floor)
}

/// NATS publisher for aircraft position fixes
#[derive(Clone)]
pub struct NatsFixPublisher {
    nats_client: Arc<Client>,
}

impl NatsFixPublisher {
    /// Create a new NATS publisher for position fixes
    pub async fn new(nats_url: &str) -> Result<Self> {
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

impl NatsFixPublisher {
    /// Process a fix and publish it to NATS
    pub fn process_fix(&self, fix: Fix, _raw_message: &str) {
        // Clone the client for the async task
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
