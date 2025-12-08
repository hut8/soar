use anyhow::Result;
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
        _day_end: chrono::DateTime<Utc>,
    ) -> Result<()> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Calculate partition name from day_start
            // Partitions are named like fixes_p20251114 for 2025-11-14
            let partition_date = day_start.format("%Y%m%d");
            let partition_name = format!("fixes_p{}", partition_date);

            // Drop the partition directly - much faster than DELETE
            // This works because:
            // 1. Each partition contains exactly one day's data
            // 2. Foreign keys are enforced at the partition level
            // 3. We've already archived the data to disk
            conn.transaction::<_, anyhow::Error, _>(|conn| {
                // Detach the partition first (optional but makes it invisible to queries immediately)
                let detach_sql = format!("ALTER TABLE fixes DETACH PARTITION {}", partition_name);
                diesel::sql_query(&detach_sql).execute(conn)?;
                info!("Detached partition {} from fixes table", partition_name);

                // Drop the partition table
                let drop_sql = format!("DROP TABLE IF EXISTS {}", partition_name);
                diesel::sql_query(&drop_sql).execute(conn)?;
                info!(
                    "Dropped partition {} for day starting {}",
                    partition_name, day_start
                );

                Ok(())
            })?;
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
