use chrono::{Duration, Utc};
use metrics::{counter, gauge};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::agl_batch_writer::batch_writer_task;
use crate::elevation::{AglDatabaseTask, ElevationDB};
use crate::fixes_repo::{BackfillFix, FixesRepository};

/// Lightweight struct containing only the data needed for AGL backfill processing
/// Note: altitude_msl_feet is non-optional because we only backfill fixes that have this value
#[derive(Debug, Clone)]
struct BackfillTask {
    id: Uuid,
    latitude: f64,
    longitude: f64,
    altitude_msl_feet: i32,
}

impl From<BackfillFix> for BackfillTask {
    fn from(fix: BackfillFix) -> Self {
        Self {
            id: fix.id,
            latitude: fix.latitude,
            longitude: fix.longitude,
            // No need to unwrap - BackfillFix.altitude_msl_feet is guaranteed non-null
            altitude_msl_feet: fix.altitude_msl_feet,
        }
    }
}

/// Producer task that loads batches of fixes needing backfill and sends them to workers
async fn producer_task(fixes_repo: FixesRepository, tx: mpsc::Sender<BackfillTask>) {
    info!("Starting AGL backfill producer task");

    // Track when we last updated the pending count metric (expensive query)
    let mut last_count_update = std::time::Instant::now();
    let count_update_interval = tokio::time::Duration::from_secs(60); // Update count every 60 seconds
    let mut cached_total_pending: Option<i64> = None;

    loop {
        // Find fixes that are:
        // 1. At least one hour old
        // 2. Have altitude_msl_feet (can calculate AGL)
        // 3. Don't have altitude_agl_valid = true (haven't been looked up yet)
        // 4. is_active = true (only backfill active aircraft)
        let one_hour_ago = Utc::now() - Duration::hours(1);

        // Only run the expensive count query periodically (every 60 seconds)
        // This query can be expensive on large databases, so we don't run it on every iteration
        if last_count_update.elapsed() >= count_update_interval {
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
            last_count_update = std::time::Instant::now();
            cached_total_pending = Some(total_pending);

            if total_pending == 0 {
                // Caught up! Sleep for an hour before checking again
                info!(
                    "AGL backfill caught up - no more fixes need backfilling. Sleeping for 1 hour."
                );
                sleep(tokio::time::Duration::from_secs(3600)).await;
                continue;
            }
        }

        // Fetch a batch of fixes to process
        match fixes_repo
            .get_fixes_needing_backfill(one_hour_ago, 1000)
            .await
        {
            Ok(fixes) => {
                let batch_size = fixes.len();
                if let Some(total) = cached_total_pending {
                    info!(
                        "Producer: Found {} total fixes needing AGL backfill, processing batch of {}",
                        total, batch_size
                    );
                } else {
                    info!("Producer: Processing batch of {} fixes", batch_size);
                }

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

                info!("Producer: Sent {} tasks to workers", sent_count);

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

/// Calculate AGL altitude directly from BackfillTask data
/// This is a simplified version of calculate_altitude_agl that works with our task struct
async fn calculate_agl_from_task(elevation_db: &ElevationDB, task: &BackfillTask) -> Option<i32> {
    // Run blocking elevation lookup in a separate thread
    let elevation_result = elevation_db
        .elevation_egm2008(task.latitude, task.longitude)
        .await
        .ok()?;

    // Get true elevation at this location (in meters)
    match elevation_result {
        Some(elevation_m) => {
            // Convert elevation from meters to feet (1 meter = 3.28084 feet)
            let elevation_ft = elevation_m * 3.28084;
            // Calculate AGL (Above Ground Level)
            let agl = task.altitude_msl_feet as f64 - elevation_ft;

            Some(agl.round() as i32)
        }
        None => {
            // No elevation data available (e.g., ocean)
            None
        }
    }
}

/// Consumer task that processes backfill tasks from the queue
async fn consumer_task(
    worker_id: usize,
    elevation_db: ElevationDB,
    rx: std::sync::Arc<tokio::sync::Mutex<mpsc::Receiver<BackfillTask>>>,
    db_tx: mpsc::Sender<AglDatabaseTask>,
) {
    info!("Starting AGL backfill consumer worker {}", worker_id);

    let mut processed_count = 0;
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

        // Calculate AGL directly from task data
        let agl = calculate_agl_from_task(&elevation_db, &task).await;

        // Send to batch writer for database update
        let db_task = AglDatabaseTask {
            fix_id: task.id,
            altitude_agl_feet: agl,
        };

        if db_tx.send(db_task).await.is_err() {
            warn!(
                "Worker {}: Failed to send database task (channel closed)",
                worker_id
            );
            break;
        }

        // Update metrics
        counter!("agl_backfill_fixes_processed_total").increment(1);

        if let Some(agl_val) = agl {
            agl_computed_count += 1;
            counter!("agl_backfill_altitudes_computed_total").increment(1);
            debug!(
                "Worker {}: Computed AGL for fix {} ({} MSL -> {} AGL)",
                worker_id, task.id, task.altitude_msl_feet, agl_val
            );
        } else {
            counter!("agl_backfill_no_elevation_data_total").increment(1);
            debug!(
                "Worker {}: Computed AGL for fix {} (no elevation data available)",
                worker_id, task.id
            );
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
        "Worker {}: Shutting down after processing {} fixes ({} AGL computed)",
        worker_id, processed_count, agl_computed_count
    );
}

/// Background task that backfills AGL altitudes for old fixes that were missed
/// due to elevation processor queue overflow or system restarts
///
/// This uses a producer/consumer pattern with multiple workers to parallelize
/// the elevation lookups, and a batch writer for efficient database updates.
pub async fn agl_backfill_task(fixes_repo: FixesRepository, elevation_db: ElevationDB) {
    const NUM_WORKERS: usize = 5;
    const CHANNEL_CAPACITY: usize = 2000; // Buffer for 2 batches
    const DB_CHANNEL_CAPACITY: usize = 10000; // Match real-time processor capacity

    info!(
        "Starting AGL backfill with {} workers (task channel: {}, db channel: {})",
        NUM_WORKERS, CHANNEL_CAPACITY, DB_CHANNEL_CAPACITY
    );

    // Create channel for communication between producer and consumers
    let (tx, rx) = mpsc::channel::<BackfillTask>(CHANNEL_CAPACITY);

    // Create channel for database updates (consumers -> batch writer)
    let (db_tx, db_rx) = mpsc::channel::<AglDatabaseTask>(DB_CHANNEL_CAPACITY);

    // Wrap receiver in Arc<Mutex> to share among workers
    let rx = std::sync::Arc::new(tokio::sync::Mutex::new(rx));

    // Spawn batch writer task
    let batch_writer_fixes_repo = fixes_repo.clone();
    let batch_writer_handle = tokio::spawn(async move {
        batch_writer_task(db_rx, batch_writer_fixes_repo).await;
    });

    // Spawn producer task
    let producer_fixes_repo = fixes_repo.clone();
    let producer_handle = tokio::spawn(async move {
        producer_task(producer_fixes_repo, tx).await;
    });

    // Spawn consumer workers
    let mut consumer_handles = vec![];
    for worker_id in 0..NUM_WORKERS {
        let worker_elevation_db = elevation_db.clone();
        let worker_rx = rx.clone();
        let worker_db_tx = db_tx.clone();

        let handle = tokio::spawn(async move {
            consumer_task(worker_id, worker_elevation_db, worker_rx, worker_db_tx).await;
        });
        consumer_handles.push(handle);
    }

    // Drop the original db_tx so batch writer knows when all workers are done
    drop(db_tx);

    // Wait for all tasks (they run forever)
    let _ = tokio::join!(producer_handle, batch_writer_handle);
    for handle in consumer_handles {
        let _ = handle.await;
    }
}
