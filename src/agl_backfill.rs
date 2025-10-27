use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use metrics::{counter, gauge, histogram};
use tokio::time::sleep;
use tracing::{debug, info, warn};

use crate::Fix;
use crate::elevation::ElevationDB;
use crate::fixes_repo::FixesRepository;
use crate::flight_tracker::altitude::calculate_altitude_agl;

/// Background task that backfills AGL altitudes for old fixes that were missed
/// due to elevation processor queue overflow or system restarts
pub async fn agl_backfill_task(fixes_repo: FixesRepository, elevation_db: ElevationDB) {
    info!("Starting AGL backfill background task");

    loop {
        // Find fixes that are:
        // 1. At least one hour old
        // 2. Have altitude_msl_feet (can calculate AGL)
        // 3. Don't have altitude_agl_valid = true (haven't been looked up yet)
        // 4. is_active = true (only backfill active aircraft)
        let one_hour_ago = Utc::now() - Duration::hours(1);

        match fetch_fixes_needing_backfill(&fixes_repo, one_hour_ago, 1000).await {
            Ok(fixes) => {
                if fixes.is_empty() {
                    // Caught up! Sleep for an hour before checking again
                    info!(
                        "AGL backfill caught up - no more fixes need backfilling. Sleeping for 1 hour."
                    );
                    gauge!("agl_backfill_pending_fixes").set(0.0);
                    sleep(tokio::time::Duration::from_secs(3600)).await;
                    continue;
                }

                info!(
                    "Found {} fixes needing AGL backfill (oldest: {})",
                    fixes.len(),
                    fixes
                        .first()
                        .map(|f| f.timestamp.to_rfc3339())
                        .unwrap_or_default()
                );

                // Record number of pending fixes
                gauge!("agl_backfill_pending_fixes").set(fixes.len() as f64);

                // Process each fix
                let mut processed_count = 0;
                let mut success_count = 0;
                let mut agl_computed_count = 0;
                let total_count = fixes.len();
                let batch_start = std::time::Instant::now();

                for fix in fixes {
                    // Calculate AGL
                    let agl = calculate_altitude_agl(&elevation_db, &fix).await;

                    // Update the database (sets both altitude_agl_feet and altitude_agl_valid=true)
                    match fixes_repo.update_altitude_agl(fix.id, agl).await {
                        Ok(_) => {
                            success_count += 1;
                            counter!("agl_backfill_fixes_processed_total").increment(1);

                            if let Some(agl_val) = agl {
                                agl_computed_count += 1;
                                counter!("agl_backfill_altitudes_computed_total").increment(1);
                                debug!(
                                    "Backfilled AGL for fix {} ({} MSL -> {} AGL)",
                                    fix.id,
                                    fix.altitude_msl_feet.unwrap_or(0),
                                    agl_val
                                );
                            } else {
                                counter!("agl_backfill_no_elevation_data_total").increment(1);
                                debug!(
                                    "Backfilled AGL for fix {} (no elevation data available)",
                                    fix.id
                                );
                            }
                        }
                        Err(e) => {
                            counter!("agl_backfill_errors_total").increment(1);
                            warn!("Failed to update AGL for fix {}: {}", fix.id, e);
                        }
                    }

                    processed_count += 1;

                    // Small delay to avoid overwhelming the database
                    if processed_count % 100 == 0 {
                        info!("Backfilled {} / {} fixes", processed_count, total_count);
                        sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                }

                let batch_duration = batch_start.elapsed();
                let altitudes_per_minute = if batch_duration.as_secs() > 0 {
                    (agl_computed_count as f64 / batch_duration.as_secs() as f64) * 60.0
                } else {
                    agl_computed_count as f64
                };

                info!(
                    "Backfill batch complete: {} processed, {} successful, {} altitudes computed ({:.1} alt/min)",
                    processed_count, success_count, agl_computed_count, altitudes_per_minute
                );

                // Record batch processing rate
                histogram!("agl_backfill_batch_duration_seconds")
                    .record(batch_duration.as_secs_f64());
                gauge!("agl_backfill_altitudes_per_minute").set(altitudes_per_minute);

                // Small delay before next batch
                sleep(tokio::time::Duration::from_secs(1)).await;
            }
            Err(e) => {
                counter!("agl_backfill_fetch_errors_total").increment(1);
                warn!("Failed to fetch fixes for backfill: {}", e);
                // Sleep for a bit before retrying
                sleep(tokio::time::Duration::from_secs(60)).await;
            }
        }
    }
}

/// Fetch fixes that need AGL backfilling
/// Returns fixes ordered by timestamp (oldest first)
async fn fetch_fixes_needing_backfill(
    fixes_repo: &FixesRepository,
    before_timestamp: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<Fix>> {
    // Use the public method instead of accessing private pool field
    fixes_repo
        .get_fixes_needing_backfill(before_timestamp, limit)
        .await
}
