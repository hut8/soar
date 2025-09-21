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

use crate::live_fixes::{LiveFix, LiveFixService};
use crate::web::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionMessage {
    pub action: String, // "subscribe" or "unsubscribe"
    pub aircraft_ids: Vec<String>,
}

pub async fn fixes_live_websocket(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

async fn handle_websocket(mut socket: WebSocket, _state: AppState) {
    info!("New WebSocket connection established for live fixes");

    // Get live fix service - for now we'll simulate it since we need to pass it through app state
    // In a real implementation, you would get this from app state
    let live_fix_service = match std::env::var("NATS_URL") {
        Ok(nats_url) => match LiveFixService::new(&nats_url).await {
            Ok(service) => Some(service),
            Err(e) => {
                error!("Failed to create live fix service: {}", e);
                let _ = socket
                    .send(Message::Text(
                        serde_json::json!({"error": "Live fixes service not available"}).to_string().into(),
                    ))
                    .await;
                return;
            }
        },
        Err(_) => {
            warn!("NATS_URL not configured, live fixes not available");
            let _ = socket
                .send(Message::Text(
                    serde_json::json!({"error": "Live fixes service not configured"}).to_string().into(),
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
                                        info!("Client subscribing to aircraft: {:?}", sub_msg.aircraft_ids);
                                        for aircraft_id in sub_msg.aircraft_ids {
                                            if !subscribed_aircraft.contains(&aircraft_id) {
                                                subscribed_aircraft.push(aircraft_id.clone());
                                                if let Some(ref service) = live_fix_service {
                                                    let receiver = service.get_receiver(&aircraft_id).await;
                                                    receivers.insert(aircraft_id.clone(), receiver);
                                                }
                                            }
                                        }
                                    }
                                    "unsubscribe" => {
                                        info!("Client unsubscribing from aircraft: {:?}", sub_msg.aircraft_ids);
                                        for aircraft_id in &sub_msg.aircraft_ids {
                                            subscribed_aircraft.retain(|id| id != aircraft_id);
                                            receivers.remove(aircraft_id);
                                            if let Some(ref service) = live_fix_service {
                                                service.cleanup_aircraft(aircraft_id).await;
                                            }
                                        }
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
    if let Some(ref service) = live_fix_service {
        for aircraft_id in &subscribed_aircraft {
            service.cleanup_aircraft(aircraft_id).await;
        }
    }

    info!("WebSocket connection terminated");
}
