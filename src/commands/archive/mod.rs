mod aprs_messages;
mod archiver;
mod fixes;
mod flights;
mod receiver_statuses;

use anyhow::{Context, Result};
use archiver::{PgPool, archive, resurrect};
use chrono::{NaiveDate, Utc};
use soar::aprs_messages_repo::AprsMessage;
use soar::fixes::Fix;
use soar::flights::FlightModel;
use soar::receiver_statuses::ReceiverStatus;
use std::fs;
use std::path::Path;
use tracing::info;

/// Handle the archive command
/// Archives data in the correct order to respect foreign key constraints:
/// 1. Flights (8+ days old)
/// 2. Fixes and ReceiverStatuses (9+ days old)
/// 3. AprsMessages (10+ days old)
pub async fn handle_archive(pool: PgPool, before: String, archive_path: String) -> Result<()> {
    // Parse the before date
    let before_date = NaiveDate::parse_from_str(&before, "%Y-%m-%d").context(format!(
        "Invalid date format '{}'. Expected YYYY-MM-DD",
        before
    ))?;

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

    // Archive in order to respect foreign key constraints
    // Flights first (8+ days old)
    info!("=== Archiving flights (8+ days old) ===");
    let flights_before = before_date - chrono::Duration::days(8);
    archive::<FlightModel>(&pool, flights_before, archive_dir).await?;

    // Fixes and ReceiverStatuses next (9+ days old)
    info!("=== Archiving fixes and receiver_statuses (9+ days old) ===");
    let fixes_before = before_date - chrono::Duration::days(9);
    archive::<Fix>(&pool, fixes_before, archive_dir).await?;
    archive::<ReceiverStatus>(&pool, fixes_before, archive_dir).await?;

    // AprsMessages last (10+ days old)
    info!("=== Archiving aprs_messages (10+ days old) ===");
    let messages_before = before_date - chrono::Duration::days(10);
    archive::<AprsMessage>(&pool, messages_before, archive_dir).await?;

    info!("Archive process completed successfully");
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
        resurrect::<AprsMessage>(&pool, &aprs_messages_file).await?;
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
