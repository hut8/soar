use crate::devices::AddressType;
use chrono::Utc;
use metrics::{gauge, histogram};
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, info};
use uuid::Uuid;

use super::aircraft_tracker::AircraftTracker;

/// Helper function to format device address with type for logging
pub(crate) fn format_device_address_with_type(
    device_address: &str,
    address_type: AddressType,
) -> String {
    match address_type {
        AddressType::Flarm => format!("FLARM-{}", device_address),
        AddressType::Ogn => format!("OGN-{}", device_address),
        AddressType::Icao => format!("ICAO-{}", device_address),
        AddressType::Unknown => device_address.to_string(),
    }
}

/// Clean up old trackers for aircraft that haven't been seen recently
pub(crate) async fn cleanup_old_trackers(trackers: &mut HashMap<Uuid, AircraftTracker>) {
    let start = Instant::now();
    let cutoff_time = Utc::now() - chrono::Duration::hours(2);

    let old_count = trackers.len();
    trackers.retain(|device_address, tracker| {
        if tracker.last_update < cutoff_time {
            debug!("Removing stale tracker for aircraft {}", device_address);
            false
        } else {
            true
        }
    });

    let removed_count = old_count - trackers.len();
    if removed_count > 0 {
        info!("Cleaned up {} stale aircraft trackers", removed_count);
    }

    // Record metrics for cleanup performance
    histogram!("flight_tracker_cleanup_duration_seconds").record(start.elapsed().as_secs_f64());
    gauge!("flight_tracker_active_devices").set(trackers.len() as f64);
    gauge!("flight_tracker_cleanup_removed_count").set(removed_count as f64);
}
