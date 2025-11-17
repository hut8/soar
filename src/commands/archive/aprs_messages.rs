use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use hex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;
use uuid::Uuid;

use soar::raw_messages_repo::AprsMessage;
use soar::schema::raw_messages;

use super::archiver::{Archivable, PgPool, write_records_to_file};

/// CSV-serializable version of AprsMessage with hex-encoded hash
/// This is used for archival to avoid CSV serialization issues with Vec<u8>
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AprsMessageCsv {
    id: Uuid,
    raw_message: String,
    received_at: DateTime<Utc>,
    receiver_id: Uuid,
    unparsed: Option<String>,
    raw_message_hash: String, // Hex-encoded instead of Vec<u8>
}

impl From<AprsMessage> for AprsMessageCsv {
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

impl From<AprsMessageCsv> for AprsMessage {
    fn from(csv: AprsMessageCsv) -> Self {
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

impl Archivable for AprsMessageCsv {
    fn table_name() -> &'static str {
        "aprs_messages"
    }

    async fn get_oldest_date(pool: &PgPool) -> Result<Option<NaiveDate>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            // Instead of MIN() which scans all partitions, use ORDER BY + LIMIT
            // which allows the query planner to scan only the oldest partition
            let oldest_timestamp: Option<chrono::DateTime<Utc>> = raw_messages::table
                .select(raw_messages::received_at)
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

            // Convert AprsMessage to AprsMessageCsv for CSV serialization
            let csv_messages_iter = messages_iter.map(|result| result.map(AprsMessageCsv::from));

            write_records_to_file(csv_messages_iter, &file_path, Self::table_name())
        })
        .await?
    }

    async fn delete_for_day(
        pool: &PgPool,
        day_start: chrono::DateTime<Utc>,
        day_end: chrono::DateTime<Utc>,
    ) -> Result<()> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            conn.transaction::<_, anyhow::Error, _>(|conn| {
                let deleted_count = diesel::delete(
                    raw_messages::table
                        .filter(raw_messages::received_at.ge(day_start))
                        .filter(raw_messages::received_at.lt(day_end)),
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

    fn insert_batch(conn: &mut PgConnection, batch: &[Self]) -> Result<()> {
        conn.transaction::<_, anyhow::Error, _>(|conn| {
            for csv_message in batch {
                // Convert CSV format back to database format
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
