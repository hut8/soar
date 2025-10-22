use crate::locations::Point;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Club {
    pub id: Uuid,
    pub name: String,
    pub is_soaring: Option<bool>,
    pub home_base_airport_id: Option<i32>,

    // Location normalization
    pub location_id: Option<Uuid>, // Foreign key to locations table

    // Address fields
    pub street1: Option<String>,
    pub street2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip_code: Option<String>,
    pub region_code: Option<String>,
    pub country_mail_code: Option<String>,

    pub base_location: Option<Point>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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

/// Diesel model for the clubs table - used for database operations
#[derive(Debug, Clone, Queryable, Selectable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::clubs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ClubModel {
    pub id: Uuid,
    pub name: String,
    pub is_soaring: Option<bool>,
    pub home_base_airport_id: Option<i32>,
    pub location_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insert model for new clubs
#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::clubs)]
pub struct NewClubModel {
    pub id: Uuid,
    pub name: String,
    pub is_soaring: Option<bool>,
    pub home_base_airport_id: Option<i32>,
    pub location_id: Option<Uuid>,
}

/// Conversion from Club (API model) to ClubModel (database model)
impl From<Club> for ClubModel {
    fn from(club: Club) -> Self {
        Self {
            id: club.id,
            name: club.name,
            is_soaring: club.is_soaring,
            home_base_airport_id: club.home_base_airport_id,
            location_id: club.location_id,
            created_at: club.created_at,
            updated_at: club.updated_at,
        }
    }
}

/// Conversion from Club (API model) to NewClubModel (insert model)
impl From<Club> for NewClubModel {
    fn from(club: Club) -> Self {
        Self {
            id: club.id,
            name: club.name,
            is_soaring: club.is_soaring,
            home_base_airport_id: club.home_base_airport_id,
            location_id: club.location_id,
        }
    }
}

/// Conversion from ClubModel (database model) to Club (API model)
impl From<ClubModel> for Club {
    fn from(club_model: ClubModel) -> Self {
        Self {
            id: club_model.id,
            name: club_model.name,
            is_soaring: club_model.is_soaring,
            home_base_airport_id: club_model.home_base_airport_id,
            location_id: club_model.location_id,
            street1: None,
            street2: None,
            city: None,
            state: None,
            zip_code: None,
            region_code: None,
            country_mail_code: None,
            base_location: None,
            created_at: club_model.created_at,
            updated_at: club_model.updated_at,
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
            location_id: None,
            street1: None,
            street2: None,
            city: None,
            state: None,
            zip_code: None,
            region_code: None,
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
            location_id: None,
            street1: Some("123 Main St".to_string()),
            street2: Some("Suite 100".to_string()),
            city: Some("Anytown".to_string()),
            state: Some("CA".to_string()),
            zip_code: Some("12345".to_string()),
            region_code: None,
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
            location_id: None,
            street1: None,
            street2: None,
            city: None,
            state: None,
            zip_code: None,
            region_code: None,
            country_mail_code: None,
            base_location: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(club.address_string(), None);
    }
}
