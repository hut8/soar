use anyhow::Result;
use async_nats::{Client, Subscriber};
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast, watch};
use tracing::Instrument;
use tracing::{error, info, warn};

use crate::connection_status::{BrowserConnectionStatus, ConnectionStatus};

/// Get the topic prefix based on the environment
fn get_topic_prefix() -> &'static str {
    match std::env::var("SOAR_ENV") {
        Ok(env) if env == "production" => "aircraft",
        _ => "staging.aircraft",
    }
}

/// Get the status topic based on the environment
fn get_status_topic() -> &'static str {
    match std::env::var("SOAR_ENV") {
        Ok(env) if env == "production" => "status.connections",
        _ => "staging.status.connections",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveFix {
    pub id: String,
    pub aircraft_id: String,
    pub timestamp: String, // ISO 8601 string format for frontend compatibility
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub track: f64,
    pub ground_speed: f64,
    pub climb_rate: f64,
}

use crate::actions::views::Aircraft;

// Enhanced WebSocket message system with typed messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    #[serde(rename = "fix")]
    Fix(Box<crate::fixes::Fix>),

    #[serde(rename = "aircraft")]
    Aircraft(Box<Aircraft>),

    #[serde(rename = "connection_status")]
    ConnectionStatus(BrowserConnectionStatus),
}

// Subscription management structure (used for both aircraft and area subscriptions)
struct Subscription {
    broadcaster: broadcast::Sender<WebSocketMessage>,
    subscriber_count: usize,
    task_handle: tokio::task::JoinHandle<()>,
}

// Shared service that manages on-demand NATS subscriptions
#[derive(Clone)]
pub struct LiveFixService {
    nats_client: Arc<Client>,
    subscriptions: Arc<Mutex<HashMap<String, Subscription>>>,
    /// Watch channel for connection status updates
    status_tx: watch::Sender<BrowserConnectionStatus>,
    status_rx: watch::Receiver<BrowserConnectionStatus>,
}

impl LiveFixService {
    pub async fn new(nats_url: &str) -> Result<Self> {
        let nats_client_name = crate::nats_client_name("web");
        let nats_client = async_nats::ConnectOptions::new()
            .name(&nats_client_name)
            .connect(nats_url)
            .await?;

        // Create watch channel for connection status
        let (status_tx, status_rx) = watch::channel(BrowserConnectionStatus::default());

        let service = Self {
            nats_client: Arc::new(nats_client),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
            status_tx,
            status_rx,
        };

        // Start status subscription
        service.start_status_subscription().await;

        Ok(service)
    }

    /// Subscribe to connection status updates from ingest
    async fn start_status_subscription(&self) {
        let topic = get_status_topic();
        let status_tx = self.status_tx.clone();

        match self.nats_client.subscribe(topic).await {
            Ok(mut subscriber) => {
                info!("Subscribed to connection status on {}", topic);

                tokio::spawn(async move {
                    while let Some(msg) = subscriber.next().await {
                        match serde_json::from_slice::<ConnectionStatus>(&msg.payload) {
                            Ok(full_status) => {
                                let browser_status = BrowserConnectionStatus::from(&full_status);
                                if status_tx.send(browser_status).is_err() {
                                    // All receivers dropped, stop the task
                                    warn!("Status receivers dropped, stopping status subscription");
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse connection status message: {}", e);
                            }
                        }
                    }
                    warn!("Connection status subscription ended");
                });
            }
            Err(e) => {
                warn!(
                    "Failed to subscribe to connection status (status updates disabled): {}",
                    e
                );
            }
        }
    }

    /// Get a receiver for status updates
    pub fn status_receiver(&self) -> watch::Receiver<BrowserConnectionStatus> {
        self.status_rx.clone()
    }

    /// Get current connection status
    pub fn current_status(&self) -> BrowserConnectionStatus {
        self.status_rx.borrow().clone()
    }

    // Subscribe to a specific aircraft - creates NATS subscription on-demand
    pub async fn subscribe_to_aircraft(
        &self,
        aircraft_id: &str,
    ) -> Result<broadcast::Receiver<WebSocketMessage>> {
        let mut subscriptions = self.subscriptions.lock().await;

        // If we already have a subscription for this aircraft, just create a new receiver
        if let Some(subscription) = subscriptions.get_mut(aircraft_id) {
            subscription.subscriber_count += 1;
            info!(
                "Added subscriber for aircraft {} (total: {})",
                aircraft_id, subscription.subscriber_count
            );
            return Ok(subscription.broadcaster.subscribe());
        }

        // Create new NATS subscription for this specific aircraft
        let topic_prefix = get_topic_prefix();
        let subject = format!("{}.fix.{}", topic_prefix, aircraft_id);
        let subscriber = self.nats_client.subscribe(subject.clone()).await?;
        let (broadcaster, receiver) = broadcast::channel(100);

        info!(
            "Creating new NATS subscription for aircraft: {} on subject: {}",
            aircraft_id, subject
        );

        // Spawn task to handle messages for this aircraft
        let aircraft_id_clone = aircraft_id.to_string();
        let broadcaster_clone = broadcaster.clone();

        let task_handle = tokio::spawn(
            async move {
                Self::handle_aircraft_messages(subscriber, aircraft_id_clone, broadcaster_clone)
                    .await;
            }
            .instrument(
                tracing::info_span!("live_fix_aircraft_handler", aircraft_id = %aircraft_id),
            ),
        );

        // Store the subscription
        let subscription = Subscription {
            broadcaster: broadcaster.clone(),
            subscriber_count: 1,
            task_handle,
        };

        subscriptions.insert(aircraft_id.to_string(), subscription);

        Ok(receiver)
    }

    // Subscribe to a specific area - creates NATS subscription on-demand
    pub async fn subscribe_to_area(
        &self,
        latitude: i32,
        longitude: i32,
    ) -> Result<broadcast::Receiver<WebSocketMessage>> {
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

        let task_handle = tokio::spawn(
            async move {
                Self::handle_area_messages(subscriber, area_key_clone, broadcaster_clone).await;
            }
            .instrument(tracing::info_span!("live_fix_area_handler", latitude = %latitude, longitude = %longitude))
        );

        // Store the subscription
        let subscription = Subscription {
            broadcaster: broadcaster.clone(),
            subscriber_count: 1,
            task_handle,
        };

        subscriptions.insert(area_key, subscription);

        Ok(receiver)
    }

    // Handle messages for a specific aircraft
    async fn handle_aircraft_messages(
        mut subscriber: Subscriber,
        aircraft_id: String,
        broadcaster: broadcast::Sender<WebSocketMessage>,
    ) {
        info!("Started message handler for aircraft: {}", aircraft_id);
        let mut consecutive_no_receivers = 0;
        const MAX_NO_RECEIVER_ATTEMPTS: usize = 3;

        while let Some(msg) = subscriber.next().await {
            match Self::process_fix_message(msg, &aircraft_id).await {
                Ok(fix_with_flight) => {
                    let websocket_message = WebSocketMessage::Fix(Box::new(fix_with_flight));
                    match broadcaster.send(websocket_message) {
                        Ok(receiver_count) => {
                            consecutive_no_receivers = 0; // Reset counter on successful send
                            info!(
                                "Broadcasted live fix for aircraft {} to {} receivers",
                                aircraft_id, receiver_count
                            );
                        }
                        Err(broadcast::error::SendError(_)) => {
                            consecutive_no_receivers += 1;
                            info!(
                                "No active receivers for aircraft {}, fix dropped ({}/{} consecutive failures)",
                                aircraft_id, consecutive_no_receivers, MAX_NO_RECEIVER_ATTEMPTS
                            );

                            // Stop processing if consistently no receivers
                            if consecutive_no_receivers >= MAX_NO_RECEIVER_ATTEMPTS {
                                warn!(
                                    "Aircraft {} has no active receivers after {} consecutive messages, stopping message handler",
                                    aircraft_id, MAX_NO_RECEIVER_ATTEMPTS
                                );
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(
                        aircraft_id = %aircraft_id, error = %e,
                        "Failed to process fix message for aircraft"
                    );
                }
            }
        }

        warn!("Message handler for aircraft {} has stopped", aircraft_id);
    }

    // Handle messages for a specific area
    async fn handle_area_messages(
        mut subscriber: Subscriber,
        area_key: String,
        broadcaster: broadcast::Sender<WebSocketMessage>,
    ) {
        info!("Started message handler for area: {}", area_key);
        let mut consecutive_no_receivers = 0;
        const MAX_NO_RECEIVER_ATTEMPTS: usize = 3;

        while let Some(msg) = subscriber.next().await {
            match Self::process_area_fix_message(msg, &area_key).await {
                Ok(fix_with_flight) => {
                    let websocket_message = WebSocketMessage::Fix(Box::new(fix_with_flight));
                    match broadcaster.send(websocket_message) {
                        Ok(receiver_count) => {
                            consecutive_no_receivers = 0; // Reset counter on successful send
                            info!(
                                "Broadcasted live fix for area {} to {} receivers",
                                area_key, receiver_count
                            );
                        }
                        Err(broadcast::error::SendError(_)) => {
                            consecutive_no_receivers += 1;
                            info!(
                                "No active receivers for area {}, fix dropped ({}/{} consecutive failures)",
                                area_key, consecutive_no_receivers, MAX_NO_RECEIVER_ATTEMPTS
                            );

                            // Stop processing if consistently no receivers
                            if consecutive_no_receivers >= MAX_NO_RECEIVER_ATTEMPTS {
                                warn!(
                                    "Area {} has no active receivers after {} consecutive messages, stopping message handler",
                                    area_key, MAX_NO_RECEIVER_ATTEMPTS
                                );
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(area_key = %area_key, error = %e, "Failed to process fix message for area");
                }
            }
        }

        warn!("Message handler for area {} has stopped", area_key);
    }

    // Process a single fix message
    async fn process_fix_message(
        msg: async_nats::Message,
        expected_device_id: &str,
    ) -> Result<crate::fixes::Fix> {
        // Parse the Fix data from NATS
        let fix: crate::fixes::Fix = serde_json::from_slice(&msg.payload)?;

        info!(
            "Processing live fix for device {} at ({}, {}) alt={}ft",
            expected_device_id,
            fix.latitude,
            fix.longitude,
            fix.altitude_msl_feet.unwrap_or(0)
        );

        Ok(fix)
    }

    // Process a single fix message from area subscription
    async fn process_area_fix_message(
        msg: async_nats::Message,
        area_key: &str,
    ) -> Result<crate::fixes::Fix> {
        // Parse the Fix data from NATS
        let fix: crate::fixes::Fix = serde_json::from_slice(&msg.payload)?;

        info!(
            "Processing live fix from area {} for device {} at ({}, {}) alt={}ft",
            area_key,
            fix.aircraft_id,
            fix.latitude,
            fix.longitude,
            fix.altitude_msl_feet.unwrap_or(0)
        );

        Ok(fix)
    }

    // Unsubscribe from an aircraft - removes NATS subscription when no more clients
    pub async fn unsubscribe_from_aircraft(&self, aircraft_id: &str) -> Result<()> {
        let mut subscriptions = self.subscriptions.lock().await;

        if let Some(subscription) = subscriptions.get_mut(aircraft_id) {
            subscription.subscriber_count -= 1;
            info!(
                "Removed subscriber for aircraft {} (remaining: {})",
                aircraft_id, subscription.subscriber_count
            );

            // If no more subscribers, clean up the subscription
            if subscription.subscriber_count == 0 {
                info!(
                    "No more subscribers for aircraft {}, cleaning up NATS subscription",
                    aircraft_id
                );

                // Cancel the message handler task
                subscription.task_handle.abort();

                // Remove from our subscriptions map
                subscriptions.remove(aircraft_id);
            }
        } else {
            warn!(
                "Attempted to unsubscribe from device {} but no subscription found",
                aircraft_id
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
}
