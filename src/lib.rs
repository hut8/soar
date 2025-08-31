//! SOAR - APRS client library with message archiving capabilities
//!
//! This library provides an APRS client that can connect to APRS-IS servers
//! and optionally archive all incoming messages to daily log files.

pub mod ddb;
pub mod faa;
pub mod ogn_aprs_aircraft;
pub mod aprs_client;
pub mod airports;

pub use aprs_client::{AprsClient, AprsClientConfig, AprsClientConfigBuilder, MessageProcessor};
