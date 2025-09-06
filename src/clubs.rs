use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Club {
    pub id: Uuid,
    pub name: String,
    pub is_soaring: Option<bool>,
    pub home_base_airport_id: Option<i32>,

    // Address fields (matching aircraft registration table)
    pub street1: Option<String>,
    pub street2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip_code: Option<String>,
    pub region_code: Option<String>,
    pub county_mail_code: Option<String>,
    pub country_mail_code: Option<String>,

    // Location points - we'll use custom types for PostGIS points
    pub base_location: Option<Point>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Simple Point struct for WGS84 coordinates
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

impl Club {
    /// Check if this club is likely related to soaring based on its name
    pub fn is_soaring_related(&self) -> bool {
        let name_upper = self.name.to_uppercase();
        name_upper.contains("SOAR")
            || name_upper.contains("GLIDING")
            || name_upper.contains("SAILPLANE")
            || name_upper.contains("GLIDER")
    }

    /// Get a complete address string for geocoding
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

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_soaring_related() {
        let mut club = Club {
            id: Uuid::new_v4(),
            name: "Mountain Soaring Club".to_string(),
            is_soaring: None,
            home_base_airport_id: None,
            street1: None,
            street2: None,
            city: None,
            state: None,
            zip_code: None,
            region_code: None,
            county_mail_code: None,
            country_mail_code: None,
            base_location: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(club.is_soaring_related());

        club.name = "Valley Gliding Association".to_string();
        assert!(club.is_soaring_related());

        club.name = "Desert Sailplane Club".to_string();
        assert!(club.is_soaring_related());

        club.name = "Ridge Glider Society".to_string();
        assert!(club.is_soaring_related());

        club.name = "City Flying Club".to_string();
        assert!(!club.is_soaring_related());
    }

    #[test]
    fn test_address_string() {
        let club = Club {
            id: Uuid::new_v4(),
            name: "Test Club".to_string(),
            is_soaring: None,
            home_base_airport_id: None,
            street1: Some("123 Main St".to_string()),
            street2: Some("Suite 100".to_string()),
            city: Some("Anytown".to_string()),
            state: Some("CA".to_string()),
            zip_code: Some("12345".to_string()),
            region_code: None,
            county_mail_code: None,
            country_mail_code: None,
            base_location: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(
            club.address_string(),
            Some("123 Main St, Suite 100, Anytown, CA, 12345".to_string())
        );
    }

    #[test]
    fn test_empty_address_string() {
        let club = Club {
            id: Uuid::new_v4(),
            name: "Test Club".to_string(),
            is_soaring: None,
            home_base_airport_id: None,
            street1: None,
            street2: None,
            city: None,
            state: None,
            zip_code: None,
            region_code: None,
            county_mail_code: None,
            country_mail_code: None,
            base_location: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(club.address_string(), None);
    }
}
