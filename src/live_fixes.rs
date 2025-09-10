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
    pub aircraft_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<i32>,
    pub ground_speed: Option<f32>,
    pub track: Option<f32>,
}

pub type FixBroadcaster = Arc<RwLock<HashMap<String, broadcast::Sender<LiveFix>>>>;

pub struct LiveFixService {
    nats_client: Client,
    broadcasters: FixBroadcaster,
}

impl LiveFixService {
    pub async fn new(nats_url: &str) -> Result<Self> {
        let nats_client = async_nats::connect(nats_url).await?;
        let broadcasters = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            nats_client,
            broadcasters,
        })
    }

    pub async fn start_listening(&self) -> Result<()> {
        let mut subscriber = self.nats_client.subscribe("fixes.live.*").await?;
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
        // Extract aircraft ID from subject (e.g., "fixes.live.39D304")
        let subject_parts: Vec<&str> = msg.subject.split('.').collect();
        if subject_parts.len() != 3 || subject_parts[0] != "fixes" || subject_parts[1] != "live" {
            warn!("Invalid subject format: {}", msg.subject);
            return Ok(());
        }

        let aircraft_id = subject_parts[2].to_string();

        // Parse the fix data
        let live_fix: LiveFix = match serde_json::from_slice(&msg.payload) {
            Ok(fix) => fix,
            Err(e) => {
                error!("Failed to parse live fix JSON: {}", e);
                return Ok(());
            }
        };

        // Get or create broadcaster for this aircraft
        let broadcaster = {
            let mut broadcasters_write = broadcasters.write().await;
            broadcasters_write
                .entry(aircraft_id.clone())
                .or_insert_with(|| {
                    let (tx, _) = broadcast::channel(100);
                    tx
                })
                .clone()
        };

        // Broadcast the fix to all subscribers
        if let Err(e) = broadcaster.send(live_fix) {
            // This error occurs when there are no receivers, which is normal
            if !matches!(e, broadcast::error::SendError(_)) {
                error!(
                    "Failed to broadcast live fix for aircraft {}: {}",
                    aircraft_id, e
                );
            }
        }

        Ok(())
    }

    pub async fn get_receiver(&self, aircraft_id: &str) -> broadcast::Receiver<LiveFix> {
        let mut broadcasters_write = self.broadcasters.write().await;
        let broadcaster = broadcasters_write
            .entry(aircraft_id.to_string())
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(100);
                tx
            });
        broadcaster.subscribe()
    }

    pub async fn cleanup_aircraft(&self, aircraft_id: &str) {
        let mut broadcasters_write = self.broadcasters.write().await;
        if let Some(broadcaster) = broadcasters_write.get(aircraft_id)
            && broadcaster.receiver_count() == 0
        {
            broadcasters_write.remove(aircraft_id);
            info!("Cleaned up broadcaster for aircraft: {}", aircraft_id);
        }
    }
}
