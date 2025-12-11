use metrics::gauge;
use std::collections::HashMap;
use uuid::Uuid;

use super::CurrentFlightState;

/// Update flight tracker metrics based on current active flights
/// This should be called regularly to export metrics even when cleanup doesn't run
pub(crate) fn update_flight_tracker_metrics(active_flights: &HashMap<Uuid, CurrentFlightState>) {
    let total_active = active_flights.len();

    gauge!("flight_tracker_active_aircraft").set(total_active as f64);
}
