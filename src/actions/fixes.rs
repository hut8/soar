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
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::fixes_repo::FixesRepository;
use crate::live_fixes::{LiveFix, WebSocketMessage};
use crate::web::AppState;

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

// Helper function to send recent fixes for a device immediately upon subscription
async fn send_recent_device_fix(
    device_id_str: &str,
    fix_tx: &mpsc::UnboundedSender<WebSocketMessage>,
    pool: &crate::web::PgPool,
) {
    // Parse device ID as UUID
    let device_uuid = match Uuid::parse_str(device_id_str) {
        Ok(uuid) => uuid,
        Err(e) => {
            error!("Invalid device ID format {}: {}", device_id_str, e);
            return;
        }
    };

    let fixes_repo = FixesRepository::new(pool.clone());

    // Get the most recent fix within last 24 hours
    match fixes_repo.get_fixes_for_device(device_uuid, Some(1)).await {
        Ok(fixes) => {
            if let Some(fix) = fixes.first() {
                // Check if fix is within last 24 hours
                let now = Utc::now();
                let twenty_four_hours_ago = now - Duration::hours(24);

                if fix.timestamp >= twenty_four_hours_ago {
                    // Convert Fix to LiveFix
                    let live_fix = LiveFix {
                        id: fix.id.to_string(),
                        device_id: device_id_str.to_string(),
                        timestamp: fix.timestamp.to_rfc3339(),
                        latitude: fix.latitude,
                        longitude: fix.longitude,
                        altitude: fix.altitude_feet.unwrap_or(0) as f64,
                        track: fix.track_degrees.unwrap_or(0.0) as f64,
                        ground_speed: fix.ground_speed_knots.unwrap_or(0.0) as f64,
                        climb_rate: fix.climb_fpm.unwrap_or(0) as f64,
                    };

                    let websocket_message = WebSocketMessage::Fix(live_fix);
                    if let Err(e) = fix_tx.send(websocket_message) {
                        error!(
                            "Failed to send recent fix for device {}: {}",
                            device_id_str, e
                        );
                    } else {
                        info!(
                            "Sent recent fix for device {} immediately upon subscription",
                            device_id_str
                        );
                    }
                } else {
                    info!(
                        "Most recent fix for device {} is older than 24 hours, not sending",
                        device_id_str
                    );
                }
            } else {
                info!("No recent fixes found for device {}", device_id_str);
            }
        }
        Err(e) => {
            error!(
                "Failed to fetch recent fixes for device {}: {}",
                device_id_str, e
            );
        }
    }
}

// Enhanced helper function to send device data and recent fixes for devices in an area
// This replaces the inefficient global fetch + filter approach with proper spatial queries
async fn send_recent_area_devices_and_fixes(
    latitude: i32,
    longitude: i32,
    fix_tx: &mpsc::UnboundedSender<WebSocketMessage>,
    pool: &crate::web::PgPool,
) {
    let fixes_repo = FixesRepository::new(pool.clone());
    let aircraft_registrations_repo =
        crate::aircraft_registrations_repo::AircraftRegistrationsRepository::new(pool.clone());
    let aircraft_model_repo =
        crate::faa::aircraft_model_repo::AircraftModelRepository::new(pool.clone());

    // Convert integer lat/lng to proper bounding box (±0.5 degrees for better coverage)
    let center_lat = latitude as f64;
    let center_lng = longitude as f64;
    let radius = 0.5; // 0.5 degree radius

    let nw_lat = center_lat + radius;
    let nw_lng = center_lng - radius;
    let se_lat = center_lat - radius;
    let se_lng = center_lng + radius;

    // Use the new efficient spatial query
    let cutoff_time = Utc::now() - Duration::hours(24);
    match fixes_repo
        .get_devices_with_fixes_in_bounding_box(
            nw_lat,
            nw_lng,
            se_lat,
            se_lng,
            cutoff_time,
            Some(5),
        )
        .await
    {
        Ok(devices_with_fixes) => {
            let mut sent_device_count = 0;

            // Send DeviceWithFixes messages for each device found in the area
            for (device_model, device_fixes) in devices_with_fixes {
                // Fetch aircraft registration data
                let aircraft_registration = aircraft_registrations_repo
                    .get_aircraft_registration_by_device_id(device_model.id)
                    .await
                    .ok()
                    .flatten();

                // Fetch aircraft model data if we have registration
                let aircraft_model = if let Some(ref reg) = aircraft_registration {
                    aircraft_model_repo
                        .get_aircraft_model_by_key(
                            &reg.manufacturer_code,
                            &reg.model_code,
                            &reg.series_code,
                        )
                        .await
                        .ok()
                        .flatten()
                } else {
                    None
                };

                // Create DeviceWithFixes message
                let device_with_fixes = crate::live_fixes::DeviceWithFixes {
                    device: device_model.clone(),
                    aircraft_registration,
                    aircraft_model,
                    recent_fixes: device_fixes,
                };

                let websocket_message =
                    crate::live_fixes::WebSocketMessage::Device(Box::new(device_with_fixes));
                if let Err(e) = fix_tx.send(websocket_message) {
                    error!(
                        "Failed to send device data for device {}: {}",
                        device_model.registration, e
                    );
                } else {
                    sent_device_count += 1;
                }
            }

            info!(
                "Sent complete data for {} devices in area {},{} immediately upon subscription",
                sent_device_count, latitude, longitude
            );
        }
        Err(e) => {
            error!(
                "Failed to fetch devices with fixes for area {},{}: {}",
                latitude, longitude, e
            );
        }
    }
}

/// Helper function to enrich devices with aircraft registration and model data
/// Optimized with batch queries to avoid N+1 query problem
async fn enrich_devices_with_aircraft_data(
    devices_with_fixes: Vec<(crate::devices::DeviceModel, Vec<crate::fixes::Fix>)>,
    pool: crate::web::PgPool,
) -> Vec<crate::live_fixes::DeviceWithFixes> {
    use std::collections::HashMap;

    if devices_with_fixes.is_empty() {
        return Vec::new();
    }

    let aircraft_registrations_repo =
        crate::aircraft_registrations_repo::AircraftRegistrationsRepository::new(pool.clone());
    let aircraft_model_repo = crate::faa::aircraft_model_repo::AircraftModelRepository::new(pool);

    // Step 1: Collect all device IDs
    let device_ids: Vec<uuid::Uuid> = devices_with_fixes
        .iter()
        .map(|(device, _)| device.id)
        .collect();

    // Step 2: Batch fetch all aircraft registrations
    let registrations = aircraft_registrations_repo
        .get_aircraft_registrations_by_device_ids(&device_ids)
        .await
        .unwrap_or_default();

    // Step 3: Build HashMap for O(1) lookup: device_id -> registration
    // Filter out registrations without device_id
    let registration_map: HashMap<
        uuid::Uuid,
        crate::aircraft_registrations::AircraftRegistrationModel,
    > = registrations
        .into_iter()
        .filter_map(|reg| reg.device_id.map(|id| (id, reg)))
        .collect();

    // Step 4: Collect all unique (manufacturer, model, series) keys
    let model_keys: Vec<(String, String, String)> = registration_map
        .values()
        .map(|reg| {
            (
                reg.manufacturer_code.clone(),
                reg.model_code.clone(),
                reg.series_code.clone(),
            )
        })
        .collect();

    // Step 5: Batch fetch all aircraft models
    let models = aircraft_model_repo
        .get_aircraft_models_by_keys(&model_keys)
        .await
        .unwrap_or_default();

    // Step 6: Build HashMap for O(1) lookup: (manufacturer, model, series) -> aircraft_model
    let model_map: HashMap<(String, String, String), crate::faa::aircraft_models::AircraftModel> =
        models
            .into_iter()
            .map(|model| {
                (
                    (
                        model.manufacturer_code.clone(),
                        model.model_code.clone(),
                        model.series_code.clone(),
                    ),
                    model,
                )
            })
            .collect();

    // Step 7: Build enriched results
    let mut enriched = Vec::new();
    for (device_model, device_fixes) in devices_with_fixes {
        let aircraft_registration = registration_map.get(&device_model.id).cloned();

        let aircraft_model = aircraft_registration.as_ref().and_then(|reg| {
            model_map
                .get(&(
                    reg.manufacturer_code.clone(),
                    reg.model_code.clone(),
                    reg.series_code.clone(),
                ))
                .cloned()
        });

        enriched.push(crate::live_fixes::DeviceWithFixes {
            device: device_model,
            aircraft_registration,
            aircraft_model,
            recent_fixes: device_fixes,
        });
    }

    enriched
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
    let (fix_tx, fix_rx) = mpsc::unbounded_channel::<WebSocketMessage>();

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
                error!("WebSocket error: {}", e);
                break;
            }
        }
    }
}

async fn handle_websocket_write(
    mut sender: futures_util::stream::SplitSink<WebSocket, Message>,
    mut fix_rx: mpsc::UnboundedReceiver<WebSocketMessage>,
) {
    while let Some(websocket_message) = fix_rx.recv().await {
        let fix_json = match serde_json::to_string(&websocket_message) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize WebSocket message: {}", e);
                continue;
            }
        };

        if let Err(e) = sender.send(Message::Text(fix_json.into())).await {
            error!("Failed to send WebSocket message to client: {}", e);
            break;
        }
    }
}

async fn handle_subscriptions(
    live_fix_service: crate::live_fixes::LiveFixService,
    mut subscription_rx: mpsc::UnboundedReceiver<SubscriptionMessage>,
    fix_tx: mpsc::UnboundedSender<WebSocketMessage>,
    pool: crate::web::PgPool,
) {
    let mut subscribed_aircraft: Vec<String> = Vec::new();
    let mut receivers: HashMap<String, broadcast::Receiver<WebSocketMessage>> = HashMap::new();

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

                                                    // Send recent fix immediately upon subscription
                                                    send_recent_device_fix(&id, &fix_tx, &pool).await;
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

                                                    // Send recent fixes for devices in this area immediately upon subscription
                                                    send_recent_area_devices_and_fixes(latitude, longitude, &fix_tx, &pool).await;
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

pub async fn search_fixes(
    State(state): State<AppState>,
    Query(params): Query<FixesQueryParams>,
) -> impl IntoResponse {
    let fixes_repo = FixesRepository::new(state.pool.clone());

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

    // If any bounding box parameter is provided, all four must be provided
    if has_bounding_box {
        match (
            params.latitude_max,
            params.latitude_min,
            params.longitude_max,
            params.longitude_min,
        ) {
            (Some(lat_max), Some(lat_min), Some(lon_max), Some(lon_min)) => {
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
                let cutoff_time = params.after.unwrap_or_else(|| Utc::now() - Duration::hours(24));

                // Perform bounding box search
                match fixes_repo
                    .get_devices_with_fixes_in_bounding_box(
                        lat_max, lon_min, lat_min, lon_max, cutoff_time, None,
                    )
                    .await
                {
                    Ok(devices_with_fixes) => {
                        // Enrich with aircraft registration and model data
                        let enriched = enrich_devices_with_aircraft_data(
                            devices_with_fixes,
                            state.pool.clone(),
                        )
                        .await;
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
            _ => json_error(
                StatusCode::BAD_REQUEST,
                "When using bounding box search, all four parameters must be provided: latitude_max, latitude_min, longitude_max, longitude_min",
            )
            .into_response(),
        }
    } else if let Some(device_id) = params.device_id {
        match fixes_repo
            .get_fixes_for_device(device_id, Some(params.limit.unwrap_or(1000)))
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
    } else if let Some(flight_id) = params.flight_id {
        // Get fixes for the specific flight
        match fixes_repo
            .get_fixes_for_flight(flight_id, params.limit)
            .await
        {
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
    } else {
        match fixes_repo
            .get_recent_fixes(params.limit.unwrap_or(100))
            .await
        {
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
}
