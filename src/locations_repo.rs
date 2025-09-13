use anyhow::Result;
use diesel::prelude::*;
use diesel::sql_types::*;
use uuid::Uuid;

use crate::locations::{Location, LocationModel, Point};
use crate::web::PgPool;

#[derive(QueryableByName, Debug)]
struct LocationForGeocoding {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
    #[diesel(sql_type = Nullable<Varchar>)]
    street1: Option<String>,
    #[diesel(sql_type = Nullable<Varchar>)]
    street2: Option<String>,
    #[diesel(sql_type = Nullable<Varchar>)]
    city: Option<String>,
    #[diesel(sql_type = Nullable<Varchar>)]
    state: Option<String>,
    #[diesel(sql_type = Nullable<Varchar>)]
    zip_code: Option<String>,
    #[diesel(sql_type = Nullable<Varchar>)]
    region_code: Option<String>,
    #[diesel(sql_type = Nullable<Varchar>)]
    county_mail_code: Option<String>,
    #[diesel(sql_type = Nullable<Varchar>)]
    country_mail_code: Option<String>,
    #[diesel(sql_type = Timestamptz)]
    created_at: chrono::DateTime<chrono::Utc>,
    #[diesel(sql_type = Timestamptz)]
    updated_at: chrono::DateTime<chrono::Utc>,
}

pub struct LocationsRepository {
    pool: PgPool,
}

impl LocationsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get location by ID
    pub async fn get_by_id(&self, location_id: Uuid) -> Result<Option<Location>> {
        use crate::schema::locations::dsl::*;
        
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            
            let location_model: Option<LocationModel> = locations
                .filter(id.eq(location_id))
                .first::<LocationModel>(&mut conn)
                .optional()?;
                
            Ok::<Option<LocationModel>, anyhow::Error>(location_model)
        }).await??;

        Ok(result.map(|model| model.into()))
    }

    /// Find location by address fields
    pub async fn find_by_address(
        &self,
        street1_param: Option<&str>,
        street2_param: Option<&str>,
        city_param: Option<&str>,
        state_param: Option<&str>,
        zip_code_param: Option<&str>,
        country_mail_code_param: Option<&str>,
    ) -> Result<Option<Location>> {
        use crate::schema::locations::dsl::*;
        
        let pool = self.pool.clone();
        let street1_val = street1_param.map(|s| s.to_string());
        let street2_val = street2_param.map(|s| s.to_string());
        let city_val = city_param.map(|s| s.to_string());
        let state_val = state_param.map(|s| s.to_string());
        let zip_code_val = zip_code_param.map(|s| s.to_string());
        let country_val = country_mail_code_param.unwrap_or("US").to_string();
        
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            
            let location_model: Option<LocationModel> = locations
                .filter(
                    street1.eq(&street1_val)
                    .and(street2.eq(&street2_val))
                    .and(city.eq(&city_val))
                    .and(state.eq(&state_val))
                    .and(zip_code.eq(&zip_code_val))
                    .and(country_mail_code.eq(&country_val))
                )
                .first::<LocationModel>(&mut conn)
                .optional()?;
                
            Ok::<Option<LocationModel>, anyhow::Error>(location_model)
        }).await??;

        Ok(result.map(|model| model.into()))
    }

    /// Insert a new location
    pub async fn insert(&self, location: &Location) -> Result<()> {
        use crate::schema::locations;
        
        let pool = self.pool.clone();
        let location_model: LocationModel = location.clone().into();
        
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            
            diesel::insert_into(locations::table)
                .values(&location_model)
                .execute(&mut conn)?;
                
            Ok::<(), anyhow::Error>(())
        }).await??;
        
        Ok(())
    }

    /// Update geolocation for a location
    pub async fn update_geolocation(&self, location_id: Uuid, existing_geolocation: Point) -> Result<bool> {
        use crate::schema::locations::dsl::*;
        use chrono::Utc;
        
        let pool = self.pool.clone();
        let geolocation_text = format!("POINT({} {})", existing_geolocation.longitude, existing_geolocation.latitude);
        
        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            
            let rows = diesel::update(locations.filter(id.eq(location_id)))
                .set((
                    geolocation.eq(&geolocation_text),
                    updated_at.eq(Utc::now())
                ))
                .execute(&mut conn)?;
                
            Ok::<usize, anyhow::Error>(rows)
        }).await??;

        Ok(rows_affected > 0)
    }

    /// Get locations that need geocoding (have address but no geolocation)
    pub async fn get_locations_for_geocoding(&self, limit: Option<i64>) -> Result<Vec<Location>> {
        let pool = self.pool.clone();
        let query_limit = limit.unwrap_or(100);
        
        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            
            let raw_query = format!(
                "SELECT id, street1, street2, city, state, zip_code, region_code,
                        county_mail_code, country_mail_code, created_at, updated_at
                 FROM locations
                 WHERE geolocation IS NULL
                   AND (street1 IS NOT NULL OR city IS NOT NULL OR state IS NOT NULL)
                   AND EXISTS (
                       SELECT 1 FROM clubs c
                       WHERE c.location_id = locations.id
                       AND c.is_soaring = TRUE
                   )
                 ORDER BY created_at ASC
                 LIMIT {}",
                query_limit
            );
            
            let location_results: Vec<LocationForGeocoding> = diesel::sql_query(&raw_query)
                .load::<LocationForGeocoding>(&mut conn)?;
                
            Ok::<Vec<LocationForGeocoding>, anyhow::Error>(location_results)
        }).await??;

        let mut locations = Vec::new();
        for row in results {
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
                geolocation: None, // These locations need geocoding, so no geolocation
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
        if let Some(existing) = self
            .find_by_address(
                street1.as_deref(),
                street2.as_deref(),
                city.as_deref(),
                state.as_deref(),
                zip_code.as_deref(),
                country_mail_code.as_deref(),
            )
            .await?
        {
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
