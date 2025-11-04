use anyhow::{Context, Result};
use chrono::{NaiveDate, TimeZone, Utc};
use csv::{Reader, Writer};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::Path;
use tracing::info;
use zstd::stream::read::Decoder as ZstdDecoder;
use zstd::stream::write::Encoder as ZstdEncoder;

use soar::aprs_messages_repo::AprsMessage;
use soar::fixes::Fix;
use soar::flights::FlightModel;
use soar::receiver_statuses::ReceiverStatus;
use soar::schema::{aprs_messages, fixes, flights, receiver_statuses};

type PgPool = Pool<ConnectionManager<PgConnection>>;

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
    archive_flights(&pool, flights_before, archive_dir).await?;

    // Fixes and ReceiverStatuses next (9+ days old)
    info!("=== Archiving fixes and receiver_statuses (9+ days old) ===");
    let fixes_before = before_date - chrono::Duration::days(9);
    archive_fixes(&pool, fixes_before, archive_dir).await?;
    archive_receiver_statuses(&pool, fixes_before, archive_dir).await?;

    // AprsMessages last (10+ days old)
    info!("=== Archiving aprs_messages (10+ days old) ===");
    let messages_before = before_date - chrono::Duration::days(10);
    archive_aprs_messages(&pool, messages_before, archive_dir).await?;

    info!("Archive process completed successfully");
    Ok(())
}

// ============================================================================
// FLIGHTS ARCHIVAL
// ============================================================================

async fn archive_flights(pool: &PgPool, before_date: NaiveDate, archive_dir: &Path) -> Result<()> {
    info!(
        "Starting archive process for flights before {}",
        before_date
    );

    let oldest_date = get_oldest_flight_date(pool).await?;
    match oldest_date {
        None => {
            info!("No flights found in database. Nothing to archive.");
            return Ok(());
        }
        Some(oldest) => {
            info!("Oldest flight date in database: {}", oldest);
            let mut current_date = oldest;
            while current_date < before_date {
                archive_flights_day(pool, current_date, archive_dir).await?;
                current_date = current_date.succ_opt().context(format!(
                    "Failed to calculate next day after {}",
                    current_date
                ))?;
            }
            info!("Archive process completed successfully for flights");
        }
    }
    Ok(())
}

async fn get_oldest_flight_date(pool: &PgPool) -> Result<Option<NaiveDate>> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        let oldest_timestamp: Option<chrono::DateTime<Utc>> = flights::table
            .select(diesel::dsl::min(flights::last_fix_at))
            .first::<Option<chrono::DateTime<Utc>>>(&mut conn)?;
        Ok(oldest_timestamp.map(|ts| ts.date_naive()))
    })
    .await?
}

async fn archive_flights_day(pool: &PgPool, date: NaiveDate, archive_dir: &Path) -> Result<()> {
    info!("Archiving flights for {}", date);
    let (day_start, day_end) = get_day_boundaries(date)?;
    let date_str = date.format("%Y%m%d").to_string();
    let final_filename = format!("{}-flights.csv.zst", date_str);
    let temp_filename = format!(".{}.tmp", final_filename);
    let final_path = archive_dir.join(&final_filename);
    let temp_path = archive_dir.join(&temp_filename);

    let count = count_flights_for_day(pool, day_start, day_end).await?;
    if count == 0 {
        info!("No flights found for {}. Skipping.", date);
        return Ok(());
    }
    info!("Found {} flights for {}", count, date);

    write_flights_to_file(pool, day_start, day_end, &temp_path).await?;
    info!(
        "Successfully wrote {} flights to {}",
        count,
        temp_path.display()
    );

    delete_flights_for_day(pool, day_start, day_end).await?;
    info!("Successfully deleted {} flights from database", count);

    fs::rename(&temp_path, &final_path).context(format!(
        "Failed to rename {} to {}",
        temp_path.display(),
        final_path.display()
    ))?;
    info!("Successfully archived {} to {}", date, final_path.display());

    // Run VACUUM ANALYZE on flights table after deletion
    vacuum_analyze_table(pool, "flights").await?;

    Ok(())
}

async fn count_flights_for_day(
    pool: &PgPool,
    day_start: chrono::DateTime<Utc>,
    day_end: chrono::DateTime<Utc>,
) -> Result<i64> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        let count = flights::table
            .filter(flights::last_fix_at.ge(day_start))
            .filter(flights::last_fix_at.lt(day_end))
            .count()
            .get_result::<i64>(&mut conn)?;
        Ok(count)
    })
    .await?
}

async fn write_flights_to_file(
    pool: &PgPool,
    day_start: chrono::DateTime<Utc>,
    day_end: chrono::DateTime<Utc>,
    file_path: &Path,
) -> Result<()> {
    let pool = pool.clone();
    let file_path = file_path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        let file = File::create(&file_path)
            .context(format!("Failed to create file: {}", file_path.display()))?;
        let buf_writer = BufWriter::new(file);
        let zstd_encoder =
            ZstdEncoder::new(buf_writer, 3).context("Failed to create zstd encoder")?;
        let mut csv_writer = Writer::from_writer(zstd_encoder);

        let flights_iter = flights::table
            .filter(flights::last_fix_at.ge(day_start))
            .filter(flights::last_fix_at.lt(day_end))
            .order(flights::last_fix_at.asc())
            .select(FlightModel::as_select())
            .load_iter::<FlightModel, diesel::pg::PgRowByRowLoadingMode>(&mut conn)?;

        let mut count = 0;
        for flight_result in flights_iter {
            let flight = flight_result?;
            csv_writer.serialize(&flight)?;
            count += 1;
            if count % 10000 == 0 {
                info!("Streamed {} flights to file...", count);
            }
        }

        csv_writer.flush()?;
        let zstd_encoder = csv_writer
            .into_inner()
            .context("Failed to get zstd encoder from CSV writer")?;
        let buf_writer = zstd_encoder
            .finish()
            .context("Failed to finish zstd compression")?;
        let file = buf_writer
            .into_inner()
            .map_err(|e| anyhow::anyhow!("Failed to flush buffer writer: {}", e))?;
        file.sync_all().context("Failed to sync file to disk")?;
        info!("Successfully wrote and synced {} flights to file", count);
        Ok(())
    })
    .await?
}

async fn delete_flights_for_day(
    pool: &PgPool,
    day_start: chrono::DateTime<Utc>,
    day_end: chrono::DateTime<Utc>,
) -> Result<()> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        conn.transaction::<_, anyhow::Error, _>(|conn| {
            let deleted_count = diesel::delete(
                flights::table
                    .filter(flights::last_fix_at.ge(day_start))
                    .filter(flights::last_fix_at.lt(day_end)),
            )
            .execute(conn)?;
            info!(
                "Deleted {} flights for day starting {}",
                deleted_count, day_start
            );
            Ok(())
        })?;
        Ok(())
    })
    .await?
}

// ============================================================================
// FIXES ARCHIVAL
// ============================================================================

async fn archive_fixes(pool: &PgPool, before_date: NaiveDate, archive_dir: &Path) -> Result<()> {
    info!("Starting archive process for fixes before {}", before_date);

    let oldest_date = get_oldest_fix_date(pool).await?;
    match oldest_date {
        None => {
            info!("No fixes found in database. Nothing to archive.");
            return Ok(());
        }
        Some(oldest) => {
            info!("Oldest fix date in database: {}", oldest);
            let mut current_date = oldest;
            while current_date < before_date {
                archive_fixes_day(pool, current_date, archive_dir).await?;
                current_date = current_date.succ_opt().context(format!(
                    "Failed to calculate next day after {}",
                    current_date
                ))?;
            }
            info!("Archive process completed successfully for fixes");
        }
    }
    Ok(())
}

async fn get_oldest_fix_date(pool: &PgPool) -> Result<Option<NaiveDate>> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        let oldest_timestamp: Option<chrono::DateTime<Utc>> = fixes::table
            .select(diesel::dsl::min(fixes::timestamp))
            .first::<Option<chrono::DateTime<Utc>>>(&mut conn)?;
        Ok(oldest_timestamp.map(|ts| ts.date_naive()))
    })
    .await?
}

async fn archive_fixes_day(pool: &PgPool, date: NaiveDate, archive_dir: &Path) -> Result<()> {
    info!("Archiving fixes for {}", date);
    let (day_start, day_end) = get_day_boundaries(date)?;
    let date_str = date.format("%Y%m%d").to_string();
    let final_filename = format!("{}-fixes.csv.zst", date_str);
    let temp_filename = format!(".{}.tmp", final_filename);
    let final_path = archive_dir.join(&final_filename);
    let temp_path = archive_dir.join(&temp_filename);

    let count = count_fixes_for_day(pool, day_start, day_end).await?;
    if count == 0 {
        info!("No fixes found for {}. Skipping.", date);
        return Ok(());
    }
    info!("Found {} fixes for {}", count, date);

    write_fixes_to_file(pool, day_start, day_end, &temp_path).await?;
    info!(
        "Successfully wrote {} fixes to {}",
        count,
        temp_path.display()
    );

    delete_fixes_for_day(pool, day_start, day_end).await?;
    info!("Successfully deleted {} fixes from database", count);

    fs::rename(&temp_path, &final_path).context(format!(
        "Failed to rename {} to {}",
        temp_path.display(),
        final_path.display()
    ))?;
    info!("Successfully archived {} to {}", date, final_path.display());

    // Run VACUUM ANALYZE on fixes table after deletion
    vacuum_analyze_table(pool, "fixes").await?;

    Ok(())
}

async fn count_fixes_for_day(
    pool: &PgPool,
    day_start: chrono::DateTime<Utc>,
    day_end: chrono::DateTime<Utc>,
) -> Result<i64> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        let count = fixes::table
            .filter(fixes::timestamp.ge(day_start))
            .filter(fixes::timestamp.lt(day_end))
            .count()
            .get_result::<i64>(&mut conn)?;
        Ok(count)
    })
    .await?
}

async fn write_fixes_to_file(
    pool: &PgPool,
    day_start: chrono::DateTime<Utc>,
    day_end: chrono::DateTime<Utc>,
    file_path: &Path,
) -> Result<()> {
    let pool = pool.clone();
    let file_path = file_path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        let file = File::create(&file_path)
            .context(format!("Failed to create file: {}", file_path.display()))?;
        let buf_writer = BufWriter::new(file);
        let zstd_encoder =
            ZstdEncoder::new(buf_writer, 3).context("Failed to create zstd encoder")?;
        let mut csv_writer = Writer::from_writer(zstd_encoder);

        let fixes_iter = fixes::table
            .filter(fixes::timestamp.ge(day_start))
            .filter(fixes::timestamp.lt(day_end))
            .order(fixes::timestamp.asc())
            .select(Fix::as_select())
            .load_iter::<Fix, diesel::pg::PgRowByRowLoadingMode>(&mut conn)?;

        let mut count = 0;
        for fix_result in fixes_iter {
            let fix = fix_result?;
            csv_writer.serialize(&fix)?;
            count += 1;
            if count % 10000 == 0 {
                info!("Streamed {} fixes to file...", count);
            }
        }

        csv_writer.flush()?;
        let zstd_encoder = csv_writer
            .into_inner()
            .context("Failed to get zstd encoder from CSV writer")?;
        let buf_writer = zstd_encoder
            .finish()
            .context("Failed to finish zstd compression")?;
        let file = buf_writer
            .into_inner()
            .map_err(|e| anyhow::anyhow!("Failed to flush buffer writer: {}", e))?;
        file.sync_all().context("Failed to sync file to disk")?;
        info!("Successfully wrote and synced {} fixes to file", count);
        Ok(())
    })
    .await?
}

async fn delete_fixes_for_day(
    pool: &PgPool,
    day_start: chrono::DateTime<Utc>,
    day_end: chrono::DateTime<Utc>,
) -> Result<()> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        conn.transaction::<_, anyhow::Error, _>(|conn| {
            let deleted_count = diesel::delete(
                fixes::table
                    .filter(fixes::timestamp.ge(day_start))
                    .filter(fixes::timestamp.lt(day_end)),
            )
            .execute(conn)?;
            info!(
                "Deleted {} fixes for day starting {}",
                deleted_count, day_start
            );
            Ok(())
        })?;
        Ok(())
    })
    .await?
}

// ============================================================================
// RECEIVER_STATUSES ARCHIVAL
// ============================================================================

async fn archive_receiver_statuses(
    pool: &PgPool,
    before_date: NaiveDate,
    archive_dir: &Path,
) -> Result<()> {
    info!(
        "Starting archive process for receiver_statuses before {}",
        before_date
    );

    let oldest_date = get_oldest_receiver_status_date(pool).await?;
    match oldest_date {
        None => {
            info!("No receiver_statuses found in database. Nothing to archive.");
            return Ok(());
        }
        Some(oldest) => {
            info!("Oldest receiver_status date in database: {}", oldest);
            let mut current_date = oldest;
            while current_date < before_date {
                archive_receiver_statuses_day(pool, current_date, archive_dir).await?;
                current_date = current_date.succ_opt().context(format!(
                    "Failed to calculate next day after {}",
                    current_date
                ))?;
            }
            info!("Archive process completed successfully for receiver_statuses");
        }
    }
    Ok(())
}

async fn get_oldest_receiver_status_date(pool: &PgPool) -> Result<Option<NaiveDate>> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        let oldest_timestamp: Option<chrono::DateTime<Utc>> = receiver_statuses::table
            .select(diesel::dsl::min(receiver_statuses::received_at))
            .first::<Option<chrono::DateTime<Utc>>>(&mut conn)?;
        Ok(oldest_timestamp.map(|ts| ts.date_naive()))
    })
    .await?
}

async fn archive_receiver_statuses_day(
    pool: &PgPool,
    date: NaiveDate,
    archive_dir: &Path,
) -> Result<()> {
    info!("Archiving receiver_statuses for {}", date);
    let (day_start, day_end) = get_day_boundaries(date)?;
    let date_str = date.format("%Y%m%d").to_string();
    let final_filename = format!("{}-receiver_statuses.csv.zst", date_str);
    let temp_filename = format!(".{}.tmp", final_filename);
    let final_path = archive_dir.join(&final_filename);
    let temp_path = archive_dir.join(&temp_filename);

    let count = count_receiver_statuses_for_day(pool, day_start, day_end).await?;
    if count == 0 {
        info!("No receiver_statuses found for {}. Skipping.", date);
        return Ok(());
    }
    info!("Found {} receiver_statuses for {}", count, date);

    write_receiver_statuses_to_file(pool, day_start, day_end, &temp_path).await?;
    info!(
        "Successfully wrote {} receiver_statuses to {}",
        count,
        temp_path.display()
    );

    delete_receiver_statuses_for_day(pool, day_start, day_end).await?;
    info!(
        "Successfully deleted {} receiver_statuses from database",
        count
    );

    fs::rename(&temp_path, &final_path).context(format!(
        "Failed to rename {} to {}",
        temp_path.display(),
        final_path.display()
    ))?;
    info!("Successfully archived {} to {}", date, final_path.display());

    // Run VACUUM ANALYZE on receiver_statuses table after deletion
    vacuum_analyze_table(pool, "receiver_statuses").await?;

    Ok(())
}

async fn count_receiver_statuses_for_day(
    pool: &PgPool,
    day_start: chrono::DateTime<Utc>,
    day_end: chrono::DateTime<Utc>,
) -> Result<i64> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        let count = receiver_statuses::table
            .filter(receiver_statuses::received_at.ge(day_start))
            .filter(receiver_statuses::received_at.lt(day_end))
            .count()
            .get_result::<i64>(&mut conn)?;
        Ok(count)
    })
    .await?
}

async fn write_receiver_statuses_to_file(
    pool: &PgPool,
    day_start: chrono::DateTime<Utc>,
    day_end: chrono::DateTime<Utc>,
    file_path: &Path,
) -> Result<()> {
    let pool = pool.clone();
    let file_path = file_path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        let file = File::create(&file_path)
            .context(format!("Failed to create file: {}", file_path.display()))?;
        let buf_writer = BufWriter::new(file);
        let zstd_encoder =
            ZstdEncoder::new(buf_writer, 3).context("Failed to create zstd encoder")?;
        let mut csv_writer = Writer::from_writer(zstd_encoder);

        let statuses_iter = receiver_statuses::table
            .filter(receiver_statuses::received_at.ge(day_start))
            .filter(receiver_statuses::received_at.lt(day_end))
            .order(receiver_statuses::received_at.asc())
            .select(ReceiverStatus::as_select())
            .load_iter::<ReceiverStatus, diesel::pg::PgRowByRowLoadingMode>(&mut conn)?;

        let mut count = 0;
        for status_result in statuses_iter {
            let status = status_result?;
            csv_writer.serialize(&status)?;
            count += 1;
            if count % 10000 == 0 {
                info!("Streamed {} receiver_statuses to file...", count);
            }
        }

        csv_writer.flush()?;
        let zstd_encoder = csv_writer
            .into_inner()
            .context("Failed to get zstd encoder from CSV writer")?;
        let buf_writer = zstd_encoder
            .finish()
            .context("Failed to finish zstd compression")?;
        let file = buf_writer
            .into_inner()
            .map_err(|e| anyhow::anyhow!("Failed to flush buffer writer: {}", e))?;
        file.sync_all().context("Failed to sync file to disk")?;
        info!(
            "Successfully wrote and synced {} receiver_statuses to file",
            count
        );
        Ok(())
    })
    .await?
}

async fn delete_receiver_statuses_for_day(
    pool: &PgPool,
    day_start: chrono::DateTime<Utc>,
    day_end: chrono::DateTime<Utc>,
) -> Result<()> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        conn.transaction::<_, anyhow::Error, _>(|conn| {
            let deleted_count = diesel::delete(
                receiver_statuses::table
                    .filter(receiver_statuses::received_at.ge(day_start))
                    .filter(receiver_statuses::received_at.lt(day_end)),
            )
            .execute(conn)?;
            info!(
                "Deleted {} receiver_statuses for day starting {}",
                deleted_count, day_start
            );
            Ok(())
        })?;
        Ok(())
    })
    .await?
}

// ============================================================================
// APRS_MESSAGES ARCHIVAL
// ============================================================================

async fn archive_aprs_messages(
    pool: &PgPool,
    before_date: NaiveDate,
    archive_dir: &Path,
) -> Result<()> {
    info!(
        "Starting archive process for aprs_messages before {}",
        before_date
    );

    let oldest_date = get_oldest_aprs_message_date(pool).await?;
    match oldest_date {
        None => {
            info!("No aprs_messages found in database. Nothing to archive.");
            return Ok(());
        }
        Some(oldest) => {
            info!("Oldest aprs_message date in database: {}", oldest);
            let mut current_date = oldest;
            while current_date < before_date {
                archive_aprs_messages_day(pool, current_date, archive_dir).await?;
                current_date = current_date.succ_opt().context(format!(
                    "Failed to calculate next day after {}",
                    current_date
                ))?;
            }
            info!("Archive process completed successfully for aprs_messages");
        }
    }
    Ok(())
}

async fn get_oldest_aprs_message_date(pool: &PgPool) -> Result<Option<NaiveDate>> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        let oldest_timestamp: Option<chrono::DateTime<Utc>> = aprs_messages::table
            .select(diesel::dsl::min(aprs_messages::received_at))
            .first::<Option<chrono::DateTime<Utc>>>(&mut conn)?;
        Ok(oldest_timestamp.map(|ts| ts.date_naive()))
    })
    .await?
}

async fn archive_aprs_messages_day(
    pool: &PgPool,
    date: NaiveDate,
    archive_dir: &Path,
) -> Result<()> {
    info!("Archiving aprs_messages for {}", date);
    let (day_start, day_end) = get_day_boundaries(date)?;
    let date_str = date.format("%Y%m%d").to_string();
    let final_filename = format!("{}-aprs_messages.csv.zst", date_str);
    let temp_filename = format!(".{}.tmp", final_filename);
    let final_path = archive_dir.join(&final_filename);
    let temp_path = archive_dir.join(&temp_filename);

    let count = count_aprs_messages_for_day(pool, day_start, day_end).await?;
    if count == 0 {
        info!("No aprs_messages found for {}. Skipping.", date);
        return Ok(());
    }
    info!("Found {} aprs_messages for {}", count, date);

    write_aprs_messages_to_file(pool, day_start, day_end, &temp_path).await?;
    info!(
        "Successfully wrote {} aprs_messages to {}",
        count,
        temp_path.display()
    );

    delete_aprs_messages_for_day(pool, day_start, day_end).await?;
    info!("Successfully deleted {} aprs_messages from database", count);

    fs::rename(&temp_path, &final_path).context(format!(
        "Failed to rename {} to {}",
        temp_path.display(),
        final_path.display()
    ))?;
    info!("Successfully archived {} to {}", date, final_path.display());

    // Run VACUUM ANALYZE on aprs_messages table after deletion
    vacuum_analyze_table(pool, "aprs_messages").await?;

    Ok(())
}

async fn count_aprs_messages_for_day(
    pool: &PgPool,
    day_start: chrono::DateTime<Utc>,
    day_end: chrono::DateTime<Utc>,
) -> Result<i64> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        let count = aprs_messages::table
            .filter(aprs_messages::received_at.ge(day_start))
            .filter(aprs_messages::received_at.lt(day_end))
            .count()
            .get_result::<i64>(&mut conn)?;
        Ok(count)
    })
    .await?
}

async fn write_aprs_messages_to_file(
    pool: &PgPool,
    day_start: chrono::DateTime<Utc>,
    day_end: chrono::DateTime<Utc>,
    file_path: &Path,
) -> Result<()> {
    let pool = pool.clone();
    let file_path = file_path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        let file = File::create(&file_path)
            .context(format!("Failed to create file: {}", file_path.display()))?;
        let buf_writer = BufWriter::new(file);
        let zstd_encoder =
            ZstdEncoder::new(buf_writer, 3).context("Failed to create zstd encoder")?;
        let mut csv_writer = Writer::from_writer(zstd_encoder);

        let messages_iter = aprs_messages::table
            .filter(aprs_messages::received_at.ge(day_start))
            .filter(aprs_messages::received_at.lt(day_end))
            .order(aprs_messages::received_at.asc())
            .select(AprsMessage::as_select())
            .load_iter::<AprsMessage, diesel::pg::PgRowByRowLoadingMode>(&mut conn)?;

        let mut count = 0;
        for message_result in messages_iter {
            let message = message_result?;
            csv_writer.serialize(&message)?;
            count += 1;
            if count % 10000 == 0 {
                info!("Streamed {} aprs_messages to file...", count);
            }
        }

        csv_writer.flush()?;
        let zstd_encoder = csv_writer
            .into_inner()
            .context("Failed to get zstd encoder from CSV writer")?;
        let buf_writer = zstd_encoder
            .finish()
            .context("Failed to finish zstd compression")?;
        let file = buf_writer
            .into_inner()
            .map_err(|e| anyhow::anyhow!("Failed to flush buffer writer: {}", e))?;
        file.sync_all().context("Failed to sync file to disk")?;
        info!(
            "Successfully wrote and synced {} aprs_messages to file",
            count
        );
        Ok(())
    })
    .await?
}

async fn delete_aprs_messages_for_day(
    pool: &PgPool,
    day_start: chrono::DateTime<Utc>,
    day_end: chrono::DateTime<Utc>,
) -> Result<()> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        conn.transaction::<_, anyhow::Error, _>(|conn| {
            let deleted_count = diesel::delete(
                aprs_messages::table
                    .filter(aprs_messages::received_at.ge(day_start))
                    .filter(aprs_messages::received_at.lt(day_end)),
            )
            .execute(conn)?;
            info!(
                "Deleted {} aprs_messages for day starting {}",
                deleted_count, day_start
            );
            Ok(())
        })?;
        Ok(())
    })
    .await?
}

// ============================================================================
// RESURRECT COMMAND - Restore archived data back to database
// ============================================================================

/// Handle the resurrect command
/// Restores archived data in the reverse order of archival to respect foreign key constraints:
/// 1. AprsMessages (must be restored first since fixes and receiver_statuses reference them)
/// 2. Fixes and ReceiverStatuses (depend on aprs_messages)
/// 3. Flights (depend on fixes)
pub async fn handle_resurrect(pool: PgPool, date: String, archive_path: String) -> Result<()> {
    // Parse the date
    let resurrect_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").context(format!(
        "Invalid date format '{}'. Expected YYYY-MM-DD",
        date
    ))?;

    let archive_dir = Path::new(&archive_path);

    // Verify archive directory exists
    if !archive_dir.exists() {
        anyhow::bail!("Archive directory does not exist: {}", archive_path);
    }

    info!("Starting resurrect process for {}", resurrect_date);
    info!("Archive directory: {}", archive_path);

    // Resurrect in reverse order of archival to respect foreign key constraints
    // AprsMessages first (no dependencies)
    info!("=== Resurrecting aprs_messages ===");
    resurrect_aprs_messages(&pool, resurrect_date, archive_dir).await?;

    // Fixes and ReceiverStatuses next (depend on aprs_messages)
    info!("=== Resurrecting fixes and receiver_statuses ===");
    resurrect_fixes(&pool, resurrect_date, archive_dir).await?;
    resurrect_receiver_statuses(&pool, resurrect_date, archive_dir).await?;

    // Flights last (depend on fixes)
    info!("=== Resurrecting flights ===");
    resurrect_flights(&pool, resurrect_date, archive_dir).await?;

    info!(
        "Resurrect process completed successfully for {}",
        resurrect_date
    );
    Ok(())
}

// ============================================================================
// APRS_MESSAGES RESURRECTION
// ============================================================================

async fn resurrect_aprs_messages(pool: &PgPool, date: NaiveDate, archive_dir: &Path) -> Result<()> {
    let date_str = date.format("%Y%m%d").to_string();
    let filename = format!("{}-aprs_messages.csv.zst", date_str);
    let file_path = archive_dir.join(&filename);

    if !file_path.exists() {
        info!(
            "No archive file found for aprs_messages: {}. Skipping.",
            filename
        );
        return Ok(());
    }

    info!("Reading aprs_messages from {}", file_path.display());

    let pool = pool.clone();
    let file_path = file_path.to_path_buf();

    tokio::task::spawn_blocking(move || {
        let file = File::open(&file_path)
            .context(format!("Failed to open file: {}", file_path.display()))?;
        let buf_reader = BufReader::new(file);
        let zstd_decoder = ZstdDecoder::new(buf_reader).context("Failed to create zstd decoder")?;
        let mut csv_reader = Reader::from_reader(zstd_decoder);

        let mut conn = pool.get()?;
        let mut count = 0;
        let mut batch = Vec::new();
        const BATCH_SIZE: usize = 1000;

        for result in csv_reader.deserialize() {
            let message: AprsMessage = result?;
            batch.push(message);
            count += 1;

            if batch.len() >= BATCH_SIZE {
                insert_aprs_messages_batch(&mut conn, &batch)?;
                info!("Inserted {} aprs_messages...", count);
                batch.clear();
            }
        }

        // Insert remaining records
        if !batch.is_empty() {
            insert_aprs_messages_batch(&mut conn, &batch)?;
        }

        info!("Successfully resurrected {} aprs_messages", count);
        Ok(())
    })
    .await?
}

fn insert_aprs_messages_batch(conn: &mut PgConnection, batch: &[AprsMessage]) -> Result<()> {
    use soar::aprs_messages_repo::NewAprsMessage;
    use soar::schema::aprs_messages;

    conn.transaction::<_, anyhow::Error, _>(|conn| {
        for message in batch {
            let mut new_message = NewAprsMessage::new(
                message.raw_message.clone(),
                message.received_at,
                message.receiver_id,
                message.unparsed.clone(),
            );
            // Preserve the original message ID from archive
            new_message.id = message.id;

            diesel::insert_into(aprs_messages::table)
                .values(&new_message)
                .on_conflict_do_nothing()
                .execute(conn)?;
        }
        Ok(())
    })?;
    Ok(())
}

// ============================================================================
// FIXES RESURRECTION
// ============================================================================

async fn resurrect_fixes(pool: &PgPool, date: NaiveDate, archive_dir: &Path) -> Result<()> {
    let date_str = date.format("%Y%m%d").to_string();
    let filename = format!("{}-fixes.csv.zst", date_str);
    let file_path = archive_dir.join(&filename);

    if !file_path.exists() {
        info!("No archive file found for fixes: {}. Skipping.", filename);
        return Ok(());
    }

    info!("Reading fixes from {}", file_path.display());

    let pool = pool.clone();
    let file_path = file_path.to_path_buf();

    tokio::task::spawn_blocking(move || {
        let file = File::open(&file_path)
            .context(format!("Failed to open file: {}", file_path.display()))?;
        let buf_reader = BufReader::new(file);
        let zstd_decoder = ZstdDecoder::new(buf_reader).context("Failed to create zstd decoder")?;
        let mut csv_reader = Reader::from_reader(zstd_decoder);

        let mut conn = pool.get()?;
        let mut count = 0;
        let mut batch = Vec::new();
        const BATCH_SIZE: usize = 1000;

        for result in csv_reader.deserialize() {
            let fix: Fix = result?;
            batch.push(fix);
            count += 1;

            if batch.len() >= BATCH_SIZE {
                insert_fixes_batch(&mut conn, &batch)?;
                info!("Inserted {} fixes...", count);
                batch.clear();
            }
        }

        // Insert remaining records
        if !batch.is_empty() {
            insert_fixes_batch(&mut conn, &batch)?;
        }

        info!("Successfully resurrected {} fixes", count);
        Ok(())
    })
    .await?
}

fn insert_fixes_batch(conn: &mut PgConnection, batch: &[Fix]) -> Result<()> {
    use soar::schema::fixes;

    conn.transaction::<_, anyhow::Error, _>(|conn| {
        for fix in batch {
            diesel::insert_into(fixes::table)
                .values(fix)
                .on_conflict_do_nothing()
                .execute(conn)?;
        }
        Ok(())
    })?;
    Ok(())
}

// ============================================================================
// RECEIVER_STATUSES RESURRECTION
// ============================================================================

async fn resurrect_receiver_statuses(
    pool: &PgPool,
    date: NaiveDate,
    archive_dir: &Path,
) -> Result<()> {
    let date_str = date.format("%Y%m%d").to_string();
    let filename = format!("{}-receiver_statuses.csv.zst", date_str);
    let file_path = archive_dir.join(&filename);

    if !file_path.exists() {
        info!(
            "No archive file found for receiver_statuses: {}. Skipping.",
            filename
        );
        return Ok(());
    }

    info!("Reading receiver_statuses from {}", file_path.display());

    let pool = pool.clone();
    let file_path = file_path.to_path_buf();

    tokio::task::spawn_blocking(move || {
        let file = File::open(&file_path)
            .context(format!("Failed to open file: {}", file_path.display()))?;
        let buf_reader = BufReader::new(file);
        let zstd_decoder = ZstdDecoder::new(buf_reader).context("Failed to create zstd decoder")?;
        let mut csv_reader = Reader::from_reader(zstd_decoder);

        let mut conn = pool.get()?;
        let mut count = 0;
        let mut batch = Vec::new();
        const BATCH_SIZE: usize = 1000;

        for result in csv_reader.deserialize() {
            let status: ReceiverStatus = result?;
            batch.push(status);
            count += 1;

            if batch.len() >= BATCH_SIZE {
                insert_receiver_statuses_batch(&mut conn, &batch)?;
                info!("Inserted {} receiver_statuses...", count);
                batch.clear();
            }
        }

        // Insert remaining records
        if !batch.is_empty() {
            insert_receiver_statuses_batch(&mut conn, &batch)?;
        }

        info!("Successfully resurrected {} receiver_statuses", count);
        Ok(())
    })
    .await?
}

fn insert_receiver_statuses_batch(conn: &mut PgConnection, batch: &[ReceiverStatus]) -> Result<()> {
    use soar::schema::receiver_statuses;

    conn.transaction::<_, anyhow::Error, _>(|conn| {
        for status in batch {
            diesel::insert_into(receiver_statuses::table)
                .values(status)
                .on_conflict_do_nothing()
                .execute(conn)?;
        }
        Ok(())
    })?;
    Ok(())
}

// ============================================================================
// FLIGHTS RESURRECTION
// ============================================================================

async fn resurrect_flights(pool: &PgPool, date: NaiveDate, archive_dir: &Path) -> Result<()> {
    let date_str = date.format("%Y%m%d").to_string();
    let filename = format!("{}-flights.csv.zst", date_str);
    let file_path = archive_dir.join(&filename);

    if !file_path.exists() {
        info!("No archive file found for flights: {}. Skipping.", filename);
        return Ok(());
    }

    info!("Reading flights from {}", file_path.display());

    let pool = pool.clone();
    let file_path = file_path.to_path_buf();

    tokio::task::spawn_blocking(move || {
        let file = File::open(&file_path)
            .context(format!("Failed to open file: {}", file_path.display()))?;
        let buf_reader = BufReader::new(file);
        let zstd_decoder = ZstdDecoder::new(buf_reader).context("Failed to create zstd decoder")?;
        let mut csv_reader = Reader::from_reader(zstd_decoder);

        let mut conn = pool.get()?;
        let mut count = 0;
        let mut batch = Vec::new();
        const BATCH_SIZE: usize = 1000;

        for result in csv_reader.deserialize() {
            let flight: FlightModel = result?;
            batch.push(flight);
            count += 1;

            if batch.len() >= BATCH_SIZE {
                insert_flights_batch(&mut conn, &batch)?;
                info!("Inserted {} flights...", count);
                batch.clear();
            }
        }

        // Insert remaining records
        if !batch.is_empty() {
            insert_flights_batch(&mut conn, &batch)?;
        }

        info!("Successfully resurrected {} flights", count);
        Ok(())
    })
    .await?
}

fn insert_flights_batch(conn: &mut PgConnection, batch: &[FlightModel]) -> Result<()> {
    use soar::schema::flights;

    conn.transaction::<_, anyhow::Error, _>(|conn| {
        for flight in batch {
            diesel::insert_into(flights::table)
                .values(flight)
                .on_conflict_do_nothing()
                .execute(conn)?;
        }
        Ok(())
    })?;
    Ok(())
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Run VACUUM ANALYZE on a table to reclaim space and update statistics
/// This improves query performance and reduces table bloat after deletions
async fn vacuum_analyze_table(pool: &PgPool, table_name: &str) -> Result<()> {
    info!("Running VACUUM ANALYZE on table '{}'...", table_name);
    let pool = pool.clone();
    let table_name = table_name.to_string();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        // VACUUM ANALYZE cannot run inside a transaction, so we use SimpleConnection
        // which executes the command directly without a transaction
        use diesel::connection::SimpleConnection;
        let query = format!("VACUUM ANALYZE {}", table_name);
        conn.batch_execute(&query)
            .context(format!("Failed to VACUUM ANALYZE table '{}'", table_name))?;
        info!(
            "Successfully completed VACUUM ANALYZE on table '{}'",
            table_name
        );
        Ok::<(), anyhow::Error>(())
    })
    .await??;

    Ok(())
}

/// Get day boundaries (start and end) for a given date in UTC
fn get_day_boundaries(date: NaiveDate) -> Result<(chrono::DateTime<Utc>, chrono::DateTime<Utc>)> {
    let day_start = Utc
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
        .single()
        .context(format!("Failed to create start datetime for {}", date))?;

    let day_end = Utc
        .from_local_datetime(
            &date
                .succ_opt()
                .context(format!("Failed to calculate day after {}", date))?
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        )
        .single()
        .context(format!("Failed to create end datetime for {}", date))?;

    Ok((day_start, day_end))
}
