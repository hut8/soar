use metrics::{counter, gauge, histogram};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::elevation::AglDatabaseTask;
use crate::fixes_repo::FixesRepository;

const BATCH_SIZE: usize = 100;
const BATCH_TIMEOUT_SECONDS: u64 = 5;
const QUEUE_WARN_THRESHOLD_80: usize = 8_000;
const QUEUE_WARN_THRESHOLD_95: usize = 9_500;

/// Batch writer task that accumulates AGL updates and writes them to database in batches
/// This dramatically reduces database load by replacing 100+ individual UPDATEs with a single batch UPDATE
pub async fn batch_writer_task(
    mut rx: mpsc::Receiver<AglDatabaseTask>,
    fixes_repo: FixesRepository,
) {
    info!("Starting AGL batch database writer");

    let mut batch: Vec<AglDatabaseTask> = Vec::with_capacity(BATCH_SIZE);
    let mut last_flush = Instant::now();
    let mut warned_80 = false;
    let mut warned_95 = false;

    loop {
        // Try to receive a task with timeout
        match tokio::time::timeout(Duration::from_secs(BATCH_TIMEOUT_SECONDS), rx.recv()).await {
            Ok(Some(task)) => {
                batch.push(task);

                // Check queue depth and warn if backing up
                let queue_depth = rx.len();
                gauge!("agl_db_queue.size").set(queue_depth as f64);

                if queue_depth >= QUEUE_WARN_THRESHOLD_95 && !warned_95 {
                    warn!(
                        "AGL database queue is 95% full ({} / 10,000 tasks) - batch writer may be falling behind!",
                        queue_depth
                    );
                    warned_95 = true;
                    warned_80 = true; // Don't spam 80% warning after 95%
                } else if queue_depth >= QUEUE_WARN_THRESHOLD_80 && !warned_80 {
                    warn!(
                        "AGL database queue is 80% full ({} / 10,000 tasks) - batch writer may be falling behind",
                        queue_depth
                    );
                    warned_80 = true;
                } else if queue_depth < QUEUE_WARN_THRESHOLD_80 {
                    // Reset warnings when queue drains
                    warned_80 = false;
                    warned_95 = false;
                }

                // Flush batch if it's full
                if batch.len() >= BATCH_SIZE {
                    flush_batch(&mut batch, &fixes_repo, &mut last_flush).await;
                }
            }
            Ok(None) => {
                // Channel closed, flush remaining and exit
                if !batch.is_empty() {
                    info!(
                        "AGL database queue closed, flushing final batch of {} items",
                        batch.len()
                    );
                    flush_batch(&mut batch, &fixes_repo, &mut last_flush).await;
                }
                info!("AGL batch writer shutting down");
                break;
            }
            Err(_) => {
                // Timeout - flush batch even if not full (ensures timely updates)
                if !batch.is_empty() {
                    debug!(
                        "Batch timeout reached, flushing {} items ({}s since last flush)",
                        batch.len(),
                        last_flush.elapsed().as_secs()
                    );
                    flush_batch(&mut batch, &fixes_repo, &mut last_flush).await;
                }
            }
        }
    }
}

/// Flush accumulated batch to database using a single batched UPDATE query
async fn flush_batch(
    batch: &mut Vec<AglDatabaseTask>,
    fixes_repo: &FixesRepository,
    last_flush: &mut Instant,
) {
    if batch.is_empty() {
        return;
    }

    let batch_size = batch.len();
    let start = Instant::now();

    match fixes_repo.batch_update_altitude_agl(batch).await {
        Ok(updated_count) => {
            let duration = start.elapsed();
            counter!("agl_batch.updates_total").increment(updated_count as u64);
            histogram!("agl_batch.size").record(batch_size as f64);
            histogram!("agl_batch.duration_ms").record(duration.as_millis() as f64);

            debug!(
                "Flushed AGL batch: {} fixes updated in {:.1}ms ({:.1} updates/sec)",
                updated_count,
                duration.as_millis(),
                (updated_count as f64 / duration.as_secs_f64())
            );
        }
        Err(e) => {
            warn!("Failed to flush AGL batch of {} items: {}", batch_size, e);
            counter!("agl_batch.errors_total").increment(1);
        }
    }

    batch.clear();
    *last_flush = Instant::now();
}
