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
use tracing::{debug, info, warn};

use crate::locations::Point;

use self::google_maps::GoogleMapsGeocoderClient;
use nominatim::NominatimClient;
use pelias::PeliasClient;
use photon::PhotonClient;

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
/// ### Basic usage without Google Maps fallback:
/// ```rust,no_run
/// use soar::geocoding::Geocoder;
///
/// # async fn example() -> anyhow::Result<()> {
/// let geocoder = Geocoder::new();
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
/// let geocoder = Geocoder::new();
/// let point = geocoder.geocode_address("123 Hard to Find Address").await?;
/// // Will try Nominatim first, then Google Maps if it fails
/// # Ok(())
/// # }
/// ```
pub struct Geocoder {
    nominatim: Option<NominatimClient>,
    photon: Option<PhotonClient>,
    pelias: Option<PeliasClient>,
    google_maps: Option<GoogleMapsGeocoderClient>,
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
        let photon = None;
        // let photon = if let Ok(url) = env::var("PHOTON_BASE_URL") {
        //     if !url.trim().is_empty() {
        //         debug!("Using Photon geocoding server at: {}", url);
        //         Some(PhotonClient::new(client.clone(), url.trim().to_string()))
        //     } else {
        //         debug!("PHOTON_BASE_URL is set but empty, skipping Photon");
        //         None
        //     }
        // } else {
        //     debug!("PHOTON_BASE_URL not set, Photon geocoding unavailable");
        //     None
        // };

        // Initialize Pelias client if configured
        let pelias = if let Ok(url) = env::var("PELIAS_BASE_URL") {
            if !url.trim().is_empty() {
                debug!("Using Pelias geocoding server at: {}", url);
                Some(PeliasClient::new(client.clone(), url.trim().to_string()))
            } else {
                debug!("PELIAS_BASE_URL is set but empty, skipping Pelias");
                None
            }
        } else {
            debug!("PELIAS_BASE_URL not set, Pelias geocoding unavailable");
            None
        };

        // Initialize Nominatim client
        let nominatim = Some(NominatimClient::new(
            client.clone(),
            "https://nominatim.openstreetmap.org".to_string(),
            "SOAR Aircraft Geocoder/1.0 (https://github.com/hut8/soar)".to_string(),
        ));

        // Initialize Google Maps client if API key is available
        let google_maps = if let Ok(api_key) = env::var("GOOGLE_MAPS_API_KEY") {
            if !api_key.trim().is_empty() {
                debug!("Initializing Google Maps client as geocoding fallback");
                match GoogleMapsClient::try_new(&api_key) {
                    Ok(client) => Some(GoogleMapsGeocoderClient::new(client)),
                    Err(e) => {
                        warn!("Failed to create Google Maps client: {}", e);
                        None
                    }
                }
            } else {
                debug!("GOOGLE_MAPS_API_KEY is set but empty, skipping Google Maps initialization");
                None
            }
        } else {
            debug!("GOOGLE_MAPS_API_KEY not set, Google Maps fallback unavailable");
            None
        };

        Self {
            nominatim,
            photon,
            pelias,
            google_maps,
        }
    }

    /// Create a new Geocoder with custom settings
    pub fn with_settings(base_url: String, user_agent: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

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
        let photon = None;
        // let photon = if let Ok(url) = env::var("PHOTON_BASE_URL") {
        //     if !url.trim().is_empty() {
        //         debug!("Using Photon geocoding server at: {}", url);
        //         Some(PhotonClient::new(client.clone(), url.trim().to_string()))
        //     } else {
        //         debug!("PHOTON_BASE_URL is set but empty, skipping Photon");
        //         None
        //     }
        // } else {
        //     debug!("PHOTON_BASE_URL not set, Photon geocoding unavailable");
        //     None
        // };

        // Initialize Pelias client if configured
        let pelias = if let Ok(url) = env::var("PELIAS_BASE_URL") {
            if !url.trim().is_empty() {
                debug!("Using Pelias geocoding server at: {}", url);
                Some(PeliasClient::new(client.clone(), url.trim().to_string()))
            } else {
                debug!("PELIAS_BASE_URL is set but empty, skipping Pelias");
                None
            }
        } else {
            debug!("PELIAS_BASE_URL not set, Pelias geocoding unavailable");
            None
        };

        // Initialize Nominatim client with custom settings
        let nominatim = Some(NominatimClient::new(client.clone(), base_url, user_agent));

        // Initialize Google Maps client if API key is available
        let google_maps = if let Ok(api_key) = env::var("GOOGLE_MAPS_API_KEY") {
            if !api_key.trim().is_empty() {
                debug!("Initializing Google Maps client as geocoding fallback");
                match GoogleMapsClient::try_new(&api_key) {
                    Ok(client) => Some(GoogleMapsGeocoderClient::new(client)),
                    Err(e) => {
                        warn!("Failed to create Google Maps client: {}", e);
                        None
                    }
                }
            } else {
                debug!("GOOGLE_MAPS_API_KEY is set but empty, skipping Google Maps initialization");
                None
            }
        } else {
            debug!("GOOGLE_MAPS_API_KEY not set, Google Maps fallback unavailable");
            None
        };

        Self {
            nominatim,
            photon,
            pelias,
            google_maps,
        }
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

    /// Reverse geocode coordinates to address with Photon, Nominatim, and Google Maps fallback
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

        debug!(
            "Reverse geocoding coordinates: ({}, {})",
            latitude, longitude
        );

        // First try Photon (local geocoding server on production/localhost)
        if let Some(photon) = &self.photon {
            match photon.reverse_geocode(latitude, longitude).await {
                Ok(result) => {
                    debug!(
                        "Successfully reverse geocoded with Photon: ({}, {})",
                        latitude, longitude
                    );
                    return Ok(result);
                }
                Err(photon_error) => {
                    debug!(
                        "Photon reverse geocoding failed for ({}, {}): {}",
                        latitude, longitude, photon_error
                    );
                    // Continue to fallback services
                }
            }
        }

        // Try Pelias (city-level geocoding with Who's on First data)
        if let Some(pelias) = &self.pelias {
            match pelias.reverse_geocode(latitude, longitude).await {
                Ok(result) => {
                    debug!(
                        "Successfully reverse geocoded with Pelias: ({}, {})",
                        latitude, longitude
                    );
                    return Ok(result);
                }
                Err(pelias_error) => {
                    debug!(
                        "Pelias reverse geocoding failed for ({}, {}): {}",
                        latitude, longitude, pelias_error
                    );
                    // Continue to fallback services
                }
            }
        }

        // Fallback to Nominatim
        if let Some(nominatim) = &self.nominatim {
            match nominatim.reverse_geocode(latitude, longitude).await {
                Ok(result) => {
                    debug!(
                        "Successfully reverse geocoded with Nominatim: ({}, {})",
                        latitude, longitude
                    );
                    return Ok(result);
                }
                Err(nominatim_error) => {
                    warn!(
                        "Nominatim reverse geocoding failed for ({}, {}): {}",
                        latitude, longitude, nominatim_error
                    );
                    // Continue to Google Maps fallback
                }
            }
        }

        // Try Google Maps as final fallback if available
        if let Some(google_maps) = &self.google_maps {
            info!(
                "Attempting Google Maps fallback for reverse geocoding: ({}, {})",
                latitude, longitude
            );
            match google_maps.reverse_geocode(latitude, longitude).await {
                Ok(result) => {
                    info!(
                        "Successfully reverse geocoded with Google Maps fallback: ({}, {})",
                        latitude, longitude
                    );
                    return Ok(result);
                }
                Err(google_error) => {
                    warn!(
                        "Google Maps reverse geocoding also failed for ({}, {}): {}",
                        latitude, longitude, google_error
                    );
                    return Err(anyhow!(
                        "All reverse geocoding services failed for ({}, {}). Photon, Pelias, Nominatim and Google Maps all failed.",
                        latitude,
                        longitude
                    ));
                }
            }
        }

        Err(anyhow!(
            "No reverse geocoding services available for ({}, {})",
            latitude,
            longitude
        ))
    }

    /// Geocode an address string to a WGS84 coordinate with Photon → Nominatim → Google Maps fallback
    /// Note: Pelias is not used for forward geocoding, only reverse geocoding
    pub async fn geocode_address(&self, address: &str) -> Result<GeocodeResult> {
        if address.trim().is_empty() {
            return Err(anyhow!("Address cannot be empty"));
        }

        debug!("Geocoding address: {}", address);

        // First try Photon if configured
        if let Some(photon) = &self.photon {
            match photon.geocode(address).await {
                Ok(point) => {
                    debug!("Successfully geocoded with Photon: {}", address);
                    return Ok(GeocodeResult::new(point, GeocodingService::Photon));
                }
                Err(photon_error) => {
                    debug!(
                        "Photon geocoding failed for '{}': {}",
                        address, photon_error
                    );
                    // Continue to fallback services
                }
            }
        }

        // Fallback to Nominatim
        if let Some(nominatim) = &self.nominatim {
            match nominatim.geocode(address).await {
                Ok(point) => {
                    debug!("Successfully geocoded with Nominatim: {}", address);
                    return Ok(GeocodeResult::new(point, GeocodingService::Nominatim));
                }
                Err(nominatim_error) => {
                    warn!(
                        "Nominatim geocoding failed for '{}': {}",
                        address, nominatim_error
                    );
                    // Continue to Google Maps fallback
                }
            }
        }

        // Try Google Maps as final fallback if available
        if let Some(google_maps) = &self.google_maps {
            info!("Attempting Google Maps fallback for: {}", address);
            match google_maps.geocode(address).await {
                Ok(point) => {
                    info!(
                        "Successfully geocoded with Google Maps fallback: {}",
                        address
                    );
                    return Ok(GeocodeResult::new(point, GeocodingService::GoogleMaps));
                }
                Err(google_error) => {
                    warn!(
                        "Google Maps geocoding also failed for '{}': {}",
                        address, google_error
                    );
                    return Err(anyhow!(
                        "All geocoding services failed for '{}'. Photon/Nominatim/Google Maps all failed.",
                        address
                    ));
                }
            }
        }

        Err(anyhow!("No geocoding services available for '{}'", address))
    }
}

/// Convenience function to geocode a single address (returns just the point for backwards compatibility)
pub async fn geocode(address: &str) -> Result<Point> {
    let geocoder = Geocoder::new();
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
    let geocoder = Geocoder::new();
    geocoder.geocode_address(&address).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geocoder_creation() {
        let geocoder = Geocoder::new();
        assert!(geocoder.nominatim.is_some());
        // Google Maps client should be None unless API key is set
        if env::var("GOOGLE_MAPS_API_KEY").is_ok() {
            assert!(geocoder.google_maps.is_some());
        } else {
            assert!(geocoder.google_maps.is_none());
        }
    }

    #[test]
    fn test_custom_geocoder() {
        let geocoder =
            Geocoder::with_settings("https://example.com".to_string(), "Test Agent".to_string());
        assert!(geocoder.nominatim.is_some());
        // Google Maps client availability depends on environment variable
        if env::var("GOOGLE_MAPS_API_KEY").is_ok() {
            assert!(geocoder.google_maps.is_some());
        } else {
            assert!(geocoder.google_maps.is_none());
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
