use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

use crate::receiver_statuses::ReceiverStatusWithRaw;

/// API view of a receiver status with raw APRS message data.
/// Flattened from ReceiverStatusWithRaw for ts-rs compatibility
/// (ts-rs does not support #[serde(flatten)]).
/// BigDecimal fields serialize as strings in JSON.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ReceiverStatusView {
    pub id: Uuid,
    pub received_at: chrono::DateTime<chrono::Utc>,
    pub version: Option<String>,
    pub platform: Option<String>,
    #[ts(type = "string | null")]
    pub cpu_load: Option<String>,
    #[ts(type = "string | null")]
    pub ram_free: Option<String>,
    #[ts(type = "string | null")]
    pub ram_total: Option<String>,
    #[ts(type = "string | null")]
    pub ntp_offset: Option<String>,
    #[ts(type = "string | null")]
    pub ntp_correction: Option<String>,
    #[ts(type = "string | null")]
    pub voltage: Option<String>,
    #[ts(type = "string | null")]
    pub amperage: Option<String>,
    #[ts(type = "string | null")]
    pub cpu_temperature: Option<String>,
    pub visible_senders: Option<i16>,
    #[ts(type = "string | null")]
    pub latency: Option<String>,
    pub senders: Option<i16>,
    pub rf_correction_manual: Option<i16>,
    #[ts(type = "string | null")]
    pub rf_correction_automatic: Option<String>,
    #[ts(type = "string | null")]
    pub noise: Option<String>,
    #[ts(type = "string | null")]
    pub senders_signal_quality: Option<String>,
    pub senders_messages: Option<i32>,
    #[ts(type = "string | null")]
    pub good_senders_signal_quality: Option<String>,
    pub good_senders: Option<i16>,
    pub good_and_bad_senders: Option<i16>,
    pub geoid_offset: Option<i16>,
    pub name: Option<String>,
    #[ts(type = "string | null")]
    pub demodulation_snr_db: Option<String>,
    pub ognr_pilotaware_version: Option<String>,
    pub unparsed_data: Option<String>,
    pub lag: Option<i32>,
    pub receiver_id: Uuid,
    pub raw_message_id: Option<Uuid>,
    pub raw_data: String,
}

impl From<ReceiverStatusWithRaw> for ReceiverStatusView {
    fn from(r: ReceiverStatusWithRaw) -> Self {
        let s = r.status;
        Self {
            id: s.id,
            received_at: s.received_at,
            version: s.version,
            platform: s.platform,
            cpu_load: s.cpu_load.map(|v| v.to_string()),
            ram_free: s.ram_free.map(|v| v.to_string()),
            ram_total: s.ram_total.map(|v| v.to_string()),
            ntp_offset: s.ntp_offset.map(|v| v.to_string()),
            ntp_correction: s.ntp_correction.map(|v| v.to_string()),
            voltage: s.voltage.map(|v| v.to_string()),
            amperage: s.amperage.map(|v| v.to_string()),
            cpu_temperature: s.cpu_temperature.map(|v| v.to_string()),
            visible_senders: s.visible_senders,
            latency: s.latency.map(|v| v.to_string()),
            senders: s.senders,
            rf_correction_manual: s.rf_correction_manual,
            rf_correction_automatic: s.rf_correction_automatic.map(|v| v.to_string()),
            noise: s.noise.map(|v| v.to_string()),
            senders_signal_quality: s.senders_signal_quality.map(|v| v.to_string()),
            senders_messages: s.senders_messages,
            good_senders_signal_quality: s.good_senders_signal_quality.map(|v| v.to_string()),
            good_senders: s.good_senders,
            good_and_bad_senders: s.good_and_bad_senders,
            geoid_offset: s.geoid_offset,
            name: s.name,
            demodulation_snr_db: s.demodulation_snr_db.map(|v| v.to_string()),
            ognr_pilotaware_version: s.ognr_pilotaware_version,
            unparsed_data: s.unparsed_data,
            lag: s.lag,
            receiver_id: s.receiver_id,
            raw_message_id: s.raw_message_id,
            raw_data: r.raw_data,
        }
    }
}
