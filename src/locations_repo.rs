use anyhow::Result;
use sqlx::postgres::types::PgPoint;
use sqlx::PgPool;
use sqlx::types::Uuid;

use crate::locations::{Location, Point};

pub struct LocationsRepository {
    pool: PgPool,
}

impl LocationsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get location by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Location>> {
        let result = sqlx::query!(
            r#"
            SELECT id, street1, street2, city, state, zip_code, region_code,
                   county_mail_code, country_mail_code,
                   ST_X(geolocation::geometry) as longitude, ST_Y(geolocation::geometry) as latitude,
                   created_at, updated_at
            FROM locations
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            let geolocation = if row.longitude.is_some() && row.latitude.is_some() {
                Some(Point::new(
                    row.latitude.unwrap(),
                    row.longitude.unwrap(),
                ))
            } else {
                None
            };

            Ok(Some(Location {
                id: row.id,
                street1: row.street1,
                street2: row.street2,
                city: row.city,
                state: row.state,
                zip_code: row.zip_code,
                region_code: row.region_code,
                county_mail_code: row.county_mail_code,
                country_mail_code: row.country_mail_code,
                geolocation,
                created_at: row.created_at,
                updated_at: row.updated_at,
            }))
        } else {
            Ok(None)
        }
    }

    /// Find location by address fields
    pub async fn find_by_address(
        &self,
        street1: Option<&str>,
        street2: Option<&str>,
        city: Option<&str>,
        state: Option<&str>,
        zip_code: Option<&str>,
        country_mail_code: Option<&str>,
    ) -> Result<Option<Location>> {
        let result = sqlx::query!(
            r#"
            SELECT id, street1, street2, city, state, zip_code, region_code,
                   county_mail_code, country_mail_code,
                   ST_X(geolocation::geometry) as longitude, ST_Y(geolocation::geometry) as latitude,
                   created_at, updated_at
            FROM locations
            WHERE COALESCE(street1, '') = COALESCE($1, '')
              AND COALESCE(street2, '') = COALESCE($2, '')
              AND COALESCE(city, '') = COALESCE($3, '')
              AND COALESCE(state, '') = COALESCE($4, '')
              AND COALESCE(zip_code, '') = COALESCE($5, '')
              AND COALESCE(country_mail_code, 'US') = COALESCE($6, 'US')
            "#,
            street1,
            street2,
            city,
            state,
            zip_code,
            country_mail_code.unwrap_or("US")
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            let geolocation = if row.longitude.is_some() && row.latitude.is_some() {
                Some(Point::new(
                    row.latitude.unwrap(),
                    row.longitude.unwrap(),
                ))
            } else {
                None
            };

            Ok(Some(Location {
                id: row.id,
                street1: row.street1,
                street2: row.street2,
                city: row.city,
                state: row.state,
                zip_code: row.zip_code,
                region_code: row.region_code,
                county_mail_code: row.county_mail_code,
                country_mail_code: row.country_mail_code,
                geolocation,
                created_at: row.created_at,
                updated_at: row.updated_at,
            }))
        } else {
            Ok(None)
        }
    }

    /// Insert a new location
    pub async fn insert(&self, location: &Location) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO locations (
                id, street1, street2, city, state, zip_code, region_code,
                county_mail_code, country_mail_code, geolocation,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
            location.id,
            location.street1,
            location.street2,
            location.city,
            location.state,
            location.zip_code,
            location.region_code,
            location.county_mail_code,
            location.country_mail_code,
            location.geolocation.as_ref().map(|p| PgPoint { x: p.longitude, y: p.latitude }),
            location.created_at,
            location.updated_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update geolocation for a location
    pub async fn update_geolocation(&self, location_id: Uuid, geolocation: Point) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            UPDATE locations
            SET geolocation = $2, updated_at = NOW()
            WHERE id = $1
            "#,
            location_id,
            PgPoint { x: geolocation.longitude, y: geolocation.latitude }
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get locations that need geocoding (have address but no geolocation)
    pub async fn get_locations_for_geocoding(&self, limit: Option<i64>) -> Result<Vec<Location>> {
        let limit = limit.unwrap_or(100);

        let results = sqlx::query!(
            r#"
            SELECT id, street1, street2, city, state, zip_code, region_code,
                   county_mail_code, country_mail_code,
                   ST_X(geolocation::geometry) as longitude, ST_Y(geolocation::geometry) as latitude,
                   created_at, updated_at
            FROM locations
            WHERE geolocation IS NULL
              AND (street1 IS NOT NULL OR city IS NOT NULL OR state IS NOT NULL)
            ORDER BY created_at ASC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        let mut locations = Vec::new();
        for row in results {
            let geolocation = if row.longitude.is_some() && row.latitude.is_some() {
                Some(Point::new(
                    row.latitude.unwrap(),
                    row.longitude.unwrap(),
                ))
            } else {
                None
            };

            locations.push(Location {
                id: row.id,
                street1: row.street1,
                street2: row.street2,
                city: row.city,
                state: row.state,
                zip_code: row.zip_code,
                region_code: row.region_code,
                county_mail_code: row.county_mail_code,
                country_mail_code: row.country_mail_code,
                geolocation,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(locations)
    }

    /// Find or create a location by address
    #[allow(clippy::too_many_arguments)]
    pub async fn find_or_create(
        &self,
        street1: Option<String>,
        street2: Option<String>,
        city: Option<String>,
        state: Option<String>,
        zip_code: Option<String>,
        region_code: Option<String>,
        county_mail_code: Option<String>,
        country_mail_code: Option<String>,
        geolocation: Option<Point>,
    ) -> Result<Location> {
        // Try to find existing location
        if let Some(existing) = self.find_by_address(
            street1.as_deref(),
            street2.as_deref(),
            city.as_deref(),
            state.as_deref(),
            zip_code.as_deref(),
            country_mail_code.as_deref(),
        ).await? {
            return Ok(existing);
        }

        // Create new location
        let location = Location::new(
            street1,
            street2,
            city,
            state,
            zip_code,
            region_code,
            county_mail_code,
            country_mail_code,
            geolocation,
        );

        self.insert(&location).await?;
        Ok(location)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_location() -> Location {
        Location {
            id: Uuid::new_v4(),
            street1: Some("123 Main St".to_string()),
            street2: Some("Suite 100".to_string()),
            city: Some("Anytown".to_string()),
            state: Some("CA".to_string()),
            zip_code: Some("12345".to_string()),
            region_code: Some("4".to_string()),
            county_mail_code: Some("037".to_string()),
            country_mail_code: Some("US".to_string()),
            geolocation: Some(Point::new(34.0522, -118.2437)),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_location_creation() {
        let location = create_test_location();
        assert!(location.id != Uuid::nil());
        assert_eq!(location.street1, Some("123 Main St".to_string()));
        assert_eq!(location.city, Some("Anytown".to_string()));
        assert!(location.has_coordinates());
    }
}
