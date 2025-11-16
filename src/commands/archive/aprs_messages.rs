use anyhow::Result;
use chrono::{NaiveDate, Utc};
use diesel::prelude::*;
use std::path::Path;
use tracing::info;

use soar::aprs_messages_repo::AprsMessage;
use soar::schema::aprs_messages;

use super::archiver::{Archivable, PgPool, write_records_to_file};

impl Archivable for AprsMessage {
    fn table_name() -> &'static str {
        "aprs_messages"
    }

    async fn get_oldest_date(pool: &PgPool) -> Result<Option<NaiveDate>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            // Instead of MIN() which scans all partitions, use ORDER BY + LIMIT
            // which allows the query planner to scan only the oldest partition
            let oldest_timestamp: Option<chrono::DateTime<Utc>> = aprs_messages::table
                .select(aprs_messages::received_at)
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
            let messages_iter = aprs_messages::table
                .filter(aprs_messages::received_at.ge(day_start))
                .filter(aprs_messages::received_at.lt(day_end))
                .order(aprs_messages::received_at.asc())
                .select(AprsMessage::as_select())
                .load_iter::<AprsMessage, diesel::pg::PgRowByRowLoadingMode>(&mut conn)?;

            write_records_to_file(messages_iter, &file_path, Self::table_name())
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
            for message in batch {
                diesel::insert_into(aprs_messages::table)
                    .values(message)
                    .on_conflict_do_nothing()
                    .execute(conn)?;
            }
            Ok(())
        })?;
        Ok(())
    }
}
