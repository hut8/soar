use anyhow::{Context, Result};
use chrono::{NaiveDate, TimeZone, Utc};
use csv::{Reader, Writer};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::Path;
use tracing::info;
use zstd::stream::read::Decoder as ZstdDecoder;
use zstd::stream::write::Encoder as ZstdEncoder;

pub type PgPool = Pool<ConnectionManager<PgConnection>>;

/// Trait for types that can be archived to compressed CSV files
///
/// Each table that needs archiving should implement this trait to provide
/// table-specific logic while reusing common archival infrastructure.
pub trait Archivable: Sized + Serialize + for<'de> Deserialize<'de> + Send + 'static {
    /// Human-readable name of the table (e.g., "flights", "fixes")
    fn table_name() -> &'static str;

    /// Filename suffix for archive files (e.g., "flights" -> "20250101-flights.csv.zst")
    fn filename_suffix() -> &'static str {
        Self::table_name()
    }

    /// Get the oldest date of records in the database
    async fn get_oldest_date(pool: &PgPool) -> Result<Option<NaiveDate>>;

    /// Count records for a specific day range
    async fn count_for_day(
        pool: &PgPool,
        day_start: chrono::DateTime<Utc>,
        day_end: chrono::DateTime<Utc>,
    ) -> Result<i64>;

    /// Get daily counts grouped by date (efficient GROUP BY query)
    /// Returns a vector of (date, count) tuples for the given date range
    async fn get_daily_counts_grouped(
        pool: &PgPool,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<(NaiveDate, i64)>>;

    /// Write records for a specific day to a file
    async fn write_to_file(
        pool: &PgPool,
        day_start: chrono::DateTime<Utc>,
        day_end: chrono::DateTime<Utc>,
        file_path: &Path,
    ) -> Result<()>;

    /// Delete records for a specific day
    async fn delete_for_day(
        pool: &PgPool,
        day_start: chrono::DateTime<Utc>,
        day_end: chrono::DateTime<Utc>,
    ) -> Result<()>;

    /// Insert a batch of records into the database
    fn insert_batch(conn: &mut PgConnection, batch: &[Self]) -> Result<()>;
}

/// Metrics returned from archiving a table
#[derive(Debug, Clone)]
pub struct ArchiveMetrics {
    pub total_rows_deleted: usize,
    pub archive_files: Vec<ArchiveFile>,
}

#[derive(Debug, Clone)]
pub struct ArchiveFile {
    pub path: String,
    pub size_bytes: u64,
}

impl ArchiveMetrics {
    pub fn new() -> Self {
        Self {
            total_rows_deleted: 0,
            archive_files: Vec::new(),
        }
    }
}

/// Archive all records before a given date
pub async fn archive<T: Archivable>(
    pool: &PgPool,
    before_date: NaiveDate,
    archive_dir: &Path,
) -> Result<ArchiveMetrics> {
    info!(
        "Starting archive process for {} before {}",
        T::table_name(),
        before_date
    );

    let mut metrics = ArchiveMetrics::new();

    let oldest_date = T::get_oldest_date(pool).await?;
    match oldest_date {
        None => {
            info!(
                "No {} found in database. Nothing to archive.",
                T::table_name()
            );
            Ok(metrics)
        }
        Some(oldest) => {
            info!("Oldest {} date in database: {}", T::table_name(), oldest);
            let mut current_date = oldest;
            while current_date < before_date {
                let (rows, file_path, file_size) =
                    archive_day::<T>(pool, current_date, archive_dir).await?;
                metrics.total_rows_deleted += rows;
                if let Some(path) = file_path {
                    metrics.archive_files.push(ArchiveFile {
                        path,
                        size_bytes: file_size,
                    });
                }
                current_date = current_date.succ_opt().context(format!(
                    "Failed to calculate next day after {}",
                    current_date
                ))?;
            }
            info!(
                "Archive process completed successfully for {}",
                T::table_name()
            );
            Ok(metrics)
        }
    }
}

/// Archive records for a single day
/// Returns (rows_deleted, file_path, file_size_bytes)
pub async fn archive_day<T: Archivable>(
    pool: &PgPool,
    date: NaiveDate,
    archive_dir: &Path,
) -> Result<(usize, Option<String>, u64)> {
    info!("Archiving {} for {}", T::table_name(), date);
    let (day_start, day_end) = get_day_boundaries(date)?;
    let date_str = date.format("%Y%m%d").to_string();
    let final_filename = format!("{}-{}.csv.zst", date_str, T::filename_suffix());
    let temp_filename = format!(".{}.tmp", final_filename);
    let final_path = archive_dir.join(&final_filename);
    let temp_path = archive_dir.join(&temp_filename);

    let count = T::count_for_day(pool, day_start, day_end).await?;
    if count == 0 {
        info!("No {} found for {}. Skipping.", T::table_name(), date);
        return Ok((0, None, 0));
    }
    info!("Found {} {} for {}", count, T::table_name(), date);

    T::write_to_file(pool, day_start, day_end, &temp_path).await?;
    info!(
        "Successfully wrote {} {} to {}",
        count,
        T::table_name(),
        temp_path.display()
    );

    T::delete_for_day(pool, day_start, day_end).await?;
    info!(
        "Successfully deleted {} {} from database",
        count,
        T::table_name()
    );

    fs::rename(&temp_path, &final_path).context(format!(
        "Failed to rename {} to {}",
        temp_path.display(),
        final_path.display()
    ))?;
    info!("Successfully archived {} to {}", date, final_path.display());

    // Get file size
    let file_size = fs::metadata(&final_path)
        .context("Failed to get file metadata")?
        .len();

    // Run VACUUM ANALYZE on table after deletion
    vacuum_analyze_table(pool, T::table_name()).await?;

    Ok((
        count as usize,
        Some(final_path.to_string_lossy().to_string()),
        file_size,
    ))
}

/// Resurrect (restore) archived records from a file back into the database
pub async fn resurrect<T: Archivable>(pool: &PgPool, file_path: &Path) -> Result<()> {
    info!(
        "Resurrecting {} from {}",
        T::table_name(),
        file_path.display()
    );

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
            let record: T = result?;
            batch.push(record);
            count += 1;

            if batch.len() >= BATCH_SIZE {
                T::insert_batch(&mut conn, &batch)?;
                info!("Inserted {} {}...", count, T::table_name());
                batch.clear();
            }
        }

        // Insert remaining records
        if !batch.is_empty() {
            T::insert_batch(&mut conn, &batch)?;
        }

        info!("Successfully resurrected {} {}", count, T::table_name());
        Ok(())
    })
    .await?
}

/// Get the start and end timestamps for a given date (UTC)
pub fn get_day_boundaries(
    date: NaiveDate,
) -> Result<(chrono::DateTime<Utc>, chrono::DateTime<Utc>)> {
    let day_start = Utc
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
        .single()
        .context(format!("Failed to create start datetime for {}", date))?;

    let next_day = date
        .succ_opt()
        .context(format!("Failed to calculate next day after {}", date))?;
    let day_end = Utc
        .from_local_datetime(&next_day.and_hms_opt(0, 0, 0).unwrap())
        .single()
        .context(format!("Failed to create end datetime for {}", next_day))?;

    Ok((day_start, day_end))
}

/// Run VACUUM ANALYZE on a table to reclaim space and update statistics
/// This improves query performance and reduces table bloat after deletions
pub async fn vacuum_analyze_table(pool: &PgPool, table_name: &str) -> Result<()> {
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

/// Helper for writing records to compressed CSV file with streaming
pub fn write_records_to_file<T, I>(
    records_iter: I,
    file_path: &Path,
    table_name: &str,
) -> Result<()>
where
    T: Serialize,
    I: Iterator<Item = Result<T, diesel::result::Error>>,
{
    let file = File::create(file_path)
        .context(format!("Failed to create file: {}", file_path.display()))?;
    let buf_writer = BufWriter::new(file);
    let zstd_encoder = ZstdEncoder::new(buf_writer, 3).context("Failed to create zstd encoder")?;
    let mut csv_writer = Writer::from_writer(zstd_encoder);

    let mut count = 0;
    for record_result in records_iter {
        let record = record_result?;
        csv_writer.serialize(&record)?;
        count += 1;
        if count % 10000 == 0 {
            info!("Streamed {} {} to file...", count, table_name);
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
        "Successfully wrote and synced {} {} to file",
        count, table_name
    );
    Ok(())
}

/// Collect daily counts using an efficient GROUP BY query instead of N individual queries
/// This reduces 30 queries per table to just 1 query per table
pub async fn collect_daily_counts_grouped<T: Archivable>(
    pool: &PgPool,
    start_date: NaiveDate,
    end_date: NaiveDate,
    archived_before: NaiveDate,
) -> Result<Vec<soar::archive_email_reporter::DailyCount>> {
    use soar::archive_email_reporter::DailyCount;
    use std::collections::HashMap;

    // Get grouped counts from database
    let counts_map: HashMap<NaiveDate, i64> =
        T::get_daily_counts_grouped(pool, start_date, end_date)
            .await?
            .into_iter()
            .collect();

    // Build result vector with all dates in range, filling in zeros for missing dates
    let mut result = Vec::new();
    let mut current_date = start_date;
    while current_date < end_date {
        let count = counts_map.get(&current_date).copied().unwrap_or(0);
        let archived = current_date < archived_before;
        result.push(DailyCount {
            date: current_date,
            count,
            archived,
        });
        current_date = current_date.succ_opt().context(format!(
            "Failed to calculate next day after {}",
            current_date
        ))?;
    }

    Ok(result)
}
