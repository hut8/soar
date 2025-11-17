use anyhow::Result;
use chrono::{NaiveDate, Utc};
use diesel::prelude::*;
use std::path::Path;
use tracing::info;

use soar::flights::FlightModel;
use soar::schema::flights;

use super::archiver::{Archivable, PgPool, write_records_to_file};

impl Archivable for FlightModel {
    fn table_name() -> &'static str {
        "flights"
    }

    async fn get_oldest_date(pool: &PgPool) -> Result<Option<NaiveDate>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let oldest_timestamp: Option<chrono::DateTime<Utc>> = flights::table
                .select(diesel::dsl::min(flights::last_fix_at))
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
            let count = flights::table
                .filter(flights::last_fix_at.ge(day_start))
                .filter(flights::last_fix_at.lt(day_end))
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
            let results: Vec<(NaiveDate, i64)> = flights::table
                .filter(flights::last_fix_at.ge(start_datetime))
                .filter(flights::last_fix_at.lt(end_datetime))
                .select((sql::<Date>("DATE(last_fix_at)"), sql::<BigInt>("COUNT(*)")))
                .group_by(sql::<Date>("DATE(last_fix_at)"))
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
            let flights_iter = flights::table
                .filter(flights::last_fix_at.ge(day_start))
                .filter(flights::last_fix_at.lt(day_end))
                .order(flights::last_fix_at.asc())
                .select(FlightModel::as_select())
                .load_iter::<FlightModel, diesel::pg::PgRowByRowLoadingMode>(&mut conn)?;

            write_records_to_file(flights_iter, &file_path, Self::table_name())
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
                    flights::table
                        .filter(flights::last_fix_at.ge(day_start))
                        .filter(flights::last_fix_at.lt(day_end)),
                )
                .execute(conn)?;
                info!(
                    "Deleted {} flights for day starting {}",
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
            for flight in batch {
                diesel::insert_into(flights::table)
                    .values(flight)
                    .on_conflict_do_nothing()
                    .execute(conn)?;
            }
            Ok(())
        })?;
        Ok(())
    }
}
