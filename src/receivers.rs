use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// A link associated with a receiver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiverLink {
    pub rel: String,
    pub href: String,
}

/// A receiver from the glidernet.org list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receiver {
    pub callsign: Option<String>,
    pub description: Option<String>,
    pub photos: Option<Vec<String>>,
    pub contact: Option<String>,
    pub email: Option<String>,
    pub links: Option<Vec<ReceiverLink>>,
    pub country: Option<String>,
}

/// Root structure for the receivers JSON file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiversData {
    pub version: String,
    pub timestamp: Option<String>,
    pub receivers: Option<Vec<Receiver>>,
}

/// Database representation of a receiver
#[derive(Debug, Clone)]
pub struct ReceiverRecord {
    pub id: i32,
    pub callsign: String,
    pub description: Option<String>,
    pub contact: Option<String>,
    pub email: Option<String>,
    pub country: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Database representation of a receiver photo
#[derive(Debug, Clone)]
pub struct ReceiverPhotoRecord {
    pub id: i32,
    pub receiver_id: i32,
    pub photo_url: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Database representation of a receiver link
#[derive(Debug, Clone)]
pub struct ReceiverLinkRecord {
    pub id: i32,
    pub receiver_id: i32,
    pub rel: String,
    pub href: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Receiver {
    /// Convert a JSON receiver to database records
    pub fn to_records(&self) -> (ReceiverRecord, Vec<String>, Vec<ReceiverLink>) {
        let photos = self.photos.clone().unwrap_or_default();
        let links = self.links.clone().unwrap_or_default();

        let receiver_record = ReceiverRecord {
            id: 0, // Will be set by database
            callsign: self.callsign.clone().unwrap_or_default(),
            description: self.description.clone(),
            contact: self.contact.clone(),
            email: self.email.clone(),
            country: self.country.clone(),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        };

        (receiver_record, photos, links)
    }
}

/// Read a receivers JSON file and parse it
pub fn read_receivers_file<P: AsRef<Path>>(path: P) -> Result<ReceiversData> {
    let file = File::open(path.as_ref())
        .with_context(|| format!("Opening {:?}", path.as_ref()))?;
    let reader = BufReader::new(file);

    let data: ReceiversData = serde_json::from_reader(reader)
        .with_context(|| format!("Parsing JSON from {:?}", path.as_ref()))?;

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receiver_deserialization() {
        let json = r#"
        {
            "callsign": "TEST123",
            "description": "Test receiver",
            "photos": ["http://example.com/photo1.jpg", "http://example.com/photo2.jpg"],
            "contact": "Test Contact",
            "email": "test@example.com",
            "links": [
                {"rel": "homepage", "href": "http://example.com"},
                {"rel": "photo", "href": "http://example.com/photo.jpg"}
            ],
            "country": "US"
        }
        "#;

        let receiver: Receiver = serde_json::from_str(json).unwrap();

        assert_eq!(receiver.callsign, Some("TEST123".to_string()));
        assert_eq!(receiver.description, Some("Test receiver".to_string()));
        assert_eq!(receiver.photos.as_ref().unwrap().len(), 2);
        assert_eq!(receiver.contact, Some("Test Contact".to_string()));
        assert_eq!(receiver.email, Some("test@example.com".to_string()));
        assert_eq!(receiver.links.as_ref().unwrap().len(), 2);
        assert_eq!(receiver.country, Some("US".to_string()));

        let links = receiver.links.as_ref().unwrap();
        assert_eq!(links[0].rel, "homepage");
        assert_eq!(links[0].href, "http://example.com");
        assert_eq!(links[1].rel, "photo");
        assert_eq!(links[1].href, "http://example.com/photo.jpg");
    }

    #[test]
    fn test_receivers_data_deserialization() {
        let json = r#"
        {
            "version": "0.2.2",
            "timestamp": "2023-01-01T00:00:00Z",
            "receivers": [
                {
                    "callsign": "TEST123",
                    "description": "Test receiver",
                    "country": "US"
                }
            ]
        }
        "#;

        let data: ReceiversData = serde_json::from_str(json).unwrap();

        assert_eq!(data.version, "0.2.2");
        assert_eq!(data.timestamp, Some("2023-01-01T00:00:00Z".to_string()));
        assert_eq!(data.receivers.as_ref().unwrap().len(), 1);

        let receiver = &data.receivers.as_ref().unwrap()[0];
        assert_eq!(receiver.callsign, Some("TEST123".to_string()));
        assert_eq!(receiver.description, Some("Test receiver".to_string()));
        assert_eq!(receiver.country, Some("US".to_string()));
    }

    #[test]
    fn test_receiver_to_records() {
        let receiver = Receiver {
            callsign: Some("TEST123".to_string()),
            description: Some("Test receiver".to_string()),
            photos: Some(vec!["photo1.jpg".to_string(), "photo2.jpg".to_string()]),
            contact: Some("Test Contact".to_string()),
            email: Some("test@example.com".to_string()),
            links: Some(vec![
                ReceiverLink {
                    rel: "homepage".to_string(),
                    href: "http://example.com".to_string(),
                }
            ]),
            country: Some("US".to_string()),
        };

        let (record, photos, links) = receiver.to_records();

        assert_eq!(record.callsign, "TEST123".to_string());
        assert_eq!(record.description, Some("Test receiver".to_string()));
        assert_eq!(record.contact, Some("Test Contact".to_string()));
        assert_eq!(record.email, Some("test@example.com".to_string()));
        assert_eq!(record.country, Some("US".to_string()));

        assert_eq!(photos.len(), 2);
        assert_eq!(photos[0], "photo1.jpg");
        assert_eq!(photos[1], "photo2.jpg");

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].rel, "homepage");
        assert_eq!(links[0].href, "http://example.com");
    }
}
