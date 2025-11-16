use anyhow::Result;
use chrono::{NaiveDate, Utc};
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

    async fn get_oldest_date(pool: &PgPool) -> Result<Option<NaiveDate>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let oldest_timestamp: Option<chrono::DateTime<Utc>> = fixes::table
                .select(diesel::dsl::min(fixes::received_at))
                .first::<Option<chrono::DateTime<Utc>>>(&mut conn)?;
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
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            conn.transaction::<_, anyhow::Error, _>(|conn| {
                let deleted_count = diesel::delete(
                    fixes::table
                        .filter(fixes::received_at.ge(day_start))
                        .filter(fixes::received_at.lt(day_end)),
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
