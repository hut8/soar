use crate::airports_repo::AirportsRepository;
use crate::geocoding::Geocoder;
use crate::locations::Location;
use crate::locations_repo::LocationsRepository;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Reverse geocoding is disabled for flight takeoffs and landings
/// When this is false, create_or_find_location will not perform reverse geocoding
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
                None,                                                // region_code
                None,                                                // county_mail_code
                result.country.map(|c| c.chars().take(2).collect()), // country code (first 2 chars)
                Some(crate::locations::Point::new(latitude, longitude)),
            );

            // Use find_or_create to avoid duplicate locations
            match locations_repo
                .find_or_create(
                    location.street1.clone(),
                    location.street2.clone(),
                    location.city.clone(),
                    location.state.clone(),
                    location.zip_code.clone(),
                    location.region_code.clone(),
                    location.county_mail_code.clone(),
                    location.country_mail_code.clone(),
                    location.geolocation,
                )
                .await
            {
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
