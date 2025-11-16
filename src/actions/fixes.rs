use axum::{
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use chrono::{Duration, Utc};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::broadcast;
use tracing::{error, info, instrument, warn};

use crate::fixes_repo::FixesRepository;
use crate::live_fixes::WebSocketMessage;
use crate::web::AppState;

use super::devices::enrich_devices_with_aircraft_data;
use super::json_error;

#[derive(Debug, Deserialize)]
pub struct FixesQueryParams {
    pub device_id: Option<uuid::Uuid>,
    pub flight_id: Option<uuid::Uuid>,
    pub limit: Option<i64>,
    pub latitude_max: Option<f64>,
    pub latitude_min: Option<f64>,
    pub longitude_max: Option<f64>,
    pub longitude_min: Option<f64>,
    pub after: Option<chrono::DateTime<Utc>>,
}

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

    // Track active WebSocket connections
    metrics::gauge!("websocket_connections").increment(1.0);

    // Get live fix service from app state
    let live_fix_service = match &state.live_fix_service {
        Some(service) => service.clone(),
        None => {
            warn!("Live fix service not available");
            metrics::gauge!("websocket_connections").decrement(1.0);
            return;
        }
    };

    // Split the socket for concurrent read/write
    let (sender, receiver) = socket.split();

    // Create channels for communication between tasks
    let (subscription_tx, subscription_rx) = flume::unbounded::<SubscriptionMessage>();
    let (fix_tx, fix_rx) = flume::unbounded::<WebSocketMessage>();

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
        handle_subscriptions(
            live_fix_service,
            subscription_rx,
            fix_tx,
            state.pool.clone(),
        )
        .await;
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

    // Decrement active connection count
    metrics::gauge!("websocket_connections").decrement(1.0);
    info!("WebSocket connection terminated");
}

async fn handle_websocket_read(
    mut receiver: futures_util::stream::SplitStream<WebSocket>,
    subscription_tx: flume::Sender<SubscriptionMessage>,
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
                    error!("Failed to parse subscription message [{text}]: {e}");
                }
            },
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed by client");
                break;
            }
            Ok(msg) => {
                warn!(
                    "Received unhandled WebSocket message: {}",
                    msg.to_text().unwrap_or("<binary>")
                );
                // Ignore other message types (binary, ping, pong)
            }
            Err(e) => {
                // Connection reset errors are normal when browsers close abruptly
                // Don't report these to Sentry as errors
                let error_msg = e.to_string();
                if error_msg.contains("Connection reset without closing handshake")
                    || error_msg.contains("Connection reset")
                {
                    info!(
                        "WebSocket connection reset by client (browser closed): {}",
                        e
                    );
                } else {
                    error!("WebSocket error: {}", e);
                }
                break;
            }
        }
    }
}

async fn handle_websocket_write(
    mut sender: futures_util::stream::SplitSink<WebSocket, Message>,
    fix_rx: flume::Receiver<WebSocketMessage>,
) {
    while let Ok(websocket_message) = fix_rx.recv_async().await {
        // Track WebSocket queue depth (messages buffered for sending to client)
        // Note: This is an approximation since we can't directly measure unbounded channel depth
        // We track this after recv to see how many are still waiting
        let queue_depth = fix_rx.len();
        metrics::gauge!("websocket_queue_depth").set(queue_depth as f64);

        if queue_depth > 100 {
            warn!(
                "WebSocket queue depth is high ({} messages) - client may be slow to consume messages",
                queue_depth
            );
        }

        let fix_json = match serde_json::to_string(&websocket_message) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize WebSocket message: {}", e);
                metrics::counter!("websocket_serialization_errors").increment(1);
                continue;
            }
        };

        if let Err(e) = sender.send(Message::Text(fix_json.into())).await {
            error!("Failed to send WebSocket message to client: {}", e);
            metrics::counter!("websocket_send_errors").increment(1);
            break;
        }

        metrics::counter!("websocket_messages_sent").increment(1);
    }
}

async fn handle_subscriptions(
    live_fix_service: crate::live_fixes::LiveFixService,
    subscription_rx: flume::Receiver<SubscriptionMessage>,
    fix_tx: flume::Sender<WebSocketMessage>,
    _pool: crate::web::PgPool,
) {
    let mut subscribed_aircraft: Vec<String> = Vec::new();
    let mut receivers: HashMap<String, broadcast::Receiver<WebSocketMessage>> = HashMap::new();

    loop {
        tokio::select! {
            // Handle subscription messages from WebSocket
            sub_msg = subscription_rx.recv_async() => {
                match sub_msg {
                    Ok(sub_msg) => {
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
                                                    metrics::gauge!("websocket_active_subscriptions").increment(1.0);
                                                    metrics::counter!("websocket_device_subscribes").increment(1);
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
                                            metrics::gauge!("websocket_active_subscriptions").decrement(1.0);
                                            metrics::counter!("websocket_device_unsubscribes").increment(1);
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
                                                    metrics::gauge!("websocket_active_subscriptions").increment(1.0);
                                                    metrics::counter!("websocket_area_subscribes").increment(1);
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
                                            metrics::gauge!("websocket_active_subscriptions").decrement(1.0);
                                            metrics::counter!("websocket_area_unsubscribes").increment(1);
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
                    Err(_) => {
                        // Subscription channel closed, WebSocket disconnected
                        break;
                    }
                }
            }

            // Handle incoming fixes from NATS for any subscribed device
            fix_result = async {
                if receivers.is_empty() {
                    // No active subscriptions, wait indefinitely
                    std::future::pending::<Option<WebSocketMessage>>().await
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
                            Ok(websocket_message) => {
                                info!("Received WebSocket message for device: {}", device_id);
                                Some(websocket_message)
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
                        std::future::pending::<Option<WebSocketMessage>>().await
                    }
                }
            } => {
                if let Some(websocket_message) = fix_result
                    && fix_tx.send(websocket_message).is_err() {
                        error!("Failed to send WebSocket message to writer");
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
                        metrics::gauge!("websocket_active_subscriptions").decrement(1.0);
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
                metrics::gauge!("websocket_active_subscriptions").decrement(1.0);
                info!("Cleaned up device subscription: {}", subscription_key);
            }
        }
    }
}

/// Search for devices with fixes in a bounding box (returns enriched devices, not fixes)
async fn search_fixes_by_bbox(
    lat_max: f64,
    lat_min: f64,
    lon_max: f64,
    lon_min: f64,
    after: Option<chrono::DateTime<Utc>>,
    pool: crate::web::PgPool,
) -> impl IntoResponse {
    // Validate coordinates
    if !(-90.0..=90.0).contains(&lat_max) || !(-90.0..=90.0).contains(&lat_min) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Latitude must be between -90 and 90 degrees",
        )
        .into_response();
    }

    if !(-180.0..=180.0).contains(&lon_max) || !(-180.0..=180.0).contains(&lon_min) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Longitude must be between -180 and 180 degrees",
        )
        .into_response();
    }

    if lat_min >= lat_max {
        return json_error(
            StatusCode::BAD_REQUEST,
            "latitude_min must be less than latitude_max",
        )
        .into_response();
    }

    if lon_min >= lon_max {
        return json_error(
            StatusCode::BAD_REQUEST,
            "longitude_min must be less than longitude_max",
        )
        .into_response();
    }

    // Set default cutoff time to 24 hours ago if not provided
    let cutoff_time = after.unwrap_or_else(|| Utc::now() - Duration::hours(24));

    info!(
        "Performing bounding box search with cutoff_time: {}",
        cutoff_time
    );

    let fixes_repo = FixesRepository::new(pool.clone());

    // Perform bounding box search
    match fixes_repo
        .get_devices_with_fixes_in_bounding_box(
            lat_max,
            lon_min,
            lat_min,
            lon_max,
            cutoff_time,
            None,
        )
        .await
    {
        Ok(devices_with_fixes) => {
            info!("Found {} devices in bounding box", devices_with_fixes.len());

            // Enrich with aircraft registration and model data
            let enriched =
                enrich_devices_with_aircraft_data(devices_with_fixes, pool.clone()).await;

            info!("Enriched {} devices, returning response", enriched.len());
            Json(enriched).into_response()
        }
        Err(e) => {
            error!("Failed to get devices with fixes in bounding box: {}", e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get devices with fixes in bounding box",
            )
            .into_response()
        }
    }
}

/// Get fixes for a specific device
async fn get_fixes_by_device_id(
    device_id: uuid::Uuid,
    limit: Option<i64>,
    pool: crate::web::PgPool,
) -> impl IntoResponse {
    let fixes_repo = FixesRepository::new(pool);

    match fixes_repo
        .get_fixes_for_device(device_id, Some(limit.unwrap_or(1000)), None)
        .await
    {
        Ok(fixes) => Json(fixes).into_response(),
        Err(e) => {
            error!("Failed to get fixes by device ID: {}", e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get fixes by device ID",
            )
            .into_response()
        }
    }
}

/// Get fixes for a specific flight
async fn get_fixes_by_flight_id(
    flight_id: uuid::Uuid,
    limit: Option<i64>,
    pool: crate::web::PgPool,
) -> impl IntoResponse {
    let fixes_repo = FixesRepository::new(pool);

    match fixes_repo.get_fixes_for_flight(flight_id, limit).await {
        Ok(fixes) => Json(fixes).into_response(),
        Err(e) => {
            error!("Failed to get fixes by flight ID: {}", e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get fixes by flight ID",
            )
            .into_response()
        }
    }
}

/// Get recent fixes (default search with no specific filters)
async fn get_recent_fixes_default(
    limit: Option<i64>,
    pool: crate::web::PgPool,
) -> impl IntoResponse {
    let fixes_repo = FixesRepository::new(pool);

    match fixes_repo.get_recent_fixes(limit.unwrap_or(100)).await {
        Ok(fixes) => Json(fixes).into_response(),
        Err(e) => {
            error!("Failed to get recent fixes: {}", e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get recent fixes",
            )
            .into_response()
        }
    }
}

#[instrument(skip(state), fields(
    has_bbox = params.latitude_max.is_some(),
    has_device = params.device_id.is_some(),
    has_flight = params.flight_id.is_some()
))]
pub async fn search_fixes(
    State(state): State<AppState>,
    Query(params): Query<FixesQueryParams>,
) -> impl IntoResponse {
    info!("Starting search_fixes request");

    // Check if bounding box parameters are provided
    let has_bounding_box = params.latitude_max.is_some()
        || params.latitude_min.is_some()
        || params.longitude_max.is_some()
        || params.longitude_min.is_some();

    // Check if device or flight parameters are provided
    let has_device_or_flight = params.device_id.is_some() || params.flight_id.is_some();

    // Validate mutual exclusivity
    if has_bounding_box && has_device_or_flight {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Bounding box search is mutually exclusive with device_id and flight_id parameters",
        )
        .into_response();
    }

    if has_bounding_box && params.limit.is_some() {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Bounding box search is mutually exclusive with limit parameter",
        )
        .into_response();
    }

    // Route to appropriate handler
    if has_bounding_box {
        // Validate all four bounding box parameters are provided
        match (
            params.latitude_max,
            params.latitude_min,
            params.longitude_max,
            params.longitude_min,
        ) {
            (Some(lat_max), Some(lat_min), Some(lon_max), Some(lon_min)) => {
                search_fixes_by_bbox(lat_max, lat_min, lon_max, lon_min, params.after, state.pool).await.into_response()
            }
            _ => json_error(
                StatusCode::BAD_REQUEST,
                "When using bounding box search, all four parameters must be provided: latitude_max, latitude_min, longitude_max, longitude_min",
            )
            .into_response(),
        }
    } else if let Some(device_id) = params.device_id {
        get_fixes_by_device_id(device_id, params.limit, state.pool)
            .await
            .into_response()
    } else if let Some(flight_id) = params.flight_id {
        get_fixes_by_flight_id(flight_id, params.limit, state.pool)
            .await
            .into_response()
    } else {
        get_recent_fixes_default(params.limit, state.pool)
            .await
            .into_response()
    }
}
