use metrics::gauge;

use super::AircraftStatesMap;

/// Update flight tracker metrics based on current aircraft states
/// This should be called regularly to export metrics even when cleanup doesn't run
pub(crate) fn update_flight_tracker_metrics(aircraft_states: &AircraftStatesMap) {
    // Count aircraft with active flights
    let total_with_flights = aircraft_states
        .iter()
        .filter(|entry| entry.current_flight_id.is_some())
        .count();

    // Total aircraft tracked (including those on ground in last 18 hours)
    let total_tracked = aircraft_states.len();

    gauge!("flight_tracker_active_aircraft").set(total_with_flights as f64);
    gauge!("flight_tracker_tracked_aircraft").set(total_tracked as f64);
}
