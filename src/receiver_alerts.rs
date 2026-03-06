use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// API view of a receiver alert subscription
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ReceiverAlertView {
    pub id: Uuid,
    pub user_id: Uuid,
    pub receiver_id: Uuid,

    pub alert_on_down: bool,
    pub down_after_minutes: i32,

    pub alert_on_high_cpu: bool,
    #[ts(type = "number")]
    pub cpu_threshold: f64,

    pub alert_on_high_temperature: bool,
    #[ts(type = "number")]
    pub temperature_threshold_c: f64,

    pub send_email: bool,

    pub base_cooldown_minutes: i32,
    pub consecutive_alerts: i32,
    pub last_alerted_at: Option<DateTime<Utc>>,
    pub last_condition: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create or update a receiver alert subscription
#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct UpsertReceiverAlertRequest {
    #[serde(default = "default_true")]
    pub alert_on_down: bool,
    #[serde(default = "default_down_minutes")]
    pub down_after_minutes: i32,

    #[serde(default)]
    pub alert_on_high_cpu: bool,
    #[serde(default = "default_cpu_threshold")]
    pub cpu_threshold: f64,

    #[serde(default)]
    pub alert_on_high_temperature: bool,
    #[serde(default = "default_temperature_threshold")]
    pub temperature_threshold_c: f64,

    #[serde(default = "default_true")]
    pub send_email: bool,

    #[serde(default = "default_cooldown_minutes")]
    pub base_cooldown_minutes: i32,
}

fn default_true() -> bool {
    true
}
fn default_down_minutes() -> i32 {
    30
}
fn default_cpu_threshold() -> f64 {
    0.9
}
fn default_temperature_threshold() -> f64 {
    70.0
}
fn default_cooldown_minutes() -> i32 {
    30
}

/// Internal model for converting BigDecimal fields to f64 for the API view
pub fn big_decimal_to_f64(bd: &BigDecimal) -> f64 {
    use std::str::FromStr;
    f64::from_str(&bd.to_string()).unwrap_or(0.0)
}
