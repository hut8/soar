use anyhow::Result;
use async_nats::{Client, Event};
use futures_util::FutureExt;
use serde_json;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

use crate::fixes::Fix;

/// Timeout for individual NATS publish operations.
/// If NATS is slow or disconnected, we drop the message rather than blocking forever.
const NATS_PUBLISH_TIMEOUT: Duration = Duration::from_secs(5);

// Queue size for NATS publish queue
const NATS_PUBLISH_QUEUE_SIZE: usize = 200;

/// Timeout for enqueuing a fix to the NATS publish queue.
/// If the queue is full for longer than this, we skip the NATS publish.
/// The fix is already saved to the database at this point, so this only
/// affects live streaming — not data persistence.
const NATS_QUEUE_SEND_TIMEOUT: Duration = Duration::from_millis(100);

/// Interval between heartbeat log messages from the publisher task.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);

/// Get the topic prefix based on the environment
fn get_topic_prefix() -> &'static str {
    match std::env::var("SOAR_ENV") {
        Ok(env) if env == "production" => "aircraft",
        _ => "staging.aircraft",
    }
}

/// Publish Fix to NATS (both device and area topics)
#[tracing::instrument(skip(nats_client, fix), fields(aircraft_id = %aircraft_id))]
async fn publish_to_nats(nats_client: &Client, aircraft_id: &str, fix: &Fix) -> Result<()> {
    let topic_prefix = get_topic_prefix();

    // Serialize the Fix to JSON once
    let payload = serde_json::to_vec(fix)?;

    // TODO: Temporarily disabled to reduce NATS publish load
    // Publish by device
    // let device_subject = format!("{}.fix.{}", topic_prefix, aircraft_id);
    // nats_client
    //     .publish(device_subject.clone(), payload.clone().into())
    //     .await?;

    // Publish by area
    let area_subject = get_area_subject(topic_prefix, fix.latitude, fix.longitude);

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

/// Run the NATS publisher loop. Returns Ok(()) when the channel is closed (clean shutdown).
/// Panics are caught by the supervisor that calls this function.
async fn run_publisher_loop(client: &Client, rx: &flume::Receiver<Fix>) {
    info!("NATS publisher loop started");
    metrics::gauge!("nats.fix_publisher.alive").set(1.0);

    let mut fixes_published = 0u64;
    let mut last_heartbeat = std::time::Instant::now();

    while let Ok(fix) = rx.recv_async().await {
        metrics::gauge!("worker.active", "type" => "nats_publisher").increment(1.0);
        let aircraft_id = fix.aircraft_id.to_string();

        match tokio::time::timeout(
            NATS_PUBLISH_TIMEOUT,
            publish_to_nats(client, &aircraft_id, &fix),
        )
        .await
        {
            Ok(Ok(())) => {
                fixes_published += 1;
                metrics::counter!("nats.fix_publisher.published_total").increment(1);
            }
            Ok(Err(e)) => {
                error!("Failed to publish fix for device {}: {}", aircraft_id, e);
                metrics::counter!("nats.fix_publisher.errors_total").increment(1);
            }
            Err(_) => {
                warn!(
                    "NATS publish timed out after {}s for device {}",
                    NATS_PUBLISH_TIMEOUT.as_secs(),
                    aircraft_id
                );
                metrics::counter!("nats.fix_publisher.timeout_total").increment(1);
            }
        }
        metrics::gauge!("worker.active", "type" => "nats_publisher").decrement(1.0);

        // Update queue depth metric
        metrics::gauge!("nats.fix_publisher.queue_depth").set(rx.len() as f64);

        // Log heartbeat every 60 seconds
        if last_heartbeat.elapsed() >= HEARTBEAT_INTERVAL {
            let queue_len = rx.len();
            info!(
                "NATS publisher alive: {} published since last heartbeat, {} in queue",
                fixes_published, queue_len
            );
            if queue_len > 100 {
                warn!(
                    "NATS publisher queue is building up ({} fixes) - NATS publishing may be slow",
                    queue_len
                );
            }
            fixes_published = 0;
            last_heartbeat = std::time::Instant::now();
        }
    }

    warn!("NATS publisher loop exited (channel closed)");
    metrics::gauge!("nats.fix_publisher.alive").set(0.0);
}

/// NATS publisher for aircraft position fixes
#[derive(Clone)]
pub struct NatsFixPublisher {
    // Keep the Arc<Client> alive to maintain the NATS connection
    // Even though we don't use it directly after initialization,
    // dropping it would close the connection
    _nats_client: Arc<Client>,
    fix_sender: flume::Sender<Fix>,
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

        // Create bounded channel for fixes
        let (fix_sender, fix_receiver) = flume::bounded::<Fix>(NATS_PUBLISH_QUEUE_SIZE);

        info!(
            "NATS publisher initialized with bounded channel (capacity: {} fixes)",
            NATS_PUBLISH_QUEUE_SIZE
        );

        // Spawn supervisor task that restarts the publisher on panic
        let client_clone = Arc::clone(&nats_client);
        tokio::spawn(async move {
            info!("NATS publisher supervisor started");
            loop {
                let result =
                    std::panic::AssertUnwindSafe(run_publisher_loop(&client_clone, &fix_receiver))
                        .catch_unwind()
                        .await;

                match result {
                    Ok(()) => {
                        // Channel closed — clean shutdown
                        warn!("NATS publisher supervisor: loop exited cleanly (channel closed)");
                        metrics::gauge!("nats.fix_publisher.alive").set(0.0);
                        break;
                    }
                    Err(panic_info) => {
                        error!(
                            "NATS publisher task panicked: {:?}. Restarting in 1s...",
                            panic_info
                        );
                        metrics::gauge!("nats.fix_publisher.alive").set(0.0);
                        metrics::counter!("nats.fix_publisher.panic_total").increment(1);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
            warn!("NATS publisher supervisor exiting");
        });

        Ok(Self {
            _nats_client: nats_client,
            fix_sender,
        })
    }
}

impl NatsFixPublisher {
    /// Process a fix and publish it to NATS (non-blocking with timeout)
    ///
    /// The fix has already been saved to the database before this is called.
    /// If the NATS queue is full (e.g., NATS is slow/disconnected), we skip
    /// the publish after a short timeout rather than blocking the processing
    /// pipeline. This prevents a slow NATS connection from stalling all workers.
    pub async fn process_fix(&self, fix: Fix, _raw_message: &str) {
        // Track if send will block (queue is full)
        if self.fix_sender.is_full() {
            metrics::counter!("queue.send_blocked_total", "queue" => "nats_publisher").increment(1);
        }

        // Use send_async with a timeout to prevent blocking the processing pipeline.
        // If the NATS publisher can't keep up (slow connection, disconnected), the queue
        // fills up and workers would block indefinitely without this timeout.
        match tokio::time::timeout(NATS_QUEUE_SEND_TIMEOUT, self.fix_sender.send_async(fix)).await {
            Ok(Ok(_)) => {
                // Fix successfully queued for publishing
            }
            Ok(Err(flume::SendError(_))) => {
                // NATS publisher task has shut down
                error!("NATS publisher channel is closed - cannot publish fix");
                metrics::counter!("nats.fix_publisher.errors_total").increment(1);
            }
            Err(_) => {
                // Queue full for too long — skip NATS publish to keep pipeline flowing.
                // The fix is already in the database; only live streaming is affected.
                warn!(
                    "NATS publish queue full for >{}ms - dropping fix (live streaming affected, data saved to DB)",
                    NATS_QUEUE_SEND_TIMEOUT.as_millis()
                );
                metrics::counter!("nats.fix_publisher.queue_full_drops_total").increment(1);
            }
        }
    }
}
