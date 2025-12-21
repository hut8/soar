use crate::airports_repo::AirportsRepository;
use crate::geocoding::Geocoder;
use crate::locations::Location;
use crate::locations_repo::LocationsRepository;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Reverse geocoding is disabled for flight takeoffs and landings (takeoff_location_id/landing_location_id)
/// When this is false, create_or_find_location will not perform reverse geocoding
/// Note: start_location_id and end_location_id always use reverse geocoding via create_start_end_location
const GEOCODING_ENABLED_FOR_TAKEOFF_LANDING: bool = false;

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

/// Create or find a location for given coordinates using reverse geocoding
/// If an airport is present at these coordinates, use the airport's location instead
/// Otherwise, perform reverse geocoding via Nominatim and create a new location
/// Returns the location ID if successful
///
/// Note: Geocoding is currently disabled via GEOCODING_ENABLED_FOR_TAKEOFF_LANDING constant
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
/// This function ONLY uses Photon reverse geocoding (no fallbacks to avoid hitting external APIs frequently)
/// Used for start_location_id and end_location_id fields to provide address context
///
/// # Arguments
/// * `locations_repo` - Repository for location database operations
/// * `latitude` - Latitude coordinate to reverse geocode
/// * `longitude` - Longitude coordinate to reverse geocode
/// * `context` - Description for logging (e.g., "takeoff", "landing", "timeout")
///
/// # Returns
/// * `Some(Uuid)` - Location ID if reverse geocoding and creation succeeded
/// * `None` - If reverse geocoding or location creation failed
pub(crate) async fn create_start_end_location(
    locations_repo: &LocationsRepository,
    latitude: f64,
    longitude: f64,
    context: &str,
) -> Option<Uuid> {
    use std::time::Instant;

    debug!(
        "Creating {} location for coordinates ({}, {}) with Photon reverse geocoding",
        context, latitude, longitude
    );

    let geocoder = Geocoder::new();
    let start = Instant::now();

    // Only use Photon - don't fall back to Nominatim or Google Maps for flight locations
    match geocoder
        .reverse_geocode_with_photon_only(latitude, longitude)
        .await
    {
        Ok(result) => {
            let latency_ms = start.elapsed().as_millis() as f64;
            metrics::histogram!("flight_tracker.location.photon.latency_ms").record(latency_ms);
            metrics::counter!("flight_tracker.location.photon.success").increment(1);

            // Check if we got structured data (city/state/country) vs just a generic name
            let has_structured_data =
                result.city.is_some() || result.state.is_some() || result.country.is_some();

            if !has_structured_data {
                metrics::counter!("flight_tracker.location.photon.no_structured_data").increment(1);
                debug!(
                    "Photon returned no structured data for {} location ({}, {}), only name: {}",
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
                    metrics::counter!("flight_tracker.location.created", "type" => metric_type)
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
            metrics::counter!("flight_tracker.location.photon.failure").increment(1);
            warn!(
                "Reverse geocoding failed for {} location at coordinates ({}, {}): {}",
                context, latitude, longitude, e
            );
            None
        }
    }
}
