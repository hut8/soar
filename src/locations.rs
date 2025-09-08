use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Location {
    pub id: Uuid,
    pub street1: Option<String>,
    pub street2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip_code: Option<String>,
    pub region_code: Option<String>,
    pub county_mail_code: Option<String>,
    pub country_mail_code: Option<String>,
    pub geolocation: Option<Point>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Simple Point struct for WGS84 coordinates (reuse from clubs.rs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub latitude: f64,
    pub longitude: f64,
}

impl Point {
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
        }
    }
}

// Custom sqlx type implementation for PostGIS POINT
impl sqlx::Type<sqlx::Postgres> for Point {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("point")
    }
}

impl sqlx::Encode<'_, sqlx::Postgres> for Point {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let point_str = format!("({},{})", self.longitude, self.latitude);
        sqlx::Encode::<sqlx::Postgres>::encode_by_ref(&point_str, buf)
    }
}

impl sqlx::Decode<'_, sqlx::Postgres> for Point {
    fn decode(value: sqlx::postgres::PgValueRef<'_>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <&str as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        // Parse PostgreSQL point format: "(longitude,latitude)"
        let s = s.trim_start_matches('(').trim_end_matches(')');
        let coords: Vec<&str> = s.split(',').collect();

        if coords.len() != 2 {
            return Err("Invalid point format".into());
        }

        let longitude: f64 = coords[0].parse()?;
        let latitude: f64 = coords[1].parse()?;

        Ok(Point::new(latitude, longitude))
    }
}

impl Location {
    /// Create a new Location with generated UUID and current timestamps
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        street1: Option<String>,
        street2: Option<String>,
        city: Option<String>,
        state: Option<String>,
        zip_code: Option<String>,
        region_code: Option<String>,
        county_mail_code: Option<String>,
        country_mail_code: Option<String>,
        geolocation: Option<Point>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            street1,
            street2,
            city,
            state,
            zip_code,
            region_code,
            county_mail_code,
            country_mail_code,
            geolocation,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get a complete address string for display or geocoding
    pub fn address_string(&self) -> Option<String> {
        let mut parts = Vec::new();

        if let Some(street1) = &self.street1
            && !street1.trim().is_empty()
        {
            parts.push(street1.trim().to_string());
        }

        if let Some(street2) = &self.street2
            && !street2.trim().is_empty()
        {
            parts.push(street2.trim().to_string());
        }

        if let Some(city) = &self.city
            && !city.trim().is_empty()
        {
            parts.push(city.trim().to_string());
        }

        if let Some(state) = &self.state
            && !state.trim().is_empty()
        {
            parts.push(state.trim().to_string());
        }

        if let Some(zip) = &self.zip_code
            && !zip.trim().is_empty()
        {
            parts.push(zip.trim().to_string());
        }

        // Add country if not US
        if let Some(country) = &self.country_mail_code
            && country != "US" && !country.trim().is_empty() {
                parts.push(country.trim().to_string());
            }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(", "))
        }
    }

    /// Check if this location has geolocation data
    pub fn has_coordinates(&self) -> bool {
        self.geolocation.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_creation() {
        let location = Location::new(
            Some("123 Main St".to_string()),
            Some("Suite 100".to_string()),
            Some("Anytown".to_string()),
            Some("CA".to_string()),
            Some("12345".to_string()),
            Some("4".to_string()),
            Some("037".to_string()),
            Some("US".to_string()),
            Some(Point::new(34.0522, -118.2437)),
        );

        assert!(location.id != Uuid::nil());
        assert_eq!(location.street1, Some("123 Main St".to_string()));
        assert_eq!(location.city, Some("Anytown".to_string()));
        assert_eq!(location.state, Some("CA".to_string()));
        assert!(location.has_coordinates());
    }

    #[test]
    fn test_address_string() {
        let location = Location::new(
            Some("123 Main St".to_string()),
            Some("Suite 100".to_string()),
            Some("Anytown".to_string()),
            Some("CA".to_string()),
            Some("12345".to_string()),
            None,
            None,
            Some("US".to_string()),
            None,
        );

        assert_eq!(
            location.address_string(),
            Some("123 Main St, Suite 100, Anytown, CA, 12345".to_string())
        );
    }

    #[test]
    fn test_address_string_with_country() {
        let location = Location::new(
            Some("123 Rue de la Paix".to_string()),
            None,
            Some("Paris".to_string()),
            None,
            Some("75001".to_string()),
            None,
            None,
            Some("FR".to_string()),
            None,
        );

        assert_eq!(
            location.address_string(),
            Some("123 Rue de la Paix, Paris, 75001, FR".to_string())
        );
    }

    #[test]
    fn test_empty_address_string() {
        let location = Location::new(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );

        assert_eq!(location.address_string(), None);
    }
}
