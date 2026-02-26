use anyhow::{Context, Result};
use chrono::{NaiveDate, TimeZone, Utc};
use diesel::prelude::*;
use std::path::Path;
use tracing::info;

use soar::fixes::Fix;
use soar::schema::fixes;

use super::archiver::{Archivable, PgPool, write_records_to_file};

impl Archivable for Fix {
    fn table_name() -> &'static str {
        "fixes"
    }

    fn is_hypertable() -> bool {
        true // fixes is a TimescaleDB hypertable partitioned by received_at
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

            let oldest_timestamp: Option<chrono::DateTime<Utc>> = fixes::table
                .select(fixes::received_at)
                .filter(fixes::received_at.lt(cutoff))
                .order(fixes::received_at.asc())
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
            let count = fixes::table
                .filter(fixes::received_at.ge(day_start))
                .filter(fixes::received_at.lt(day_end))
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
            let results: Vec<(NaiveDate, i64)> = fixes::table
                .filter(fixes::received_at.ge(start_datetime))
                .filter(fixes::received_at.lt(end_datetime))
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

            let fixes_iter = fixes::table
                .filter(fixes::received_at.ge(day_start))
                .filter(fixes::received_at.lt(day_end))
                .order(fixes::received_at.asc())
                .select(Fix::as_select())
                .load_iter::<Fix, diesel::pg::PgRowByRowLoadingMode>(&mut conn)?;

            write_records_to_file(fixes_iter, &file_path, Self::table_name())
        })
        .await?
    }

    async fn delete_for_day(
        pool: &PgPool,
        day_start: chrono::DateTime<Utc>,
        day_end: chrono::DateTime<Utc>,
    ) -> Result<()> {
        // Use TimescaleDB drop_chunks to remove the chunk(s) covering this day.
        // drop_chunks handles compressed chunks automatically (decompresses then drops).
        // We use older_than = day_end and newer_than = day_start to target exactly one day.
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            diesel::sql_query("SELECT drop_chunks('fixes', older_than => $1, newer_than => $2)")
                .bind::<diesel::sql_types::Timestamptz, _>(day_end)
                .bind::<diesel::sql_types::Timestamptz, _>(day_start)
                .execute(&mut conn)
                .context("Failed to drop chunks for fixes")?;

            info!(
                "Dropped TimescaleDB chunks for fixes between {} and {}",
                day_start, day_end
            );

            Ok(())
        })
        .await?
    }

    fn insert_batch(conn: &mut PgConnection, batch: &[Self]) -> Result<()> {
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
}
