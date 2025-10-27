use chrono::Utc;
use metrics::gauge;
use std::collections::HashMap;
use uuid::Uuid;

use super::CurrentFlightState;

/// Update flight tracker metrics based on current active flights
/// This should be called regularly to export metrics even when cleanup doesn't run
pub(crate) fn update_flight_tracker_metrics(active_flights: &HashMap<Uuid, CurrentFlightState>) {
    let now = Utc::now();
    let stale_threshold = chrono::Duration::hours(2);

    let total_active = active_flights.len();
    let stale_count = active_flights
        .values()
        .filter(|state| {
            let elapsed = now.signed_duration_since(state.last_update_time);
            elapsed > stale_threshold
        })
        .count();

    gauge!("flight_tracker_active_devices").set(total_active as f64);
    gauge!("flight_tracker_stale_devices").set(stale_count as f64);
}
