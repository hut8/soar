use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use ogn_parser::StatusComment;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

/// Database model for receiver status information
#[derive(
    Debug,
    Clone,
    Queryable,
    Selectable,
    Insertable,
    AsChangeset,
    QueryableByName,
    Serialize,
    Deserialize,
)]
#[diesel(table_name = crate::schema::receiver_statuses)]
pub struct ReceiverStatus {
    pub id: Uuid,
    pub receiver_id: i32,
    pub received_at: DateTime<Utc>,

    // Status fields from StatusComment
    pub version: Option<String>,
    pub platform: Option<String>,
    pub cpu_load: Option<BigDecimal>,
    pub ram_free: Option<BigDecimal>,
    pub ram_total: Option<BigDecimal>,
    pub ntp_offset: Option<BigDecimal>,
    pub ntp_correction: Option<BigDecimal>,
    pub voltage: Option<BigDecimal>,
    pub amperage: Option<BigDecimal>,
    pub cpu_temperature: Option<BigDecimal>,
    pub visible_senders: Option<i16>,
    pub latency: Option<BigDecimal>,
    pub senders: Option<i16>,
    pub rf_correction_manual: Option<i16>,
    pub rf_correction_automatic: Option<BigDecimal>,
    pub noise: Option<BigDecimal>,
    pub senders_signal_quality: Option<BigDecimal>,
    pub senders_messages: Option<i32>,
    pub good_senders_signal_quality: Option<BigDecimal>,
    pub good_senders: Option<i16>,
    pub good_and_bad_senders: Option<i16>,
    pub geoid_offset: Option<i16>,
    pub name: Option<String>,
    pub demodulation_snr_db: Option<BigDecimal>,
    pub ognr_pilotaware_version: Option<String>,
    pub unparsed_data: Option<String>,

    // Computed lag in milliseconds
    pub lag: Option<i32>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// For inserting new receiver statuses (without auto-generated fields)
#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::receiver_statuses)]
pub struct NewReceiverStatus {
    pub receiver_id: i32,
    pub received_at: DateTime<Utc>,

    // Status fields from StatusComment
    pub version: Option<String>,
    pub platform: Option<String>,
    pub cpu_load: Option<BigDecimal>,
    pub ram_free: Option<BigDecimal>,
    pub ram_total: Option<BigDecimal>,
    pub ntp_offset: Option<BigDecimal>,
    pub ntp_correction: Option<BigDecimal>,
    pub voltage: Option<BigDecimal>,
    pub amperage: Option<BigDecimal>,
    pub cpu_temperature: Option<BigDecimal>,
    pub visible_senders: Option<i16>,
    pub latency: Option<BigDecimal>,
    pub senders: Option<i16>,
    pub rf_correction_manual: Option<i16>,
    pub rf_correction_automatic: Option<BigDecimal>,
    pub noise: Option<BigDecimal>,
    pub senders_signal_quality: Option<BigDecimal>,
    pub senders_messages: Option<i32>,
    pub good_senders_signal_quality: Option<BigDecimal>,
    pub good_senders: Option<i16>,
    pub good_and_bad_senders: Option<i16>,
    pub geoid_offset: Option<i16>,
    pub name: Option<String>,
    pub demodulation_snr_db: Option<BigDecimal>,
    pub ognr_pilotaware_version: Option<String>,
    pub unparsed_data: Option<String>,

    // Computed lag in milliseconds
    pub lag: Option<i32>,
}

impl NewReceiverStatus {
    /// Create a new receiver status from an OGN StatusComment
    /// The lag will be computed based on packet_timestamp vs received_at
    pub fn from_status_comment(
        receiver_id: i32,
        status_comment: &StatusComment,
        packet_timestamp: DateTime<Utc>,
        received_at: DateTime<Utc>,
    ) -> Self {
        // Calculate lag in milliseconds
        let lag = Some((received_at - packet_timestamp).num_milliseconds() as i32);

        Self {
            receiver_id,
            received_at,
            version: status_comment.version.clone(),
            platform: status_comment.platform.clone(),
            cpu_load: Self::convert_decimal(status_comment.cpu_load.as_ref()),
            ram_free: Self::convert_decimal(status_comment.ram_free.as_ref()),
            ram_total: Self::convert_decimal(status_comment.ram_total.as_ref()),
            ntp_offset: Self::convert_decimal(status_comment.ntp_offset.as_ref()),
            ntp_correction: Self::convert_decimal(status_comment.ntp_correction.as_ref()),
            voltage: Self::convert_decimal(status_comment.voltage.as_ref()),
            amperage: Self::convert_decimal(status_comment.amperage.as_ref()),
            cpu_temperature: Self::convert_decimal(status_comment.cpu_temperature.as_ref()),
            visible_senders: status_comment.visible_senders.map(|v| v as i16),
            latency: Self::convert_decimal(status_comment.latency.as_ref()),
            senders: status_comment.senders.map(|v| v as i16),
            rf_correction_manual: status_comment.rf_correction_manual,
            rf_correction_automatic: Self::convert_decimal(
                status_comment.rf_correction_automatic.as_ref(),
            ),
            noise: Self::convert_decimal(status_comment.noise.as_ref()),
            senders_signal_quality: Self::convert_decimal(
                status_comment.senders_signal_quality.as_ref(),
            ),
            senders_messages: status_comment.senders_messages.map(|v| v as i32),
            good_senders_signal_quality: Self::convert_decimal(
                status_comment.good_senders_signal_quality.as_ref(),
            ),
            good_senders: status_comment.good_senders.map(|v| v as i16),
            good_and_bad_senders: status_comment.good_and_bad_senders.map(|v| v as i16),
            geoid_offset: status_comment.geoid_offset,
            name: status_comment.name.clone(),
            demodulation_snr_db: Self::convert_decimal(status_comment.demodulation_snr_db.as_ref()),
            ognr_pilotaware_version: status_comment.ognr_pilotaware_version.clone(),
            unparsed_data: status_comment.unparsed.clone(),
            lag,
        }
    }

    /// Convert rust_decimal::Decimal to BigDecimal
    fn convert_decimal(decimal: Option<&rust_decimal::Decimal>) -> Option<BigDecimal> {
        decimal.and_then(|d| BigDecimal::from_str(&d.to_string()).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bigdecimal::BigDecimal;
    use chrono::TimeZone;

    #[test]
    fn test_new_receiver_status_from_status_comment() {
        use rust_decimal::Decimal;

        let status_comment = StatusComment {
            version: Some("1.0.0".to_string()),
            platform: Some("Linux".to_string()),
            cpu_load: Some(Decimal::new(75, 2)),        // 0.75
            ram_free: Some(Decimal::new(1024, 0)),      // 1024
            ram_total: Some(Decimal::new(2048, 0)),     // 2048
            ntp_offset: Some(Decimal::new(5, 3)),       // 0.005
            ntp_correction: Some(Decimal::new(10, 3)),  // 0.010
            voltage: Some(Decimal::new(12, 1)),         // 1.2
            amperage: Some(Decimal::new(500, 3)),       // 0.500
            cpu_temperature: Some(Decimal::new(45, 1)), // 4.5
            visible_senders: Some(25),
            latency: Some(Decimal::new(100, 3)), // 0.100
            senders: Some(30),
            rf_correction_manual: Some(10),
            rf_correction_automatic: Some(Decimal::new(15, 2)), // 0.15
            noise: Some(Decimal::new(25, 2)),                   // 0.25
            senders_signal_quality: Some(Decimal::new(85, 2)),  // 0.85
            senders_messages: Some(1000),
            good_senders_signal_quality: Some(Decimal::new(90, 2)), // 0.90
            good_senders: Some(20),
            good_and_bad_senders: Some(25),
            geoid_offset: Some(15),
            name: Some("Test Receiver".to_string()),
            demodulation_snr_db: Some(Decimal::new(25, 1)), // 2.5
            ognr_pilotaware_version: Some("2.0.0".to_string()),
            unparsed: Some("unparsed data".to_string()),
        };

        let packet_time = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let received_time = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 1).unwrap(); // 1 second later

        let new_status = NewReceiverStatus::from_status_comment(
            123,
            &status_comment,
            packet_time,
            received_time,
        );

        assert_eq!(new_status.receiver_id, 123);
        assert_eq!(new_status.received_at, received_time);
        assert_eq!(new_status.version, Some("1.0.0".to_string()));
        assert_eq!(new_status.platform, Some("Linux".to_string()));
        assert_eq!(new_status.lag, Some(1000)); // 1 second = 1000ms
        assert_eq!(new_status.unparsed_data, Some("unparsed data".to_string()));
    }
}
