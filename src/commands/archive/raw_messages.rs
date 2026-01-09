use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use diesel::prelude::*;
use hex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;
use uuid::Uuid;

use soar::raw_messages_repo::AprsMessage;
use soar::schema::raw_messages;

use super::archiver::{Archivable, PgPool, finalize_pending_detachment, write_records_to_file};

/// Serializable version of RawMessage (AprsMessage) with hex-encoded hash
/// This is used for archival with text-friendly encoding for binary fields
/// Handles both APRS and ADS-B messages from the raw_messages table
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RawMessageCsv {
    id: Uuid,
    raw_message: String,
    received_at: DateTime<Utc>,
    receiver_id: Option<Uuid>, // NULL for ADS-B messages
    unparsed: Option<String>,
    raw_message_hash: String, // Hex-encoded instead of Vec<u8>
}

impl From<AprsMessage> for RawMessageCsv {
    fn from(msg: AprsMessage) -> Self {
        Self {
            id: msg.id,
            raw_message: String::from_utf8_lossy(&msg.raw_message).to_string(), // Decode bytes to UTF-8
            received_at: msg.received_at,
            receiver_id: msg.receiver_id,
            unparsed: msg.unparsed,
            raw_message_hash: hex::encode(msg.raw_message_hash),
        }
    }
}

impl From<RawMessageCsv> for AprsMessage {
    fn from(csv: RawMessageCsv) -> Self {
        Self {
            id: csv.id,
            raw_message: csv.raw_message.into_bytes(), // Encode UTF-8 text to bytes
            received_at: csv.received_at,
            receiver_id: csv.receiver_id,
            unparsed: csv.unparsed,
            raw_message_hash: hex::decode(csv.raw_message_hash).unwrap_or_default(),
        }
    }
}

impl Archivable for RawMessageCsv {
    fn table_name() -> &'static str {
        "raw_messages"
    }

    fn is_partitioned() -> bool {
        true // raw_messages table is partitioned by received_at date
    }

    async fn get_oldest_date(pool: &PgPool, before_date: NaiveDate) -> Result<Option<NaiveDate>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            // Add WHERE clause to enable partition pruning - only scan partitions before the cutoff
            // This is much more efficient than scanning all partitions
            let cutoff = Utc
                .from_local_datetime(&before_date.and_hms_opt(0, 0, 0).unwrap())
                .single()
                .ok_or_else(|| anyhow::anyhow!("Failed to create cutoff datetime"))?;

            let oldest_timestamp: Option<chrono::DateTime<Utc>> = raw_messages::table
                .select(raw_messages::received_at)
                .filter(raw_messages::received_at.lt(cutoff))
                .order(raw_messages::received_at.asc())
                .limit(1)
                .first::<chrono::DateTime<Utc>>(&mut conn)
                .optional()?;
            Ok(oldest_timestamp.map(|ts| ts.date_naive()))
        })
        .await?
    }

    async fn count_for_day(
        pool: &PgPool,
        day_start: chrono::DateTime<Utc>,
        day_end: chrono::DateTime<Utc>,
    ) -> Result<i64> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let count = raw_messages::table
                .filter(raw_messages::received_at.ge(day_start))
                .filter(raw_messages::received_at.lt(day_end))
                .count()
                .get_result::<i64>(&mut conn)?;
            Ok(count)
        })
        .await?
    }

    async fn get_daily_counts_grouped(
        pool: &PgPool,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<(NaiveDate, i64)>> {
        use diesel::dsl::sql;
        use diesel::sql_types::{BigInt, Date};

        let pool = pool.clone();
        let start_datetime = start_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end_datetime = end_date.and_hms_opt(0, 0, 0).unwrap().and_utc();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Use DATE() to group by day in UTC
            let results: Vec<(NaiveDate, i64)> = raw_messages::table
                .filter(raw_messages::received_at.ge(start_datetime))
                .filter(raw_messages::received_at.lt(end_datetime))
                .select((sql::<Date>("DATE(received_at)"), sql::<BigInt>("COUNT(*)")))
                .group_by(sql::<Date>("DATE(received_at)"))
                .load(&mut conn)?;

            Ok(results)
        })
        .await?
    }

    async fn write_to_file(
        pool: &PgPool,
        day_start: chrono::DateTime<Utc>,
        day_end: chrono::DateTime<Utc>,
        file_path: &Path,
    ) -> Result<()> {
        let pool = pool.clone();
        let file_path = file_path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            // Query database for AprsMessage records
            let messages_iter = raw_messages::table
                .filter(raw_messages::received_at.ge(day_start))
                .filter(raw_messages::received_at.lt(day_end))
                .order(raw_messages::received_at.asc())
                .select(AprsMessage::as_select())
                .load_iter::<AprsMessage, diesel::pg::PgRowByRowLoadingMode>(&mut conn)?;

            // Convert AprsMessage to RawMessageCsv for JSON serialization
            let csv_messages_iter = messages_iter.map(|result| result.map(RawMessageCsv::from));

            write_records_to_file(csv_messages_iter, &file_path, Self::table_name())
        })
        .await?
    }

    async fn delete_for_day(
        pool: &PgPool,
        day_start: chrono::DateTime<Utc>,
        _day_end: chrono::DateTime<Utc>,
    ) -> Result<()> {
        // Calculate partition name from day_start
        // Partitions are named like raw_messages_p20251114 for 2025-11-14
        let partition_date = day_start.format("%Y%m%d");
        let partition_name = format!("raw_messages_p{}", partition_date);

        // First, check for and finalize any pending detachment from a previous interrupted operation
        // This handles the edge case where a previous DETACH PARTITION CONCURRENTLY was interrupted
        finalize_pending_detachment(pool, "raw_messages", &partition_name).await?;

        let pool = pool.clone();
        let partition_name_clone = partition_name.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            use diesel::connection::SimpleConnection;

            // Drop the partition directly - much faster than DELETE
            // This works because:
            // 1. Each partition contains exactly one day's data
            // 2. Foreign keys are enforced at the partition level
            // 3. We've already archived the data to disk
            // 4. Child tables (fixes, receiver_statuses) have already been cleaned up

            // Detach partition using CONCURRENTLY for non-blocking operation
            // CONCURRENTLY cannot run inside a transaction, so we use SimpleConnection
            // This command blocks until detachment completes (uses two-transaction process)
            let detach_sql = format!(
                "ALTER TABLE raw_messages DETACH PARTITION {} CONCURRENTLY",
                partition_name_clone
            );
            conn.batch_execute(&detach_sql).context(format!(
                "Failed to detach partition {}",
                partition_name_clone
            ))?;
            info!(
                "Detached partition {} from raw_messages table (CONCURRENTLY)",
                partition_name_clone
            );

            // Drop the detached partition table
            // Safe to do immediately because DETACH CONCURRENTLY waits for completion
            let drop_sql = format!("DROP TABLE IF EXISTS {}", partition_name_clone);
            conn.batch_execute(&drop_sql)
                .context(format!("Failed to drop partition {}", partition_name_clone))?;
            info!(
                "Dropped partition {} for day starting {}",
                partition_name_clone, day_start
            );

            Ok(())
        })
        .await?
    }

    fn insert_batch(conn: &mut PgConnection, batch: &[Self]) -> Result<()> {
        conn.transaction::<_, anyhow::Error, _>(|conn| {
            for csv_message in batch {
                // Convert archive format back to database format
                let db_message: AprsMessage = csv_message.clone().into();
                diesel::insert_into(raw_messages::table)
                    .values(&db_message)
                    .on_conflict_do_nothing()
                    .execute(conn)?;
            }
            Ok(())
        })?;
        Ok(())
    }
}
