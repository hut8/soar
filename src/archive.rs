use anyhow::{Context, Result};
use chrono::{NaiveDate, TimeZone, Utc};
use csv::Writer;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;
use tracing::info;
use zstd::stream::write::Encoder as ZstdEncoder;

use crate::fixes::Fix;
use crate::schema::fixes;

type PgPool = Pool<ConnectionManager<PgConnection>>;

/// Handle the archive command
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

    info!("Starting archive process for fixes before {}", before_date);
    info!("Archive directory: {}", archive_path);

    // Get the oldest fix date from the database
    let oldest_date = get_oldest_fix_date(&pool).await?;

    match oldest_date {
        None => {
            info!("No fixes found in database. Nothing to archive.");
            return Ok(());
        }
        Some(oldest) => {
            info!("Oldest fix date in database: {}", oldest);

            // Archive day by day from oldest to before_date (exclusive)
            let mut current_date = oldest;

            while current_date < before_date {
                archive_day(&pool, current_date, archive_dir).await?;
                current_date = current_date.succ_opt().context(format!(
                    "Failed to calculate next day after {}",
                    current_date
                ))?;
            }

            info!("Archive process completed successfully");
        }
    }

    Ok(())
}

/// Get the oldest fix date from the database
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

/// Archive a single day's worth of fixes
async fn archive_day(pool: &PgPool, date: NaiveDate, archive_dir: &Path) -> Result<()> {
    info!("Archiving fixes for {}", date);

    // Calculate day boundaries (UTC)
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

    // Create temp file name
    let date_str = date.format("%Y%m%d").to_string();
    let final_filename = format!("{}-fixes.csv.zst", date_str);
    let temp_filename = format!(".{}.tmp", final_filename);
    let final_path = archive_dir.join(&final_filename);
    let temp_path = archive_dir.join(&temp_filename);

    // Count fixes for this day
    let count = count_fixes_for_day(pool, day_start, day_end).await?;

    if count == 0 {
        info!("No fixes found for {}. Skipping.", date);
        return Ok(());
    }

    info!("Found {} fixes for {}", count, date);

    // Write fixes to temp file
    write_fixes_to_file(pool, day_start, day_end, &temp_path).await?;

    info!(
        "Successfully wrote {} fixes to {}",
        count,
        temp_path.display()
    );

    // Delete fixes from database (this commits the transaction)
    delete_fixes_for_day(pool, day_start, day_end).await?;

    info!("Successfully deleted {} fixes from database", count);

    // Rename temp file to final file (atomic on same filesystem)
    fs::rename(&temp_path, &final_path).context(format!(
        "Failed to rename {} to {}",
        temp_path.display(),
        final_path.display()
    ))?;

    info!("Successfully archived {} to {}", date, final_path.display());

    Ok(())
}

/// Count fixes for a specific day
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

/// Write fixes to compressed CSV file
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

        // Create file with zstd compression
        let file = File::create(&file_path)
            .context(format!("Failed to create file: {}", file_path.display()))?;
        let buf_writer = BufWriter::new(file);
        let zstd_encoder = ZstdEncoder::new(buf_writer, 3) // compression level 3
            .context("Failed to create zstd encoder")?;

        let mut csv_writer = Writer::from_writer(zstd_encoder);

        // Stream fixes from database using row-by-row loading mode
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

        // Flush and finish the CSV writer
        csv_writer.flush()?;

        // Get the inner zstd encoder
        let zstd_encoder = csv_writer
            .into_inner()
            .context("Failed to get zstd encoder from CSV writer")?;

        // Finish zstd compression and get the BufWriter
        let buf_writer = zstd_encoder
            .finish()
            .context("Failed to finish zstd compression")?;

        // Flush the BufWriter
        let file = buf_writer
            .into_inner()
            .map_err(|e| anyhow::anyhow!("Failed to flush buffer writer: {}", e))?;

        // Sync to disk
        file.sync_all().context("Failed to sync file to disk")?;

        info!("Successfully wrote and synced {} fixes to file", count);

        Ok(())
    })
    .await?
}

/// Delete fixes for a specific day from the database
async fn delete_fixes_for_day(
    pool: &PgPool,
    day_start: chrono::DateTime<Utc>,
    day_end: chrono::DateTime<Utc>,
) -> Result<()> {
    let pool = pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        // Use a transaction to ensure atomicity
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
