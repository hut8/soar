use chrono::{DateTime, Utc};
use diesel::deserialize::{self, FromSql};
use diesel::expression::AsExpression;
use diesel::pg::{Pg, PgValue};
use diesel::prelude::*;
use diesel::serialize::{self, Output, ToSql};
use serde::{Deserialize, Serialize};
use std::io::Write;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, AsExpression)]
#[diesel(sql_type = crate::schema::sql_types::Point)]
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

// Implement FromSql for Point to work with PostgreSQL point type
impl FromSql<crate::schema::sql_types::Point, Pg> for Point {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        // PostgreSQL point is stored as binary data, read as bytes
        let bytes = bytes.as_bytes();
        if bytes.len() < 16 {
            return Err("Invalid point data length".into());
        }

        // PostgreSQL stores point as two 8-byte floats (longitude, latitude) in network byte order
        let longitude = f64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        let latitude = f64::from_be_bytes([
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]);

        Ok(Point::new(latitude, longitude))
    }
}

// Implement ToSql for Point to work with PostgreSQL point type
impl ToSql<crate::schema::sql_types::Point, Pg> for Point {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        // Write longitude and latitude as 8-byte floats in network byte order
        out.write_all(&self.longitude.to_be_bytes())?;
        out.write_all(&self.latitude.to_be_bytes())?;
        Ok(serialize::IsNull::No)
    }
}

/// Diesel model for the locations table - used for database operations
#[derive(Debug, Clone, Queryable, QueryableByName, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::locations)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct LocationModel {
    pub id: Uuid,
    pub street1: Option<String>,
    pub street2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip_code: Option<String>,
    pub region_code: Option<String>,
    pub county_mail_code: Option<String>,
    pub country_mail_code: Option<String>,
    pub geolocation: Option<Point>, // PostgreSQL point type
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insert model for new locations
#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::locations)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewLocationModel {
    pub id: Uuid,
    pub street1: Option<String>,
    pub street2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip_code: Option<String>,
    pub region_code: Option<String>,
    pub county_mail_code: Option<String>,
    pub country_mail_code: Option<String>,
    pub geolocation: Option<Point>, // PostgreSQL point type
}

/// Conversion from Location (API model) to LocationModel (database model)
impl From<Location> for LocationModel {
    fn from(location: Location) -> Self {
        Self {
            id: location.id,
            street1: location.street1,
            street2: location.street2,
            city: location.city,
            state: location.state,
            zip_code: location.zip_code,
            region_code: location.region_code,
            county_mail_code: location.county_mail_code,
            country_mail_code: location.country_mail_code,
            geolocation: location.geolocation,
            created_at: location.created_at,
            updated_at: location.updated_at,
        }
    }
}

/// Conversion from Location (API model) to NewLocationModel (insert model)
impl From<Location> for NewLocationModel {
    fn from(location: Location) -> Self {
        Self {
            id: location.id,
            street1: location.street1,
            street2: location.street2,
            city: location.city,
            state: location.state,
            zip_code: location.zip_code,
            region_code: location.region_code,
            county_mail_code: location.county_mail_code,
            country_mail_code: location.country_mail_code,
            geolocation: location.geolocation,
        }
    }
}

/// Conversion from LocationModel (database model) to Location (API model)
impl From<LocationModel> for Location {
    fn from(model: LocationModel) -> Self {
        Self {
            id: model.id,
            street1: model.street1,
            street2: model.street2,
            city: model.city,
            state: model.state,
            zip_code: model.zip_code,
            region_code: model.region_code,
            county_mail_code: model.county_mail_code,
            country_mail_code: model.country_mail_code,
            geolocation: model.geolocation,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
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
            && country != "US"
            && !country.trim().is_empty()
        {
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
        let location = Location::new(None, None, None, None, None, None, None, None, None);

        assert_eq!(location.address_string(), None);
    }
}
