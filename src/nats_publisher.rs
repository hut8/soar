use crate::queue_config::NATS_PUBLISH_QUEUE_SIZE;
use anyhow::Result;
use async_nats::Client;
use serde_json;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::Instrument;
use tracing::{error, info, warn};

use crate::fixes::FixWithFlightInfo;

/// Get the topic prefix based on the environment
fn get_topic_prefix() -> &'static str {
    match std::env::var("SOAR_ENV") {
        Ok(env) if env == "production" => "aircraft",
        _ => "staging.aircraft",
    }
}

/// Publish FixWithFlightInfo to NATS (both device and area topics)
#[tracing::instrument(skip(nats_client, fix_with_flight), fields(device_id = %device_id))]
async fn publish_to_nats(
    nats_client: &Client,
    device_id: &str,
    fix_with_flight: &FixWithFlightInfo,
) -> Result<()> {
    let topic_prefix = get_topic_prefix();

    // Serialize the FixWithFlightInfo to JSON once
    let payload = serde_json::to_vec(fix_with_flight)?;

    // Publish by device
    let device_subject = format!("{}.fix.{}", topic_prefix, device_id);
    nats_client
        .publish(device_subject.clone(), payload.clone().into())
        .await?;

    // Publish by area
    let area_subject = get_area_subject(
        topic_prefix,
        fix_with_flight.latitude,
        fix_with_flight.longitude,
    );
    nats_client
        .publish(area_subject.clone(), payload.into())
        .await?;

    Ok(())
}

/// Get the area subject for a given latitude and longitude
fn get_area_subject(topic_prefix: &str, latitude: f64, longitude: f64) -> String {
    let lat_floor = latitude.floor() as i32;
    let lon_floor = longitude.floor() as i32;
    format!("{}.area.{}.{}", topic_prefix, lat_floor, lon_floor)
}

/// NATS publisher for aircraft position fixes
#[derive(Clone)]
pub struct NatsFixPublisher {
    // Keep the Arc<Client> alive to maintain the NATS connection
    // Even though we don't use it directly after initialization,
    // dropping it would close the connection
    _nats_client: Arc<Client>,
    fix_sender: mpsc::Sender<FixWithFlightInfo>,
}

impl NatsFixPublisher {
    /// Create a new NATS publisher for position fixes
    pub async fn new(nats_url: &str) -> Result<Self> {
        info!("Connecting to NATS server at {}", nats_url);
        let nats_client_name = if std::env::var("SOAR_ENV") == Ok("production".into()) {
            "soar-aprs-ingester"
        } else {
            "soar-aprs-ingester-staging"
        };
        let nats_client = async_nats::ConnectOptions::new()
            .name(nats_client_name)
            .connect(nats_url)
            .await?;

        let nats_client = Arc::new(nats_client);

        // Create bounded channel for fixes (~2MB buffer)
        let (fix_sender, mut fix_receiver) =
            mpsc::channel::<FixWithFlightInfo>(NATS_PUBLISH_QUEUE_SIZE);

        info!(
            "NATS publisher initialized with bounded channel (capacity: {} fixes, ~2MB buffer)",
            NATS_PUBLISH_QUEUE_SIZE
        );

        // Spawn SINGLE background task to publish all fixes
        let client_clone = Arc::clone(&nats_client);
        tokio::spawn(
            async move {
                info!("NATS publisher background task started");
                let mut fixes_published = 0u64;
                let mut last_stats_log = std::time::Instant::now();

            while let Some(fix_with_flight) = fix_receiver.recv().await {
                let device_id = fix_with_flight.device_id.to_string();

                match publish_to_nats(&client_clone, &device_id, &fix_with_flight).await {
                    Ok(()) => {
                        fixes_published += 1;
                        metrics::counter!("nats_publisher_fixes_published").increment(1);
                    }
                    Err(e) => {
                        error!("Failed to publish fix for device {}: {}", device_id, e);
                        metrics::counter!("nats_publisher_errors").increment(1);
                    }
                }

                // Update queue depth metric
                metrics::gauge!("nats_publisher_queue_depth").set(fix_receiver.len() as f64);

                // Log statistics every 5 minutes
                if last_stats_log.elapsed().as_secs() >= 300 {
                    let queue_len = fix_receiver.len();
                    info!(
                        "NATS publisher stats: {} fixes published in last 5min, {} fixes queued",
                        fixes_published, queue_len
                    );
                    if queue_len > 500 {
                        warn!(
                            "NATS publisher queue is building up ({} fixes) - NATS publishing may be slow",
                            queue_len
                        );
                    }
                    fixes_published = 0;
                    last_stats_log = std::time::Instant::now();
                }
            }

            warn!("NATS publisher background task stopped");
        }
        .instrument(tracing::info_span!("nats_publisher_background_task"))
        );

        Ok(Self {
            _nats_client: nats_client,
            fix_sender,
        })
    }
}

impl NatsFixPublisher {
    /// Process a fix and publish it to NATS
    /// Uses try_send to avoid blocking - provides backpressure if NATS publishing is slow
    pub fn process_fix(&self, fix_with_flight: FixWithFlightInfo, _raw_message: &str) {
        // Use try_send to avoid blocking the caller
        // This provides backpressure if the NATS publisher can't keep up
        match self.fix_sender.try_send(fix_with_flight) {
            Ok(_) => {
                // Fix successfully queued for publishing
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                // Channel is full - indicates NATS publisher is falling behind
                warn!(
                    "NATS publisher channel is FULL (1000 fixes buffered) - dropping fix. \
                     This indicates NATS publishing is slower than incoming fix rate. \
                     Check NATS server performance or increase channel capacity."
                );
                metrics::counter!("nats_publisher_dropped_fixes").increment(1);
                // Fix is dropped to prevent unbounded memory growth
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                // NATS publisher task has shut down
                error!("NATS publisher channel is closed - cannot publish fix");
                metrics::counter!("nats_publisher_errors").increment(1);
            }
        }
    }
}
