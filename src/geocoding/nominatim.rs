use anyhow::{Result, anyhow};
use reqwest;
use serde::Deserialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, error};

use crate::locations::Point;

use super::ReverseGeocodeResult;

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

#[derive(Clone)]
pub struct NominatimClient {
    client: reqwest::Client,
    base_url: String,
    user_agent: String,
    /// Rate limiter for Nominatim API (1 request per second)
    last_request: Arc<Mutex<Option<Instant>>>,
}

impl NominatimClient {
    pub fn new(client: reqwest::Client, base_url: String, user_agent: String) -> Self {
        Self {
            client,
            base_url,
            user_agent,
            last_request: Arc::new(Mutex::new(None)),
        }
    }

    /// Enforce Nominatim rate limit of 1 request per second
    /// This MUST be called before every Nominatim API request
    async fn enforce_rate_limit(&self) {
        let mut last_request = self.last_request.lock().await;

        if let Some(last_time) = *last_request {
            let elapsed = last_time.elapsed();
            let min_interval = Duration::from_secs(1);

            if elapsed < min_interval {
                let sleep_duration = min_interval - elapsed;
                debug!(
                    "Nominatim rate limit: sleeping for {:?} to respect 1 req/sec limit",
                    sleep_duration
                );
                tokio::time::sleep(sleep_duration).await;
            }
        }

        // Update the timestamp to now
        *last_request = Some(Instant::now());
    }

    /// Geocode an address using Nominatim (with rate limiting respect)
    pub async fn geocode(&self, address: &str) -> Result<Point> {
        debug!("Geocoding address with Nominatim: {}", address);

        // CRITICAL: Enforce rate limit BEFORE making the request
        self.enforce_rate_limit().await;

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

        let status = response.status();

        // Special handling for 403 Forbidden (rate limit violation)
        if status == reqwest::StatusCode::FORBIDDEN {
            let error_msg = format!(
                "Nominatim geocoding request forbidden (403) for address '{}' - possible rate limit violation",
                address
            );
            error!(address = %address, "Nominatim geocoding request forbidden (403) - possible rate limit violation");

            // Report to Sentry

            return Err(anyhow!(error_msg));
        }

        if !status.is_success() {
            return Err(anyhow!(
                "Nominatim geocoding request failed with status: {}",
                status
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
    pub async fn reverse_geocode(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<ReverseGeocodeResult> {
        debug!(
            "Reverse geocoding coordinates with Nominatim: ({}, {})",
            latitude, longitude
        );

        // CRITICAL: Enforce rate limit BEFORE making the request
        self.enforce_rate_limit().await;

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

        let status = response.status();

        // Special handling for 403 Forbidden (rate limit violation)
        if status == reqwest::StatusCode::FORBIDDEN {
            let error_msg = format!(
                "Nominatim reverse geocoding request forbidden (403) for coordinates ({}, {}) - possible rate limit violation",
                latitude, longitude
            );
            error!(latitude = %latitude, longitude = %longitude, "Nominatim reverse geocoding request forbidden (403) - possible rate limit violation");

            // Report to Sentry

            return Err(anyhow!(error_msg));
        }

        if !status.is_success() {
            return Err(anyhow!(
                "Nominatim reverse geocoding request failed with status: {}",
                status
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
}

// Trait implementations
use super::{ForwardGeocoder, ReverseGeocoder};
use async_trait::async_trait;

#[async_trait]
impl ForwardGeocoder for NominatimClient {
    async fn geocode(&self, address: &str) -> Result<Point> {
        self.geocode(address).await
    }
}

#[async_trait]
impl ReverseGeocoder for NominatimClient {
    async fn reverse_geocode(&self, latitude: f64, longitude: f64) -> Result<ReverseGeocodeResult> {
        self.reverse_geocode(latitude, longitude).await
    }
}
