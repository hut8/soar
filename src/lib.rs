//! SOAR - APRS client library with message archiving capabilities
//!
//! This library provides an APRS client that can connect to APRS-IS servers
//! and optionally archive all incoming messages to daily log files.

pub mod actions;
pub mod adsb_accumulator;
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
pub mod archive_email_reporter;
pub mod archive_service;
pub mod auth;
pub mod beast;
pub mod beast_consumer_task;
pub mod club_tow_fees;
pub mod club_tow_fees_repo;
pub mod clubs;
pub mod clubs_repo;
pub mod connection_status;
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
pub mod geofence;
pub mod geofence_repo;
pub mod geometry;
pub mod icao;
pub mod igc;
pub mod ingest_config;
pub mod instance_lock;
pub mod live_fixes;
pub mod locations;
pub mod locations_repo;
pub mod magnetic;
pub mod message_sources;
pub mod metrics;
pub mod nats_publisher;
pub mod ogn;
pub mod ogn_aprs_aircraft;
pub mod openaip_client;
pub mod persistent_queue;
pub mod pilots;
pub mod pilots_repo;
pub mod postgis_functions;
pub mod protocol;
pub mod raw_messages_repo;
pub mod receiver_repo;
pub mod receiver_status_repo;
pub mod receiver_statuses;
pub mod receivers;
pub mod runways;
pub mod runways_repo;
pub mod sbs;
pub mod schema;
pub mod server_messages;
pub mod server_messages_repo;
pub mod socket_client;
pub mod socket_server;
pub mod stream_manager;
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
pub use ogn::{
    OgnGenericProcessor, PacketContext, ReceiverPositionProcessor, ReceiverStatusProcessor,
    ServerStatusProcessor,
};

#[cfg(test)]
mod ts_export;

/// Get the Unix socket path for inter-process communication based on the environment.
///
/// Environment detection:
/// - SOAR_ENV=production -> "/var/run/soar/run.sock"
/// - SOAR_ENV=staging -> "/var/run/soar/run.sock"
/// - SOAR_ENV unset or other -> "/tmp/soar-{user}/run.sock" (user-specific for development)
///
/// In development mode, uses the USER environment variable to create a user-specific
/// directory to avoid permission conflicts when multiple users run the service.
///
/// # Examples
///
/// ```
/// unsafe {
///     std::env::set_var("SOAR_ENV", "production");
/// }
/// assert_eq!(soar::socket_path(), std::path::PathBuf::from("/var/run/soar/run.sock"));
///
/// unsafe {
///     std::env::set_var("SOAR_ENV", "staging");
/// }
/// assert_eq!(soar::socket_path(), std::path::PathBuf::from("/var/run/soar/run.sock"));
/// ```
pub fn socket_path() -> std::path::PathBuf {
    match std::env::var("SOAR_ENV").as_deref() {
        Ok("production") | Ok("staging") => std::path::PathBuf::from("/var/run/soar/run.sock"),
        _ => {
            // Development mode: use user-specific directory to avoid permission conflicts
            let user = std::env::var("USER").unwrap_or_else(|_| "default".to_string());
            std::path::PathBuf::from(format!("/tmp/soar-{}/run.sock", user))
        }
    }
}

/// Get the persistent queue directory based on the environment.
///
/// Environment detection:
/// - SOAR_ENV=production -> "/var/lib/soar/queues"
/// - SOAR_ENV=staging -> "/var/lib/soar/queues"
/// - SOAR_ENV unset or other -> "$XDG_DATA_HOME/soar/queues" (defaults to ~/.local/share/soar/queues)
///
/// In development mode, follows XDG Base Directory Specification to avoid
/// requiring root permissions.
pub fn queue_dir() -> std::path::PathBuf {
    match std::env::var("SOAR_ENV").as_deref() {
        Ok("production") | Ok("staging") => std::path::PathBuf::from("/var/lib/soar/queues"),
        _ => {
            // Development mode: use XDG_DATA_HOME or fallback to ~/.local/share
            let data_home = std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                format!("{}/.local/share", home)
            });
            std::path::PathBuf::from(format!("{}/soar/queues", data_home))
        }
    }
}

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
    fn test_socket_path_production() {
        unsafe {
            std::env::set_var("SOAR_ENV", "production");
        }
        assert_eq!(
            socket_path(),
            std::path::PathBuf::from("/var/run/soar/run.sock")
        );
        unsafe {
            std::env::remove_var("SOAR_ENV");
        }
    }

    #[test]
    #[serial]
    fn test_socket_path_staging() {
        unsafe {
            std::env::set_var("SOAR_ENV", "staging");
        }
        assert_eq!(
            socket_path(),
            std::path::PathBuf::from("/var/run/soar/run.sock")
        );
        unsafe {
            std::env::remove_var("SOAR_ENV");
        }
    }

    #[test]
    #[serial]
    fn test_socket_path_dev() {
        unsafe {
            std::env::remove_var("SOAR_ENV");
        }
        let user = std::env::var("USER").unwrap_or_else(|_| "default".to_string());
        let expected = std::path::PathBuf::from(format!("/tmp/soar-{}/run.sock", user));
        assert_eq!(socket_path(), expected);
    }

    #[test]
    #[serial]
    fn test_socket_path_dev_other() {
        unsafe {
            std::env::set_var("SOAR_ENV", "development");
        }
        let user = std::env::var("USER").unwrap_or_else(|_| "default".to_string());
        let expected = std::path::PathBuf::from(format!("/tmp/soar-{}/run.sock", user));
        assert_eq!(socket_path(), expected);
        unsafe {
            std::env::remove_var("SOAR_ENV");
        }
    }

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
