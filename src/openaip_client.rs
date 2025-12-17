use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// OpenAIP API client for fetching airspace data
pub struct OpenAipClient {
    client: Client,
    api_key: String,
    base_url: String,
}

/// OpenAIP airspace response (paginated)
#[derive(Debug, Deserialize)]
pub struct AirspacesResponse {
    pub items: Vec<OpenAipAirspace>,
    #[serde(rename = "totalCount")]
    pub total_count: i32,
    #[serde(rename = "totalPages")]
    pub total_pages: i32,
    pub page: i32,
    pub limit: i32,
    #[serde(rename = "nextPage")]
    pub next_page: Option<i32>,
}

/// Single airspace from OpenAIP API
#[derive(Debug, Deserialize, Serialize)]
pub struct OpenAipAirspace {
    #[serde(rename = "_id")]
    pub id: String, // MongoDB ObjectId as string
    pub name: String,
    #[serde(rename = "type")]
    pub airspace_type: i32, // Type code (0-36)
    #[serde(rename = "icaoClass")]
    pub icao_class: Option<i32>, // ICAO class code (0-8)
    pub country: String,             // ISO 3166-1 alpha-2
    pub geometry: serde_json::Value, // GeoJSON geometry

    // Altitude limits
    #[serde(rename = "lowerLimit")]
    pub lower_limit: Option<AltitudeLimit>,
    #[serde(rename = "upperLimit")]
    pub upper_limit: Option<AltitudeLimit>,

    // Optional fields
    pub remarks: Option<String>,
    pub activity: Option<i32>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AltitudeLimit {
    pub value: Option<f64>,
    pub unit: Option<i32>, // Unit code: 1=FT, 6=FL, etc.
    #[serde(rename = "referenceDatum")]
    pub reference_datum: Option<i32>, // Reference code: 1=MSL, 2=AGL, etc.
}

impl OpenAipClient {
    /// Create new OpenAIP client
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.core.openaip.net/api".to_string(),
        }
    }

    /// Fetch airspaces with pagination
    ///
    /// # Arguments
    /// * `page` - Page number (1-indexed)
    /// * `limit` - Results per page (max 500)
    /// * `country` - Optional country filter (ISO 3166-1 alpha-2)
    /// * `updated_after` - Optional filter for incremental sync
    pub async fn fetch_airspaces(
        &self,
        page: i32,
        limit: i32,
        country: Option<&str>,
        updated_after: Option<DateTime<Utc>>,
    ) -> Result<AirspacesResponse> {
        let url = format!("{}/airspaces", self.base_url);

        // Build query parameters
        let mut params = vec![("page", page.to_string()), ("limit", limit.to_string())];

        if let Some(c) = country {
            params.push(("country", c.to_string()));
        }

        if let Some(updated) = updated_after {
            // Format as ISO 8601 UTC
            params.push(("updatedAfter", updated.to_rfc3339()));
        }

        debug!("Fetching airspaces page {} (limit {})", page, limit);

        let response = self
            .client
            .get(&url)
            .header("x-openaip-api-key", &self.api_key)
            .query(&params)
            .send()
            .await
            .context("Failed to send request to OpenAIP API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAIP API error {}: {}", status, body);
        }

        // Read response body as text first for better error reporting
        let response_text = response
            .text()
            .await
            .context("Failed to read response body")?;

        debug!(
            "OpenAIP API response (first 500 chars): {}",
            &response_text.chars().take(500).collect::<String>()
        );

        let data: AirspacesResponse = serde_json::from_str(&response_text).with_context(|| {
            format!(
                "Failed to parse OpenAIP API response. Response: {}",
                &response_text.chars().take(1000).collect::<String>()
            )
        })?;

        info!(
            "Fetched {} airspaces (page {}/{}, total {})",
            data.items.len(),
            data.page,
            data.total_pages,
            data.total_count
        );

        Ok(data)
    }

    /// Fetch all airspaces with automatic pagination
    ///
    /// # Arguments
    /// * `country` - Optional country filter
    /// * `updated_after` - Optional filter for incremental sync
    pub async fn fetch_all_airspaces(
        &self,
        country: Option<&str>,
        updated_after: Option<DateTime<Utc>>,
    ) -> Result<Vec<OpenAipAirspace>> {
        let mut all_airspaces = Vec::new();
        let mut page = 1;
        const LIMIT: i32 = 500; // Max allowed by OpenAIP

        loop {
            let response = self
                .fetch_airspaces(page, LIMIT, country, updated_after)
                .await?;
            let fetched_count = response.items.len();

            all_airspaces.extend(response.items);

            // Check if we've fetched all pages
            if fetched_count < LIMIT as usize {
                break;
            }

            page += 1;

            // Rate limiting: wait 100ms between requests
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        info!("Fetched total of {} airspaces", all_airspaces.len());
        Ok(all_airspaces)
    }
}

/// Convert OpenAIP airspace type code to our enum
/// Based on OpenAIP API documentation
pub fn map_airspace_type(type_code: i32) -> crate::airspace::AirspaceType {
    use crate::airspace::AirspaceType;

    match type_code {
        0 => AirspaceType::Other,
        1 => AirspaceType::Restricted,
        2 => AirspaceType::Danger,
        3 => AirspaceType::Prohibited,
        4 => AirspaceType::Ctr,
        5 => AirspaceType::Tmz,
        6 => AirspaceType::Rmz,
        7 => AirspaceType::Tma,
        8 => AirspaceType::Atz,
        9 => AirspaceType::Matz,
        10 => AirspaceType::Airway,
        11 => AirspaceType::Mtr,
        12 => AirspaceType::AlertArea,
        13 => AirspaceType::WarningArea,
        14 => AirspaceType::ProtectedArea,
        15 => AirspaceType::Htz,
        16 => AirspaceType::GliderProhibited,
        17 => AirspaceType::GliderSector,
        18 => AirspaceType::NoGliders,
        19 => AirspaceType::WaveWindow,
        20 => AirspaceType::Fir,
        21 => AirspaceType::Uir,
        22 => AirspaceType::Adiz,
        23 => AirspaceType::AtzP,
        24 => AirspaceType::AtzMbz,
        25 => AirspaceType::Tfr,
        26 => AirspaceType::Tra,
        27 => AirspaceType::Tsa,
        28 => AirspaceType::Fis,
        29 => AirspaceType::Uas,
        30 => AirspaceType::Rffs,
        31 => AirspaceType::Sport,
        32 => AirspaceType::DropZone,
        33 => AirspaceType::Gliding,
        34 => AirspaceType::MilitaryOps,
        _ => AirspaceType::NotAssigned,
    }
}

/// Convert OpenAIP ICAO class code to our enum
pub fn map_icao_class(class_code: Option<i32>) -> Option<crate::airspace::AirspaceClass> {
    use crate::airspace::AirspaceClass;

    match class_code {
        Some(0) => Some(AirspaceClass::A),
        Some(1) => Some(AirspaceClass::B),
        Some(2) => Some(AirspaceClass::C),
        Some(3) => Some(AirspaceClass::D),
        Some(4) => Some(AirspaceClass::E),
        Some(5) => Some(AirspaceClass::F),
        Some(6) => Some(AirspaceClass::G),
        Some(8) => Some(AirspaceClass::Sua), // SUA
        _ => None,
    }
}

/// Convert OpenAIP altitude reference code to our enum
pub fn map_altitude_reference(ref_code: Option<i32>) -> Option<crate::airspace::AltitudeReference> {
    use crate::airspace::AltitudeReference;

    match ref_code {
        Some(0) => Some(AltitudeReference::Gnd), // Ground (0 ft)
        Some(1) => Some(AltitudeReference::Msl),
        Some(2) => Some(AltitudeReference::Agl),
        Some(3) => Some(AltitudeReference::Std),
        Some(4) => Some(AltitudeReference::Gnd),
        Some(5) => Some(AltitudeReference::Unl),
        _ => None,
    }
}

/// Convert OpenAIP altitude unit code to string
pub fn map_altitude_unit(unit_code: Option<i32>) -> Option<String> {
    match unit_code {
        Some(0) => Some("FT".to_string()), // Feet (or unknown, treating as feet)
        Some(1) => Some("FT".to_string()),
        Some(2) => Some("M".to_string()),
        Some(3) => Some("SM".to_string()),
        Some(4) => Some("NM".to_string()),
        Some(5) => Some("KM".to_string()),
        Some(6) => Some("FL".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_openaip_response() {
        // Load test response from file
        let test_json = include_str!("openaip_test_response.json");

        // Parse the response
        let result: Result<AirspacesResponse, _> = serde_json::from_str(test_json);

        // Assert parsing succeeds
        assert!(
            result.is_ok(),
            "Failed to parse OpenAIP response: {:?}",
            result.err()
        );

        let response = result.unwrap();

        // Verify pagination fields
        assert_eq!(response.limit, 500);
        assert_eq!(response.total_count, 26142);
        assert_eq!(response.total_pages, 53);
        assert_eq!(response.page, 1);
        assert_eq!(response.next_page, Some(2));

        // Verify we have at least one item
        assert!(
            !response.items.is_empty(),
            "Expected at least one airspace item"
        );

        // Verify first item structure
        let first = &response.items[0];
        assert_eq!(first.id, "674c9e07d0501f5781b4c372");
        assert_eq!(first.name, "22 MITT");
        assert_eq!(first.airspace_type, 21); // UIR
        assert_eq!(first.icao_class, Some(8)); // SUA
        assert_eq!(first.country, "SE");

        // Verify altitude limits
        assert!(first.lower_limit.is_some());
        let lower = first.lower_limit.as_ref().unwrap();
        assert_eq!(lower.value, Some(1600.0));
        assert_eq!(lower.unit, Some(1)); // FT
        assert_eq!(lower.reference_datum, Some(1)); // MSL

        assert!(first.upper_limit.is_some());
        let upper = first.upper_limit.as_ref().unwrap();
        assert_eq!(upper.value, Some(90.0));
        assert_eq!(upper.unit, Some(6)); // FL
        assert_eq!(upper.reference_datum, Some(2)); // AGL

        // Test mapping functions
        assert_eq!(map_airspace_type(21), crate::airspace::AirspaceType::Uir);
        assert_eq!(
            map_icao_class(Some(8)),
            Some(crate::airspace::AirspaceClass::Sua)
        );
        assert_eq!(map_altitude_unit(Some(1)), Some("FT".to_string()));
        assert_eq!(map_altitude_unit(Some(6)), Some("FL".to_string()));
        assert_eq!(
            map_altitude_reference(Some(1)),
            Some(crate::airspace::AltitudeReference::Msl)
        );
        assert_eq!(
            map_altitude_reference(Some(2)),
            Some(crate::airspace::AltitudeReference::Agl)
        );
    }
}
