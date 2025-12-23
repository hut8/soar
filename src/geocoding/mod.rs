mod google_maps;
mod nominatim;
mod pelias;
mod photon;

use ::google_maps::Client as GoogleMapsClient;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use reqwest;
use std::env;
use std::time::Duration;
use tracing::{debug, warn};

use crate::locations::Point;

use self::google_maps::GoogleMapsGeocoderClient;
use nominatim::NominatimClient;
use pelias::PeliasClient;

/// Trait for geocoding services that support forward geocoding (address → coordinates)
///
/// Forward geocoding converts a human-readable address string into geographic coordinates.
///
/// # Example
///
/// ```rust,ignore
/// let client = NominatimClient::new(...);
/// let point = client.geocode("1600 Pennsylvania Avenue, Washington, DC").await?;
/// println!("Coordinates: {}, {}", point.latitude, point.longitude);
/// ```
#[async_trait]
pub trait ForwardGeocoder: Send + Sync {
    /// Geocode an address string to WGS84 coordinates
    ///
    /// # Arguments
    ///
    /// * `address` - The address to geocode (e.g., "123 Main St, Albany, NY 12207")
    ///
    /// # Returns
    ///
    /// * `Ok(Point)` - The geographic coordinates of the address
    /// * `Err(...)` - If the address cannot be geocoded or the service is unavailable
    async fn geocode(&self, address: &str) -> Result<Point>;
}

/// Trait for geocoding services that support reverse geocoding (coordinates → address)
///
/// Reverse geocoding converts geographic coordinates into a human-readable address.
///
/// Note: Most services use nearest-point matching rather than point-in-polygon containment.
/// See the Photon module documentation for details on this limitation.
///
/// # Example
///
/// ```rust,ignore
/// let client = NominatimClient::new(...);
/// let result = client.reverse_geocode(42.6526, -73.7562).await?;
/// println!("Address: {}", result.display_name);
/// ```
#[async_trait]
pub trait ReverseGeocoder: Send + Sync {
    /// Reverse geocode coordinates to address components
    ///
    /// # Arguments
    ///
    /// * `latitude` - Latitude in WGS84 (-90.0 to 90.0)
    /// * `longitude` - Longitude in WGS84 (-180.0 to 180.0)
    ///
    /// # Returns
    ///
    /// * `Ok(ReverseGeocodeResult)` - The address components and display name
    /// * `Err(...)` - If the coordinates cannot be reverse geocoded or the service is unavailable
    async fn reverse_geocode(&self, latitude: f64, longitude: f64) -> Result<ReverseGeocodeResult>;
}

/// Geocoding service that was used to successfully geocode an address
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeocodingService {
    Photon,
    Pelias,
    Nominatim,
    GoogleMaps,
}

/// Result from geocoding that includes both the coordinates and which service was used
#[derive(Debug, Clone)]
pub struct GeocodeResult {
    pub point: Point,
    pub service: GeocodingService,
}

impl GeocodeResult {
    pub fn new(point: Point, service: GeocodingService) -> Self {
        Self { point, service }
    }
}

/// Reverse geocoding result
#[derive(Debug, Clone)]
pub struct ReverseGeocodeResult {
    pub street1: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip_code: Option<String>,
    pub country: Option<String>,
    pub display_name: String,
}

/// Enhanced geocoding module with Google Maps fallback capability
///
/// This module provides geocoding functionality using Nominatim as the primary service
/// and Google Maps as a fallback when the GOOGLE_MAPS_API_KEY environment variable is set.
///
/// ## Usage
///
/// ### Batch geocoding (Nominatim → Google Maps):
/// ```rust,no_run
/// use soar::geocoding::Geocoder;
///
/// # async fn example() -> anyhow::Result<()> {
/// let geocoder = Geocoder::new_batch_geocoding();
/// let point = geocoder.geocode_address("1600 Pennsylvania Avenue, Washington, DC").await?;
/// # Ok(())
/// # }
/// ```
///
/// ### With Google Maps fallback:
/// Set the GOOGLE_MAPS_API_KEY environment variable:
/// ```bash
/// export GOOGLE_MAPS_API_KEY="your_api_key_here"
/// ```
///
/// Then use the geocoder normally - it will automatically fall back to Google Maps
/// when Nominatim fails:
/// ```rust,no_run
/// use soar::geocoding::Geocoder;
///
/// # async fn example() -> anyhow::Result<()> {
/// let geocoder = Geocoder::new_batch_geocoding();
/// let point = geocoder.geocode_address("123 Hard to Find Address").await?;
/// // Will try Nominatim first, then Google Maps if it fails
/// # Ok(())
/// # }
/// ```
pub struct Geocoder {
    forward_geocoders: Vec<(GeocodingService, Box<dyn ForwardGeocoder>)>,
    reverse_geocoders: Vec<(GeocodingService, Box<dyn ReverseGeocoder>)>,
}

impl Default for Geocoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Geocoder {
    /// Create a new Geocoder instance with default settings
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        let mut forward_geocoders: Vec<(GeocodingService, Box<dyn ForwardGeocoder>)> = Vec::new();
        let mut reverse_geocoders: Vec<(GeocodingService, Box<dyn ReverseGeocoder>)> = Vec::new();

        // DISABLED: Photon client temporarily disabled for reverse geocoding
        // Keeping code for potential future use
        //
        // Photon is disabled for two primary reasons:
        //
        // 1. NEAREST-POINT MATCHING INACCURACY:
        //    Photon (like Nominatim) uses nearest-point matching instead of point-in-polygon
        //    containment for reverse geocoding. This means it finds the nearest known geographic
        //    feature and returns that object's address, not the address of the query point.
        //
        //    While you can filter by layer (e.g., layer=city via https://github.com/komoot/photon?tab=readme-ov-file#filter-results-by-layer),
        //    this causes significant accuracy issues:
        //    - Example: A coordinate in Albany, NY might return "Troy, NY" if Troy's city center
        //      is geometrically closer than Albany's center
        //    - Requires large search radii (we use progressive 1km/5km/10km) to ensure we always
        //      find *some* city, making queries inefficient
        //    - No guarantee the returned city is the one that actually contains the point
        //
        // 2. SCHEMA INCONSISTENCY ACROSS ENTITY TYPES:
        //    To work around #1, we could avoid filtering by layer and let Photon return any
        //    nearby object (buildings, streets, POIs, etc.). However, this creates new problems:
        //    - Requires importing significantly more OSM data (every building, not just cities)
        //    - Different entity types return different schemas:
        //      * For type="city": city name is in the "name" field
        //      * For type="house" or buildings: city name is in the "city" field
        //    - Makes extracting consistent "city, state, postal_code, country" data extremely
        //      cumbersome and error-prone
        //
        // See also:
        // - Nominatim has the same nearest-point issue: https://nominatim.org/release-docs/latest/api/Faq/#2-when-doing-reverse-search-the-address-details-have-parts-that-dont-contain-the-point-i-was-looking-up
        // - Photon layer filtering: https://github.com/komoot/photon?tab=readme-ov-file#filter-results-by-layer
        //
        // For now, we use Nominatim/Pelias/Google Maps which handle these complexities internally.
        //
        // if let Ok(url) = env::var("PHOTON_BASE_URL") {
        //     if !url.trim().is_empty() {
        //         debug!("Using Photon geocoding server at: {}", url);
        //         let photon = PhotonClient::new(client.clone(), url.trim().to_string());
        //         forward_geocoders.push((GeocodingService::Photon, Box::new(photon.clone())));
        //         reverse_geocoders.push((GeocodingService::Photon, Box::new(photon)));
        //     } else {
        //         debug!("PHOTON_BASE_URL is set but empty, skipping Photon");
        //     }
        // } else {
        //     debug!("PHOTON_BASE_URL not set, Photon geocoding unavailable");
        // }

        // Add Pelias for reverse geocoding (city-level only, no forward geocoding)
        if let Ok(url) = env::var("PELIAS_BASE_URL") {
            if !url.trim().is_empty() {
                debug!("Using Pelias geocoding server at: {}", url);
                let pelias = PeliasClient::new(client.clone(), url.trim().to_string());
                reverse_geocoders.push((GeocodingService::Pelias, Box::new(pelias)));
            } else {
                debug!("PELIAS_BASE_URL is set but empty, skipping Pelias");
            }
        } else {
            debug!("PELIAS_BASE_URL not set, Pelias geocoding unavailable");
        }

        // Add Nominatim (always available)
        let nominatim = NominatimClient::new(
            client.clone(),
            "https://nominatim.openstreetmap.org".to_string(),
            "SOAR Aircraft Geocoder/1.0 (https://github.com/hut8/soar)".to_string(),
        );
        forward_geocoders.push((GeocodingService::Nominatim, Box::new(nominatim.clone())));
        reverse_geocoders.push((GeocodingService::Nominatim, Box::new(nominatim)));

        // Add Google Maps if API key is available
        if let Ok(api_key) = env::var("GOOGLE_MAPS_API_KEY") {
            if !api_key.trim().is_empty() {
                debug!("Initializing Google Maps client as geocoding fallback");
                match GoogleMapsClient::try_new(&api_key) {
                    Ok(gmaps_client) => {
                        let google_maps = GoogleMapsGeocoderClient::new(gmaps_client);
                        forward_geocoders
                            .push((GeocodingService::GoogleMaps, Box::new(google_maps.clone())));
                        reverse_geocoders
                            .push((GeocodingService::GoogleMaps, Box::new(google_maps)));
                    }
                    Err(e) => {
                        warn!("Failed to create Google Maps client: {}", e);
                    }
                }
            } else {
                debug!("GOOGLE_MAPS_API_KEY is set but empty, skipping Google Maps initialization");
            }
        } else {
            debug!("GOOGLE_MAPS_API_KEY not set, Google Maps fallback unavailable");
        }

        Self {
            forward_geocoders,
            reverse_geocoders,
        }
    }

    /// Create a new Geocoder with custom settings
    pub fn with_settings(base_url: String, user_agent: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        let mut forward_geocoders: Vec<(GeocodingService, Box<dyn ForwardGeocoder>)> = Vec::new();
        let mut reverse_geocoders: Vec<(GeocodingService, Box<dyn ReverseGeocoder>)> = Vec::new();

        // DISABLED: Photon - see comments in new() method for details

        // Add Pelias for reverse geocoding (city-level only, no forward geocoding)
        if let Ok(url) = env::var("PELIAS_BASE_URL") {
            if !url.trim().is_empty() {
                debug!("Using Pelias geocoding server at: {}", url);
                let pelias = PeliasClient::new(client.clone(), url.trim().to_string());
                reverse_geocoders.push((GeocodingService::Pelias, Box::new(pelias)));
            } else {
                debug!("PELIAS_BASE_URL is set but empty, skipping Pelias");
            }
        } else {
            debug!("PELIAS_BASE_URL not set, Pelias geocoding unavailable");
        }

        // Add Nominatim with custom settings
        let nominatim = NominatimClient::new(client.clone(), base_url, user_agent);
        forward_geocoders.push((GeocodingService::Nominatim, Box::new(nominatim.clone())));
        reverse_geocoders.push((GeocodingService::Nominatim, Box::new(nominatim)));

        // Add Google Maps if API key is available
        if let Ok(api_key) = env::var("GOOGLE_MAPS_API_KEY") {
            if !api_key.trim().is_empty() {
                debug!("Initializing Google Maps client as geocoding fallback");
                match GoogleMapsClient::try_new(&api_key) {
                    Ok(gmaps_client) => {
                        let google_maps = GoogleMapsGeocoderClient::new(gmaps_client);
                        forward_geocoders
                            .push((GeocodingService::GoogleMaps, Box::new(google_maps.clone())));
                        reverse_geocoders
                            .push((GeocodingService::GoogleMaps, Box::new(google_maps)));
                    }
                    Err(e) => {
                        warn!("Failed to create Google Maps client: {}", e);
                    }
                }
            } else {
                debug!("GOOGLE_MAPS_API_KEY is set but empty, skipping Google Maps initialization");
            }
        } else {
            debug!("GOOGLE_MAPS_API_KEY not set, Google Maps fallback unavailable");
        }

        Self {
            forward_geocoders,
            reverse_geocoders,
        }
    }

    /// Create a Geocoder for batch geocoding operations (receivers, clubs, airports, aircraft registrations)
    /// Uses only Nominatim → Google Maps (no Pelias)
    ///
    /// This configuration is optimized for:
    /// - High-quality address results (detailed street-level data)
    /// - Non-time-critical batch processing
    /// - Lower volume operations where rate limits are manageable
    pub fn new_batch_geocoding() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        let mut forward_geocoders: Vec<(GeocodingService, Box<dyn ForwardGeocoder>)> = Vec::new();
        let mut reverse_geocoders: Vec<(GeocodingService, Box<dyn ReverseGeocoder>)> = Vec::new();

        // Add Nominatim (always available)
        let nominatim = NominatimClient::new(
            client.clone(),
            "https://nominatim.openstreetmap.org".to_string(),
            "SOAR Aircraft Geocoder/1.0 (https://github.com/hut8/soar)".to_string(),
        );
        forward_geocoders.push((GeocodingService::Nominatim, Box::new(nominatim.clone())));
        reverse_geocoders.push((GeocodingService::Nominatim, Box::new(nominatim)));

        // Add Google Maps if API key is available
        if let Ok(api_key) = env::var("GOOGLE_MAPS_API_KEY")
            && !api_key.trim().is_empty()
        {
            debug!("Initializing Google Maps client for batch geocoding");
            match GoogleMapsClient::try_new(&api_key) {
                Ok(gmaps_client) => {
                    let google_maps = GoogleMapsGeocoderClient::new(gmaps_client);
                    forward_geocoders
                        .push((GeocodingService::GoogleMaps, Box::new(google_maps.clone())));
                    reverse_geocoders.push((GeocodingService::GoogleMaps, Box::new(google_maps)));
                }
                Err(e) => {
                    warn!("Failed to create Google Maps client: {}", e);
                }
            }
        }

        Self {
            forward_geocoders,
            reverse_geocoders,
        }
    }

    /// Create a Geocoder for real-time flight tracking
    /// Uses ONLY Pelias (city-level reverse geocoding, no fallbacks)
    ///
    /// This configuration is optimized for:
    /// - High-frequency real-time operations
    /// - City-level precision (sufficient for flight tracking)
    /// - Fast response times (no fallback delays)
    /// - No rate limiting concerns (self-hosted Pelias)
    ///
    /// Returns error if PELIAS_BASE_URL is not configured
    pub fn new_realtime_flight_tracking() -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        let forward_geocoders: Vec<(GeocodingService, Box<dyn ForwardGeocoder>)> = Vec::new();
        let mut reverse_geocoders: Vec<(GeocodingService, Box<dyn ReverseGeocoder>)> = Vec::new();

        // ONLY add Pelias for real-time flight tracking
        let url = env::var("PELIAS_BASE_URL").map_err(|_| {
            anyhow!("PELIAS_BASE_URL not set - required for real-time flight tracking")
        })?;

        if url.trim().is_empty() {
            return Err(anyhow!(
                "PELIAS_BASE_URL is empty - required for real-time flight tracking"
            ));
        }

        debug!(
            "Using Pelias geocoding server for real-time flight tracking at: {}",
            url
        );
        let pelias = PeliasClient::new(client, url.trim().to_string());
        reverse_geocoders.push((GeocodingService::Pelias, Box::new(pelias)));

        Ok(Self {
            forward_geocoders,
            reverse_geocoders,
        })
    }

    /// Reverse geocode coordinates using ONLY Photon (no fallbacks)
    /// Used for high-frequency operations like flight start/end locations
    /// Returns error if Photon is unavailable or fails
    ///
    /// CURRENTLY DISABLED: Photon geocoding is temporarily disabled
    pub async fn reverse_geocode_with_photon_only(
        &self,
        _latitude: f64,
        _longitude: f64,
    ) -> Result<ReverseGeocodeResult> {
        // DISABLED: Photon is temporarily not in use
        Err(anyhow!("Photon geocoding is currently disabled"))

        // Keeping original code for potential future use:
        // // Validate coordinates
        // if !(-90.0..=90.0).contains(&latitude) {
        //     return Err(anyhow!("Invalid latitude: {}", latitude));
        // }
        // if !(-180.0..=180.0).contains(&longitude) {
        //     return Err(anyhow!("Invalid longitude: {}", longitude));
        // }
        //
        // // Only use Photon - no fallbacks
        // let photon = self
        //     .photon
        //     .as_ref()
        //     .ok_or_else(|| anyhow!("Photon client not available"))?;
        //
        // photon.reverse_geocode(latitude, longitude).await
    }

    /// Reverse geocode coordinates to address with fallback chain
    /// Tries each available reverse geocoder in priority order until one succeeds
    pub async fn reverse_geocode(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<ReverseGeocodeResult> {
        // Validate coordinates
        if !(-90.0..=90.0).contains(&latitude) {
            return Err(anyhow!("Invalid latitude: {}", latitude));
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err(anyhow!("Invalid longitude: {}", longitude));
        }

        if self.reverse_geocoders.is_empty() {
            return Err(anyhow!(
                "No reverse geocoding services available for ({}, {})",
                latitude,
                longitude
            ));
        }

        debug!(
            "Reverse geocoding coordinates: ({}, {})",
            latitude, longitude
        );

        let mut last_error = None;

        // Try each reverse geocoder in priority order
        for (service, geocoder) in &self.reverse_geocoders {
            match geocoder.reverse_geocode(latitude, longitude).await {
                Ok(result) => {
                    debug!(
                        "Successfully reverse geocoded with {:?}: ({}, {})",
                        service, latitude, longitude
                    );
                    return Ok(result);
                }
                Err(e) => {
                    debug!(
                        "Reverse geocoding failed with {:?} for ({}, {}): {}",
                        service, latitude, longitude, e
                    );
                    last_error = Some(e);
                    // Continue to next service
                }
            }
        }

        // All services failed
        Err(last_error.unwrap_or_else(|| {
            anyhow!(
                "All reverse geocoding services failed for ({}, {})",
                latitude,
                longitude
            )
        }))
    }

    /// Geocode an address string to a WGS84 coordinate with fallback chain
    /// Tries each available forward geocoder in priority order until one succeeds
    /// Note: Pelias is not used for forward geocoding, only reverse geocoding
    pub async fn geocode_address(&self, address: &str) -> Result<GeocodeResult> {
        if address.trim().is_empty() {
            return Err(anyhow!("Address cannot be empty"));
        }

        if self.forward_geocoders.is_empty() {
            return Err(anyhow!("No geocoding services available for '{}'", address));
        }

        debug!("Geocoding address: {}", address);

        let mut last_error = None;

        // Try each forward geocoder in priority order
        for (service, geocoder) in &self.forward_geocoders {
            match geocoder.geocode(address).await {
                Ok(point) => {
                    debug!("Successfully geocoded with {:?}: {}", service, address);
                    return Ok(GeocodeResult::new(point, *service));
                }
                Err(e) => {
                    debug!(
                        "Geocoding failed with {:?} for '{}': {}",
                        service, address, e
                    );
                    last_error = Some(e);
                    // Continue to next service
                }
            }
        }

        // All services failed
        Err(last_error
            .unwrap_or_else(|| anyhow!("All geocoding services failed for '{}'", address)))
    }
}

/// Convenience function to geocode a single address (returns just the point for backwards compatibility)
/// Uses batch geocoding strategy (Nominatim → Google Maps)
pub async fn geocode(address: &str) -> Result<Point> {
    let geocoder = Geocoder::new_batch_geocoding();
    let result = geocoder.geocode_address(address).await?;
    Ok(result.point)
}

/// Geocode address components into a single address string and then to coordinates
/// Returns both the point and which service was used
pub async fn geocode_components(
    street1: Option<&str>,
    street2: Option<&str>,
    city: Option<&str>,
    state: Option<&str>,
    zip_code: Option<&str>,
    country: Option<&str>,
) -> Result<GeocodeResult> {
    let mut parts = Vec::new();

    if let Some(street1) = street1
        && !street1.trim().is_empty()
    {
        parts.push(street1.trim());
    }

    if let Some(street2) = street2
        && !street2.trim().is_empty()
    {
        parts.push(street2.trim());
    }

    if let Some(city) = city
        && !city.trim().is_empty()
    {
        parts.push(city.trim());
    }

    if let Some(state) = state
        && !state.trim().is_empty()
    {
        parts.push(state.trim());
    }

    if let Some(zip) = zip_code
        && !zip.trim().is_empty()
    {
        parts.push(zip.trim());
    }

    // Add country if provided, defaulting to US if not specified and we have other components
    if let Some(country_str) = country {
        if !country_str.trim().is_empty() {
            parts.push(country_str.trim());
        }
    } else if !parts.is_empty() {
        // Only add default country if we have other address components
        parts.push("United States");
    }

    if parts.is_empty() {
        return Err(anyhow!("No address components provided"));
    }

    let address = parts.join(", ");
    let geocoder = Geocoder::new_batch_geocoding();
    geocoder.geocode_address(&address).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geocoder_creation() {
        let geocoder = Geocoder::new_batch_geocoding();
        // Nominatim should always be in the forward and reverse geocoder lists
        assert!(
            geocoder
                .forward_geocoders
                .iter()
                .any(|(service, _)| *service == GeocodingService::Nominatim)
        );
        assert!(
            geocoder
                .reverse_geocoders
                .iter()
                .any(|(service, _)| *service == GeocodingService::Nominatim)
        );

        // Google Maps client should be in lists only if API key is set
        if env::var("GOOGLE_MAPS_API_KEY").is_ok() {
            assert!(
                geocoder
                    .forward_geocoders
                    .iter()
                    .any(|(service, _)| *service == GeocodingService::GoogleMaps)
            );
        }
    }

    #[test]
    fn test_custom_geocoder() {
        let geocoder =
            Geocoder::with_settings("https://example.com".to_string(), "Test Agent".to_string());
        // Nominatim should always be in the forward and reverse geocoder lists
        assert!(
            geocoder
                .forward_geocoders
                .iter()
                .any(|(service, _)| *service == GeocodingService::Nominatim)
        );
        assert!(
            geocoder
                .reverse_geocoders
                .iter()
                .any(|(service, _)| *service == GeocodingService::Nominatim)
        );

        // Google Maps client availability depends on environment variable
        if env::var("GOOGLE_MAPS_API_KEY").is_ok() {
            assert!(
                geocoder
                    .forward_geocoders
                    .iter()
                    .any(|(service, _)| *service == GeocodingService::GoogleMaps)
            );
        }
    }

    #[tokio::test]
    async fn test_geocode_components() {
        // This test would require network access to Nominatim
        // In a real test environment, you might want to use a mock server

        // Test empty components
        let result = geocode_components(None, None, None, None, None, None).await;
        assert!(result.is_err());

        // Test with some components (would need network to actually test)
        // let result = geocode_components(
        //     Some("1600 Pennsylvania Avenue"),
        //     None,
        //     Some("Washington"),
        //     Some("DC"),
        //     Some("20500"),
        //     Some("United States"),
        // ).await;
        // assert!(result.is_ok());
    }

    #[test]
    fn test_point_validation() {
        // Test that Point creation works
        let point = Point::new(40.7128, -74.0060); // New York City
        assert_eq!(point.latitude, 40.7128);
        assert_eq!(point.longitude, -74.0060);
    }
}
