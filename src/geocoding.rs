use anyhow::{Result, anyhow};
use google_maps::Client as GoogleMapsClient;
use num_traits::ToPrimitive;
use reqwest;
use serde::Deserialize;
use std::env;
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::locations::Point;

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

// Nominatim API response structure
#[derive(Debug, Deserialize)]
struct NominatimResponse {
    lat: String,
    lon: String,
    display_name: String,
    #[allow(dead_code)]
    importance: Option<f64>,
    #[serde(rename = "place_id")]
    #[allow(dead_code)]
    place_id: Option<i64>,
}

// Nominatim reverse geocoding response structure
#[derive(Debug, Deserialize)]
struct NominatimReverseResponse {
    #[allow(dead_code)]
    lat: String,
    #[allow(dead_code)]
    lon: String,
    display_name: String,
    address: NominatimAddress,
}

#[derive(Debug, Deserialize)]
struct NominatimAddress {
    #[serde(default)]
    house_number: Option<String>,
    #[serde(default)]
    road: Option<String>,
    #[serde(default)]
    suburb: Option<String>,
    #[serde(default)]
    city: Option<String>,
    #[serde(default)]
    town: Option<String>,
    #[serde(default)]
    village: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    county: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    postcode: Option<String>,
    #[serde(default)]
    country: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    country_code: Option<String>,
}

// Reverse geocoding result
#[derive(Debug, Clone)]
pub struct ReverseGeocodeResult {
    pub street1: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip_code: Option<String>,
    pub country: Option<String>,
    pub display_name: String,
}

pub struct Geocoder {
    client: reqwest::Client,
    base_url: String,
    user_agent: String,
    google_maps_client: Option<GoogleMapsClient>,
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

        // Initialize Google Maps client if API key is available
        let google_maps_client = if let Ok(api_key) = env::var("GOOGLE_MAPS_API_KEY") {
            if !api_key.trim().is_empty() {
                debug!("Initializing Google Maps client as geocoding fallback");
                match GoogleMapsClient::try_new(&api_key) {
                    Ok(client) => Some(client),
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
            client,
            base_url: "https://nominatim.openstreetmap.org".to_string(),
            user_agent: "SOAR Aircraft Geocoder/1.0 (https://github.com/hut8/soar)".to_string(),
            google_maps_client,
        }
    }

    /// Create a new Geocoder with custom settings
    pub fn with_settings(base_url: String, user_agent: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        // Initialize Google Maps client if API key is available
        let google_maps_client = if let Ok(api_key) = env::var("GOOGLE_MAPS_API_KEY") {
            if !api_key.trim().is_empty() {
                debug!("Initializing Google Maps client as geocoding fallback");
                match GoogleMapsClient::try_new(&api_key) {
                    Ok(client) => Some(client),
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
            client,
            base_url,
            user_agent,
            google_maps_client,
        }
    }

    /// Geocode an address using Nominatim
    async fn geocode_with_nominatim(&self, address: &str) -> Result<Point> {
        debug!("Geocoding address with Nominatim: {}", address);

        let url = format!("{}/search", self.base_url);

        let params = [
            ("q", address),
            ("format", "json"),
            ("limit", "1"),
            ("addressdetails", "1"),
        ];

        let response = self
            .client
            .get(&url)
            .query(&params)
            .header("User-Agent", &self.user_agent)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send geocoding request: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Geocoding request failed with status: {}",
                response.status()
            ));
        }

        let results: Vec<NominatimResponse> = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse geocoding response: {}", e))?;

        if results.is_empty() {
            return Err(anyhow!(
                "No geocoding results found for address: {}",
                address
            ));
        }

        let result = &results[0];

        let latitude: f64 = result
            .lat
            .parse()
            .map_err(|e| anyhow!("Invalid latitude in response: {}", e))?;
        let longitude: f64 = result
            .lon
            .parse()
            .map_err(|e| anyhow!("Invalid longitude in response: {}", e))?;

        // Validate coordinates are reasonable
        if !(-90.0..=90.0).contains(&latitude) {
            return Err(anyhow!("Invalid latitude: {}", latitude));
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err(anyhow!("Invalid longitude: {}", longitude));
        }

        debug!(
            "Nominatim geocoded '{}' to ({}, {}) - {}",
            address, latitude, longitude, result.display_name
        );

        Ok(Point::new(latitude, longitude))
    }

    /// Reverse geocode coordinates using Nominatim
    async fn reverse_geocode_with_nominatim(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<ReverseGeocodeResult> {
        debug!(
            "Reverse geocoding coordinates with Nominatim: ({}, {})",
            latitude, longitude
        );

        let url = format!("{}/reverse", self.base_url);

        let params = [
            ("lat", latitude.to_string()),
            ("lon", longitude.to_string()),
            ("format", "json".to_string()),
            ("addressdetails", "1".to_string()),
        ];

        let response = self
            .client
            .get(&url)
            .query(&params)
            .header("User-Agent", &self.user_agent)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send reverse geocoding request: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Reverse geocoding request failed with status: {}",
                response.status()
            ));
        }

        let result: NominatimReverseResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse reverse geocoding response: {}", e))?;

        // Build street address from components
        let street1 = if let Some(house) = &result.address.house_number {
            if let Some(road) = &result.address.road {
                Some(format!("{} {}", house, road))
            } else {
                result.address.road.clone()
            }
        } else {
            result.address.road.clone()
        };

        // Pick the best city name (city > town > village > suburb)
        let city = result
            .address
            .city
            .or_else(|| result.address.town.clone())
            .or_else(|| result.address.village.clone())
            .or_else(|| result.address.suburb.clone());

        debug!(
            "Nominatim reverse geocoded ({}, {}) to {}",
            latitude, longitude, result.display_name
        );

        Ok(ReverseGeocodeResult {
            street1,
            city,
            state: result.address.state,
            zip_code: result.address.postcode,
            country: result.address.country,
            display_name: result.display_name,
        })
    }

    /// Reverse geocode coordinates using Google Maps as fallback
    async fn reverse_geocode_with_google_maps(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<ReverseGeocodeResult> {
        let google_client = self
            .google_maps_client
            .as_ref()
            .ok_or_else(|| anyhow!("Google Maps client not available"))?;

        debug!(
            "Reverse geocoding coordinates with Google Maps: ({}, {})",
            latitude, longitude
        );

        use rust_decimal::Decimal;
        use std::str::FromStr;

        let lat_decimal = Decimal::from_str(&latitude.to_string())
            .map_err(|e| anyhow!("Failed to convert latitude to Decimal: {}", e))?;
        let lng_decimal = Decimal::from_str(&longitude.to_string())
            .map_err(|e| anyhow!("Failed to convert longitude to Decimal: {}", e))?;

        let latlng = google_maps::LatLng::try_from_dec(lat_decimal, lng_decimal)
            .map_err(|e| anyhow!("Invalid coordinates for Google Maps: {}", e))?;

        let geocoding_response = google_client
            .reverse_geocoding(latlng)
            .execute()
            .await
            .map_err(|e| anyhow!("Google Maps reverse geocoding request failed: {}", e))?;

        if geocoding_response.results.is_empty() {
            // No results is normal for remote/ocean locations - return empty result without error
            debug!(
                "No Google Maps reverse geocoding results found for coordinates: ({}, {}) - likely remote/ocean location",
                latitude, longitude
            );
            return Ok(ReverseGeocodeResult {
                street1: None,
                city: None,
                state: None,
                zip_code: None,
                country: None,
                display_name: format!("{}, {}", latitude, longitude),
            });
        }

        let result = &geocoding_response.results[0];
        let display_name = result.formatted_address.clone();

        // Extract address components
        let mut street_number: Option<String> = None;
        let mut route: Option<String> = None;
        let mut city: Option<String> = None;
        let mut state: Option<String> = None;
        let mut zip_code: Option<String> = None;
        let mut country: Option<String> = None;

        use google_maps::PlaceType;

        for component in &result.address_components {
            let types = &component.types;
            let long_name = &component.long_name;

            if types.contains(&PlaceType::StreetNumber) {
                street_number = Some(long_name.clone());
            } else if types.contains(&PlaceType::Route) {
                route = Some(long_name.clone());
            } else if types.contains(&PlaceType::Locality) {
                city = Some(long_name.clone());
            } else if types.contains(&PlaceType::AdministrativeAreaLevel1) {
                state = Some(component.short_name.clone());
            } else if types.contains(&PlaceType::PostalCode) {
                zip_code = Some(long_name.clone());
            } else if types.contains(&PlaceType::Country) {
                country = Some(long_name.clone());
            }
        }

        // Build street address
        let street1 = match (street_number, route) {
            (Some(num), Some(rd)) => Some(format!("{} {}", num, rd)),
            (None, Some(rd)) => Some(rd),
            (Some(num), None) => Some(num),
            (None, None) => None,
        };

        debug!(
            "Google Maps reverse geocoded ({}, {}) to {}",
            latitude, longitude, display_name
        );

        Ok(ReverseGeocodeResult {
            street1,
            city,
            state,
            zip_code,
            country,
            display_name,
        })
    }

    /// Reverse geocode coordinates to address with Google Maps fallback
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

        // First try Nominatim
        match self
            .reverse_geocode_with_nominatim(latitude, longitude)
            .await
        {
            Ok(result) => {
                debug!(
                    "Successfully reverse geocoded with Nominatim: ({}, {})",
                    latitude, longitude
                );
                Ok(result)
            }
            Err(nominatim_error) => {
                warn!(
                    "Nominatim reverse geocoding failed for ({}, {}): {}",
                    latitude, longitude, nominatim_error
                );

                // Try Google Maps as fallback if available
                if self.google_maps_client.is_some() {
                    info!(
                        "Attempting Google Maps fallback for reverse geocoding: ({}, {})",
                        latitude, longitude
                    );
                    match self
                        .reverse_geocode_with_google_maps(latitude, longitude)
                        .await
                    {
                        Ok(result) => {
                            info!(
                                "Successfully reverse geocoded with Google Maps fallback: ({}, {})",
                                latitude, longitude
                            );
                            Ok(result)
                        }
                        Err(google_error) => {
                            warn!(
                                "Google Maps reverse geocoding also failed for ({}, {}): {}",
                                latitude, longitude, google_error
                            );
                            Err(anyhow!(
                                "Both Nominatim and Google Maps reverse geocoding failed for ({}, {}). Nominatim error: {}. Google Maps error: {}",
                                latitude,
                                longitude,
                                nominatim_error,
                                google_error
                            ))
                        }
                    }
                } else {
                    Err(anyhow!(
                        "Nominatim reverse geocoding failed for ({}, {}) and Google Maps fallback not available: {}",
                        latitude,
                        longitude,
                        nominatim_error
                    ))
                }
            }
        }
    }

    /// Geocode an address using Google Maps as fallback
    async fn geocode_with_google_maps(&self, address: &str) -> Result<Point> {
        let google_client = self
            .google_maps_client
            .as_ref()
            .ok_or_else(|| anyhow!("Google Maps client not available"))?;

        debug!("Geocoding address with Google Maps: {}", address);

        let geocoding_response = google_client
            .geocoding()
            .with_address(address)
            .execute()
            .await
            .map_err(|e| anyhow!("Google Maps geocoding request failed: {}", e))?;

        if geocoding_response.results.is_empty() {
            return Err(anyhow!(
                "No Google Maps geocoding results found for address: {}",
                address
            ));
        }

        let result = &geocoding_response.results[0];
        let location = &result.geometry.location;

        let latitude = location
            .latitude()
            .to_f64()
            .ok_or_else(|| anyhow!("Failed to convert latitude to f64"))?;
        let longitude = location
            .longitude()
            .to_f64()
            .ok_or_else(|| anyhow!("Failed to convert longitude to f64"))?;

        // Validate coordinates are reasonable
        if !(-90.0..=90.0).contains(&latitude) {
            return Err(anyhow!("Invalid latitude from Google Maps: {}", latitude));
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err(anyhow!("Invalid longitude from Google Maps: {}", longitude));
        }

        debug!(
            "Google Maps geocoded '{}' to ({}, {})",
            address, latitude, longitude
        );

        Ok(Point::new(latitude, longitude))
    }

    /// Geocode an address string to a WGS84 coordinate with Google Maps fallback
    pub async fn geocode_address(&self, address: &str) -> Result<Point> {
        if address.trim().is_empty() {
            return Err(anyhow!("Address cannot be empty"));
        }

        debug!("Geocoding address: {}", address);

        // First try Nominatim
        match self.geocode_with_nominatim(address).await {
            Ok(point) => {
                debug!("Successfully geocoded with Nominatim: {}", address);
                Ok(point)
            }
            Err(nominatim_error) => {
                warn!(
                    "Nominatim geocoding failed for '{}': {}",
                    address, nominatim_error
                );

                // Try Google Maps as fallback if available
                if self.google_maps_client.is_some() {
                    info!("Attempting Google Maps fallback for: {}", address);
                    match self.geocode_with_google_maps(address).await {
                        Ok(point) => {
                            info!(
                                "Successfully geocoded with Google Maps fallback: {}",
                                address
                            );
                            Ok(point)
                        }
                        Err(google_error) => {
                            warn!(
                                "Google Maps geocoding also failed for '{}': {}",
                                address, google_error
                            );
                            Err(anyhow!(
                                "Both Nominatim and Google Maps geocoding failed for '{}'. Nominatim error: {}. Google Maps error: {}",
                                address,
                                nominatim_error,
                                google_error
                            ))
                        }
                    }
                } else {
                    Err(anyhow!(
                        "Nominatim geocoding failed for '{}' and Google Maps fallback not available: {}",
                        address,
                        nominatim_error
                    ))
                }
            }
        }
    }

    /// Geocode multiple addresses with rate limiting
    pub async fn geocode_addresses(&self, addresses: Vec<String>) -> Vec<(String, Result<Point>)> {
        let mut results = Vec::new();

        for address in addresses {
            let result = self.geocode_address(&address).await;
            results.push((address.clone(), result));

            // Rate limiting: Nominatim requests max 1 request per second
            // We'll be conservative and wait 1.1 seconds
            tokio::time::sleep(Duration::from_millis(1100)).await;
        }

        results
    }

    /// Geocode an address with retries and exponential backoff
    pub async fn geocode_address_with_retry(
        &self,
        address: &str,
        max_retries: u32,
    ) -> Result<Point> {
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match self.geocode_address(address).await {
                Ok(point) => return Ok(point),
                Err(e) => {
                    last_error = Some(e);

                    if attempt < max_retries {
                        let delay = Duration::from_millis(1000 * (2_u64.pow(attempt)));
                        warn!(
                            "Geocoding attempt {} failed for '{}', retrying in {:?}",
                            attempt + 1,
                            address,
                            delay
                        );
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("All geocoding attempts failed")))
    }
}

/// Convenience function to geocode a single address
pub async fn geocode(address: &str) -> Result<Point> {
    let geocoder = Geocoder::new();
    geocoder.geocode_address(address).await
}

/// Geocode address components into a single address string and then to coordinates
pub async fn geocode_components(
    street1: Option<&str>,
    street2: Option<&str>,
    city: Option<&str>,
    state: Option<&str>,
    zip_code: Option<&str>,
    country: Option<&str>,
) -> Result<Point> {
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
    geocode(&address).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geocoder_creation() {
        let geocoder = Geocoder::new();
        assert!(geocoder.base_url.contains("nominatim"));
        assert!(geocoder.user_agent.contains("SOAR"));
        // Google Maps client should be None unless API key is set
        if env::var("GOOGLE_MAPS_API_KEY").is_ok() {
            assert!(geocoder.google_maps_client.is_some());
        } else {
            assert!(geocoder.google_maps_client.is_none());
        }
    }

    #[test]
    fn test_custom_geocoder() {
        let geocoder =
            Geocoder::with_settings("https://example.com".to_string(), "Test Agent".to_string());
        assert_eq!(geocoder.base_url, "https://example.com");
        assert_eq!(geocoder.user_agent, "Test Agent");
        // Google Maps client availability depends on environment variable
        if env::var("GOOGLE_MAPS_API_KEY").is_ok() {
            assert!(geocoder.google_maps_client.is_some());
        } else {
            assert!(geocoder.google_maps_client.is_none());
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
