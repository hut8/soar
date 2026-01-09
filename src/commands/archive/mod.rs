mod archiver;
mod fixes;
mod flights;
mod raw_messages;
mod receiver_statuses;

use anyhow::{Context, Result};
use archiver::{Archivable, PgPool, archive, collect_daily_counts_grouped, resurrect};
use chrono::{NaiveDate, TimeZone, Utc};
use raw_messages::RawMessageCsv;
use soar::fixes::Fix;
use soar::flights::FlightModel;
use soar::receiver_statuses::ReceiverStatus;
use std::fs;
use std::path::Path;
use tracing::info;

/// Handle the archive command
/// Archives data in the correct order to respect foreign key constraints with ON DELETE RESTRICT:
/// 1. Fixes (children first - they reference flights and raw_messages)
/// 2. ReceiverStatuses (children - they reference raw_messages)
/// 3. Flights (parents - after fixes are deleted, clear self-references via towed_by_flight_id)
/// 4. RawMessages (parents last - nothing references them anymore)
///
/// All tables archive data from the same date (before_date).
/// Defaults to 45 days ago if no before date is specified.
pub async fn handle_archive(
    pool: PgPool,
    before: Option<String>,
    archive_path: String,
) -> Result<()> {
    use soar::archive_email_reporter::{
        ArchiveReport, TableArchiveMetrics, send_archive_email_report,
    };
    use soar::email_reporter::EmailConfig;
    use std::time::Instant;

    let total_start = Instant::now();

    // Parse or calculate the before date
    let before_date = if let Some(before_str) = before {
        NaiveDate::parse_from_str(&before_str, "%Y-%m-%d").context(format!(
            "Invalid date format '{}'. Expected YYYY-MM-DD",
            before_str
        ))?
    } else {
        // Default to 45 days ago
        Utc::now().date_naive() - chrono::Duration::days(45)
    };

    // Get today's date (UTC)
    let today = Utc::now().date_naive();

    // Validate that before_date is not in the future
    if before_date > today {
        anyhow::bail!(
            "Archive date {} is in the future. Cannot archive future data.",
            before_date
        );
    }

    // Create archive directory if it doesn't exist
    let archive_dir = Path::new(&archive_path);
    fs::create_dir_all(archive_dir).context(format!(
        "Failed to create archive directory: {}",
        archive_path
    ))?;

    info!("Starting archive process before {}", before_date);
    info!("Archive directory: {}", archive_path);

    let mut report = ArchiveReport::new();

    // Archive sequentially in dependency order to respect ON DELETE RESTRICT constraints
    // All tables use the same before_date (simplified from staggered dates)
    info!(
        "Starting sequential archive: fixes -> receiver_statuses -> flights -> raw_messages (all before {})",
        before_date
    );

    let archive_dir_path = archive_dir.to_path_buf();

    // Step 1: Archive fixes first (they reference flights and raw_messages)
    info!("=== Step 1/4: Archiving fixes ===");
    let start = Instant::now();
    let fixes_metrics = archive::<Fix>(&pool, before_date, &archive_dir_path).await?;
    let fixes_duration = start.elapsed().as_secs_f64();

    // Step 2: Archive receiver_statuses (they reference raw_messages)
    info!("=== Step 2/4: Archiving receiver_statuses ===");
    let start = Instant::now();
    let receiver_statuses_metrics =
        archive::<ReceiverStatus>(&pool, before_date, &archive_dir_path).await?;
    let receiver_statuses_duration = start.elapsed().as_secs_f64();

    // Step 3a: Clear towed_by_flight_id for flights that reference other flights being archived
    // This prevents FK violations when deleting flights that reference each other
    info!("=== Step 3a/4: Clearing towed_by_flight_id for flights being archived ===");
    {
        use diesel::prelude::*;
        use soar::schema::flights::dsl::*;
        let pool_clone = pool.clone();
        let before_date_clone = before_date;
        tokio::task::spawn_blocking(move || {
            let mut conn = pool_clone.get()?;
            let updated = diesel::update(
                flights.filter(
                    last_fix_at.lt(chrono::Utc
                        .from_local_datetime(&before_date_clone.and_hms_opt(0, 0, 0).unwrap())
                        .single()
                        .ok_or_else(|| anyhow::anyhow!("Failed to create cutoff datetime"))?),
                ),
            )
            .set(towed_by_flight_id.eq::<Option<uuid::Uuid>>(None))
            .execute(&mut conn)?;
            info!("Cleared towed_by_flight_id for {} flights", updated);
            Ok::<_, anyhow::Error>(())
        })
        .await??;
    }

    // Step 3b: Archive flights (after fixes are deleted and self-references cleared)
    info!("=== Step 3b/4: Archiving flights ===");
    let start = Instant::now();
    let flights_metrics = archive::<FlightModel>(&pool, before_date, &archive_dir_path).await?;
    let flights_duration = start.elapsed().as_secs_f64();

    // Step 4: Archive raw_messages last (nothing references them anymore)
    info!("=== Step 4/4: Archiving raw_messages ===");
    let start = Instant::now();
    let raw_messages_metrics =
        archive::<RawMessageCsv>(&pool, before_date, &archive_dir_path).await?;
    let raw_messages_duration = start.elapsed().as_secs_f64();

    info!("Sequential archival completed, collecting metadata...");

    // Collect metadata for flights
    // Use today + 1000 years as a "no upper bound" to get the actual oldest remaining record
    let far_future = today + chrono::Duration::days(365 * 1000);
    let flights_oldest = FlightModel::get_oldest_date(&pool, far_future).await?;
    let flights_file_size = flights_metrics
        .archive_files
        .iter()
        .map(|f| f.size_bytes)
        .sum();
    let flights_file_path = flights_metrics
        .archive_files
        .first()
        .map(|f| f.path.clone())
        .unwrap_or_else(|| "N/A".to_string());

    report.add_table(TableArchiveMetrics {
        table_name: "flights".to_string(),
        rows_deleted: flights_metrics.total_rows_deleted,
        file_path: flights_file_path,
        file_size_bytes: flights_file_size,
        duration_secs: flights_duration,
        oldest_remaining: flights_oldest,
    });

    // Collect metadata for fixes
    let fixes_oldest = Fix::get_oldest_date(&pool, far_future).await?;
    let fixes_file_size = fixes_metrics
        .archive_files
        .iter()
        .map(|f| f.size_bytes)
        .sum();
    let fixes_file_path = fixes_metrics
        .archive_files
        .first()
        .map(|f| f.path.clone())
        .unwrap_or_else(|| "N/A".to_string());

    report.add_table(TableArchiveMetrics {
        table_name: "fixes".to_string(),
        rows_deleted: fixes_metrics.total_rows_deleted,
        file_path: fixes_file_path,
        file_size_bytes: fixes_file_size,
        duration_secs: fixes_duration,
        oldest_remaining: fixes_oldest,
    });

    // Collect metadata for receiver_statuses
    let receiver_statuses_oldest = ReceiverStatus::get_oldest_date(&pool, far_future).await?;
    let receiver_statuses_file_size = receiver_statuses_metrics
        .archive_files
        .iter()
        .map(|f| f.size_bytes)
        .sum();
    let receiver_statuses_file_path = receiver_statuses_metrics
        .archive_files
        .first()
        .map(|f| f.path.clone())
        .unwrap_or_else(|| "N/A".to_string());

    report.add_table(TableArchiveMetrics {
        table_name: "receiver_statuses".to_string(),
        rows_deleted: receiver_statuses_metrics.total_rows_deleted,
        file_path: receiver_statuses_file_path,
        file_size_bytes: receiver_statuses_file_size,
        duration_secs: receiver_statuses_duration,
        oldest_remaining: receiver_statuses_oldest,
    });

    // Collect metadata for raw_messages
    let raw_messages_oldest = RawMessageCsv::get_oldest_date(&pool, far_future).await?;
    let raw_messages_file_size = raw_messages_metrics
        .archive_files
        .iter()
        .map(|f| f.size_bytes)
        .sum();
    let raw_messages_file_path = raw_messages_metrics
        .archive_files
        .first()
        .map(|f| f.path.clone())
        .unwrap_or_else(|| "N/A".to_string());

    report.add_table(TableArchiveMetrics {
        table_name: "raw_messages".to_string(),
        rows_deleted: raw_messages_metrics.total_rows_deleted,
        file_path: raw_messages_file_path,
        file_size_bytes: raw_messages_file_size,
        duration_secs: raw_messages_duration,
        oldest_remaining: raw_messages_oldest,
    });

    report.total_duration_secs = total_start.elapsed().as_secs_f64();

    // Collect daily counts for analytics (last 30 days)
    // Use efficient GROUP BY queries instead of 120 individual queries
    info!("Collecting daily counts for analytics...");
    let analytics_days = 30;
    let analytics_start_date = today - chrono::Duration::days(analytics_days);

    // Collect daily counts in parallel using GROUP BY queries
    let pool_clone1 = pool.clone();
    let pool_clone2 = pool.clone();
    let pool_clone3 = pool.clone();
    let pool_clone4 = pool.clone();

    let (
        flights_counts_result,
        fixes_counts_result,
        receiver_statuses_counts_result,
        raw_messages_counts_result,
    ) = tokio::join!(
        collect_daily_counts_grouped::<FlightModel>(
            &pool_clone1,
            analytics_start_date,
            today,
            before_date
        ),
        collect_daily_counts_grouped::<Fix>(&pool_clone2, analytics_start_date, today, before_date),
        collect_daily_counts_grouped::<ReceiverStatus>(
            &pool_clone3,
            analytics_start_date,
            today,
            before_date
        ),
        collect_daily_counts_grouped::<RawMessageCsv>(
            &pool_clone4,
            analytics_start_date,
            today,
            before_date
        ),
    );

    report.add_daily_counts("flights".to_string(), flights_counts_result?);
    report.add_daily_counts("fixes".to_string(), fixes_counts_result?);
    report.add_daily_counts(
        "receiver_statuses".to_string(),
        receiver_statuses_counts_result?,
    );
    report.add_daily_counts("raw_messages".to_string(), raw_messages_counts_result?);

    // Count unreferenced locations created in the last 7 days
    info!("Counting unreferenced locations from last 7 days...");
    let seven_days_ago = today - chrono::Duration::days(7);
    let locations_repo = soar::locations_repo::LocationsRepository::new(pool.clone());
    match locations_repo
        .count_unreferenced_locations_in_range(seven_days_ago, today)
        .await
    {
        Ok(count) => {
            info!(
                "Found {} unreferenced locations created in last 7 days",
                count
            );
            report.unreferenced_locations_7d = Some(count);
        }
        Err(e) => {
            tracing::warn!(
                "Failed to count unreferenced locations for email report: {}",
                e
            );
        }
    }

    info!("Archive process completed successfully");

    // Send email report
    match EmailConfig::from_env() {
        Ok(email_config) => {
            info!("Sending archive email report...");
            if let Err(e) = send_archive_email_report(&email_config, &report) {
                tracing::warn!("Failed to send archive email report: {}", e);
            }
        }
        Err(e) => {
            tracing::warn!(
                "Email configuration not available, skipping email report: {}",
                e
            );
        }
    }

    Ok(())
}

/// Handle the resurrect command
/// Resurrects (restores) archived data from compressed JSON Lines files back into the database
/// Restores data in the reverse order of archival to respect foreign key constraints:
/// 1. AprsMessages (must be restored first)
/// 2. Fixes and ReceiverStatuses (depend on raw_messages)
/// 3. Flights (depend on fixes)
pub async fn handle_resurrect(pool: PgPool, date: String, archive_path: String) -> Result<()> {
    // Parse the date
    let parsed_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").context(format!(
        "Invalid date format '{}'. Expected YYYY-MM-DD",
        date
    ))?;

    let archive_dir = Path::new(&archive_path);
    if !archive_dir.exists() {
        anyhow::bail!("Archive directory not found: {}", archive_path);
    }

    info!(
        "Starting resurrection from {} for date {}",
        archive_path, date
    );
    let date_str = parsed_date.format("%Y%m%d").to_string();

    // Resurrect in reverse order to respect foreign key constraints
    // AprsMessages first
    info!("=== Resurrecting raw_messages ===");
    let raw_messages_file = archive_dir.join(format!("{}-raw_messages.jsonl.zst", date_str));
    if raw_messages_file.exists() {
        resurrect::<RawMessageCsv>(&pool, &raw_messages_file).await?;
    } else {
        info!(
            "No raw_messages archive found for {} (expected: {})",
            date,
            raw_messages_file.display()
        );
    }

    // Fixes and ReceiverStatuses next
    info!("=== Resurrecting fixes and receiver_statuses ===");
    let fixes_file = archive_dir.join(format!("{}-fixes.jsonl.zst", date_str));
    if fixes_file.exists() {
        resurrect::<Fix>(&pool, &fixes_file).await?;
    } else {
        info!(
            "No fixes archive found for {} (expected: {})",
            date,
            fixes_file.display()
        );
    }

    let receiver_statuses_file =
        archive_dir.join(format!("{}-receiver_statuses.jsonl.zst", date_str));
    if receiver_statuses_file.exists() {
        resurrect::<ReceiverStatus>(&pool, &receiver_statuses_file).await?;
    } else {
        info!(
            "No receiver_statuses archive found for {} (expected: {})",
            date,
            receiver_statuses_file.display()
        );
    }

    // Flights last
    info!("=== Resurrecting flights ===");
    let flights_file = archive_dir.join(format!("{}-flights.jsonl.zst", date_str));
    if flights_file.exists() {
        resurrect::<FlightModel>(&pool, &flights_file).await?;
    } else {
        info!(
            "No flights archive found for {} (expected: {})",
            date,
            flights_file.display()
        );
    }

    info!("Resurrection process completed successfully");
    Ok(())
}
