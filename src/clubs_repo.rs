use anyhow::Result;
use chrono::Utc;
use sqlx::postgres::types::PgPoint;
use sqlx::PgPool;
use sqlx::types::Uuid;

use crate::clubs::Club;
use crate::locations_repo::LocationsRepository;

pub struct ClubsRepository {
    pool: PgPool,
    locations_repo: LocationsRepository,
}

impl ClubsRepository {
    pub fn new(pool: PgPool) -> Self {
        let locations_repo = LocationsRepository::new(pool.clone());
        Self { pool, locations_repo }
    }

    /// Get club by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Club>> {
        let result = sqlx::query!(
            r#"
            SELECT c.id, c.name, c.is_soaring, c.home_base_airport_id, c.location_id,
                   l.street1, l.street2, l.city, l.state, l.zip_code, l.region_code,
                   l.county_mail_code, l.country_mail_code,
                   ST_X(l.geolocation::geometry) as longitude, ST_Y(l.geolocation::geometry) as latitude,
                   c.created_at, c.updated_at
            FROM clubs c
            LEFT JOIN locations l ON c.location_id = l.id
            WHERE c.id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            let base_location = if row.longitude.is_some() && row.latitude.is_some() {
                Some(crate::clubs::Point::new(
                    row.latitude.unwrap(),
                    row.longitude.unwrap(),
                ))
            } else {
                None
            };

            Ok(Some(Club {
                id: row.id,
                name: row.name,
                is_soaring: row.is_soaring,
                home_base_airport_id: row.home_base_airport_id,
                location_id: row.location_id,
                street1: row.street1,
                street2: row.street2,
                city: row.city,
                state: row.state,
                zip_code: row.zip_code,
                region_code: row.region_code,
                county_mail_code: row.county_mail_code,
                country_mail_code: row.country_mail_code,
                base_location,
                created_at: row.created_at.unwrap_or_else(Utc::now),
                updated_at: row.updated_at.unwrap_or_else(Utc::now),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all clubs
    pub async fn get_all(&self) -> Result<Vec<Club>> {
        let results = sqlx::query!(
            r#"
            SELECT c.id, c.name, c.is_soaring, c.home_base_airport_id, c.location_id,
                   l.street1, l.street2, l.city, l.state, l.zip_code, l.region_code,
                   l.county_mail_code, l.country_mail_code,
                   ST_X(l.geolocation::geometry) as longitude, ST_Y(l.geolocation::geometry) as latitude,
                   c.created_at, c.updated_at
            FROM clubs c
            LEFT JOIN locations l ON c.location_id = l.id
            ORDER BY c.name
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut clubs = Vec::new();
        for row in results {
            let base_location = if row.longitude.is_some() && row.latitude.is_some() {
                Some(crate::clubs::Point::new(
                    row.latitude.unwrap(),
                    row.longitude.unwrap(),
                ))
            } else {
                None
            };

            clubs.push(Club {
                id: row.id,
                name: row.name,
                is_soaring: row.is_soaring,
                home_base_airport_id: row.home_base_airport_id,
                location_id: row.location_id,
                street1: row.street1,
                street2: row.street2,
                city: row.city,
                state: row.state,
                zip_code: row.zip_code,
                region_code: row.region_code,
                county_mail_code: row.county_mail_code,
                country_mail_code: row.country_mail_code,
                base_location,
                created_at: row.created_at.unwrap_or_else(Utc::now),
                updated_at: row.updated_at.unwrap_or_else(Utc::now),
            });
        }

        Ok(clubs)
    }

    /// Fuzzy search clubs by name using trigram similarity
    /// Returns clubs ordered by similarity score (best matches first)
    pub async fn fuzzy_search(&self, query: &str, limit: Option<i64>) -> Result<Vec<Club>> {
        let limit = limit.unwrap_or(20);
        let query_upper = query.to_uppercase();
        
        let results = sqlx::query!(
            r#"
            SELECT c.id, c.name, c.is_soaring, c.home_base_airport_id, c.location_id,
                   l.street1, l.street2, l.city, l.state, l.zip_code, l.region_code,
                   l.county_mail_code, l.country_mail_code,
                   ST_X(l.geolocation::geometry) as longitude, ST_Y(l.geolocation::geometry) as latitude,
                   c.created_at, c.updated_at,
                   SIMILARITY(UPPER(c.name), $1) as similarity_score
            FROM clubs c
            LEFT JOIN locations l ON c.location_id = l.id
            WHERE SIMILARITY(UPPER(c.name), $1) > 0.05
            ORDER BY similarity_score DESC, c.name
            LIMIT $2
            "#,
            query_upper,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        let mut clubs = Vec::new();
        for row in results {
            let base_location = if row.longitude.is_some() && row.latitude.is_some() {
                Some(crate::clubs::Point::new(
                    row.latitude.unwrap(),
                    row.longitude.unwrap(),
                ))
            } else {
                None
            };

            clubs.push(Club {
                id: row.id,
                name: row.name,
                is_soaring: row.is_soaring,
                home_base_airport_id: row.home_base_airport_id,
                location_id: row.location_id,
                street1: row.street1,
                street2: row.street2,
                city: row.city,
                state: row.state,
                zip_code: row.zip_code,
                region_code: row.region_code,
                county_mail_code: row.county_mail_code,
                country_mail_code: row.country_mail_code,
                base_location,
                created_at: row.created_at.unwrap_or_else(Utc::now),
                updated_at: row.updated_at.unwrap_or_else(Utc::now),
            });
        }

        Ok(clubs)
    }

    /// Fuzzy search soaring clubs only by name using trigram similarity
    /// Returns soaring clubs (is_soaring=true) ordered by similarity score (best matches first)
    pub async fn fuzzy_search_soaring(&self, query: &str, limit: Option<i64>) -> Result<Vec<Club>> {
        let limit = limit.unwrap_or(20);
        let query_upper = query.to_uppercase();
        
        let results = sqlx::query!(
            r#"
            SELECT c.id, c.name, c.is_soaring, c.home_base_airport_id, c.location_id,
                   l.street1, l.street2, l.city, l.state, l.zip_code, l.region_code,
                   l.county_mail_code, l.country_mail_code,
                   ST_X(l.geolocation::geometry) as longitude, ST_Y(l.geolocation::geometry) as latitude,
                   c.created_at, c.updated_at,
                   SIMILARITY(UPPER(c.name), $1) as similarity_score
            FROM clubs c
            LEFT JOIN locations l ON c.location_id = l.id
            WHERE SIMILARITY(UPPER(c.name), $1) > 0.05
            AND c.is_soaring = true
            ORDER BY similarity_score DESC, c.name
            LIMIT $2
            "#,
            query_upper,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        let mut clubs = Vec::new();
        for row in results {
            let base_location = if row.longitude.is_some() && row.latitude.is_some() {
                Some(crate::clubs::Point::new(
                    row.latitude.unwrap(),
                    row.longitude.unwrap(),
                ))
            } else {
                None
            };

            clubs.push(Club {
                id: row.id,
                name: row.name,
                is_soaring: row.is_soaring,
                home_base_airport_id: row.home_base_airport_id,
                location_id: row.location_id,
                street1: row.street1,
                street2: row.street2,
                city: row.city,
                state: row.state,
                zip_code: row.zip_code,
                region_code: row.region_code,
                county_mail_code: row.county_mail_code,
                country_mail_code: row.country_mail_code,
                base_location,
                created_at: row.created_at.unwrap_or_else(Utc::now),
                updated_at: row.updated_at.unwrap_or_else(Utc::now),
            });
        }

        Ok(clubs)
    }

    /// Search soaring clubs within a radius of a given point using PostGIS
    /// Returns soaring clubs (is_soaring=true) within the specified radius (in kilometers)
    pub async fn search_nearby_soaring(&self, latitude: f64, longitude: f64, radius_km: f64, limit: Option<i64>) -> Result<Vec<Club>> {
        let limit = limit.unwrap_or(20);
        let radius_m = radius_km * 1000.0; // Convert km to meters for PostGIS

        let results = sqlx::query!(
            r#"
            SELECT c.id, c.name, c.is_soaring, c.home_base_airport_id, c.location_id,
                   l.street1, l.street2, l.city, l.state, l.zip_code, l.region_code,
                   l.county_mail_code, l.country_mail_code,
                   ST_X(l.geolocation::geometry) as longitude, ST_Y(l.geolocation::geometry) as latitude,
                   c.created_at, c.updated_at,
                   ST_Distance(ST_SetSRID(l.geolocation::geometry, 4326)::geography, ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography) as distance_meters
            FROM clubs c
            LEFT JOIN locations l ON c.location_id = l.id
            WHERE l.geolocation IS NOT NULL
            AND c.is_soaring = true
            AND ST_DWithin(ST_SetSRID(l.geolocation::geometry, 4326)::geography, ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography, $3)
            ORDER BY distance_meters
            LIMIT $4
            "#,
            latitude,
            longitude,
            radius_m,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        let mut clubs = Vec::new();
        for row in results {
            let base_location = if row.longitude.is_some() && row.latitude.is_some() {
                Some(crate::clubs::Point::new(
                    row.latitude.unwrap(),
                    row.longitude.unwrap(),
                ))
            } else {
                None
            };

            clubs.push(Club {
                id: row.id,
                name: row.name,
                is_soaring: row.is_soaring,
                home_base_airport_id: row.home_base_airport_id,
                location_id: row.location_id,
                street1: row.street1,
                street2: row.street2,
                city: row.city,
                state: row.state,
                zip_code: row.zip_code,
                region_code: row.region_code,
                county_mail_code: row.county_mail_code,
                country_mail_code: row.country_mail_code,
                base_location,
                created_at: row.created_at.unwrap_or_else(Utc::now),
                updated_at: row.updated_at.unwrap_or_else(Utc::now),
            });
        }

        Ok(clubs)
    }

    /// Insert a new club
    pub async fn insert(&self, club: &Club) -> Result<()> {
        // First create location if we have address data
        let location_id = if let Some(location_id) = club.location_id {
            location_id
        } else if club.street1.is_some() || club.city.is_some() {
            let location_geolocation = club.base_location.as_ref().map(|loc| {
                crate::locations::Point::new(loc.latitude, loc.longitude)
            });
            
            let location = self.locations_repo.find_or_create(
                club.street1.clone(),
                club.street2.clone(),
                club.city.clone(),
                club.state.clone(),
                club.zip_code.clone(),
                club.region_code.clone(),
                club.county_mail_code.clone(),
                club.country_mail_code.clone(),
                location_geolocation,
            ).await?;
            location.id
        } else {
            return Err(anyhow::anyhow!("Club must have either location_id or address fields"));
        };

        sqlx::query!(
            r#"
            INSERT INTO clubs (
                id, name, is_soaring, home_base_airport_id, location_id,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            club.id,
            club.name,
            club.is_soaring,
            club.home_base_airport_id,
            location_id,
            club.created_at,
            club.updated_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // Note: These tests would require a test database setup
    // For now, they're just structural examples

    fn create_test_club() -> Club {
        Club {
            id: Uuid::new_v4(),
            name: "Adirondack Soaring Club".to_string(),
            is_soaring: Some(true),
            home_base_airport_id: None,
            location_id: None,
            street1: Some("123 Mountain Rd".to_string()),
            street2: None,
            city: Some("Lake Placid".to_string()),
            state: Some("NY".to_string()),
            zip_code: Some("12946".to_string()),
            region_code: None,
            county_mail_code: None,
            country_mail_code: None,
            base_location: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_club_creation() {
        let club = create_test_club();
        assert_eq!(club.name, "Adirondack Soaring Club");
        assert_eq!(club.is_soaring, Some(true));
        assert_eq!(club.city, Some("Lake Placid".to_string()));
        assert_eq!(club.state, Some("NY".to_string()));
    }
}