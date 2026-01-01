use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Source of aircraft images
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AircraftImageSource {
    /// Airport-Data.com API
    AirportData,
    /// Planespotters.net API
    Planespotters,
}

impl AircraftImageSource {
    /// Get all available image sources
    pub fn all() -> Vec<AircraftImageSource> {
        vec![
            AircraftImageSource::AirportData,
            AircraftImageSource::Planespotters,
        ]
    }

    /// Convert source to string key for HashMap
    pub fn as_key(&self) -> &'static str {
        match self {
            AircraftImageSource::AirportData => "airport_data",
            AircraftImageSource::Planespotters => "planespotters",
        }
    }

    /// Convert string key back to source
    pub fn from_key(key: &str) -> Option<AircraftImageSource> {
        match key {
            "airport_data" => Some(AircraftImageSource::AirportData),
            "planespotters" => Some(AircraftImageSource::Planespotters),
            _ => None,
        }
    }
}

/// A single aircraft image from an external source
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AircraftImage {
    /// Source of the image
    pub source: AircraftImageSource,

    /// URL to the photographer's page or photo details page
    pub page_url: String,

    /// URL to the thumbnail image (typically 200px wide)
    pub thumbnail_url: String,

    /// URL to the full-size image (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,

    /// Photographer's name (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photographer: Option<String>,
}

/// Collection of aircraft images with metadata about when they were fetched
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AircraftImageCollection {
    /// List of images (may be empty if none found)
    pub images: Vec<AircraftImage>,

    /// Timestamp of last fetch attempt per source
    /// Key is the source name as string
    pub last_fetched: HashMap<String, DateTime<Utc>>,
}

impl AircraftImageCollection {
    /// Create an empty collection
    pub fn empty() -> Self {
        Self {
            images: Vec::new(),
            last_fetched: HashMap::new(),
        }
    }

    /// Create a new collection from airport-data.com response
    pub fn from_airport_data(images: Vec<AircraftImage>) -> Self {
        let mut last_fetched = HashMap::new();
        last_fetched.insert("airport_data".to_string(), Utc::now());

        Self {
            images,
            last_fetched,
        }
    }

    /// Check if this collection has cached data from a specific source
    pub fn has_data_from(&self, source: AircraftImageSource) -> bool {
        self.last_fetched.contains_key(source.as_key())
    }

    /// Get sources that have never been queried
    pub fn unqueried_sources(&self) -> Vec<AircraftImageSource> {
        AircraftImageSource::all()
            .into_iter()
            .filter(|source| !self.has_data_from(*source))
            .collect()
    }

    /// Get sources that need re-querying (queried more than 1 week ago AND returned empty results)
    pub fn sources_needing_refresh(&self) -> Vec<AircraftImageSource> {
        let one_week_ago = Utc::now() - chrono::Duration::weeks(1);

        AircraftImageSource::all()
            .into_iter()
            .filter(|source| {
                if let Some(last_fetch_time) = self.last_fetched.get(source.as_key()) {
                    // Check if more than 1 week old
                    if *last_fetch_time < one_week_ago {
                        // Check if this source returned empty results
                        let has_images_from_source =
                            self.images.iter().any(|img| img.source == *source);
                        !has_images_from_source
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
            .collect()
    }

    /// Add images from a source and update the timestamp
    pub fn add_images_from_source(
        &mut self,
        source: AircraftImageSource,
        new_images: Vec<AircraftImage>,
    ) {
        // Remove any existing images from this source
        self.images.retain(|img| img.source != source);

        // Add new images
        self.images.extend(new_images);

        // Update timestamp
        self.last_fetched
            .insert(source.as_key().to_string(), Utc::now());
    }

    /// Update timestamp for a source (even if no images were found)
    pub fn update_timestamp(&mut self, source: AircraftImageSource) {
        self.last_fetched
            .insert(source.as_key().to_string(), Utc::now());
    }

    /// Get all sources that should be queried (unqueried + needing refresh)
    pub fn sources_to_query(&self) -> Vec<AircraftImageSource> {
        let mut sources = self.unqueried_sources();
        sources.extend(self.sources_needing_refresh());
        sources
    }
}

/// Airport-Data.com API response structure
#[derive(Debug, Deserialize)]
pub struct AirportDataResponse {
    pub status: u16,
    pub count: u32,
    #[serde(default)]
    pub data: Vec<AirportDataImage>,
}

/// Single image entry from Airport-Data.com API
#[derive(Debug, Deserialize)]
pub struct AirportDataImage {
    /// URL to thumbnail image (200px)
    pub image: String,

    /// URL to photo page
    pub link: String,

    /// Photographer's name
    #[serde(default)]
    pub photographer: String,
}

impl From<AirportDataImage> for AircraftImage {
    fn from(img: AirportDataImage) -> Self {
        AircraftImage {
            source: AircraftImageSource::AirportData,
            page_url: img.link,
            thumbnail_url: img.image,
            image_url: None, // Airport-Data.com doesn't provide full-size URLs in API
            photographer: if img.photographer.is_empty() {
                None
            } else {
                Some(img.photographer)
            },
        }
    }
}

/// Planespotters.net API response structure
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PlanespottersResponse {
    Success { photos: Vec<PlanespottersPhoto> },
    Error { error: String },
}

/// Single photo entry from Planespotters.net API
#[derive(Debug, Deserialize)]
pub struct PlanespottersPhoto {
    /// Photo ID
    pub id: String,

    /// Thumbnail image (200px wide)
    pub thumbnail: PlanespottersImage,

    /// Large thumbnail (420px wide)
    pub thumbnail_large: PlanespottersImage,

    /// Link to photo page
    pub link: String,

    /// Photographer's name
    pub photographer: String,
}

/// Image data from Planespotters.net
#[derive(Debug, Deserialize)]
pub struct PlanespottersImage {
    /// Image URL
    pub src: String,

    /// Image dimensions
    #[allow(dead_code)]
    pub size: PlanespottersImageSize,
}

/// Image size from Planespotters.net
#[derive(Debug, Deserialize)]
pub struct PlanespottersImageSize {
    #[allow(dead_code)]
    pub width: u32,
    #[allow(dead_code)]
    pub height: u32,
}

impl From<PlanespottersPhoto> for AircraftImage {
    fn from(photo: PlanespottersPhoto) -> Self {
        AircraftImage {
            source: AircraftImageSource::Planespotters,
            page_url: photo.link,
            thumbnail_url: photo.thumbnail.src,
            // Use the larger thumbnail as the "full" image
            image_url: Some(photo.thumbnail_large.src),
            photographer: if photo.photographer.is_empty() {
                None
            } else {
                Some(photo.photographer)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_collection() {
        let collection = AircraftImageCollection::empty();
        assert!(collection.images.is_empty());
        assert!(collection.last_fetched.is_empty());
    }

    #[test]
    fn test_from_airport_data() {
        let images = vec![AircraftImage {
            source: AircraftImageSource::AirportData,
            page_url: "https://example.com/photo".to_string(),
            thumbnail_url: "https://example.com/thumb.jpg".to_string(),
            image_url: None,
            photographer: Some("Test Photographer".to_string()),
        }];

        let collection = AircraftImageCollection::from_airport_data(images.clone());
        assert_eq!(collection.images.len(), 1);
        assert!(collection.has_data_from(AircraftImageSource::AirportData));
    }

    #[test]
    fn test_serialization() {
        let collection = AircraftImageCollection::empty();
        let json = serde_json::to_value(&collection).unwrap();
        let deserialized: AircraftImageCollection = serde_json::from_value(json).unwrap();
        assert_eq!(collection.images.len(), deserialized.images.len());
    }

    #[test]
    fn test_airport_data_image_conversion() {
        let api_image = AirportDataImage {
            image: "https://example.com/thumb.jpg".to_string(),
            link: "https://example.com/photo".to_string(),
            photographer: "Test Photographer".to_string(),
        };

        let aircraft_image: AircraftImage = api_image.into();
        assert_eq!(aircraft_image.source, AircraftImageSource::AirportData);
        assert_eq!(
            aircraft_image.thumbnail_url,
            "https://example.com/thumb.jpg"
        );
        assert_eq!(aircraft_image.page_url, "https://example.com/photo");
        assert_eq!(
            aircraft_image.photographer,
            Some("Test Photographer".to_string())
        );
        assert_eq!(aircraft_image.image_url, None);
    }

    #[test]
    fn test_empty_photographer_conversion() {
        let api_image = AirportDataImage {
            image: "https://example.com/thumb.jpg".to_string(),
            link: "https://example.com/photo".to_string(),
            photographer: "".to_string(),
        };

        let aircraft_image: AircraftImage = api_image.into();
        assert_eq!(aircraft_image.photographer, None);
    }
}
