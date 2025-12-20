use anyhow::{Result, anyhow};
use reqwest;
use serde::Deserialize;
use tracing::debug;

use super::ReverseGeocodeResult;

/// Pelias API response structure (GeoJSON format)
#[derive(Debug, Deserialize)]
struct PeliasResponse {
    #[serde(default)]
    features: Vec<PeliasFeature>,
}

#[derive(Debug, Deserialize)]
struct PeliasFeature {
    properties: PeliasProperties,
    #[allow(dead_code)]
    geometry: PeliasGeometry,
}

#[derive(Debug, Deserialize)]
struct PeliasGeometry {
    #[allow(dead_code)]
    coordinates: Vec<f64>,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    geom_type: String,
}

#[derive(Debug, Deserialize)]
struct PeliasProperties {
    /// Formatted label (e.g., "San Francisco, CA, USA")
    #[serde(default)]
    label: Option<String>,
    /// City/locality name
    #[serde(default)]
    locality: Option<String>,
    /// County name
    #[allow(dead_code)]
    #[serde(default)]
    county: Option<String>,
    /// State/region/province name
    #[serde(default)]
    region: Option<String>,
    /// State/region abbreviation (e.g., "CA")
    #[serde(default)]
    region_a: Option<String>,
    /// Country name
    #[serde(default)]
    country: Option<String>,
    /// Country code - 2 letter ISO 3166-1 alpha-2 (e.g., "US", "GB", "JP")
    #[serde(default)]
    country_code: Option<String>,
    /// Country code - 3 letter ISO 3166-1 alpha-3 (e.g., "USA", "GBR", "JPN")
    #[allow(dead_code)]
    #[serde(default)]
    country_a: Option<String>,
    /// Administrative level (locality, region, country, etc.)
    #[serde(default)]
    layer: Option<String>,
    /// Confidence score (0.0 to 1.0)
    #[allow(dead_code)]
    #[serde(default)]
    confidence: Option<f64>,
}

pub struct PeliasClient {
    client: reqwest::Client,
    base_url: String,
}

impl PeliasClient {
    pub fn new(client: reqwest::Client, base_url: String) -> Self {
        Self { client, base_url }
    }

    /// Reverse geocode coordinates using Pelias (city-level only)
    ///
    /// This uses the Pelias reverse geocoding endpoint which leverages the PIP
    /// (Point in Polygon) service to determine which city/region/country a coordinate
    /// falls within.
    ///
    /// Only returns city-level data - no street addresses or POIs.
    pub async fn reverse_geocode(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<ReverseGeocodeResult> {
        debug!(
            "Reverse geocoding coordinates with Pelias: ({}, {})",
            latitude, longitude
        );

        let url = format!("{}/v1/reverse", self.base_url);

        // Pelias uses point.lat and point.lon parameters
        let params = [
            ("point.lat", latitude.to_string()),
            ("point.lon", longitude.to_string()),
            // Focus on administrative boundaries (cities, regions, countries)
            ("layers", "locality,county,region,country".to_string()),
        ];

        let response = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send Pelias reverse geocoding request: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Pelias reverse geocoding request failed with status: {}",
                response.status()
            ));
        }

        let result: PeliasResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse Pelias reverse geocoding response: {}", e))?;

        if result.features.is_empty() {
            return Err(anyhow!(
                "No Pelias reverse geocoding results found for coordinates: ({}, {})",
                latitude,
                longitude
            ));
        }

        // Take the first (most confident) result
        let feature = &result.features[0];
        let props = &feature.properties;

        // Extract city-level information
        let city = props.locality.clone();
        let state = props.region_a.clone().or_else(|| props.region.clone());
        let country = props.country_code.clone().or_else(|| props.country.clone());

        // Build display name from available components
        // Prefer the label if available, otherwise construct from parts
        let display_name = if let Some(label) = &props.label {
            label.clone()
        } else {
            let mut parts = Vec::new();
            if let Some(c) = &props.locality {
                parts.push(c.clone());
            }
            if let Some(s) = &state {
                parts.push(s.clone());
            }
            if let Some(country) = &country {
                parts.push(country.clone());
            }
            if parts.is_empty() {
                format!("{}, {}", latitude, longitude)
            } else {
                parts.join(", ")
            }
        };

        debug!(
            "Pelias reverse geocoded ({}, {}) to {} (layer: {:?})",
            latitude, longitude, display_name, props.layer
        );

        Ok(ReverseGeocodeResult {
            street1: None, // Pelias city-level data doesn't include street addresses
            city,
            state,
            zip_code: None, // Not included in city-level WOF data
            country,
            display_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to parse a Pelias response from JSON test data
    fn parse_test_response(json_data: &str) -> Result<ReverseGeocodeResult> {
        let response: PeliasResponse = serde_json::from_str(json_data)?;

        if response.features.is_empty() {
            return Err(anyhow!("No features in test response"));
        }

        let feature = &response.features[0];
        let props = &feature.properties;

        let city = props.locality.clone();
        let state = props.region_a.clone().or_else(|| props.region.clone());
        let country = props.country_code.clone().or_else(|| props.country.clone());

        let display_name = if let Some(label) = &props.label {
            label.clone()
        } else {
            let mut parts = Vec::new();
            if let Some(c) = &props.locality {
                parts.push(c.clone());
            }
            if let Some(s) = &state {
                parts.push(s.clone());
            }
            if let Some(country) = &country {
                parts.push(country.clone());
            }
            if parts.is_empty() {
                "Unknown".to_string()
            } else {
                parts.join(", ")
            }
        };

        Ok(ReverseGeocodeResult {
            street1: None,
            city,
            state,
            zip_code: None,
            country,
            display_name,
        })
    }

    #[test]
    fn test_parse_pelias_nyc() {
        let json_data = include_str!("test_data/pelias_nyc.json");
        let result = parse_test_response(json_data).expect("Failed to parse NYC response");

        assert_eq!(result.city, Some("New York".to_string()));
        assert_eq!(result.state, Some("NY".to_string()));
        assert_eq!(result.country, Some("US".to_string()));
        assert!(result.display_name.contains("New York"));
        assert!(result.display_name.contains("NY"));
        assert_eq!(result.street1, None);
        assert_eq!(result.zip_code, None);
    }

    #[test]
    fn test_parse_pelias_sf() {
        let json_data = include_str!("test_data/pelias_sf.json");
        let result = parse_test_response(json_data).expect("Failed to parse SF response");

        assert_eq!(result.city, Some("San Francisco".to_string()));
        assert_eq!(result.state, Some("CA".to_string()));
        assert_eq!(result.country, Some("US".to_string()));
        assert!(result.display_name.contains("San Francisco"));
        assert!(result.display_name.contains("CA"));
        assert_eq!(result.street1, None);
        assert_eq!(result.zip_code, None);
    }

    #[test]
    fn test_parse_pelias_london() {
        let json_data = include_str!("test_data/pelias_london.json");
        let result = parse_test_response(json_data).expect("Failed to parse London response");

        assert_eq!(result.city, Some("London".to_string()));
        // London has region "Greater London" but no region_a abbreviation
        assert_eq!(result.state, Some("Greater London".to_string()));
        assert_eq!(result.country, Some("GB".to_string()));
        assert!(result.display_name.contains("London"));
        assert_eq!(result.street1, None);
        assert_eq!(result.zip_code, None);
    }

    #[test]
    fn test_parse_pelias_tokyo() {
        let json_data = include_str!("test_data/pelias_tokyo.json");
        let result = parse_test_response(json_data).expect("Failed to parse Tokyo response");

        // Tokyo returns Setagaya as the locality (district/ward)
        assert_eq!(result.city, Some("Setagaya".to_string()));
        assert_eq!(result.state, Some("TK".to_string())); // Tokyo prefecture code
        assert_eq!(result.country, Some("JP".to_string()));
        assert!(result.display_name.contains("Setagaya"));
        assert_eq!(result.street1, None);
        assert_eq!(result.zip_code, None);
    }

    #[test]
    fn test_pelias_response_empty_features() {
        let json_data = r#"{"features": []}"#;
        let result = parse_test_response(json_data);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No features"));
    }

    #[test]
    fn test_pelias_response_missing_locality() {
        let json_data = r#"{
            "features": [{
                "type": "Feature",
                "geometry": {
                    "type": "Point",
                    "coordinates": [-122.4194, 37.7749]
                },
                "properties": {
                    "region_a": "CA",
                    "country_code": "US",
                    "label": "California, USA"
                }
            }]
        }"#;

        let result = parse_test_response(json_data).expect("Failed to parse response");

        // Should still work without locality
        assert_eq!(result.city, None);
        assert_eq!(result.state, Some("CA".to_string()));
        assert_eq!(result.country, Some("US".to_string()));
        assert_eq!(result.display_name, "California, USA");
    }
}
