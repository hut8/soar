use anyhow::{Result, anyhow};
use reqwest;
use serde::Deserialize;
use std::time::Duration;
use tracing::{debug, warn};

use crate::clubs::Point;

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

pub struct Geocoder {
    client: reqwest::Client,
    base_url: String,
    user_agent: String,
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

        Self {
            client,
            base_url: "https://nominatim.openstreetmap.org".to_string(),
            user_agent: "SOAR Aircraft Geocoder/1.0 (https://github.com/your-repo/soar)"
                .to_string(),
        }
    }

    /// Create a new Geocoder with custom settings
    pub fn with_settings(base_url: String, user_agent: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url,
            user_agent,
        }
    }

    /// Geocode an address string to a WGS84 coordinate
    pub async fn geocode_address(&self, address: &str) -> Result<Point> {
        if address.trim().is_empty() {
            return Err(anyhow!("Address cannot be empty"));
        }

        debug!("Geocoding address: {}", address);

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
            "Geocoded '{}' to ({}, {}) - {}",
            address, latitude, longitude, result.display_name
        );

        Ok(Point::new(latitude, longitude))
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
    }

    #[test]
    fn test_custom_geocoder() {
        let geocoder =
            Geocoder::with_settings("https://example.com".to_string(), "Test Agent".to_string());
        assert_eq!(geocoder.base_url, "https://example.com");
        assert_eq!(geocoder.user_agent, "Test Agent");
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
