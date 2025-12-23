use anyhow::{Result, anyhow};
use google_maps::Client as GoogleMapsClient;
use num_traits::ToPrimitive;
use tracing::debug;

use crate::locations::Point;

use super::ReverseGeocodeResult;

pub struct GoogleMapsGeocoderClient {
    client: GoogleMapsClient,
}

impl GoogleMapsGeocoderClient {
    pub fn new(client: GoogleMapsClient) -> Self {
        Self { client }
    }

    /// Geocode an address using Google Maps
    pub async fn geocode(&self, address: &str) -> Result<Point> {
        debug!("Geocoding address with Google Maps: {}", address);

        let geocoding_response = self
            .client
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

    /// Reverse geocode coordinates using Google Maps
    pub async fn reverse_geocode(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<ReverseGeocodeResult> {
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

        let geocoding_response = self
            .client
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
}

// Trait implementations
use super::{ForwardGeocoder, ReverseGeocoder};
use async_trait::async_trait;

#[async_trait]
impl ForwardGeocoder for GoogleMapsGeocoderClient {
    async fn geocode(&self, address: &str) -> Result<Point> {
        self.geocode(address).await
    }
}

#[async_trait]
impl ReverseGeocoder for GoogleMapsGeocoderClient {
    async fn reverse_geocode(&self, latitude: f64, longitude: f64) -> Result<ReverseGeocodeResult> {
        self.reverse_geocode(latitude, longitude).await
    }
}
