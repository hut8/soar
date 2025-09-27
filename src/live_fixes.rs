use anyhow::Result;
use async_nats::{Client, Message};
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveFix {
    pub id: String,
    pub device_id: String,
    pub timestamp: String, // ISO 8601 string format for frontend compatibility
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub track: f64,
    pub ground_speed: f64,
    pub climb_rate: f64,
}

pub type FixBroadcaster = Arc<RwLock<HashMap<String, broadcast::Sender<LiveFix>>>>;

#[derive(Clone)]
pub struct LiveFixService {
    nats_client: Client,
    broadcasters: FixBroadcaster,
}

impl LiveFixService {
    pub async fn new(nats_url: &str) -> Result<Self> {
        let nats_client = async_nats::ConnectOptions::new()
            .name("soar-web")
            .connect(nats_url)
            .await?;
        let broadcasters = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            nats_client,
            broadcasters,
        })
    }

    pub async fn start_listening(&self) -> Result<()> {
        let mut subscriber = self.nats_client.subscribe("aircraft.fix.*").await?;
        let broadcasters = self.broadcasters.clone();

        tokio::spawn(async move {
            while let Some(msg) = subscriber.next().await {
                if let Err(e) = Self::handle_nats_message(msg, &broadcasters).await {
                    error!("Error handling NATS message: {}", e);
                }
            }
        });

        info!("Started listening for live fixes on NATS");
        Ok(())
    }

    async fn handle_nats_message(msg: Message, broadcasters: &FixBroadcaster) -> Result<()> {
        // Extract device ID from subject (e.g., "aircraft.fix.uuid")
        let subject_parts: Vec<&str> = msg.subject.split('.').collect();
        if subject_parts.len() != 3 || subject_parts[0] != "aircraft" || subject_parts[1] != "fix" {
            warn!("Invalid subject format: {}", msg.subject);
            return Ok(());
        }

        let device_id = subject_parts[2].to_string();

        // Parse the full Fix data from NATS
        let fix: crate::Fix = match serde_json::from_slice(&msg.payload) {
            Ok(fix) => fix,
            Err(e) => {
                error!("Failed to parse fix JSON from NATS: {}", e);
                return Ok(());
            }
        };

        // Convert Fix to LiveFix for WebSocket clients
        let live_fix = LiveFix {
            id: fix.id.to_string(),
            device_id: device_id.clone(),
            timestamp: fix.timestamp.to_rfc3339(),
            latitude: fix.latitude,
            longitude: fix.longitude,
            altitude: fix.altitude_feet.unwrap_or(0) as f64,
            track: fix.track_degrees.unwrap_or(0.0) as f64,
            ground_speed: fix.ground_speed_knots.unwrap_or(0.0) as f64,
            climb_rate: fix.climb_fpm.unwrap_or(0) as f64,
        };

        info!(
            "Processing live fix for device {} at ({}, {}) alt={}ft",
            device_id, live_fix.latitude, live_fix.longitude, live_fix.altitude
        );

        // Get or create broadcaster for this device
        let broadcaster = {
            let mut broadcasters_write = broadcasters.write().await;
            broadcasters_write
                .entry(device_id.clone())
                .or_insert_with(|| {
                    let (tx, _) = broadcast::channel(100);
                    tx
                })
                .clone()
        };

        // Broadcast the fix to all subscribers
        match broadcaster.send(live_fix) {
            Ok(receiver_count) => {
                info!(
                    "Broadcasted live fix for device {} to {} receivers",
                    device_id, receiver_count
                );
            }
            Err(broadcast::error::SendError(_)) => {
                // This error occurs when there are no receivers, which is normal
                info!("No active receivers for device {}, fix dropped", device_id);
            }
        }

        Ok(())
    }

    pub async fn get_receiver(&self, device_id: &str) -> broadcast::Receiver<LiveFix> {
        let mut broadcasters_write = self.broadcasters.write().await;
        let broadcaster = broadcasters_write
            .entry(device_id.to_string())
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(100);
                tx
            });
        broadcaster.subscribe()
    }

    pub async fn cleanup_aircraft(&self, device_id: &str) {
        let mut broadcasters_write = self.broadcasters.write().await;
        if let Some(broadcaster) = broadcasters_write.get(device_id)
            && broadcaster.receiver_count() == 0
        {
            broadcasters_write.remove(device_id);
            info!("Cleaned up broadcaster for device: {}", device_id);
        }
    }
}
