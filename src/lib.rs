//! SOAR - APRS client library with message archiving capabilities
//!
//! This library provides an APRS client that can connect to APRS-IS servers
//! and optionally archive all incoming messages to daily log files.

pub mod airports;
pub mod airports_repo;
pub mod aprs_client;
pub mod clubs;
pub mod clubs_repo;
pub mod database_fix_processor;
pub mod device_repo;
pub mod devices;
pub mod faa;
pub mod fixes;
pub mod fixes_repo;
pub mod flight_detection_processor;
pub mod flights;
pub mod flights_repo;
pub mod geocoding;
pub mod message_processors;
pub mod nats_publisher;
pub mod ogn_aprs_aircraft;
pub mod position;
pub mod receiver_repo;
pub mod receivers;
pub mod runways;
pub mod runways_repo;
pub mod web;

pub use aprs_client::{
    AprsClient, AprsClientConfig, AprsClientConfigBuilder, FixProcessor, MessageProcessor,
};
pub use nats_publisher::NatsFixPublisher;
pub use position::Fix;
