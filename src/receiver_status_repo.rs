use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use uuid::Uuid;

use crate::receiver_statuses::{NewReceiverStatus, ReceiverStatus, ReceiverStatusWithRaw};
use crate::schema::{raw_messages, receiver_statuses};

pub type PgPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct ReceiverStatusRepository {
    pool: PgPool,
}

impl ReceiverStatusRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new receiver status
    pub async fn insert(&self, new_status: &NewReceiverStatus) -> Result<ReceiverStatus> {
        let pool = self.pool.clone();
        let new_status = new_status.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))?;
            let status = diesel::insert_into(receiver_statuses::table)
                .values(&new_status)
                .returning(ReceiverStatus::as_returning())
                .get_result::<ReceiverStatus>(&mut conn)?;
            Ok(status)
        })
        .await?
    }

    /// Get statuses for a receiver with pagination
    pub async fn get_statuses_by_receiver_paginated(
        &self,
        receiver_id: Uuid,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<ReceiverStatus>, i64)> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))?;

            // Get total count
            let total_count: i64 = receiver_statuses::table
                .filter(receiver_statuses::receiver_id.eq(receiver_id))
                .count()
                .get_result(&mut conn)?;

            // Get paginated results
            let offset = (page - 1) * per_page;
            let statuses = receiver_statuses::table
                .filter(receiver_statuses::receiver_id.eq(receiver_id))
                .order(receiver_statuses::received_at.desc())
                .limit(per_page)
                .offset(offset)
                .load::<ReceiverStatus>(&mut conn)?;

            Ok((statuses, total_count))
        })
        .await?
    }

    /// Get statuses for a receiver with pagination, including raw APRS message data
    pub async fn get_statuses_with_raw_by_receiver_paginated(
        &self,
        receiver_id: Uuid,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<ReceiverStatusWithRaw>, i64)> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))?;

            // Get total count
            let total_count: i64 = receiver_statuses::table
                .filter(receiver_statuses::receiver_id.eq(receiver_id))
                .count()
                .get_result(&mut conn)?;

            // Get paginated results with raw message data
            let offset = (page - 1) * per_page;
            let results = receiver_statuses::table
                .inner_join(
                    raw_messages::table.on(receiver_statuses::raw_message_id
                        .eq(raw_messages::id.nullable())
                        .and(receiver_statuses::received_at.eq(raw_messages::received_at))),
                )
                .filter(receiver_statuses::receiver_id.eq(receiver_id))
                .order(receiver_statuses::received_at.desc())
                .limit(per_page)
                .offset(offset)
                .select((ReceiverStatus::as_select(), raw_messages::raw_message))
                .load::<(ReceiverStatus, Vec<u8>)>(&mut conn)?;

            // Convert to ReceiverStatusWithRaw
            let statuses_with_raw = results
                .into_iter()
                .map(|(status, raw_data_bytes)| ReceiverStatusWithRaw {
                    status,
                    raw_data: String::from_utf8_lossy(&raw_data_bytes).to_string(),
                })
                .collect();

            Ok((statuses_with_raw, total_count))
        })
        .await?
    }

    /// Get count of receiver statuses for a specific receiver
    pub async fn count_for_receiver(&self, receiver_id: Uuid) -> Result<i64> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))?;
            let count = receiver_statuses::table
                .filter(receiver_statuses::receiver_id.eq(receiver_id))
                .count()
                .get_result::<i64>(&mut conn)?;

            Ok(count)
        })
        .await?
    }

    /// Get average time between status updates for a receiver
    /// Returns the average interval in seconds between consecutive status updates
    pub async fn get_average_update_interval(
        &self,
        receiver_id: Uuid,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<Option<f64>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))?;

            // Build query based on whether time range is specified
            let sql = if start_time.is_some() && end_time.is_some() {
                r#"
                    WITH intervals AS (
                        SELECT
                            EXTRACT(EPOCH FROM (received_at - LAG(received_at) OVER (ORDER BY received_at))) as interval_seconds
                        FROM receiver_statuses
                        WHERE receiver_id = $1
                            AND received_at BETWEEN $2 AND $3
                    )
                    SELECT AVG(interval_seconds)::double precision as avg_interval
                    FROM intervals
                    WHERE interval_seconds IS NOT NULL
                "#
            } else {
                r#"
                    WITH intervals AS (
                        SELECT
                            EXTRACT(EPOCH FROM (received_at - LAG(received_at) OVER (ORDER BY received_at))) as interval_seconds
                        FROM receiver_statuses
                        WHERE receiver_id = $1
                    )
                    SELECT AVG(interval_seconds)::double precision as avg_interval
                    FROM intervals
                    WHERE interval_seconds IS NOT NULL
                "#
            };

            let result = if let (Some(start), Some(end)) = (start_time, end_time) {
                diesel::sql_query(sql)
                    .bind::<diesel::sql_types::Uuid, _>(receiver_id)
                    .bind::<diesel::sql_types::Timestamptz, _>(start)
                    .bind::<diesel::sql_types::Timestamptz, _>(end)
                    .get_result::<AverageIntervalResult>(&mut conn)
                    .optional()?
            } else {
                diesel::sql_query(sql)
                    .bind::<diesel::sql_types::Uuid, _>(receiver_id)
                    .get_result::<AverageIntervalResult>(&mut conn)
                    .optional()?
            };

            Ok(result.and_then(|r| r.avg_interval))
        })
        .await?
    }
}

/// Result struct for average interval query
#[derive(Debug, Clone, QueryableByName)]
struct AverageIntervalResult {
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Double>)]
    avg_interval: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receiver_status_repository_creation() {
        // This is a simple test to ensure the repository can be created
        // Actual database tests would require a test database setup
        use diesel::PgConnection;
        use diesel::r2d2::ConnectionManager;
        use std::env;

        // Only run if DATABASE_URL is set (for CI environments)
        if let Ok(url) = env::var("DATABASE_URL") {
            let manager = ConnectionManager::<PgConnection>::new(url);
            let pool = Pool::builder().build(manager).unwrap();
            let _repo = ReceiverStatusRepository::new(pool);
        }
    }
}
