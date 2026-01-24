//! Connection status publisher for broadcasting ingest connection state to NATS.
//!
//! This module provides a publisher that sends connection status updates for OGN (APRS)
//! and ADS-B (Beast/SBS) data sources. The web process subscribes to these updates
//! and forwards simplified status to browser clients.

use anyhow::Result;
use async_nats::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Get the status topic based on environment
fn get_status_topic() -> &'static str {
    match std::env::var("SOAR_ENV") {
        Ok(env) if env == "production" => "status.connections",
        _ => "staging.status.connections",
    }
}

/// Full connection status published to NATS (includes endpoint details for internal use)
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ConnectionStatus {
    pub ogn_connected: bool,
    pub ogn_endpoint: Option<String>,
    pub adsb_connected: bool,
    pub adsb_endpoints: Vec<String>,
    pub timestamp: String,
}

/// Simplified connection status sent to browser clients (no endpoint details)
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct BrowserConnectionStatus {
    pub ogn: bool,
    pub adsb: bool,
}

impl From<&ConnectionStatus> for BrowserConnectionStatus {
    fn from(status: &ConnectionStatus) -> Self {
        Self {
            ogn: status.ogn_connected,
            adsb: status.adsb_connected,
        }
    }
}

/// Publisher for connection status updates to NATS
pub struct ConnectionStatusPublisher {
    nats_client: Arc<Client>,
    current_status: Arc<RwLock<ConnectionStatus>>,
}

impl ConnectionStatusPublisher {
    /// Create a new connection status publisher
    pub async fn new(nats_url: &str) -> Result<Self> {
        let client_name = crate::nats_client_name("ingest-status");
        let nats_client = async_nats::ConnectOptions::new()
            .name(&client_name)
            .connect(nats_url)
            .await?;

        Ok(Self {
            nats_client: Arc::new(nats_client),
            current_status: Arc::new(RwLock::new(ConnectionStatus::default())),
        })
    }

    /// Update OGN connection status
    pub async fn set_ogn_status(&self, connected: bool, endpoint: Option<String>) {
        let mut status = self.current_status.write().await;
        let changed = status.ogn_connected != connected;
        status.ogn_connected = connected;
        status.ogn_endpoint = endpoint;
        status.timestamp = chrono::Utc::now().to_rfc3339();

        if changed {
            drop(status);
            self.publish_now().await;
        }
    }

    /// Update ADS-B connection status (Beast or SBS)
    pub async fn set_adsb_status(&self, connected: bool, endpoints: Vec<String>) {
        let mut status = self.current_status.write().await;
        let changed = status.adsb_connected != connected;
        status.adsb_connected = connected;
        status.adsb_endpoints = endpoints;
        status.timestamp = chrono::Utc::now().to_rfc3339();

        if changed {
            drop(status);
            self.publish_now().await;
        }
    }

    /// Publish current status immediately
    pub async fn publish_now(&self) {
        let status = self.current_status.read().await.clone();
        let topic = get_status_topic();

        match serde_json::to_vec(&status) {
            Ok(payload) => {
                if let Err(e) = self.nats_client.publish(topic, payload.into()).await {
                    warn!("Failed to publish connection status: {}", e);
                } else {
                    info!(
                        "Published connection status: ogn={}, adsb={}",
                        status.ogn_connected, status.adsb_connected
                    );
                }
            }
            Err(e) => warn!("Failed to serialize connection status: {}", e),
        }
    }

    /// Start periodic publishing (every 60 seconds)
    pub fn start_periodic_publish(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                self.publish_now().await;
            }
        });
    }

    /// Get current status
    pub async fn current_status(&self) -> ConnectionStatus {
        self.current_status.read().await.clone()
    }
}
