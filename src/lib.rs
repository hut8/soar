//! SOAR - APRS client library with message archiving capabilities
//!
//! This library provides an APRS client that can connect to APRS-IS servers
//! and optionally archive all incoming messages to daily log files.

pub mod devices;
pub mod device_repo;
pub mod faa;
pub mod ogn_aprs_aircraft;
pub mod aprs_client;
pub mod airports;
pub mod airports_repo;
pub mod runways;
pub mod runways_repo;
pub mod receivers;
pub mod receiver_repo;
pub mod clubs;
pub mod geocoding;
pub mod nats_publisher;
pub mod position;
pub mod message_processors;

pub use aprs_client::{AprsClient, AprsClientConfig, AprsClientConfigBuilder, MessageProcessor, FixProcessor};
pub use nats_publisher::NatsFixPublisher;
pub use position::Fix;
