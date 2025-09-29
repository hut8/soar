use anyhow::Result;
use async_nats::{Client, Subscriber};
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use tracing::{error, info, warn};

/// Get the topic prefix based on the environment
fn get_topic_prefix() -> &'static str {
    match std::env::var("SOAR_ENV") {
        Ok(env) if env == "production" => "aircraft",
        _ => "staging.aircraft",
    }
}

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

// Subscription management structure (used for both device and area subscriptions)
struct Subscription {
    broadcaster: broadcast::Sender<LiveFix>,
    subscriber_count: usize,
    task_handle: tokio::task::JoinHandle<()>,
}

// Shared service that manages on-demand NATS subscriptions
#[derive(Clone)]
pub struct LiveFixService {
    nats_client: Arc<Client>,
    subscriptions: Arc<Mutex<HashMap<String, Subscription>>>,
}

impl LiveFixService {
    pub async fn new(nats_url: &str) -> Result<Self> {
        // nats client name: soar-web (production) or soar-web-dev (development)
        let nats_client_name = if std::env::var("SOAR_ENV") == Ok("production".into()) {
            "soar-web"
        } else {
            "soar-web-dev"
        };
        let nats_client = async_nats::ConnectOptions::new()
            .name(nats_client_name)
            .connect(nats_url)
            .await?;

        Ok(Self {
            nats_client: Arc::new(nats_client),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    // Subscribe to a specific device - creates NATS subscription on-demand
    pub async fn subscribe_to_device(
        &self,
        device_id: &str,
    ) -> Result<broadcast::Receiver<LiveFix>> {
        let mut subscriptions = self.subscriptions.lock().await;

        // If we already have a subscription for this device, just create a new receiver
        if let Some(subscription) = subscriptions.get_mut(device_id) {
            subscription.subscriber_count += 1;
            info!(
                "Added subscriber for device {} (total: {})",
                device_id, subscription.subscriber_count
            );
            return Ok(subscription.broadcaster.subscribe());
        }

        // Create new NATS subscription for this specific device
        let topic_prefix = get_topic_prefix();
        let subject = format!("{}.fix.{}", topic_prefix, device_id);
        let subscriber = self.nats_client.subscribe(subject.clone()).await?;
        let (broadcaster, receiver) = broadcast::channel(100);

        info!(
            "Creating new NATS subscription for device: {} on subject: {}",
            device_id, subject
        );

        // Spawn task to handle messages for this device
        let device_id_clone = device_id.to_string();
        let broadcaster_clone = broadcaster.clone();

        let task_handle = tokio::spawn(async move {
            Self::handle_device_messages(subscriber, device_id_clone, broadcaster_clone).await;
        });

        // Store the subscription
        let subscription = Subscription {
            broadcaster: broadcaster.clone(),
            subscriber_count: 1,
            task_handle,
        };

        subscriptions.insert(device_id.to_string(), subscription);

        Ok(receiver)
    }

    // Subscribe to a specific area - creates NATS subscription on-demand
    pub async fn subscribe_to_area(
        &self,
        latitude: i32,
        longitude: i32,
    ) -> Result<broadcast::Receiver<LiveFix>> {
        let area_key = format!("area.{}.{}", latitude, longitude);
        let mut subscriptions = self.subscriptions.lock().await;

        // If we already have a subscription for this area, just create a new receiver
        if let Some(subscription) = subscriptions.get_mut(&area_key) {
            subscription.subscriber_count += 1;
            info!(
                "Added subscriber for area {}.{} (total: {})",
                latitude, longitude, subscription.subscriber_count
            );
            return Ok(subscription.broadcaster.subscribe());
        }

        // Create new NATS subscription for this specific area
        let topic_prefix = get_topic_prefix();
        let subject = format!("{}.area.{}.{}", topic_prefix, latitude, longitude);
        let subscriber = self.nats_client.subscribe(subject.clone()).await?;
        let (broadcaster, receiver) = broadcast::channel(100);

        info!(
            "Creating new NATS subscription for area {}.{} on subject: {}",
            latitude, longitude, subject
        );

        // Spawn task to handle messages for this area
        let area_key_clone = area_key.clone();
        let broadcaster_clone = broadcaster.clone();

        let task_handle = tokio::spawn(async move {
            Self::handle_area_messages(subscriber, area_key_clone, broadcaster_clone).await;
        });

        // Store the subscription
        let subscription = Subscription {
            broadcaster: broadcaster.clone(),
            subscriber_count: 1,
            task_handle,
        };

        subscriptions.insert(area_key, subscription);

        Ok(receiver)
    }

    // Handle messages for a specific device
    async fn handle_device_messages(
        mut subscriber: Subscriber,
        device_id: String,
        broadcaster: broadcast::Sender<LiveFix>,
    ) {
        info!("Started message handler for device: {}", device_id);

        while let Some(msg) = subscriber.next().await {
            match Self::process_fix_message(msg, &device_id).await {
                Ok(live_fix) => {
                    match broadcaster.send(live_fix) {
                        Ok(receiver_count) => {
                            info!(
                                "Broadcasted live fix for device {} to {} receivers",
                                device_id, receiver_count
                            );
                        }
                        Err(broadcast::error::SendError(_)) => {
                            info!("No active receivers for device {}, fix dropped", device_id);
                            // Could consider breaking here if no receivers for a while
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to process fix message for device {}: {}",
                        device_id, e
                    );
                }
            }
        }

        warn!("Message handler for device {} has stopped", device_id);
    }

    // Handle messages for a specific area
    async fn handle_area_messages(
        mut subscriber: Subscriber,
        area_key: String,
        broadcaster: broadcast::Sender<LiveFix>,
    ) {
        info!("Started message handler for area: {}", area_key);

        while let Some(msg) = subscriber.next().await {
            match Self::process_area_fix_message(msg, &area_key).await {
                Ok(live_fix) => match broadcaster.send(live_fix) {
                    Ok(receiver_count) => {
                        info!(
                            "Broadcasted live fix for area {} to {} receivers",
                            area_key, receiver_count
                        );
                    }
                    Err(broadcast::error::SendError(_)) => {
                        info!("No active receivers for area {}, fix dropped", area_key);
                    }
                },
                Err(e) => {
                    error!("Failed to process fix message for area {}: {}", area_key, e);
                }
            }
        }

        warn!("Message handler for area {} has stopped", area_key);
    }

    // Process a single fix message
    async fn process_fix_message(
        msg: async_nats::Message,
        expected_device_id: &str,
    ) -> Result<LiveFix> {
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

    // Process a single fix message from area subscription
    async fn process_area_fix_message(msg: async_nats::Message, area_key: &str) -> Result<LiveFix> {
        // Parse the full Fix data from NATS
        let fix: crate::Fix = serde_json::from_slice(&msg.payload)?;

        // Convert Fix to LiveFix for WebSocket clients
        // For area subscriptions, we use the actual device_id from the fix
        let live_fix = LiveFix {
            id: fix.id.to_string(),
            device_id: fix.device_id.to_string(),
            timestamp: fix.timestamp.to_rfc3339(),
            latitude: fix.latitude,
            longitude: fix.longitude,
            altitude: fix.altitude_feet.unwrap_or(0) as f64,
            track: fix.track_degrees.unwrap_or(0.0) as f64,
            ground_speed: fix.ground_speed_knots.unwrap_or(0.0) as f64,
            climb_rate: fix.climb_fpm.unwrap_or(0) as f64,
        };

        info!(
            "Processing live fix from area {} for device {} at ({}, {}) alt={}ft",
            area_key, live_fix.device_id, live_fix.latitude, live_fix.longitude, live_fix.altitude
        );

        Ok(live_fix)
    }

    // Unsubscribe from a device - removes NATS subscription when no more clients
    pub async fn unsubscribe_from_device(&self, device_id: &str) -> Result<()> {
        let mut subscriptions = self.subscriptions.lock().await;

        if let Some(subscription) = subscriptions.get_mut(device_id) {
            subscription.subscriber_count -= 1;
            info!(
                "Removed subscriber for device {} (remaining: {})",
                device_id, subscription.subscriber_count
            );

            // If no more subscribers, clean up the subscription
            if subscription.subscriber_count == 0 {
                info!(
                    "No more subscribers for device {}, cleaning up NATS subscription",
                    device_id
                );

                // Cancel the message handler task
                subscription.task_handle.abort();

                // Remove from our subscriptions map
                subscriptions.remove(device_id);
            }
        } else {
            warn!(
                "Attempted to unsubscribe from device {} but no subscription found",
                device_id
            );
        }

        Ok(())
    }

    // Unsubscribe from an area - removes NATS subscription when no more clients
    pub async fn unsubscribe_from_area(&self, latitude: i32, longitude: i32) -> Result<()> {
        let area_key = format!("area.{}.{}", latitude, longitude);
        let mut subscriptions = self.subscriptions.lock().await;

        if let Some(subscription) = subscriptions.get_mut(&area_key) {
            subscription.subscriber_count -= 1;
            info!(
                "Removed subscriber for area {}.{} (remaining: {})",
                latitude, longitude, subscription.subscriber_count
            );

            // If no more subscribers, clean up the subscription
            if subscription.subscriber_count == 0 {
                info!(
                    "No more subscribers for area {}.{}, cleaning up NATS subscription",
                    latitude, longitude
                );

                // Cancel the message handler task
                subscription.task_handle.abort();

                // Remove from our subscriptions map
                subscriptions.remove(&area_key);
            }
        } else {
            warn!(
                "Attempted to unsubscribe from area {}.{} but no subscription found",
                latitude, longitude
            );
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
