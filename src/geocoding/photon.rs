//! Photon geocoding client (CURRENTLY DISABLED)
//!
//! Photon is a geocoding service based on OpenStreetMap data that can be self-hosted.
//! This module is currently disabled for production use but kept for potential future use.
//!
//! # Why Photon is Disabled
//!
//! ## 1. Nearest-Point Matching Inaccuracy
//!
//! Photon (like Nominatim) uses **nearest-point matching** instead of **point-in-polygon**
//! containment for reverse geocoding. This means it finds the nearest known geographic feature
//! (like a building or city center) and returns that object's address, rather than determining
//! which administrative boundary actually contains the query point.
//!
//! ### Layer Filtering Problems
//!
//! Photon supports filtering by layer (e.g., `layer=city`) via the API:
//! <https://github.com/komoot/photon?tab=readme-ov-file#filter-results-by-layer>
//!
//! However, this creates significant accuracy issues:
//! - **Example**: A coordinate in Albany, NY might return "Troy, NY" if the Troy city center
//!   is geometrically closer than Albany's city center, even if the point is actually within
//!   Albany's boundaries
//! - Requires **large search radii** (our implementation uses progressive 1km/5km/10km retries)
//!   to ensure we always find *some* city, making queries inefficient
//! - **No guarantee** the returned city is the one that actually contains the point
//!
//! ## 2. Schema Inconsistency Across Entity Types
//!
//! To work around the nearest-point issue, we could avoid filtering by layer and let Photon
//! return any nearby object (buildings, streets, POIs, etc.). However, this creates new problems:
//!
//! - **Data Import**: Requires importing significantly more OSM data (every building, street,
//!   and POI, not just administrative boundaries)
//! - **Inconsistent Schema**: Different entity types have different field structures:
//!   - For `osm_value="city"`: city name is in the `"name"` field
//!   - For `osm_value="house"` or buildings: city name is in the `"city"` field
//! - **Extraction Complexity**: Makes extracting consistent `"city, state, postal_code, country"`
//!   data extremely cumbersome and error-prone, requiring complex conditional logic based on
//!   entity type
//!
//! ## See Also
//!
//! - Nominatim has the same nearest-point limitation:
//!   <https://nominatim.org/release-docs/latest/api/Faq/#2-when-doing-reverse-search-the-address-details-have-parts-that-dont-contain-the-point-i-was-looking-up>
//! - Photon layer filtering documentation:
//!   <https://github.com/komoot/photon?tab=readme-ov-file#filter-results-by-layer>
//!
//! ## Current Alternatives
//!
//! For now, we use Nominatim, Pelias, and Google Maps geocoding services which handle
//! these complexities internally and provide more consistent address extraction.

use anyhow::{Result, anyhow};
use reqwest;
use serde::Deserialize;
use tracing::debug;

use crate::locations::Point;

use super::ReverseGeocodeResult;

// Photon reverse geocoding response structure (GeoJSON format)
#[derive(Debug, Deserialize)]
struct PhotonResponse {
    features: Vec<PhotonFeature>,
}

#[derive(Debug, Deserialize)]
struct PhotonFeature {
    properties: PhotonProperties,
    #[allow(dead_code)]
    geometry: PhotonGeometry,
}

#[derive(Debug, Deserialize)]
struct PhotonGeometry {
    #[allow(dead_code)]
    coordinates: Vec<f64>,
}

#[derive(Debug, Deserialize)]
struct PhotonProperties {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    street: Option<String>,
    #[serde(default)]
    housenumber: Option<String>,
    #[serde(default)]
    postcode: Option<String>,
    #[serde(default)]
    city: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    country: Option<String>,
    #[serde(default)]
    countrycode: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    osm_key: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    osm_value: Option<String>,
}

pub struct PhotonClient {
    client: reqwest::Client,
    base_url: String,
}

// DISABLED: Photon is temporarily not in use, but keeping code for future use
#[allow(dead_code)]
impl PhotonClient {
    pub fn new(client: reqwest::Client, base_url: String) -> Self {
        Self { client, base_url }
    }

    /// Geocode an address using Photon
    pub async fn geocode(&self, address: &str) -> Result<Point> {
        debug!("Geocoding address with Photon: {}", address);

        let url = format!("{}/api", self.base_url);

        let params = [("q", address), ("limit", "1"), ("lang", "en")];

        let response = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send Photon geocoding request: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Photon geocoding request failed with status: {}",
                response.status()
            ));
        }

        let result: PhotonResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse Photon geocoding response: {}", e))?;

        if result.features.is_empty() {
            return Err(anyhow!(
                "No Photon geocoding results found for address: {}",
                address
            ));
        }

        let feature = &result.features[0];
        let coords = &feature.geometry.coordinates;

        if coords.len() < 2 {
            return Err(anyhow!("Invalid coordinates in Photon response"));
        }

        let longitude = coords[0];
        let latitude = coords[1];

        // Validate coordinates are reasonable
        if !(-90.0..=90.0).contains(&latitude) {
            return Err(anyhow!("Invalid latitude from Photon: {}", latitude));
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err(anyhow!("Invalid longitude from Photon: {}", longitude));
        }

        debug!(
            "Photon geocoded '{}' to ({}, {})",
            address, latitude, longitude
        );

        Ok(Point::new(latitude, longitude))
    }

    /// Reverse geocode coordinates using Photon (local geocoding server)
    /// Uses a progressive search radius strategy: tries exact location first,
    /// then expands radius if no results found (1km, 5km, 10km)
    pub async fn reverse_geocode(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<ReverseGeocodeResult> {
        debug!(
            "Reverse geocoding coordinates with Photon: ({}, {})",
            latitude, longitude
        );

        // Try with increasing radius to be more tolerant of sparse OSM data
        // This helps in rural areas or places without detailed mapping
        let radii_km = [None, Some(1.0), Some(5.0), Some(10.0)];

        for (attempt, radius) in radii_km.iter().enumerate() {
            let url = format!("{}/reverse", self.base_url);

            let mut params = vec![
                ("lon", longitude.to_string()),
                ("lat", latitude.to_string()),
                ("limit", "1".to_string()),
                ("lang", "en".to_string()),
            ];

            if let Some(r) = radius {
                params.push(("radius", r.to_string()));
                if attempt > 0 {
                    debug!(
                        "Photon retry attempt {} with radius {}km for ({}, {})",
                        attempt, r, latitude, longitude
                    );
                }
            }

            let response = self
                .client
                .get(&url)
                .query(&params)
                .send()
                .await
                .map_err(|e| anyhow!("Failed to send Photon reverse geocoding request: {}", e))?;

            if !response.status().is_success() {
                return Err(anyhow!(
                    "Photon reverse geocoding request failed with status: {}",
                    response.status()
                ));
            }

            let result: PhotonResponse = response
                .json()
                .await
                .map_err(|e| anyhow!("Failed to parse Photon reverse geocoding response: {}", e))?;

            if !result.features.is_empty() {
                // Success! Process the result and track which radius worked
                let radius_label = match radius {
                    None => "exact",
                    Some(1.0) => "1",
                    Some(5.0) => "5",
                    Some(10.0) => "10",
                    _ => "other",
                };
                metrics::counter!("flight_tracker.location.photon.retry_total", "radius_km" => radius_label)
                    .increment(1);

                if let Some(r) = radius {
                    debug!(
                        "Photon found result with radius {}km for ({}, {})",
                        r, latitude, longitude
                    );
                }
                return self.parse_photon_result(&result, latitude, longitude);
            }

            // No results with this radius, try next iteration
        }

        // All attempts failed
        Err(anyhow!(
            "No Photon reverse geocoding results found for coordinates: ({}, {}) even with 10km radius",
            latitude,
            longitude
        ))
    }

    /// Parse Photon response into ReverseGeocodeResult
    /// Extracted to deduplicate code between retry attempts
    fn parse_photon_result(
        &self,
        result: &PhotonResponse,
        latitude: f64,
        longitude: f64,
    ) -> Result<ReverseGeocodeResult> {
        if result.features.is_empty() {
            return Err(anyhow!("No features in Photon response"));
        }

        let feature = &result.features[0];
        let props = &feature.properties;

        // Build street address from components
        let street1 = if let Some(house) = &props.housenumber {
            if let Some(street) = &props.street {
                Some(format!("{} {}", house, street))
            } else {
                props.street.clone()
            }
        } else {
            props.street.clone()
        };

        // Build display name from city, state, country ONLY (ignore street addresses and generic names)
        // This is appropriate for flight start/end locations where we want locality-level precision
        let display_name = {
            let mut parts = Vec::new();
            if let Some(c) = &props.city {
                parts.push(c.clone());
            }
            if let Some(s) = &props.state {
                parts.push(s.clone());
            }
            if let Some(country) = &props.countrycode {
                parts.push(country.clone());
            }
            if parts.is_empty() {
                // Fallback: use name if available, otherwise coordinates
                if let Some(name) = &props.name {
                    name.clone()
                } else {
                    format!("{}, {}", latitude, longitude)
                }
            } else {
                parts.join(", ")
            }
        };

        debug!(
            "Photon reverse geocoded ({}, {}) to {}",
            latitude, longitude, display_name
        );

        Ok(ReverseGeocodeResult {
            street1,
            city: props.city.clone(),
            state: props.state.clone(),
            zip_code: props.postcode.clone(),
            country: props.countrycode.clone(),
            display_name,
        })
    }
}

// Trait implementations
use super::{ForwardGeocoder, ReverseGeocoder};
use async_trait::async_trait;

#[async_trait]
impl ForwardGeocoder for PhotonClient {
    async fn geocode(&self, address: &str) -> Result<Point> {
        self.geocode(address).await
    }
}

#[async_trait]
impl ReverseGeocoder for PhotonClient {
    async fn reverse_geocode(&self, latitude: f64, longitude: f64) -> Result<ReverseGeocodeResult> {
        self.reverse_geocode(latitude, longitude).await
    }
}
