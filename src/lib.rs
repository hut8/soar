//! SOAR - APRS client library with message archiving capabilities
//!
//! This library provides an APRS client that can connect to APRS-IS servers
//! and optionally archive all incoming messages to daily log files.

pub mod actions;
pub mod aircraft_registrations;
pub mod aircraft_registrations_repo;
pub mod airports;
pub mod airports_repo;
pub mod aprs_client;
pub mod auth;
pub mod clubs;
pub mod clubs_repo;
pub mod fix_processor;
pub mod device_repo;
pub mod devices;
pub mod email;
pub mod faa;
pub mod fetch_receivers;
pub mod fixes;
pub mod fixes_repo;
pub mod flight_detection_processor;
pub mod flights;
pub mod flights_repo;
pub mod geocoding;
pub mod icao;
pub mod live_fixes;
pub mod loader;
pub mod locations;
pub mod locations_repo;
pub mod message_processors;
pub mod nats_publisher;
pub mod ogn_aprs_aircraft;
pub mod position;
pub mod pull;
pub mod receiver_repo;
pub mod receiver_status_repo;
pub mod receiver_statuses;
pub mod receivers;
pub mod runways;
pub mod runways_repo;
pub mod schema;
pub mod server_messages;
pub mod server_messages_repo;
pub mod users;
pub mod users_repo;
pub mod web;

pub use aprs_client::{
    AircraftPositionProcessor, AprsClient, AprsClientConfig, AprsClientConfigBuilder,
    AprsProcessors, FixHandler, PacketHandler, PacketRouter, PositionPacketProcessor,
    PositionProcessor, ReceiverStatusProcessor, StatusProcessor,
};
pub use nats_publisher::NatsFixPublisher;
pub use position::Fix;
