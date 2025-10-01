use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use tracing::info;
use uuid::Uuid;

use crate::receiver_statuses::{NewReceiverStatus, ReceiverStatus};
use crate::schema::receiver_statuses;

pub type PgPool = Pool<ConnectionManager<PgConnection>>;
pub type PgPooledConnection = PooledConnection<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct ReceiverStatusRepository {
    pool: PgPool,
}

impl ReceiverStatusRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn get_connection(&self) -> Result<PgPooledConnection> {
        self.pool
            .get()
            .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))
    }

    /// Insert a new receiver status
    pub async fn insert(&self, new_status: &NewReceiverStatus) -> Result<ReceiverStatus> {
        let mut conn = self.get_connection()?;
        let status = diesel::insert_into(receiver_statuses::table)
            .values(new_status)
            .returning(ReceiverStatus::as_returning())
            .get_result::<ReceiverStatus>(&mut conn)?;

        Ok(status)
    }

    /// Insert multiple receiver statuses in a batch
    pub async fn insert_batch(
        &self,
        statuses: &[NewReceiverStatus],
    ) -> Result<Vec<ReceiverStatus>> {
        if statuses.is_empty() {
            return Ok(Vec::new());
        }

        let mut conn = self.get_connection()?;
        let inserted_statuses = diesel::insert_into(receiver_statuses::table)
            .values(statuses)
            .returning(ReceiverStatus::as_returning())
            .load::<ReceiverStatus>(&mut conn)?;

        info!("Inserted {} receiver statuses", inserted_statuses.len());
        Ok(inserted_statuses)
    }

    /// Get receiver status by ID
    pub async fn get_by_id(&self, status_id: Uuid) -> Result<Option<ReceiverStatus>> {
        let mut conn = self.get_connection()?;
        let status = receiver_statuses::table
            .filter(receiver_statuses::id.eq(status_id))
            .first::<ReceiverStatus>(&mut conn)
            .optional()?;

        Ok(status)
    }

    /// Get recent statuses for a specific receiver
    pub async fn get_recent_for_receiver(
        &self,
        receiver_id: i32,
        limit: Option<i64>,
    ) -> Result<Vec<ReceiverStatus>> {
        let limit = limit.unwrap_or(100);
        let mut conn = self.get_connection()?;

        let statuses = receiver_statuses::table
            .filter(receiver_statuses::receiver_id.eq(receiver_id))
            .order(receiver_statuses::received_at.desc())
            .limit(limit)
            .load::<ReceiverStatus>(&mut conn)?;

        Ok(statuses)
    }

    /// Get statuses for a receiver within a time range
    pub async fn get_for_receiver_in_time_range(
        &self,
        receiver_id: i32,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<ReceiverStatus>> {
        let limit = limit.unwrap_or(1000);
        let mut conn = self.get_connection()?;

        let statuses = receiver_statuses::table
            .filter(receiver_statuses::receiver_id.eq(receiver_id))
            .filter(receiver_statuses::received_at.between(start_time, end_time))
            .order(receiver_statuses::received_at.desc())
            .limit(limit)
            .load::<ReceiverStatus>(&mut conn)?;

        Ok(statuses)
    }

    /// Get all recent statuses across all receivers
    pub async fn get_recent_all_receivers(
        &self,
        limit: Option<i64>,
    ) -> Result<Vec<ReceiverStatus>> {
        let limit = limit.unwrap_or(100);
        let mut conn = self.get_connection()?;

        let statuses = receiver_statuses::table
            .order(receiver_statuses::received_at.desc())
            .limit(limit)
            .load::<ReceiverStatus>(&mut conn)?;

        Ok(statuses)
    }

    /// Get the latest status for each receiver
    pub async fn get_latest_for_all_receivers(&self) -> Result<Vec<ReceiverStatus>> {
        let mut conn = self.get_connection()?;

        // Use DISTINCT ON to get the latest status for each receiver
        use crate::schema::receiver_statuses::dsl::*;

        let statuses = receiver_statuses
            .order((receiver_id, received_at.desc()))
            .distinct_on(receiver_id)
            .select(ReceiverStatus::as_select())
            .load::<ReceiverStatus>(&mut conn)?;

        Ok(statuses)
    }

    /// Delete old receiver statuses beyond a retention period
    pub async fn delete_old_statuses(&self, retention_days: i32) -> Result<u64> {
        let cutoff_time = Utc::now() - chrono::Duration::days(retention_days as i64);
        let mut conn = self.get_connection()?;

        let deleted_count = diesel::delete(
            receiver_statuses::table.filter(receiver_statuses::received_at.lt(cutoff_time)),
        )
        .execute(&mut conn)?;

        info!(
            "Deleted {} old receiver statuses older than {} days",
            deleted_count, retention_days
        );
        Ok(deleted_count as u64)
    }

    /// Get count of receiver statuses
    pub async fn count(&self) -> Result<i64> {
        let mut conn = self.get_connection()?;
        let count = receiver_statuses::table
            .count()
            .get_result::<i64>(&mut conn)?;

        Ok(count)
    }

    /// Get count of receiver statuses for a specific receiver
    pub async fn count_for_receiver(&self, receiver_id: i32) -> Result<i64> {
        let mut conn = self.get_connection()?;
        let count = receiver_statuses::table
            .filter(receiver_statuses::receiver_id.eq(receiver_id))
            .count()
            .get_result::<i64>(&mut conn)?;

        Ok(count)
    }

    /// Get average statistics for a receiver over a time period
    /// Note: Uses raw SQL because Diesel's tuple size limit (12) is exceeded by the 13 aggregate fields
    pub async fn get_receiver_averages(
        &self,
        receiver_id: i32,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Option<ReceiverAverages>> {
        let mut conn = self.get_connection()?;

        let sql = r#"
            SELECT
                AVG(cpu_load) as avg_cpu_load,
                AVG(ram_free) as avg_ram_free,
                AVG(ram_total) as avg_ram_total,
                AVG(voltage) as avg_voltage,
                AVG(amperage) as avg_amperage,
                AVG(cpu_temperature) as avg_cpu_temperature,
                AVG(visible_senders) as avg_visible_senders,
                AVG(latency) as avg_latency,
                AVG(senders) as avg_senders,
                AVG(noise) as avg_noise,
                AVG(senders_signal_quality) as avg_senders_signal_quality,
                AVG(lag) as avg_lag,
                COUNT(*) as sample_count
            FROM receiver_statuses
            WHERE receiver_id = $1 AND received_at BETWEEN $2 AND $3
        "#;

        let result = diesel::sql_query(sql)
            .bind::<diesel::sql_types::Integer, _>(receiver_id)
            .bind::<diesel::sql_types::Timestamptz, _>(start_time)
            .bind::<diesel::sql_types::Timestamptz, _>(end_time)
            .get_result::<ReceiverAverages>(&mut conn)
            .optional()?;

        Ok(result)
    }
}

/// Statistics summary for a receiver over a time period
#[derive(Debug, Clone, QueryableByName)]
pub struct ReceiverAverages {
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
    pub avg_cpu_load: Option<bigdecimal::BigDecimal>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
    pub avg_ram_free: Option<bigdecimal::BigDecimal>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
    pub avg_ram_total: Option<bigdecimal::BigDecimal>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
    pub avg_voltage: Option<bigdecimal::BigDecimal>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
    pub avg_amperage: Option<bigdecimal::BigDecimal>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
    pub avg_cpu_temperature: Option<bigdecimal::BigDecimal>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Double>)]
    pub avg_visible_senders: Option<f64>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
    pub avg_latency: Option<bigdecimal::BigDecimal>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Double>)]
    pub avg_senders: Option<f64>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
    pub avg_noise: Option<bigdecimal::BigDecimal>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
    pub avg_senders_signal_quality: Option<bigdecimal::BigDecimal>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Double>)]
    pub avg_lag: Option<f64>,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub sample_count: i64,
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
