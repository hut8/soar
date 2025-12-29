use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use tracing::{debug, info, warn};

use crate::aircraft_images::{
    AircraftImage, AircraftImageSource, AirportDataResponse, PlanespottersResponse,
};

/// Client for fetching aircraft images from airport-data.com API
#[derive(Clone)]
pub struct AircraftImagesClient {
    client: Client,
    base_url: String,
}

impl AircraftImagesClient {
    /// Create a new client
    pub fn new(client: Client) -> Self {
        Self {
            client,
            base_url: "https://airport-data.com".to_string(),
        }
    }

    /// Fetch aircraft images by MODE-S address (6-character hex)
    ///
    /// # Arguments
    /// * `mode_s` - 6-character hex MODE-S code (e.g., "A12B3C")
    /// * `limit` - Maximum number of images to fetch (default: 5)
    pub async fn fetch_by_mode_s(&self, mode_s: &str, limit: u8) -> Result<Vec<AircraftImage>> {
        debug!(
            "Fetching aircraft images from airport-data.com by MODE-S: {} (limit: {})",
            mode_s, limit
        );

        let url = format!("{}/api/ac_thumb.json", self.base_url);

        let response = self
            .client
            .get(&url)
            .query(&[("m", mode_s), ("n", &limit.to_string())])
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .context("Failed to send request to airport-data.com")?;

        let status = response.status();

        // Handle rate limiting
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            warn!("Rate limited by airport-data.com API");
            return Err(anyhow!("Rate limited by airport-data.com API"));
        }

        // Handle other error status codes
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Airport-data.com API error {}: {}", status, body));
        }

        let api_response: AirportDataResponse = response
            .json()
            .await
            .context("Failed to parse airport-data.com API response")?;

        // Check API status field
        if api_response.status != 200 {
            return Err(anyhow!(
                "Airport-data.com API returned error status: {}",
                api_response.status
            ));
        }

        let images: Vec<AircraftImage> = api_response
            .data
            .into_iter()
            .map(|img| img.into())
            .collect();

        info!(
            "Fetched {} images from airport-data.com for MODE-S: {}",
            images.len(),
            mode_s
        );

        Ok(images)
    }

    /// Fetch aircraft images by registration number
    ///
    /// # Arguments
    /// * `registration` - Aircraft registration (e.g., "N8437D")
    /// * `limit` - Maximum number of images to fetch (default: 5)
    pub async fn fetch_by_registration(
        &self,
        registration: &str,
        limit: u8,
    ) -> Result<Vec<AircraftImage>> {
        debug!(
            "Fetching aircraft images from airport-data.com by registration: {} (limit: {})",
            registration, limit
        );

        let url = format!("{}/api/ac_thumb.json", self.base_url);

        let response = self
            .client
            .get(&url)
            .query(&[("r", registration), ("n", &limit.to_string())])
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .context("Failed to send request to airport-data.com")?;

        let status = response.status();

        // Handle rate limiting
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            warn!("Rate limited by airport-data.com API");
            return Err(anyhow!("Rate limited by airport-data.com API"));
        }

        // Handle other error status codes
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Airport-data.com API error {}: {}", status, body));
        }

        let api_response: AirportDataResponse = response
            .json()
            .await
            .context("Failed to parse airport-data.com API response")?;

        // Check API status field
        if api_response.status != 200 {
            return Err(anyhow!(
                "Airport-data.com API returned error status: {}",
                api_response.status
            ));
        }

        let images: Vec<AircraftImage> = api_response
            .data
            .into_iter()
            .map(|img| img.into())
            .collect();

        info!(
            "Fetched {} images from airport-data.com for registration: {}",
            images.len(),
            registration
        );

        Ok(images)
    }

    /// Fetch aircraft images using best available identifier
    /// Tries MODE-S first, falls back to registration if no results
    ///
    /// # Arguments
    /// * `mode_s` - Optional 6-character hex MODE-S code
    /// * `registration` - Optional registration number
    /// * `limit` - Maximum number of images to fetch (default: 5)
    pub async fn fetch_images(
        &self,
        mode_s: Option<&str>,
        registration: Option<&str>,
        limit: u8,
    ) -> Result<Vec<AircraftImage>> {
        // Try MODE-S first if available
        if let Some(ms) = mode_s {
            match self.fetch_by_mode_s(ms, limit).await {
                Ok(images) if !images.is_empty() => {
                    debug!("Found {} images by MODE-S", images.len());
                    return Ok(images);
                }
                Ok(_) => {
                    debug!("No images found by MODE-S, trying registration fallback");
                }
                Err(e) => {
                    warn!(
                        "Failed to fetch by MODE-S: {}, trying registration fallback",
                        e
                    );
                }
            }
        }

        // Fall back to registration if MODE-S didn't work
        if let Some(reg) = registration {
            match self.fetch_by_registration(reg, limit).await {
                Ok(images) => {
                    debug!("Found {} images by registration", images.len());
                    return Ok(images);
                }
                Err(e) => {
                    warn!("Failed to fetch by registration: {}", e);
                    return Err(e);
                }
            }
        }

        // No identifiers available or all methods failed
        Ok(Vec::new())
    }

    /// Fetch aircraft images from Planespotters.net by hex code
    ///
    /// # Arguments
    /// * `hex_code` - 6-character hex MODE-S code (e.g., "A12B3C")
    pub async fn fetch_planespotters_by_hex(&self, hex_code: &str) -> Result<Vec<AircraftImage>> {
        debug!(
            "Fetching aircraft images from planespotters.net by hex: {}",
            hex_code
        );

        let url = format!("https://api.planespotters.net/pub/photos/hex/{}", hex_code);

        let response = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .context("Failed to send request to planespotters.net")?;

        let status = response.status();

        // Handle rate limiting
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            warn!("Rate limited by planespotters.net API");
            return Err(anyhow!("Rate limited by planespotters.net API"));
        }

        // Handle other error status codes
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Planespotters.net API error {}: {}", status, body));
        }

        let api_response: PlanespottersResponse = response
            .json()
            .await
            .context("Failed to parse planespotters.net API response")?;

        match api_response {
            PlanespottersResponse::Success { photos } => {
                let images: Vec<AircraftImage> =
                    photos.into_iter().map(|photo| photo.into()).collect();

                info!(
                    "Fetched {} images from planespotters.net for hex: {}",
                    images.len(),
                    hex_code
                );

                Ok(images)
            }
            PlanespottersResponse::Error { error } => {
                warn!("Planespotters.net API returned error: {}", error);
                Ok(Vec::new()) // Return empty vec on API error
            }
        }
    }

    /// Fetch aircraft images from Planespotters.net by registration
    ///
    /// # Arguments
    /// * `registration` - Aircraft registration (e.g., "N8437D", "D-ABCD")
    pub async fn fetch_planespotters_by_registration(
        &self,
        registration: &str,
    ) -> Result<Vec<AircraftImage>> {
        debug!(
            "Fetching aircraft images from planespotters.net by registration: {}",
            registration
        );

        let url = format!(
            "https://api.planespotters.net/pub/photos/reg/{}",
            registration
        );

        let response = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .context("Failed to send request to planespotters.net")?;

        let status = response.status();

        // Handle rate limiting
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            warn!("Rate limited by planespotters.net API");
            return Err(anyhow!("Rate limited by planespotters.net API"));
        }

        // Handle other error status codes
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Planespotters.net API error {}: {}", status, body));
        }

        let api_response: PlanespottersResponse = response
            .json()
            .await
            .context("Failed to parse planespotters.net API response")?;

        match api_response {
            PlanespottersResponse::Success { photos } => {
                let images: Vec<AircraftImage> =
                    photos.into_iter().map(|photo| photo.into()).collect();

                info!(
                    "Fetched {} images from planespotters.net for registration: {}",
                    images.len(),
                    registration
                );

                Ok(images)
            }
            PlanespottersResponse::Error { error } => {
                warn!("Planespotters.net API returned error: {}", error);
                Ok(Vec::new()) // Return empty vec on API error
            }
        }
    }

    /// Fetch aircraft images from Planespotters.net API
    /// Tries hex code first, falls back to registration if no results
    ///
    /// # Arguments
    /// * `hex_code` - Optional 6-character hex MODE-S code
    /// * `registration` - Optional aircraft registration
    pub async fn fetch_from_planespotters(
        &self,
        hex_code: Option<&str>,
        registration: Option<&str>,
    ) -> Result<Vec<AircraftImage>> {
        // Try hex code first if available
        if let Some(hex) = hex_code {
            match self.fetch_planespotters_by_hex(hex).await {
                Ok(images) if !images.is_empty() => {
                    debug!("Found {} images by hex from planespotters", images.len());
                    return Ok(images);
                }
                Ok(_) => {
                    debug!(
                        "No images found by hex from planespotters, trying registration fallback"
                    );
                }
                Err(e) => {
                    warn!(
                        "Failed to fetch by hex from planespotters: {}, trying registration fallback",
                        e
                    );
                }
            }
        }

        // Fall back to registration if hex didn't work
        if let Some(reg) = registration {
            match self.fetch_planespotters_by_registration(reg).await {
                Ok(images) => {
                    debug!(
                        "Found {} images by registration from planespotters",
                        images.len()
                    );
                    return Ok(images);
                }
                Err(e) => {
                    warn!("Failed to fetch by registration from planespotters: {}", e);
                    return Err(e);
                }
            }
        }

        // No identifiers available or all methods failed
        Ok(Vec::new())
    }

    /// Fetch images from a specific source
    ///
    /// # Arguments
    /// * `source` - Which image source to query
    /// * `mode_s` - Optional MODE-S code (hex)
    /// * `registration` - Optional registration
    /// * `limit` - Maximum number of images to fetch
    pub async fn fetch_from_source(
        &self,
        source: AircraftImageSource,
        mode_s: Option<&str>,
        registration: Option<&str>,
        limit: u8,
    ) -> Result<Vec<AircraftImage>> {
        match source {
            AircraftImageSource::AirportData => {
                self.fetch_images(mode_s, registration, limit).await
            }
            AircraftImageSource::Planespotters => {
                self.fetch_from_planespotters(mode_s, registration).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_by_mode_s() {
        let client = AircraftImagesClient::new(Client::new());
        // This is a real MODE-S code that should have images
        let result = client.fetch_by_mode_s("A12B3C", 5).await;
        // Don't assert success since we don't want test to depend on external API
        // Just verify it doesn't panic
        let _ = result;
    }

    #[tokio::test]
    async fn test_fetch_by_registration() {
        let client = AircraftImagesClient::new(Client::new());
        // This is a real registration that should have images
        let result = client.fetch_by_registration("N8437D", 5).await;
        // Don't assert success since we don't want test to depend on external API
        // Just verify it doesn't panic
        let _ = result;
    }

    #[tokio::test]
    async fn test_fetch_images_with_both_identifiers() {
        let client = AircraftImagesClient::new(Client::new());
        let result = client.fetch_images(Some("A12B3C"), Some("N8437D"), 5).await;
        // Should not panic
        let _ = result;
    }

    #[tokio::test]
    async fn test_fetch_images_with_no_identifiers() {
        let client = AircraftImagesClient::new(Client::new());
        let result = client.fetch_images(None, None, 5).await;
        // Should return empty vec
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
