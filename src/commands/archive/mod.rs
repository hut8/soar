mod aprs_messages;
mod archiver;
mod fixes;
mod flights;
mod receiver_statuses;

use anyhow::{Context, Result};
use aprs_messages::AprsMessageCsv;
use archiver::{Archivable, PgPool, archive, resurrect};
use chrono::{NaiveDate, Utc};
use soar::fixes::Fix;
use soar::flights::FlightModel;
use soar::receiver_statuses::ReceiverStatus;
use std::fs;
use std::path::Path;
use tracing::info;

/// Handle the archive command
/// Archives data in the correct order to respect foreign key constraints:
/// 1. Flights (before_date + 0 days)
/// 2. Fixes and ReceiverStatuses (before_date + 1 day)
/// 3. AprsMessages (before_date + 2 days)
///
/// Defaults to 21 days ago if no before date is specified:
/// - Flights: 21 days old
/// - Fixes and ReceiverStatuses: 22 days old
/// - AprsMessages: 23 days old
pub async fn handle_archive(
    pool: PgPool,
    before: Option<String>,
    archive_path: String,
) -> Result<()> {
    use soar::archive_email_reporter::{
        ArchiveReport, DailyCount, TableArchiveMetrics, send_archive_email_report,
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
        // Default to 21 days ago
        Utc::now().date_naive() - chrono::Duration::days(21)
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

    // Archive in parallel with staggered retention to respect foreign key constraints
    // Using different date ranges allows safe parallel execution:
    // - Flights: before_date + 0 days
    // - Fixes and ReceiverStatuses: before_date + 1 day
    // - AprsMessages: before_date + 2 days
    info!(
        "Starting parallel archive: flights (before {}), fixes/receiver_statuses (before {}), aprs_messages (before {})",
        before_date,
        before_date + chrono::Duration::days(1),
        before_date + chrono::Duration::days(2)
    );

    let flights_before = before_date;
    let fixes_before = before_date + chrono::Duration::days(1);
    let messages_before = before_date + chrono::Duration::days(2);

    let archive_dir_path = archive_dir.to_path_buf();
    let pool_clone1 = pool.clone();
    let pool_clone2 = pool.clone();
    let pool_clone3 = pool.clone();
    let pool_clone4 = pool.clone();

    // Archive all tables in parallel
    let (flights_result, fixes_result, receiver_statuses_result, aprs_messages_result) = tokio::join!(
        async {
            let start = Instant::now();
            let metrics =
                match archive::<FlightModel>(&pool_clone1, flights_before, &archive_dir_path).await
                {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::error!("Failed to archive flights: {}", e);
                        return Err(anyhow::anyhow!("Failed to archive flights: {}", e));
                    }
                };
            Ok((metrics, start.elapsed().as_secs_f64()))
        },
        async {
            let start = Instant::now();
            let metrics = match archive::<Fix>(&pool_clone2, fixes_before, &archive_dir_path).await
            {
                Ok(m) => m,
                Err(e) => {
                    tracing::error!("Failed to archive fixes: {}", e);
                    return Err(anyhow::anyhow!("Failed to archive fixes: {}", e));
                }
            };
            Ok((metrics, start.elapsed().as_secs_f64()))
        },
        async {
            let start = Instant::now();
            let metrics = match archive::<ReceiverStatus>(
                &pool_clone3,
                fixes_before,
                &archive_dir_path,
            )
            .await
            {
                Ok(m) => m,
                Err(e) => {
                    tracing::error!("Failed to archive receiver_statuses: {}", e);
                    return Err(anyhow::anyhow!(
                        "Failed to archive receiver_statuses: {}",
                        e
                    ));
                }
            };
            Ok((metrics, start.elapsed().as_secs_f64()))
        },
        async {
            let start = Instant::now();
            let metrics =
                match archive::<AprsMessageCsv>(&pool_clone4, messages_before, &archive_dir_path)
                    .await
                {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::error!("Failed to archive aprs_messages: {}", e);
                        return Err(anyhow::anyhow!("Failed to archive aprs_messages: {}", e));
                    }
                };
            Ok((metrics, start.elapsed().as_secs_f64()))
        }
    );

    // Check if any archival task failed
    let (flights_metrics, flights_duration) = flights_result?;
    let (fixes_metrics, fixes_duration) = fixes_result?;
    let (receiver_statuses_metrics, receiver_statuses_duration) = receiver_statuses_result?;
    let (aprs_messages_metrics, aprs_messages_duration) = aprs_messages_result?;

    info!("Parallel archival completed, collecting metadata...");

    // Collect metadata for flights
    let flights_oldest = FlightModel::get_oldest_date(&pool).await?;
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
    let fixes_oldest = Fix::get_oldest_date(&pool).await?;
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
    let receiver_statuses_oldest = ReceiverStatus::get_oldest_date(&pool).await?;
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

    // Collect metadata for aprs_messages
    let aprs_messages_oldest = AprsMessageCsv::get_oldest_date(&pool).await?;
    let aprs_messages_file_size = aprs_messages_metrics
        .archive_files
        .iter()
        .map(|f| f.size_bytes)
        .sum();
    let aprs_messages_file_path = aprs_messages_metrics
        .archive_files
        .first()
        .map(|f| f.path.clone())
        .unwrap_or_else(|| "N/A".to_string());

    report.add_table(TableArchiveMetrics {
        table_name: "aprs_messages".to_string(),
        rows_deleted: aprs_messages_metrics.total_rows_deleted,
        file_path: aprs_messages_file_path,
        file_size_bytes: aprs_messages_file_size,
        duration_secs: aprs_messages_duration,
        oldest_remaining: aprs_messages_oldest,
    });

    report.total_duration_secs = total_start.elapsed().as_secs_f64();

    // Collect daily counts for analytics (last 30 days)
    info!("Collecting daily counts for analytics...");
    let analytics_days = 30;
    let analytics_start_date = today - chrono::Duration::days(analytics_days);

    // Collect for flights
    let mut flights_counts = Vec::new();
    for day_offset in 0..analytics_days {
        let date = analytics_start_date + chrono::Duration::days(day_offset);
        if date >= today {
            continue; // Don't include today or future dates
        }
        let day_start = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let day_end = (date + chrono::Duration::days(1))
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let count = FlightModel::count_for_day(&pool, day_start, day_end).await?;
        let archived = date < flights_before;
        flights_counts.push(DailyCount {
            date,
            count,
            archived,
        });
    }
    report.add_daily_counts("flights".to_string(), flights_counts);

    // Collect for fixes
    let mut fixes_counts = Vec::new();
    for day_offset in 0..analytics_days {
        let date = analytics_start_date + chrono::Duration::days(day_offset);
        if date >= today {
            continue;
        }
        let day_start = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let day_end = (date + chrono::Duration::days(1))
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let count = Fix::count_for_day(&pool, day_start, day_end).await?;
        let archived = date < fixes_before;
        fixes_counts.push(DailyCount {
            date,
            count,
            archived,
        });
    }
    report.add_daily_counts("fixes".to_string(), fixes_counts);

    // Collect for receiver_statuses
    let mut receiver_statuses_counts = Vec::new();
    for day_offset in 0..analytics_days {
        let date = analytics_start_date + chrono::Duration::days(day_offset);
        if date >= today {
            continue;
        }
        let day_start = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let day_end = (date + chrono::Duration::days(1))
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let count = ReceiverStatus::count_for_day(&pool, day_start, day_end).await?;
        let archived = date < fixes_before;
        receiver_statuses_counts.push(DailyCount {
            date,
            count,
            archived,
        });
    }
    report.add_daily_counts("receiver_statuses".to_string(), receiver_statuses_counts);

    // Collect for aprs_messages
    let mut aprs_messages_counts = Vec::new();
    for day_offset in 0..analytics_days {
        let date = analytics_start_date + chrono::Duration::days(day_offset);
        if date >= today {
            continue;
        }
        let day_start = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let day_end = (date + chrono::Duration::days(1))
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let count = AprsMessageCsv::count_for_day(&pool, day_start, day_end).await?;
        let archived = date < messages_before;
        aprs_messages_counts.push(DailyCount {
            date,
            count,
            archived,
        });
    }
    report.add_daily_counts("aprs_messages".to_string(), aprs_messages_counts);

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
/// Resurrects (restores) archived data from compressed CSV files back into the database
/// Restores data in the reverse order of archival to respect foreign key constraints:
/// 1. AprsMessages (must be restored first)
/// 2. Fixes and ReceiverStatuses (depend on aprs_messages)
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
    info!("=== Resurrecting aprs_messages ===");
    let aprs_messages_file = archive_dir.join(format!("{}-aprs_messages.csv.zst", date_str));
    if aprs_messages_file.exists() {
        resurrect::<AprsMessageCsv>(&pool, &aprs_messages_file).await?;
    } else {
        info!(
            "No aprs_messages archive found for {} (expected: {})",
            date,
            aprs_messages_file.display()
        );
    }

    // Fixes and ReceiverStatuses next
    info!("=== Resurrecting fixes and receiver_statuses ===");
    let fixes_file = archive_dir.join(format!("{}-fixes.csv.zst", date_str));
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
        archive_dir.join(format!("{}-receiver_statuses.csv.zst", date_str));
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
    let flights_file = archive_dir.join(format!("{}-flights.csv.zst", date_str));
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
