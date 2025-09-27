use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

use crate::live_fixes::LiveFix;
use crate::web::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionMessage {
    pub action: String, // "subscribe" or "unsubscribe"
    pub device_id: String, // Single device ID to match frontend expectations
}

pub async fn fixes_live_websocket(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

async fn handle_websocket(mut socket: WebSocket, state: AppState) {
    info!("New WebSocket connection established for live fixes");

    // Get live fix service from app state
    let live_fix_service = match &state.live_fix_service {
        Some(service) => service.clone(),
        None => {
            warn!("Live fix service not available");
            let _ = socket
                .send(Message::Text(
                    serde_json::json!({"error": "Live fixes service not available"})
                        .to_string()
                        .into(),
                ))
                .await;
            return;
        }
    };

    let mut subscribed_aircraft: Vec<String> = Vec::new();
    let mut receivers: HashMap<String, broadcast::Receiver<LiveFix>> = HashMap::new();

    loop {
        tokio::select! {
            // Handle incoming WebSocket messages from client
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<SubscriptionMessage>(&text) {
                            Ok(sub_msg) => {
                                match sub_msg.action.as_str() {
                                    "subscribe" => {
                                        info!("Client subscribing to device: {}", sub_msg.device_id);
                                        let device_id = sub_msg.device_id;
                                        if !subscribed_aircraft.contains(&device_id) {
                                            subscribed_aircraft.push(device_id.clone());
                                            let receiver = live_fix_service.get_receiver(&device_id).await;
                                            receivers.insert(device_id.clone(), receiver);
                                        }
                                    }
                                    "unsubscribe" => {
                                        info!("Client unsubscribing from device: {}", sub_msg.device_id);
                                        let device_id = &sub_msg.device_id;
                                        subscribed_aircraft.retain(|id| id != device_id);
                                        receivers.remove(device_id);
                                        live_fix_service.cleanup_aircraft(device_id).await;
                                    }
                                    _ => {
                                        warn!("Unknown subscription action: {}", sub_msg.action);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse subscription message: {}", e);
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket connection closed by client");
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        info!("WebSocket connection closed");
                        break;
                    }
                    _ => {
                        // Ignore other message types (binary, ping, pong)
                    }
                }
            }

            // Handle incoming fixes from NATS for subscribed aircraft
            fix_result = async {
                if receivers.is_empty() {
                    // No active subscriptions, wait indefinitely
                    std::future::pending::<Result<(String, LiveFix), broadcast::error::RecvError>>().await
                } else {
                    // Check all receivers for new fixes
                    for (aircraft_id, receiver) in receivers.iter_mut() {
                        match receiver.try_recv() {
                            Ok(live_fix) => {
                                return Ok((aircraft_id.clone(), live_fix));
                            }
                            Err(broadcast::error::TryRecvError::Empty) => {
                                // No message available, continue to next receiver
                                continue;
                            }
                            Err(broadcast::error::TryRecvError::Lagged(_)) => {
                                warn!("Receiver lagged for aircraft: {}", aircraft_id);
                                continue;
                            }
                            Err(broadcast::error::TryRecvError::Closed) => {
                                error!("Receiver closed for aircraft: {}", aircraft_id);
                                return Err(broadcast::error::RecvError::Closed);
                            }
                        }
                    }
                    // No messages from any receiver, yield control
                    tokio::task::yield_now().await;
                    Err(broadcast::error::RecvError::Closed) // Temporary error to continue loop
                }
            } => {
                match fix_result {
                    Ok((_aircraft_id, live_fix)) => {
                        // Send the fix to the WebSocket client
                        let fix_json = match serde_json::to_string(&live_fix) {
                            Ok(json) => json,
                            Err(e) => {
                                error!("Failed to serialize live fix: {}", e);
                                continue;
                            }
                        };

                        if let Err(e) = socket.send(Message::Text(fix_json.into())).await {
                            error!("Failed to send fix to WebSocket client: {}", e);
                            break;
                        }
                    }
                    Err(_) => {
                        // Continue the loop for temporary errors
                        continue;
                    }
                }
            }
        }
    }

    // Cleanup subscriptions when connection closes
    for aircraft_id in &subscribed_aircraft {
        live_fix_service.cleanup_aircraft(aircraft_id).await;
    }

    info!("WebSocket connection terminated");
}
