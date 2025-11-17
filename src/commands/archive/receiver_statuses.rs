use anyhow::Result;
use chrono::{NaiveDate, TimeZone, Utc};
use diesel::prelude::*;
use std::path::Path;
use tracing::info;

use soar::receiver_statuses::ReceiverStatus;
use soar::schema::receiver_statuses;

use super::archiver::{Archivable, PgPool, write_records_to_file};

impl Archivable for ReceiverStatus {
    fn table_name() -> &'static str {
        "receiver_statuses"
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

            let oldest_timestamp: Option<chrono::DateTime<Utc>> = receiver_statuses::table
                .select(receiver_statuses::received_at)
                .filter(receiver_statuses::received_at.lt(cutoff))
                .order(receiver_statuses::received_at.asc())
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
            let count = receiver_statuses::table
                .filter(receiver_statuses::received_at.ge(day_start))
                .filter(receiver_statuses::received_at.lt(day_end))
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
            let results: Vec<(NaiveDate, i64)> = receiver_statuses::table
                .filter(receiver_statuses::received_at.ge(start_datetime))
                .filter(receiver_statuses::received_at.lt(end_datetime))
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
            let statuses_iter = receiver_statuses::table
                .filter(receiver_statuses::received_at.ge(day_start))
                .filter(receiver_statuses::received_at.lt(day_end))
                .order(receiver_statuses::received_at.asc())
                .select(ReceiverStatus::as_select())
                .load_iter::<ReceiverStatus, diesel::pg::PgRowByRowLoadingMode>(&mut conn)?;

            write_records_to_file(statuses_iter, &file_path, Self::table_name())
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

    fn insert_batch(conn: &mut PgConnection, batch: &[Self]) -> Result<()> {
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
}
