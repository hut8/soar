use anyhow::Result;
use async_nats::{Client, Subscriber};
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
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

// Subscription management structure
struct DeviceSubscription {
    broadcaster: broadcast::Sender<LiveFix>,
    subscriber_count: usize,
    task_handle: tokio::task::JoinHandle<()>,
}

// Shared service that manages on-demand NATS subscriptions
#[derive(Clone)]
pub struct LiveFixService {
    nats_client: Arc<Client>,
    subscriptions: Arc<Mutex<HashMap<String, DeviceSubscription>>>,
}

impl LiveFixService {
    pub async fn new(nats_url: &str) -> Result<Self> {
        let nats_client = async_nats::ConnectOptions::new()
            .name("soar-web")
            .connect(nats_url)
            .await?;

        Ok(Self {
            nats_client: Arc::new(nats_client),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    // Subscribe to a specific device - creates NATS subscription on-demand
    pub async fn subscribe_to_device(&self, device_id: &str) -> Result<broadcast::Receiver<LiveFix>> {
        let mut subscriptions = self.subscriptions.lock().await;

        // If we already have a subscription for this device, just create a new receiver
        if let Some(subscription) = subscriptions.get_mut(device_id) {
            subscription.subscriber_count += 1;
            info!("Added subscriber for device {} (total: {})", device_id, subscription.subscriber_count);
            return Ok(subscription.broadcaster.subscribe());
        }

        // Create new NATS subscription for this specific device
        let subject = format!("aircraft.fix.{}", device_id);
        let subscriber = self.nats_client.subscribe(subject).await?;
        let (broadcaster, receiver) = broadcast::channel(100);

        info!("Creating new NATS subscription for device: {} on subject: aircraft.fix.{}", device_id, device_id);

        // Spawn task to handle messages for this device
        let device_id_clone = device_id.to_string();
        let broadcaster_clone = broadcaster.clone();

        let task_handle = tokio::spawn(async move {
            Self::handle_device_messages(subscriber, device_id_clone, broadcaster_clone).await;
        });

        // Store the subscription
        let subscription = DeviceSubscription {
            broadcaster: broadcaster.clone(),
            subscriber_count: 1,
            task_handle,
        };

        subscriptions.insert(device_id.to_string(), subscription);

        Ok(receiver)
    }

    // Handle messages for a specific device
    async fn handle_device_messages(
        mut subscriber: Subscriber,
        device_id: String,
        broadcaster: broadcast::Sender<LiveFix>
    ) {
        info!("Started message handler for device: {}", device_id);

        while let Some(msg) = subscriber.next().await {
            match Self::process_fix_message(msg, &device_id).await {
                Ok(live_fix) => {
                    match broadcaster.send(live_fix) {
                        Ok(receiver_count) => {
                            info!("Broadcasted live fix for device {} to {} receivers", device_id, receiver_count);
                        }
                        Err(broadcast::error::SendError(_)) => {
                            info!("No active receivers for device {}, fix dropped", device_id);
                            // Could consider breaking here if no receivers for a while
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to process fix message for device {}: {}", device_id, e);
                }
            }
        }

        warn!("Message handler for device {} has stopped", device_id);
    }

    // Process a single fix message
    async fn process_fix_message(msg: async_nats::Message, expected_device_id: &str) -> Result<LiveFix> {
        // Parse the full Fix data from NATS
        let fix: crate::Fix = serde_json::from_slice(&msg.payload)?;

        // Convert Fix to LiveFix for WebSocket clients
        let live_fix = LiveFix {
            id: fix.id.to_string(),
            device_id: expected_device_id.to_string(),
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
            expected_device_id, live_fix.latitude, live_fix.longitude, live_fix.altitude
        );

        Ok(live_fix)
    }

    // Unsubscribe from a device - removes NATS subscription when no more clients
    pub async fn unsubscribe_from_device(&self, device_id: &str) -> Result<()> {
        let mut subscriptions = self.subscriptions.lock().await;

        if let Some(subscription) = subscriptions.get_mut(device_id) {
            subscription.subscriber_count -= 1;
            info!("Removed subscriber for device {} (remaining: {})", device_id, subscription.subscriber_count);

            // If no more subscribers, clean up the subscription
            if subscription.subscriber_count == 0 {
                info!("No more subscribers for device {}, cleaning up NATS subscription", device_id);

                // Cancel the message handler task
                subscription.task_handle.abort();

                // Remove from our subscriptions map
                subscriptions.remove(device_id);
            }
        } else {
            warn!("Attempted to unsubscribe from device {} but no subscription found", device_id);
        }

        Ok(())
    }

    // Get a receiver for an existing subscription (deprecated - use subscribe_to_device)
    pub async fn get_receiver(&self, device_id: &str) -> broadcast::Receiver<LiveFix> {
        match self.subscribe_to_device(device_id).await {
            Ok(receiver) => receiver,
            Err(e) => {
                error!("Failed to subscribe to device {}: {}", device_id, e);
                // Return a dummy receiver that will never get messages
                let (_tx, rx) = broadcast::channel(1);
                rx
            }
        }
    }

    // Cleanup method (deprecated - use unsubscribe_from_device)
    pub async fn cleanup_aircraft(&self, device_id: &str) {
        if let Err(e) = self.unsubscribe_from_device(device_id).await {
            error!("Failed to unsubscribe from device {}: {}", device_id, e);
        }
    }
}
