//! SOAR - APRS client library with message archiving capabilities
//!
//! This library provides an APRS client that can connect to APRS-IS servers
//! and optionally archive all incoming messages to daily log files.

pub mod actions;
pub mod agl_batch_writer;
pub mod aircraft;
pub mod aircraft_images;
pub mod aircraft_images_client;
pub mod aircraft_registrations;
pub mod aircraft_registrations_repo;
pub mod aircraft_repo;
pub mod aircraft_types;
pub mod airports;
pub mod airports_repo;
pub mod airspace;
pub mod airspaces_repo;
pub mod analytics;
pub mod analytics_cache;
pub mod analytics_repo;
pub mod aprs_client;
pub mod aprs_filters;
pub mod aprs_nats_publisher;
pub mod archive_email_reporter;
pub mod archive_service;
pub mod auth;
pub mod beast;
pub mod beast_consumer_task;
pub mod beast_nats_publisher;
pub mod clubs;
pub mod clubs_repo;
pub mod coverage;
pub mod coverage_cache;
pub mod coverage_repo;
pub mod elevation;
pub mod email;
pub mod email_reporter;
pub mod faa;
pub mod fetch_receivers;
pub mod fix_processor;
pub mod fixes;
pub mod fixes_repo;
pub mod flight_tracker;
pub mod flights;
pub mod flights_repo;
pub mod geocoding;
pub mod geometry;
pub mod icao;
pub mod instance_lock;
pub mod live_fixes;
pub mod locations;
pub mod locations_repo;
pub mod message_sources;
pub mod metrics;
pub mod nats_publisher;
pub mod ogn_aprs_aircraft;
pub mod openaip_client;
pub mod packet_processors;
pub mod pilots;
pub mod pilots_repo;
pub mod raw_messages_repo;
pub mod receiver_repo;
pub mod receiver_status_repo;
pub mod receiver_statuses;
pub mod receivers;
pub mod runways;
pub mod runways_repo;
pub mod schema;
pub mod server_messages;
pub mod server_messages_repo;
pub mod user_fixes;
pub mod user_fixes_repo;
pub mod users;
pub mod users_repo;
pub mod watchlist;
pub mod watchlist_repo;
pub mod web;

pub use aprs_client::{AprsClient, AprsClientConfig, AprsClientConfigBuilder};
pub use archive_service::ArchiveService;
pub use fixes::Fix;
pub use nats_publisher::NatsFixPublisher;
pub use packet_processors::{
    AircraftPositionProcessor, PacketRouter, PositionPacketProcessor, ReceiverPositionProcessor,
    ReceiverStatusProcessor, ServerStatusProcessor,
};

/// Get the NATS client name for a given process based on the environment.
///
/// Environment detection:
/// - SOAR_ENV=production -> "soar-{process}"
/// - SOAR_ENV=staging -> "soar-{process}-staging"
/// - SOAR_ENV unset or other -> "soar-{process}-dev"
///
/// # Examples
///
/// ```
/// unsafe {
///     std::env::set_var("SOAR_ENV", "production");
/// }
/// assert_eq!(soar::nats_client_name("web"), "soar-web");
///
/// unsafe {
///     std::env::set_var("SOAR_ENV", "staging");
/// }
/// assert_eq!(soar::nats_client_name("web"), "soar-web-staging");
///
/// unsafe {
///     std::env::remove_var("SOAR_ENV");
/// }
/// assert_eq!(soar::nats_client_name("web"), "soar-web-dev");
/// ```
pub fn nats_client_name(process_name: &str) -> String {
    match std::env::var("SOAR_ENV").as_deref() {
        Ok("production") => format!("soar-{}", process_name),
        Ok("staging") => format!("soar-{}-staging", process_name),
        _ => format!("soar-{}-dev", process_name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_nats_client_name_production() {
        unsafe {
            std::env::set_var("SOAR_ENV", "production");
        }
        assert_eq!(nats_client_name("web"), "soar-web");
        assert_eq!(nats_client_name("run"), "soar-run");
        unsafe {
            std::env::remove_var("SOAR_ENV");
        }
    }

    #[test]
    #[serial]
    fn test_nats_client_name_staging() {
        unsafe {
            std::env::set_var("SOAR_ENV", "staging");
        }
        assert_eq!(nats_client_name("web"), "soar-web-staging");
        assert_eq!(
            nats_client_name("aprs-ingester"),
            "soar-aprs-ingester-staging"
        );
        unsafe {
            std::env::remove_var("SOAR_ENV");
        }
    }

    #[test]
    #[serial]
    fn test_nats_client_name_dev_unset() {
        unsafe {
            std::env::remove_var("SOAR_ENV");
        }
        assert_eq!(nats_client_name("web"), "soar-web-dev");
        assert_eq!(nats_client_name("run"), "soar-run-dev");
    }

    #[test]
    #[serial]
    fn test_nats_client_name_dev_other() {
        unsafe {
            std::env::set_var("SOAR_ENV", "development");
        }
        assert_eq!(nats_client_name("web"), "soar-web-dev");
        unsafe {
            std::env::set_var("SOAR_ENV", "local");
        }
        assert_eq!(
            nats_client_name("beast-ingester"),
            "soar-beast-ingester-dev"
        );
        unsafe {
            std::env::remove_var("SOAR_ENV");
        }
    }
}
