use anyhow::Result;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use std::str::FromStr;
use uuid::Uuid;

use crate::receiver_alerts::{ReceiverAlertView, UpsertReceiverAlertRequest, big_decimal_to_f64};
use crate::schema::receiver_alerts;

type PgPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = receiver_alerts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ReceiverAlertRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub receiver_id: Uuid,
    pub alert_on_down: bool,
    pub down_after_minutes: i32,
    pub alert_on_high_cpu: bool,
    pub cpu_threshold: BigDecimal,
    pub alert_on_high_temperature: bool,
    pub temperature_threshold_c: BigDecimal,
    pub send_email: bool,
    pub base_cooldown_minutes: i32,
    pub consecutive_alerts: i32,
    pub last_alerted_at: Option<DateTime<Utc>>,
    pub last_condition: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ReceiverAlertRecord> for ReceiverAlertView {
    fn from(r: ReceiverAlertRecord) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            receiver_id: r.receiver_id,
            alert_on_down: r.alert_on_down,
            down_after_minutes: r.down_after_minutes,
            alert_on_high_cpu: r.alert_on_high_cpu,
            cpu_threshold: big_decimal_to_f64(&r.cpu_threshold),
            alert_on_high_temperature: r.alert_on_high_temperature,
            temperature_threshold_c: big_decimal_to_f64(&r.temperature_threshold_c),
            send_email: r.send_email,
            base_cooldown_minutes: r.base_cooldown_minutes,
            consecutive_alerts: r.consecutive_alerts,
            last_alerted_at: r.last_alerted_at,
            last_condition: r.last_condition,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

pub struct ReceiverAlertsRepository {
    pool: PgPool,
}

impl ReceiverAlertsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get all alerts for a user
    pub async fn get_by_user(&self, user_id: Uuid) -> Result<Vec<ReceiverAlertView>> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<ReceiverAlertView>> {
            let mut conn = pool.get()?;
            let records = receiver_alerts::table
                .filter(receiver_alerts::user_id.eq(user_id))
                .order(receiver_alerts::created_at.desc())
                .load::<ReceiverAlertRecord>(&mut conn)?;
            Ok(records.into_iter().map(|r| r.into()).collect())
        })
        .await?
    }

    /// Get alert for a specific user+receiver pair
    pub async fn get_by_user_and_receiver(
        &self,
        user_id: Uuid,
        receiver_id: Uuid,
    ) -> Result<Option<ReceiverAlertView>> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<Option<ReceiverAlertView>> {
            let mut conn = pool.get()?;
            let record = receiver_alerts::table
                .filter(receiver_alerts::user_id.eq(user_id))
                .filter(receiver_alerts::receiver_id.eq(receiver_id))
                .first::<ReceiverAlertRecord>(&mut conn)
                .optional()?;
            Ok(record.map(|r| r.into()))
        })
        .await?
    }

    /// Create or update an alert subscription (upsert on user_id + receiver_id)
    pub async fn upsert(
        &self,
        user_id: Uuid,
        receiver_id: Uuid,
        req: &UpsertReceiverAlertRequest,
    ) -> Result<ReceiverAlertView> {
        let pool = self.pool.clone();
        let cpu_threshold =
            BigDecimal::from_str(&req.cpu_threshold.to_string()).unwrap_or_default();
        let temp_threshold =
            BigDecimal::from_str(&req.temperature_threshold_c.to_string()).unwrap_or_default();
        let alert_on_down = req.alert_on_down;
        let down_after_minutes = req.down_after_minutes;
        let alert_on_high_cpu = req.alert_on_high_cpu;
        let alert_on_high_temperature = req.alert_on_high_temperature;
        let send_email = req.send_email;
        let base_cooldown_minutes = req.base_cooldown_minutes;

        tokio::task::spawn_blocking(move || -> Result<ReceiverAlertView> {
            let mut conn = pool.get()?;
            let record = diesel::insert_into(receiver_alerts::table)
                .values((
                    receiver_alerts::user_id.eq(user_id),
                    receiver_alerts::receiver_id.eq(receiver_id),
                    receiver_alerts::alert_on_down.eq(alert_on_down),
                    receiver_alerts::down_after_minutes.eq(down_after_minutes),
                    receiver_alerts::alert_on_high_cpu.eq(alert_on_high_cpu),
                    receiver_alerts::cpu_threshold.eq(&cpu_threshold),
                    receiver_alerts::alert_on_high_temperature.eq(alert_on_high_temperature),
                    receiver_alerts::temperature_threshold_c.eq(&temp_threshold),
                    receiver_alerts::send_email.eq(send_email),
                    receiver_alerts::base_cooldown_minutes.eq(base_cooldown_minutes),
                ))
                .on_conflict((receiver_alerts::user_id, receiver_alerts::receiver_id))
                .do_update()
                .set((
                    receiver_alerts::alert_on_down.eq(alert_on_down),
                    receiver_alerts::down_after_minutes.eq(down_after_minutes),
                    receiver_alerts::alert_on_high_cpu.eq(alert_on_high_cpu),
                    receiver_alerts::cpu_threshold.eq(&cpu_threshold),
                    receiver_alerts::alert_on_high_temperature.eq(alert_on_high_temperature),
                    receiver_alerts::temperature_threshold_c.eq(&temp_threshold),
                    receiver_alerts::send_email.eq(send_email),
                    receiver_alerts::base_cooldown_minutes.eq(base_cooldown_minutes),
                    receiver_alerts::updated_at.eq(diesel::dsl::now),
                ))
                .get_result::<ReceiverAlertRecord>(&mut conn)?;
            Ok(record.into())
        })
        .await?
    }

    /// Delete an alert subscription
    pub async fn delete(&self, user_id: Uuid, receiver_id: Uuid) -> Result<bool> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;
            let rows = diesel::delete(receiver_alerts::table)
                .filter(receiver_alerts::user_id.eq(user_id))
                .filter(receiver_alerts::receiver_id.eq(receiver_id))
                .execute(&mut conn)?;
            Ok(rows > 0)
        })
        .await?
    }

    /// Get all active alert subscriptions that want email notifications.
    /// Used by the background checker.
    pub async fn get_all_active_email_alerts(&self) -> Result<Vec<ReceiverAlertRecord>> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<ReceiverAlertRecord>> {
            let mut conn = pool.get()?;
            let records = receiver_alerts::table
                .filter(receiver_alerts::send_email.eq(true))
                .load::<ReceiverAlertRecord>(&mut conn)?;
            Ok(records)
        })
        .await?
    }

    /// Update the alert state after sending a notification (increment consecutive_alerts, set last_alerted_at)
    pub async fn record_alert_sent(&self, alert_id: Uuid, condition: &str) -> Result<()> {
        let pool = self.pool.clone();
        let condition = condition.to_string();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut conn = pool.get()?;
            diesel::update(receiver_alerts::table.find(alert_id))
                .set((
                    receiver_alerts::consecutive_alerts.eq(receiver_alerts::consecutive_alerts + 1),
                    receiver_alerts::last_alerted_at.eq(diesel::dsl::now),
                    receiver_alerts::last_condition.eq(&condition),
                    receiver_alerts::updated_at.eq(diesel::dsl::now),
                ))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }

    /// Reset alert state when condition clears (receiver comes back online, metrics normalize)
    pub async fn reset_alert_state(&self, alert_id: Uuid) -> Result<()> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut conn = pool.get()?;
            diesel::update(receiver_alerts::table.find(alert_id))
                .set((
                    receiver_alerts::consecutive_alerts.eq(0),
                    receiver_alerts::last_condition.eq(None::<String>),
                    receiver_alerts::updated_at.eq(diesel::dsl::now),
                ))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }
}
