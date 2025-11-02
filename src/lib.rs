//! SOAR - APRS client library with message archiving capabilities
//!
//! This library provides an APRS client that can connect to APRS-IS servers
//! and optionally archive all incoming messages to daily log files.

pub mod actions;
pub mod agl_backfill;
pub mod agl_batch_writer;
pub mod aircraft_registrations;
pub mod aircraft_registrations_repo;
pub mod aircraft_types;
pub mod airports;
pub mod airports_repo;
pub mod aprs_client;
pub mod aprs_filters;
pub mod aprs_jetstream_consumer;
pub mod aprs_jetstream_publisher;
pub mod aprs_messages_repo;
pub mod auth;
pub mod clubs;
pub mod clubs_repo;
pub mod device_repo;
pub mod devices;
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
pub mod metrics;
pub mod nats_publisher;
pub mod ogn_aprs_aircraft;
pub mod packet_processors;
pub mod pilots;
pub mod pilots_repo;
pub mod queue_config;
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

pub use aprs_client::{AprsClient, AprsClientConfig, AprsClientConfigBuilder, ArchiveService};
pub use fixes::Fix;
pub use nats_publisher::NatsFixPublisher;
pub use packet_processors::{
    AircraftPositionProcessor, PacketRouter, PositionPacketProcessor, ReceiverPositionProcessor,
    ReceiverStatusProcessor, ServerStatusProcessor,
};
