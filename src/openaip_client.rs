use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
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
    pub page: i32,
    pub limit: i32,
}

/// Single airspace from OpenAIP API
#[derive(Debug, Deserialize)]
pub struct OpenAipAirspace {
    #[serde(rename = "_id")]
    pub id: i32,
    pub name: String,
    #[serde(rename = "type")]
    pub airspace_type: i32, // Type code (0-36)
    #[serde(rename = "icaoClass")]
    pub icao_class: Option<String>, // "A", "B", "C", etc.
    pub country: String,             // ISO 3166-1 alpha-2
    pub geometry: serde_json::Value, // GeoJSON geometry

    // Altitude limits
    #[serde(rename = "lowerLimit")]
    pub lower_limit: Option<AltitudeLimit>,
    #[serde(rename = "upperLimit")]
    pub upper_limit: Option<AltitudeLimit>,

    // Optional fields
    pub remarks: Option<String>,
    pub activity: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct AltitudeLimit {
    pub value: Option<f64>,
    pub unit: Option<String>, // "FT", "FL", "M"
    #[serde(rename = "referenceDatum")]
    pub reference_datum: Option<String>, // "MSL", "AGL", "STD", "GND", "UNL"
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

        let data: AirspacesResponse = response
            .json()
            .await
            .context("Failed to parse OpenAIP API response")?;

        info!(
            "Fetched {} airspaces (page {}/{}, total {})",
            data.items.len(),
            data.page,
            (data.total_count as f64 / data.limit as f64).ceil() as i32,
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

/// Convert OpenAIP ICAO class string to our enum
pub fn map_icao_class(class_str: Option<&str>) -> Option<crate::airspace::AirspaceClass> {
    use crate::airspace::AirspaceClass;

    match class_str {
        Some("A") => Some(AirspaceClass::A),
        Some("B") => Some(AirspaceClass::B),
        Some("C") => Some(AirspaceClass::C),
        Some("D") => Some(AirspaceClass::D),
        Some("E") => Some(AirspaceClass::E),
        Some("F") => Some(AirspaceClass::F),
        Some("G") => Some(AirspaceClass::G),
        Some("SUA") => Some(AirspaceClass::Sua),
        _ => None,
    }
}

/// Convert OpenAIP altitude reference to our enum
pub fn map_altitude_reference(ref_str: Option<&str>) -> Option<crate::airspace::AltitudeReference> {
    use crate::airspace::AltitudeReference;

    match ref_str {
        Some("MSL") => Some(AltitudeReference::Msl),
        Some("AGL") => Some(AltitudeReference::Agl),
        Some("STD") => Some(AltitudeReference::Std),
        Some("GND") => Some(AltitudeReference::Gnd),
        Some("UNL") => Some(AltitudeReference::Unl),
        _ => None,
    }
}
