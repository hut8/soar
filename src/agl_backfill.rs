use chrono::{Duration, Utc};
use metrics::{counter, gauge};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::elevation::ElevationDB;
use crate::fixes::Fix;
use crate::fixes_repo::FixesRepository;
use crate::flight_tracker::altitude::calculate_altitude_agl;

/// Lightweight struct containing only the data needed for AGL backfill processing
#[derive(Debug, Clone)]
struct BackfillTask {
    id: Uuid,
    latitude: f64,
    longitude: f64,
    altitude_msl_feet: Option<i32>,
}

impl From<Fix> for BackfillTask {
    fn from(fix: Fix) -> Self {
        Self {
            id: fix.id,
            latitude: fix.latitude,
            longitude: fix.longitude,
            altitude_msl_feet: fix.altitude_msl_feet,
        }
    }
}

/// Producer task that loads batches of fixes needing backfill and sends them to workers
async fn producer_task(
    fixes_repo: FixesRepository,
    tx: mpsc::Sender<BackfillTask>,
    num_workers: usize,
) {
    info!("Starting AGL backfill producer task");

    loop {
        // Find fixes that are:
        // 1. At least one hour old
        // 2. Have altitude_msl_feet (can calculate AGL)
        // 3. Don't have altitude_agl_valid = true (haven't been looked up yet)
        // 4. is_active = true (only backfill active aircraft)
        let one_hour_ago = Utc::now() - Duration::hours(1);

        // First, get the actual count of all pending fixes (not just the batch)
        let total_pending = match fixes_repo.count_fixes_needing_backfill(one_hour_ago).await {
            Ok(count) => count,
            Err(e) => {
                warn!("Failed to count pending fixes: {}", e);
                counter!("agl_backfill_fetch_errors_total").increment(1);
                sleep(tokio::time::Duration::from_secs(60)).await;
                continue;
            }
        };

        // Update metric with actual count
        gauge!("agl_backfill_pending_fixes").set(total_pending as f64);

        if total_pending == 0 {
            // Caught up! Sleep for an hour before checking again
            info!("AGL backfill caught up - no more fixes need backfilling. Sleeping for 1 hour.");
            sleep(tokio::time::Duration::from_secs(3600)).await;
            continue;
        }

        // Fetch a batch of fixes to process
        match fixes_repo
            .get_fixes_needing_backfill(one_hour_ago, 1000)
            .await
        {
            Ok(fixes) => {
                let batch_size = fixes.len();
                info!(
                    "Producer: Found {} total fixes needing AGL backfill, processing batch of {} (oldest: {})",
                    total_pending,
                    batch_size,
                    fixes
                        .first()
                        .map(|f| f.timestamp.to_rfc3339())
                        .unwrap_or_default()
                );

                // Send fixes to workers
                let mut sent_count = 0;
                for fix in fixes {
                    let task = BackfillTask::from(fix);
                    if tx.send(task).await.is_err() {
                        warn!("Producer: Failed to send task to workers (channel closed)");
                        break;
                    }
                    sent_count += 1;
                }

                info!(
                    "Producer: Sent {} tasks to {} workers",
                    sent_count, num_workers
                );

                // Small delay before next batch
                sleep(tokio::time::Duration::from_secs(1)).await;
            }
            Err(e) => {
                counter!("agl_backfill_fetch_errors_total").increment(1);
                warn!("Producer: Failed to fetch fixes for backfill: {}", e);
                // Sleep for a bit before retrying
                sleep(tokio::time::Duration::from_secs(60)).await;
            }
        }
    }
}

/// Consumer task that processes backfill tasks from the queue
async fn consumer_task(
    worker_id: usize,
    fixes_repo: FixesRepository,
    elevation_db: ElevationDB,
    rx: std::sync::Arc<tokio::sync::Mutex<mpsc::Receiver<BackfillTask>>>,
) {
    info!("Starting AGL backfill consumer worker {}", worker_id);

    let mut processed_count = 0;
    let mut success_count = 0;
    let mut agl_computed_count = 0;
    let worker_start = std::time::Instant::now();

    loop {
        // Lock the receiver and try to get a task
        let task = {
            let mut rx_guard = rx.lock().await;
            rx_guard.recv().await
        };

        let task = match task {
            Some(t) => t,
            None => {
                // Channel closed, no more tasks
                break;
            }
        };
        // Create a minimal Fix struct for AGL calculation
        let fix = Fix {
            id: task.id,
            latitude: task.latitude,
            longitude: task.longitude,
            altitude_msl_feet: task.altitude_msl_feet,
            // Other fields are not used in calculate_altitude_agl
            source: String::new(),
            aprs_type: String::new(),
            via: vec![],
            timestamp: Utc::now(),
            altitude_agl_feet: None,
            device_address: 0,
            address_type: crate::devices::AddressType::Icao,
            aircraft_type_ogn: None,
            flight_number: None,
            registration: None,
            squawk: None,
            ground_speed_knots: None,
            track_degrees: None,
            climb_fpm: None,
            turn_rate_rot: None,
            snr_db: None,
            bit_errors_corrected: None,
            freq_offset_khz: None,
            gnss_horizontal_resolution: None,
            gnss_vertical_resolution: None,
            flight_id: None,
            device_id: Uuid::nil(),
            received_at: Utc::now(),
            is_active: true,
            receiver_id: None,
            aprs_message_id: None,
            altitude_agl_valid: false,
        };

        // Calculate AGL
        let agl = calculate_altitude_agl(&elevation_db, &fix).await;

        // Update the database (sets both altitude_agl_feet and altitude_agl_valid=true)
        match fixes_repo.update_altitude_agl(task.id, agl).await {
            Ok(_) => {
                success_count += 1;
                counter!("agl_backfill_fixes_processed_total").increment(1);

                if let Some(agl_val) = agl {
                    agl_computed_count += 1;
                    counter!("agl_backfill_altitudes_computed_total").increment(1);
                    debug!(
                        "Worker {}: Backfilled AGL for fix {} ({} MSL -> {} AGL)",
                        worker_id,
                        task.id,
                        task.altitude_msl_feet.unwrap_or(0),
                        agl_val
                    );
                } else {
                    counter!("agl_backfill_no_elevation_data_total").increment(1);
                    debug!(
                        "Worker {}: Backfilled AGL for fix {} (no elevation data available)",
                        worker_id, task.id
                    );
                }
            }
            Err(e) => {
                counter!("agl_backfill_errors_total").increment(1);
                warn!(
                    "Worker {}: Failed to update AGL for fix {}: {}",
                    worker_id, task.id, e
                );
            }
        }

        processed_count += 1;

        // Log progress every 100 fixes
        if processed_count % 100 == 0 {
            let elapsed = worker_start.elapsed();
            let rate = if elapsed.as_secs() > 0 {
                (processed_count as f64 / elapsed.as_secs() as f64) * 60.0
            } else {
                processed_count as f64
            };
            info!(
                "Worker {}: Processed {} fixes ({:.1} fixes/min, {} AGL computed)",
                worker_id, processed_count, rate, agl_computed_count
            );
        }
    }

    info!(
        "Worker {}: Shutting down after processing {} fixes ({} successful, {} AGL computed)",
        worker_id, processed_count, success_count, agl_computed_count
    );
}

/// Background task that backfills AGL altitudes for old fixes that were missed
/// due to elevation processor queue overflow or system restarts
///
/// This uses a producer/consumer pattern with multiple workers to parallelize
/// the elevation lookups and database updates.
pub async fn agl_backfill_task(fixes_repo: FixesRepository, elevation_db: ElevationDB) {
    const NUM_WORKERS: usize = 5;
    const CHANNEL_CAPACITY: usize = 2000; // Buffer for 2 batches

    info!(
        "Starting AGL backfill with {} workers (channel capacity: {})",
        NUM_WORKERS, CHANNEL_CAPACITY
    );

    // Create channel for communication between producer and consumers
    let (tx, rx) = mpsc::channel::<BackfillTask>(CHANNEL_CAPACITY);

    // Wrap receiver in Arc<Mutex> to share among workers
    let rx = std::sync::Arc::new(tokio::sync::Mutex::new(rx));

    // Spawn producer task
    let producer_fixes_repo = fixes_repo.clone();
    let producer_handle = tokio::spawn(async move {
        producer_task(producer_fixes_repo, tx, NUM_WORKERS).await;
    });

    // Spawn consumer workers
    let mut consumer_handles = vec![];
    for worker_id in 0..NUM_WORKERS {
        let worker_fixes_repo = fixes_repo.clone();
        let worker_elevation_db = elevation_db.clone();
        let worker_rx = rx.clone();

        let handle = tokio::spawn(async move {
            consumer_task(worker_id, worker_fixes_repo, worker_elevation_db, worker_rx).await;
        });
        consumer_handles.push(handle);
    }

    // Wait for all tasks (they run forever)
    let _ = tokio::join!(producer_handle);
    for handle in consumer_handles {
        let _ = handle.await;
    }
}
