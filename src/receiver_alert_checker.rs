use anyhow::Result;
use chrono::Utc;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use std::collections::HashMap;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::email::EmailService;
use crate::receiver_alerts::big_decimal_to_f64;
use crate::receiver_alerts_repo::{ReceiverAlertRecord, ReceiverAlertsRepository};
use crate::receiver_repo::ReceiverRepository;
use crate::receiver_status_repo::ReceiverStatusRepository;
use crate::receivers::ReceiverModel;
use crate::users::User;
use crate::users_repo::UsersRepository;

type PgPool = Pool<ConnectionManager<PgConnection>>;

/// Detected alert condition for a receiver
#[derive(Debug)]
enum AlertCondition {
    Down { minutes_since_last_packet: i64 },
    HighCpu { cpu_load: f64 },
    HighTemperature { temperature_c: f64 },
}

impl AlertCondition {
    fn condition_key(&self) -> &str {
        match self {
            AlertCondition::Down { .. } => "down",
            AlertCondition::HighCpu { .. } => "high_cpu",
            AlertCondition::HighTemperature { .. } => "high_temperature",
        }
    }

    fn description(&self) -> String {
        match self {
            AlertCondition::Down {
                minutes_since_last_packet,
            } => {
                format!("No data received for {} minutes", minutes_since_last_packet)
            }
            AlertCondition::HighCpu { cpu_load } => {
                format!("CPU load at {:.0}%", cpu_load * 100.0)
            }
            AlertCondition::HighTemperature { temperature_c } => {
                format!("Temperature at {:.1}\u{00B0}C", temperature_c)
            }
        }
    }
}

/// Compute the effective cooldown in minutes using exponential backoff.
/// cooldown = base * 2^(consecutive_alerts - 1), capped at 24 hours.
fn effective_cooldown_minutes(base_cooldown: i32, consecutive_alerts: i32) -> i64 {
    if consecutive_alerts <= 0 {
        return 0; // No cooldown before first alert
    }
    let exponent = (consecutive_alerts - 1).min(10) as u32; // Cap exponent to avoid overflow
    let cooldown = (base_cooldown as i64).saturating_mul(2_i64.saturating_pow(exponent));
    cooldown.min(24 * 60) // Cap at 24 hours
}

/// Start the receiver alert checker as a background task.
/// Runs every `interval_secs` seconds.
pub fn start_receiver_alert_checker(pool: PgPool, interval_secs: u64) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        // Skip the first immediate tick
        interval.tick().await;

        info!(
            "Receiver alert checker started (interval: {}s)",
            interval_secs
        );

        loop {
            interval.tick().await;
            if let Err(e) = run_alert_check(&pool).await {
                error!(error = %e, "Receiver alert check failed");
            }
        }
    });
}

async fn run_alert_check(pool: &PgPool) -> Result<()> {
    let alerts_repo = ReceiverAlertsRepository::new(pool.clone());
    let receiver_repo = ReceiverRepository::new(pool.clone());
    let status_repo = ReceiverStatusRepository::new(pool.clone());
    let users_repo = UsersRepository::new(pool.clone());

    // Get all active email alert subscriptions
    let alerts = alerts_repo.get_all_active_email_alerts().await?;
    if alerts.is_empty() {
        return Ok(());
    }

    metrics::gauge!("receiver_alerts.active_subscriptions").set(alerts.len() as f64);

    // Group alerts by receiver_id so we fetch each receiver's data once
    let mut alerts_by_receiver: HashMap<Uuid, Vec<ReceiverAlertRecord>> = HashMap::new();
    for alert in alerts {
        alerts_by_receiver
            .entry(alert.receiver_id)
            .or_default()
            .push(alert);
    }

    // Pre-fetch all users we might need
    let user_ids: Vec<Uuid> = alerts_by_receiver
        .values()
        .flat_map(|alerts| alerts.iter().map(|a| a.user_id))
        .collect();
    let users = fetch_users(&users_repo, &user_ids).await;

    // Try to create email service (may fail if SMTP not configured)
    let email_service = match EmailService::new() {
        Ok(svc) => svc,
        Err(e) => {
            warn!(
                "Email service not available for receiver alerts: {}. Skipping check.",
                e
            );
            return Ok(());
        }
    };

    let now = Utc::now();
    let mut alerts_sent = 0u64;
    let mut alerts_cleared = 0u64;

    for (receiver_id, receiver_alerts) in &alerts_by_receiver {
        // Fetch receiver info
        let receiver = match receiver_repo.get_receiver_by_id(*receiver_id).await {
            Ok(Some(r)) => r,
            Ok(None) => {
                warn!(receiver_id = %receiver_id, "Receiver not found for alert subscription");
                continue;
            }
            Err(e) => {
                error!(error = %e, receiver_id = %receiver_id, "Failed to fetch receiver");
                continue;
            }
        };

        // Fetch the latest status for CPU/temperature checks
        let latest_status = status_repo
            .get_latest_status_for_receiver(*receiver_id)
            .await
            .ok()
            .flatten();

        for alert in receiver_alerts {
            let conditions = evaluate_conditions(alert, &receiver, &latest_status, now);

            if conditions.is_empty() {
                // Condition has cleared — reset backoff if it was previously active
                if alert.consecutive_alerts > 0 {
                    if let Err(e) = alerts_repo.reset_alert_state(alert.id).await {
                        error!(error = %e, alert_id = %alert.id, "Failed to reset alert state");
                    } else {
                        alerts_cleared += 1;
                    }
                }
                continue;
            }

            // Check cooldown with exponential backoff
            if let Some(last_alerted) = alert.last_alerted_at {
                let cooldown_mins = effective_cooldown_minutes(
                    alert.base_cooldown_minutes,
                    alert.consecutive_alerts,
                );
                let next_alert_at = last_alerted + chrono::Duration::minutes(cooldown_mins);
                if now < next_alert_at {
                    continue; // Still in cooldown
                }
            }

            // Send alert email
            let user = match users.get(&alert.user_id) {
                Some(u) => u,
                None => continue,
            };

            let user_email = match &user.email {
                Some(e) => e.clone(),
                None => continue,
            };

            let user_name = format!("{} {}", user.first_name, user.last_name);

            // Send one alert per subscription per check cycle (highest priority condition first)
            if let Some(condition) = conditions.first() {
                match email_service
                    .send_receiver_alert_email(
                        &user_email,
                        &user_name,
                        &receiver.callsign,
                        condition.description(),
                        condition.condition_key(),
                        alert.consecutive_alerts + 1,
                        receiver.id,
                    )
                    .await
                {
                    Ok(_) => {
                        info!(
                            receiver = %receiver.callsign,
                            user = %user_email,
                            condition = %condition.condition_key(),
                            "Receiver alert email sent"
                        );
                        if let Err(e) = alerts_repo
                            .record_alert_sent(alert.id, condition.condition_key())
                            .await
                        {
                            error!(error = %e, "Failed to record alert sent");
                        }
                        alerts_sent += 1;
                        metrics::counter!("receiver_alerts.emails_sent_total").increment(1);
                    }
                    Err(e) => {
                        error!(
                            error = %e,
                            receiver = %receiver.callsign,
                            condition = %condition.condition_key(),
                            "Failed to send receiver alert email"
                        );
                        metrics::counter!("receiver_alerts.emails_failed_total").increment(1);
                    }
                }
            }
        }
    }

    if alerts_sent > 0 || alerts_cleared > 0 {
        info!(
            alerts_sent = alerts_sent,
            alerts_cleared = alerts_cleared,
            "Receiver alert check complete"
        );
    }

    Ok(())
}

fn evaluate_conditions(
    alert: &ReceiverAlertRecord,
    receiver: &ReceiverModel,
    latest_status: &Option<crate::receiver_statuses::ReceiverStatus>,
    now: chrono::DateTime<Utc>,
) -> Vec<AlertCondition> {
    let mut conditions = Vec::new();

    // Check for receiver down
    if alert.alert_on_down {
        let threshold = chrono::Duration::minutes(alert.down_after_minutes as i64);
        let is_down = match receiver.latest_packet_at {
            Some(last_packet) => (now - last_packet) > threshold,
            None => true, // Never received a packet — consider it down
        };
        if is_down {
            let minutes_since = receiver
                .latest_packet_at
                .map(|lp| (now - lp).num_minutes())
                .unwrap_or(-1);
            conditions.push(AlertCondition::Down {
                minutes_since_last_packet: minutes_since,
            });
        }
    }

    // Check CPU and temperature from latest status
    if let Some(status) = latest_status {
        if alert.alert_on_high_cpu
            && let Some(cpu_load) = &status.cpu_load
        {
            let threshold = big_decimal_to_f64(&alert.cpu_threshold);
            let current = big_decimal_to_f64(cpu_load);
            if current > threshold {
                conditions.push(AlertCondition::HighCpu { cpu_load: current });
            }
        }

        if alert.alert_on_high_temperature
            && let Some(temp) = &status.cpu_temperature
        {
            let threshold = big_decimal_to_f64(&alert.temperature_threshold_c);
            let current = big_decimal_to_f64(temp);
            if current > threshold {
                conditions.push(AlertCondition::HighTemperature {
                    temperature_c: current,
                });
            }
        }
    }

    conditions
}

async fn fetch_users(users_repo: &UsersRepository, user_ids: &[Uuid]) -> HashMap<Uuid, User> {
    let mut map = HashMap::new();
    for user_id in user_ids {
        match users_repo.get_by_id(*user_id).await {
            Ok(Some(user)) => {
                map.insert(*user_id, user);
            }
            Ok(None) => {
                warn!(user_id = %user_id, "User not found for receiver alert");
            }
            Err(e) => {
                error!(error = %e, user_id = %user_id, "Failed to fetch user");
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effective_cooldown_minutes() {
        // Before first alert, no cooldown
        assert_eq!(effective_cooldown_minutes(30, 0), 0);

        // After 1st alert: base * 2^0 = 30
        assert_eq!(effective_cooldown_minutes(30, 1), 30);

        // After 2nd alert: base * 2^1 = 60
        assert_eq!(effective_cooldown_minutes(30, 2), 60);

        // After 3rd alert: base * 2^2 = 120
        assert_eq!(effective_cooldown_minutes(30, 3), 120);

        // After 4th alert: base * 2^3 = 240
        assert_eq!(effective_cooldown_minutes(30, 4), 240);

        // After 5th alert: base * 2^4 = 480
        assert_eq!(effective_cooldown_minutes(30, 5), 480);

        // Should cap at 24 hours (1440 minutes)
        assert_eq!(effective_cooldown_minutes(30, 10), 1440);
        assert_eq!(effective_cooldown_minutes(30, 20), 1440);
    }

    #[test]
    fn test_effective_cooldown_with_different_base() {
        assert_eq!(effective_cooldown_minutes(15, 1), 15);
        assert_eq!(effective_cooldown_minutes(15, 2), 30);
        assert_eq!(effective_cooldown_minutes(60, 1), 60);
        assert_eq!(effective_cooldown_minutes(60, 2), 120);
    }

    #[test]
    fn test_condition_keys() {
        assert_eq!(
            AlertCondition::Down {
                minutes_since_last_packet: 30
            }
            .condition_key(),
            "down"
        );
        assert_eq!(
            AlertCondition::HighCpu { cpu_load: 0.95 }.condition_key(),
            "high_cpu"
        );
        assert_eq!(
            AlertCondition::HighTemperature {
                temperature_c: 75.0
            }
            .condition_key(),
            "high_temperature"
        );
    }
}
