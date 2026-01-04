use crate::airports_repo::AirportsRepository;
use crate::geocoding::Geocoder;
use crate::locations::Location;
use crate::locations_repo::LocationsRepository;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// DEPRECATED: Reverse geocoding for takeoff_location_id/landing_location_id fields (no longer used)
/// These fields have been replaced by start_location_id/end_location_id
/// This constant and create_or_find_location() are kept for backward compatibility only
const GEOCODING_ENABLED_FOR_TAKEOFF_LANDING: bool = false;

/// Enable reverse geocoding for start/end locations using Pelias
/// When true, create_start_end_location() will use Pelias city-level reverse geocoding
/// When false, start_location_id and end_location_id will be null
const GEOCODING_ENABLED_FOR_START_END: bool = true;

/// Find nearest airport within 2km of given coordinates
/// Returns the airport ID (not the identifier string)
pub(crate) async fn find_nearby_airport(
    airports_repo: &AirportsRepository,
    latitude: f64,
    longitude: f64,
) -> Option<i32> {
    match airports_repo
        .find_nearest_airports(latitude, longitude, 2000.0, 1) // 2km radius, 1 result
        .await
    {
        Ok(airports) if !airports.is_empty() => Some(airports[0].0.id),
        _ => None,
    }
}

/// Get an airport's existing location_id if it has one
/// Returns None if the airport doesn't exist or doesn't have a location_id
pub(crate) async fn get_airport_location_id(
    airports_repo: &AirportsRepository,
    airport_id: i32,
) -> Option<Uuid> {
    match airports_repo.get_airport_by_id(airport_id).await {
        Ok(Some(airport)) => airport.location_id,
        _ => None,
    }
}

/// DEPRECATED: Create or find a location for takeoff_location_id/landing_location_id (no longer used)
/// This function is kept for backward compatibility but is not called in production.
/// Use create_start_end_location() for start_location_id/end_location_id instead.
///
/// Note: Geocoding is permanently disabled via GEOCODING_ENABLED_FOR_TAKEOFF_LANDING constant
#[allow(dead_code)]
pub(crate) async fn create_or_find_location(
    _airports_repo: &AirportsRepository,
    locations_repo: &LocationsRepository,
    latitude: f64,
    longitude: f64,
    _airport_id: Option<i32>,
) -> Option<Uuid> {
    // Check if geocoding is enabled
    if !GEOCODING_ENABLED_FOR_TAKEOFF_LANDING {
        debug!(
            "Geocoding disabled for takeoff/landing at coordinates ({}, {}), skipping location creation",
            latitude, longitude
        );
        return None;
    }

    // Perform reverse geocoding
    debug!(
        "Performing reverse geocoding for coordinates ({}, {})",
        latitude, longitude
    );

    let geocoder = Geocoder::new();
    match geocoder.reverse_geocode(latitude, longitude).await {
        Ok(result) => {
            debug!(
                "Reverse geocoded ({}, {}) to: {}",
                latitude, longitude, result.display_name
            );

            // Create a location with the reverse geocoded address
            let location = Location::new(
                result.street1,
                None, // street2
                result.city,
                result.state,
                result.zip_code,
                result.country.map(|c| c.chars().take(2).collect()), // country code (first 2 chars)
                Some(crate::locations::Point::new(latitude, longitude)),
            );

            // Use find_or_create to avoid duplicate locations
            let params = crate::locations_repo::LocationParams {
                street1: location.street1.clone(),
                street2: location.street2.clone(),
                city: location.city.clone(),
                state: location.state.clone(),
                zip_code: location.zip_code.clone(),
                country_code: location.country_code.clone(),
                geolocation: location.geolocation,
            };
            match locations_repo.find_or_create(params).await {
                Ok(created_location) => {
                    info!(
                        "Created/found location {} for coordinates ({}, {}): {}",
                        created_location.id, latitude, longitude, result.display_name
                    );
                    Some(created_location.id)
                }
                Err(e) => {
                    warn!(
                        "Failed to create location for coordinates ({}, {}): {}",
                        latitude, longitude, e
                    );
                    None
                }
            }
        }
        Err(e) => {
            warn!(
                "Reverse geocoding failed for coordinates ({}, {}): {}",
                latitude, longitude, e
            );
            None
        }
    }
}

/// Create a start or end location with reverse geocoding for flight start/end tracking
/// Uses Pelias city-level reverse geocoding (self-hosted, fast, no rate limits)
/// Used for start_location_id and end_location_id fields to provide address context
///
/// This provides city-level precision (e.g., "Albany, NY") which is sufficient for
/// flight tracking without the overhead of detailed street-level geocoding.
///
/// # Arguments
/// * `locations_repo` - Repository for location database operations
/// * `latitude` - Latitude coordinate to reverse geocode
/// * `longitude` - Longitude coordinate to reverse geocode
/// * `context` - Description for logging (e.g., "start (takeoff)", "end (landing)")
///
/// # Returns
/// * `Some(Uuid)` - Location ID if reverse geocoding and creation succeeded
/// * `None` - If geocoding is disabled, Pelias unavailable, or location creation failed
pub(crate) async fn create_start_end_location(
    locations_repo: &LocationsRepository,
    latitude: f64,
    longitude: f64,
    context: &str,
) -> Option<Uuid> {
    // Check if geocoding is enabled for start/end locations
    if !GEOCODING_ENABLED_FOR_START_END {
        debug!(
            "Geocoding disabled for {} location at coordinates ({}, {}), skipping location creation",
            context, latitude, longitude
        );
        return None;
    }

    use std::time::Instant;

    debug!(
        "Creating {} location for coordinates ({}, {}) with Pelias reverse geocoding",
        context, latitude, longitude
    );

    // Use Pelias-only geocoder for real-time flight tracking
    let geocoder = match Geocoder::new_realtime_flight_tracking() {
        Ok(g) => g,
        Err(e) => {
            warn!(
                "Pelias not configured for real-time flight tracking: {}. Skipping location creation for {} at ({}, {})",
                e, context, latitude, longitude
            );
            return None;
        }
    };

    let start = Instant::now();

    // Only use Pelias - don't fall back to external APIs for real-time flight locations
    match geocoder.reverse_geocode(latitude, longitude).await {
        Ok(result) => {
            let latency_ms = start.elapsed().as_millis() as f64;
            metrics::histogram!("flight_tracker.location.pelias.latency_ms").record(latency_ms);
            metrics::counter!("flight_tracker.location.pelias.success_total").increment(1);

            // Check if we got structured data (city/state/country) vs just a generic name
            let has_structured_data =
                result.city.is_some() || result.state.is_some() || result.country.is_some();

            if !has_structured_data {
                metrics::counter!("flight_tracker.location.pelias.no_structured_data_total")
                    .increment(1);
                debug!(
                    "Pelias returned no structured data for {} location ({}, {}), only name: {}",
                    context, latitude, longitude, result.display_name
                );
            }
            debug!(
                "Reverse geocoded {} location ({}, {}) to: {}",
                context, latitude, longitude, result.display_name
            );

            // Create a location with the reverse geocoded address
            // NOTE: We intentionally omit street1 for flight start/end locations
            // to keep them at city/state/country precision level only
            let location = Location::new(
                None, // street1 - intentionally omitted for flight locations
                None, // street2
                result.city,
                result.state,
                result.zip_code,
                result.country.map(|c| c.chars().take(2).collect()), // country code (first 2 chars)
                Some(crate::locations::Point::new(latitude, longitude)),
            );

            // Use find_or_create to avoid duplicate locations
            let params = crate::locations_repo::LocationParams {
                street1: location.street1.clone(),
                street2: location.street2.clone(),
                city: location.city.clone(),
                state: location.state.clone(),
                zip_code: location.zip_code.clone(),
                country_code: location.country_code.clone(),
                geolocation: location.geolocation,
            };
            match locations_repo.find_or_create(params).await {
                Ok(created_location) => {
                    info!(
                        "Created/found {} location {} for coordinates ({}, {}): {}",
                        context, created_location.id, latitude, longitude, result.display_name
                    );

                    // Track location creation by type
                    let metric_type = match context {
                        "start (takeoff)" => "start_takeoff",
                        "start (airborne)" => "start_airborne",
                        "end (landing)" => "end_landing",
                        "end (timeout)" => "end_timeout",
                        _ => "unknown",
                    };
                    metrics::counter!("flight_tracker.location.created_total", "type" => metric_type)
                        .increment(1);

                    Some(created_location.id)
                }
                Err(e) => {
                    warn!(
                        "Failed to create {} location for coordinates ({}, {}): {}",
                        context, latitude, longitude, e
                    );
                    None
                }
            }
        }
        Err(e) => {
            metrics::counter!("flight_tracker.location.pelias.failure_total").increment(1);
            warn!(
                "Reverse geocoding failed for {} location at coordinates ({}, {}): {}",
                context, latitude, longitude, e
            );
            None
        }
    }
}
