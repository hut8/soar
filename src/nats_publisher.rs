use anyhow::Result;
use async_nats::{Client, Event};
use serde_json;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::fixes::FixWithFlightInfo;

// Queue size for NATS publish queue
const NATS_PUBLISH_QUEUE_SIZE: usize = 1000;

/// Get the topic prefix based on the environment
fn get_topic_prefix() -> &'static str {
    match std::env::var("SOAR_ENV") {
        Ok(env) if env == "production" => "aircraft",
        _ => "staging.aircraft",
    }
}

/// Publish FixWithFlightInfo to NATS (both device and area topics)
#[tracing::instrument(skip(nats_client, fix_with_flight), fields(aircraft_id = %aircraft_id))]
async fn publish_to_nats(
    nats_client: &Client,
    aircraft_id: &str,
    fix_with_flight: &FixWithFlightInfo,
) -> Result<()> {
    let topic_prefix = get_topic_prefix();

    // Serialize the FixWithFlightInfo to JSON once
    let payload = serde_json::to_vec(fix_with_flight)?;

    // TODO: Temporarily disabled to reduce NATS publish load
    // Publish by device
    // let device_subject = format!("{}.fix.{}", topic_prefix, aircraft_id);
    // nats_client
    //     .publish(device_subject.clone(), payload.clone().into())
    //     .await?;

    // Publish by area
    let area_subject = get_area_subject(
        topic_prefix,
        fix_with_flight.latitude,
        fix_with_flight.longitude,
    );

    let publish_start = std::time::Instant::now();
    nats_client.publish(area_subject, payload.into()).await?;
    let publish_duration = publish_start.elapsed();
    metrics::histogram!("nats.fix_publisher.publish_latency_us")
        .record(publish_duration.as_micros() as f64);

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
    fix_sender: flume::Sender<FixWithFlightInfo>,
}

impl NatsFixPublisher {
    /// Create a new NATS publisher for position fixes
    pub async fn new(nats_url: &str) -> Result<Self> {
        info!("Connecting to NATS server at {}", nats_url);
        let nats_client_name = crate::nats_client_name("nats-publisher");
        let nats_client = async_nats::ConnectOptions::new()
            .name(&nats_client_name)
            .event_callback(|event| async move {
                match event {
                    Event::Connected => {
                        info!("NATS publisher connected");
                        metrics::counter!("nats.fix_publisher.connected_total").increment(1);
                    }
                    Event::Disconnected => {
                        warn!("NATS publisher disconnected");
                        metrics::counter!("nats.fix_publisher.disconnected_total").increment(1);
                    }
                    Event::SlowConsumer(sid) => {
                        warn!("NATS slow consumer detected for subscription {}", sid);
                        metrics::counter!("nats.fix_publisher.slow_consumer_total").increment(1);
                    }
                    Event::ServerError(err) => {
                        error!("NATS server error: {}", err);
                        metrics::counter!("nats.fix_publisher.server_error_total").increment(1);
                    }
                    Event::ClientError(err) => {
                        error!("NATS client error: {}", err);
                        metrics::counter!("nats.fix_publisher.client_error_total").increment(1);
                    }
                    Event::LameDuckMode => {
                        warn!("NATS server entering lame duck mode (preparing to shutdown)");
                    }
                    _ => {}
                }
            })
            .connect(nats_url)
            .await?;

        let nats_client = Arc::new(nats_client);

        // Create bounded channel for fixes (~2MB buffer)
        let (fix_sender, fix_receiver) =
            flume::bounded::<FixWithFlightInfo>(NATS_PUBLISH_QUEUE_SIZE);

        info!(
            "NATS publisher initialized with bounded channel (capacity: {} fixes, ~2MB buffer)",
            NATS_PUBLISH_QUEUE_SIZE
        );

        // Spawn SINGLE background task to publish all fixes
        let client_clone = Arc::clone(&nats_client);
        tokio::spawn(async move {
            info!("NATS publisher background task started");
            let mut fixes_published = 0u64;
            let mut last_stats_log = std::time::Instant::now();

            while let Ok(fix_with_flight) = fix_receiver.recv_async().await {
                metrics::gauge!("worker.active", "type" => "nats_publisher").increment(1.0);
                let aircraft_id = fix_with_flight.aircraft_id.to_string();

                match publish_to_nats(&client_clone, &aircraft_id, &fix_with_flight).await {
                    Ok(()) => {
                        fixes_published += 1;
                        metrics::counter!("nats.fix_publisher.published_total").increment(1);
                    }
                    Err(e) => {
                        error!("Failed to publish fix for device {}: {}", aircraft_id, e);
                        metrics::counter!("nats.fix_publisher.errors_total").increment(1);
                    }
                }
                metrics::gauge!("worker.active", "type" => "nats_publisher").decrement(1.0);

                // Update queue depth metric
                metrics::gauge!("nats.fix_publisher.queue_depth").set(fix_receiver.len() as f64);

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
        });

        Ok(Self {
            _nats_client: nats_client,
            fix_sender,
        })
    }
}

impl NatsFixPublisher {
    /// Process a fix and publish it to NATS (blocking)
    /// This will block if the queue is full, applying backpressure to the caller
    pub async fn process_fix(&self, fix_with_flight: FixWithFlightInfo, _raw_message: &str) {
        // Track if send will block (queue is full)
        if self.fix_sender.is_full() {
            metrics::counter!("queue.send_blocked_total", "queue" => "nats_publisher").increment(1);
        }

        // Use send_async to block until space is available - never drop fixes
        match self.fix_sender.send_async(fix_with_flight).await {
            Ok(_) => {
                // Fix successfully queued for publishing
            }
            Err(flume::SendError(_)) => {
                // NATS publisher task has shut down
                error!("NATS publisher channel is closed - cannot publish fix");
                metrics::counter!("nats.fix_publisher.errors_total").increment(1);
            }
        }
    }
}
