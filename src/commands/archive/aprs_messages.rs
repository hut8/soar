use anyhow::Result;
use chrono::{NaiveDate, TimeZone, Utc};
use diesel::prelude::*;
use hex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;
use uuid::Uuid;

use soar::aprs_messages_repo::AprsMessage;
use soar::schema::aprs_messages;

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
            raw_message: msg.raw_message,
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
            raw_message: csv.raw_message,
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

            let oldest_timestamp: Option<chrono::DateTime<Utc>> = aprs_messages::table
                .select(aprs_messages::received_at)
                .filter(aprs_messages::received_at.lt(cutoff))
                .order(aprs_messages::received_at.asc())
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
            let count = aprs_messages::table
                .filter(aprs_messages::received_at.ge(day_start))
                .filter(aprs_messages::received_at.lt(day_end))
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
            let results: Vec<(NaiveDate, i64)> = aprs_messages::table
                .filter(aprs_messages::received_at.ge(start_datetime))
                .filter(aprs_messages::received_at.lt(end_datetime))
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
            let messages_iter = aprs_messages::table
                .filter(aprs_messages::received_at.ge(day_start))
                .filter(aprs_messages::received_at.lt(day_end))
                .order(aprs_messages::received_at.asc())
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

    fn insert_batch(conn: &mut PgConnection, batch: &[Self]) -> Result<()> {
        conn.transaction::<_, anyhow::Error, _>(|conn| {
            for csv_message in batch {
                // Convert CSV format back to database format
                let db_message: AprsMessage = csv_message.clone().into();
                diesel::insert_into(aprs_messages::table)
                    .values(&db_message)
                    .on_conflict_do_nothing()
                    .execute(conn)?;
            }
            Ok(())
        })?;
        Ok(())
    }
}
