use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info, warn};

use crate::live_fixes::LiveFix;
use crate::web::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SubscriptionMessage {
    #[serde(rename = "device")]
    Device {
        action: String, // "subscribe" or "unsubscribe"
        id: String,     // Device UUID
    },
    #[serde(rename = "area")]
    Area {
        action: String, // "subscribe" or "unsubscribe"
        latitude: i32,  // Integer latitude
        longitude: i32, // Integer longitude
    },
}

pub async fn fixes_live_websocket(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

async fn handle_websocket(socket: WebSocket, state: AppState) {
    info!("New WebSocket connection established for live fixes");

    // Get live fix service from app state
    let live_fix_service = match &state.live_fix_service {
        Some(service) => service.clone(),
        None => {
            warn!("Live fix service not available");
            return;
        }
    };

    // Split the socket for concurrent read/write
    let (sender, receiver) = socket.split();

    // Create channels for communication between tasks
    let (subscription_tx, subscription_rx) = mpsc::unbounded_channel::<SubscriptionMessage>();
    let (fix_tx, fix_rx) = mpsc::unbounded_channel::<LiveFix>();

    // Spawn task to handle incoming WebSocket messages (subscriptions)
    let subscription_tx_clone = subscription_tx.clone();
    let read_task = tokio::spawn(async move {
        handle_websocket_read(receiver, subscription_tx_clone).await;
    });

    // Spawn task to handle outgoing WebSocket messages (live fixes)
    let write_task = tokio::spawn(async move {
        handle_websocket_write(sender, fix_rx).await;
    });

    // Main task to handle subscriptions and NATS messages
    let subscription_task = tokio::spawn(async move {
        handle_subscriptions(live_fix_service, subscription_rx, fix_tx).await;
    });

    // Wait for any task to complete (usually means connection closed)
    tokio::select! {
        _ = read_task => {
            info!("WebSocket read task completed");
        }
        _ = write_task => {
            info!("WebSocket write task completed");
        }
        _ = subscription_task => {
            info!("Subscription task completed");
        }
    }

    info!("WebSocket connection terminated");
}

async fn handle_websocket_read(
    mut receiver: futures_util::stream::SplitStream<WebSocket>,
    subscription_tx: mpsc::UnboundedSender<SubscriptionMessage>,
) {
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => match serde_json::from_str::<SubscriptionMessage>(&text) {
                Ok(sub_msg) => {
                    if subscription_tx.send(sub_msg).is_err() {
                        error!("Failed to send subscription message to handler");
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to parse subscription message: {}", e);
                }
            },
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed by client");
                break;
            }
            Ok(_) => {
                // Ignore other message types (binary, ping, pong)
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
        }
    }
}

async fn handle_websocket_write(
    mut sender: futures_util::stream::SplitSink<WebSocket, Message>,
    mut fix_rx: mpsc::UnboundedReceiver<LiveFix>,
) {
    while let Some(live_fix) = fix_rx.recv().await {
        let fix_json = match serde_json::to_string(&live_fix) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize live fix: {}", e);
                continue;
            }
        };

        if let Err(e) = sender.send(Message::Text(fix_json.into())).await {
            error!("Failed to send fix to WebSocket client: {}", e);
            break;
        }
    }
}

async fn handle_subscriptions(
    live_fix_service: crate::live_fixes::LiveFixService,
    mut subscription_rx: mpsc::UnboundedReceiver<SubscriptionMessage>,
    fix_tx: mpsc::UnboundedSender<LiveFix>,
) {
    let mut subscribed_aircraft: Vec<String> = Vec::new();
    let mut receivers: HashMap<String, broadcast::Receiver<LiveFix>> = HashMap::new();

    loop {
        tokio::select! {
            // Handle subscription messages from WebSocket
            sub_msg = subscription_rx.recv() => {
                match sub_msg {
                    Some(sub_msg) => {
                        match sub_msg {
                            SubscriptionMessage::Device { action, id } => {
                                match action.as_str() {
                                    "subscribe" => {
                                        info!("Client subscribing to device: {}", id);
                                        if !subscribed_aircraft.contains(&id) {
                                            match live_fix_service.subscribe_to_device(&id).await {
                                                Ok(receiver) => {
                                                    subscribed_aircraft.push(id.clone());
                                                    receivers.insert(id.clone(), receiver);
                                                    info!("Successfully subscribed to device: {}", id);
                                                }
                                                Err(e) => {
                                                    error!("Failed to subscribe to device {}: {}", id, e);
                                                }
                                            }
                                        }
                                    }
                                    "unsubscribe" => {
                                        info!("Client unsubscribing from device: {}", id);
                                        if subscribed_aircraft.contains(&id) {
                                            subscribed_aircraft.retain(|device_id| device_id != &id);
                                            receivers.remove(&id);
                                            if let Err(e) = live_fix_service.unsubscribe_from_device(&id).await {
                                                error!("Failed to unsubscribe from device {}: {}", id, e);
                                            } else {
                                                info!("Successfully unsubscribed from device: {}", id);
                                            }
                                        }
                                    }
                                    _ => {
                                        warn!("Unknown device subscription action: {}", action);
                                    }
                                }
                            }
                            SubscriptionMessage::Area { action, latitude, longitude } => {
                                match action.as_str() {
                                    "subscribe" => {
                                        info!("Client subscribing to area: lat={}, lon={}", latitude, longitude);
                                        let area_key = format!("area.{}.{}", latitude, longitude);
                                        if !subscribed_aircraft.contains(&area_key) {
                                            match live_fix_service.subscribe_to_area(latitude, longitude).await {
                                                Ok(receiver) => {
                                                    subscribed_aircraft.push(area_key.clone());
                                                    receivers.insert(area_key, receiver);
                                                    info!("Successfully subscribed to area: lat={}, lon={}", latitude, longitude);
                                                }
                                                Err(e) => {
                                                    error!("Failed to subscribe to area lat={}, lon={}: {}", latitude, longitude, e);
                                                }
                                            }
                                        }
                                    }
                                    "unsubscribe" => {
                                        info!("Client unsubscribing from area: lat={}, lon={}", latitude, longitude);
                                        let area_key = format!("area.{}.{}", latitude, longitude);
                                        if subscribed_aircraft.contains(&area_key) {
                                            subscribed_aircraft.retain(|key| key != &area_key);
                                            receivers.remove(&area_key);
                                            if let Err(e) = live_fix_service.unsubscribe_from_area(latitude, longitude).await {
                                                error!("Failed to unsubscribe from area lat={}, lon={}: {}", latitude, longitude, e);
                                            } else {
                                                info!("Successfully unsubscribed from area: lat={}, lon={}", latitude, longitude);
                                            }
                                        }
                                    }
                                    _ => {
                                        warn!("Unknown area subscription action: {}", action);
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        // Subscription channel closed, WebSocket disconnected
                        break;
                    }
                }
            }

            // Handle incoming fixes from NATS for any subscribed device
            fix_result = async {
                if receivers.is_empty() {
                    // No active subscriptions, wait indefinitely
                    std::future::pending::<Option<LiveFix>>().await
                } else {
                    // Use futures::select_all to wait for any receiver to have a message
                    use futures_util::future::select_all;

                    // Create a vec of futures for all receivers
                    let mut futures = Vec::new();
                    let mut device_ids = Vec::new();

                    for (device_id, receiver) in receivers.iter_mut() {
                        futures.push(Box::pin(receiver.recv()));
                        device_ids.push(device_id.clone());
                    }

                    if !futures.is_empty() {
                        let (result, index, _) = select_all(futures).await;
                        let device_id = &device_ids[index];

                        match result {
                            Ok(live_fix) => {
                                info!("Received live fix for device: {}", device_id);
                                Some(live_fix)
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                error!("Receiver closed for device: {}", device_id);
                                None
                            }
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                warn!("Receiver lagged {} messages for device: {}", n, device_id);
                                None
                            }
                        }
                    } else {
                        std::future::pending::<Option<LiveFix>>().await
                    }
                }
            } => {
                if let Some(live_fix) = fix_result
                    && fix_tx.send(live_fix).is_err() {
                        error!("Failed to send live fix to WebSocket writer");
                        return;
                    }
            }
        }
    }

    // Cleanup subscriptions when connection closes
    for subscription_key in &subscribed_aircraft {
        if subscription_key.starts_with("area.") {
            // Parse area subscription key: "area.lat.lon"
            let parts: Vec<&str> = subscription_key.split('.').collect();
            if parts.len() == 3 {
                if let (Ok(lat), Ok(lon)) = (parts[1].parse::<i32>(), parts[2].parse::<i32>()) {
                    if let Err(e) = live_fix_service.unsubscribe_from_area(lat, lon).await {
                        error!(
                            "Failed to cleanup area subscription {}: {}",
                            subscription_key, e
                        );
                    } else {
                        info!("Cleaned up area subscription: {}", subscription_key);
                    }
                } else {
                    error!("Invalid area subscription key format: {}", subscription_key);
                }
            } else {
                error!("Invalid area subscription key format: {}", subscription_key);
            }
        } else {
            // Device subscription
            if let Err(e) = live_fix_service
                .unsubscribe_from_device(subscription_key)
                .await
            {
                error!(
                    "Failed to cleanup device subscription {}: {}",
                    subscription_key, e
                );
            } else {
                info!("Cleaned up device subscription: {}", subscription_key);
            }
        }
    }
}
