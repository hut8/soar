use anyhow::Result;
use diesel::prelude::*;
use diesel::sql_types::*;
use uuid::Uuid;

use crate::locations::{Location, LocationModel, NewLocationModel, Point};
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
                .select(LocationModel::as_select())
                .first(&mut conn)
                .optional()?;

            Ok::<Option<LocationModel>, anyhow::Error>(location_model)
        })
        .await??;

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
                    street1
                        .eq(&street1_val)
                        .and(street2.eq(&street2_val))
                        .and(city.eq(&city_val))
                        .and(state.eq(&state_val))
                        .and(zip_code.eq(&zip_code_val))
                        .and(country_mail_code.eq(&country_val)),
                )
                .select(LocationModel::as_select())
                .first(&mut conn)
                .optional()?;

            Ok::<Option<LocationModel>, anyhow::Error>(location_model)
        })
        .await??;

        Ok(result.map(|model| model.into()))
    }

    /// Insert a new location
    pub async fn insert(&self, location: &Location) -> Result<()> {
        use crate::schema::locations;

        let pool = self.pool.clone();
        let new_location_model: NewLocationModel = location.clone().into();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            diesel::insert_into(locations::table)
                .values(&new_location_model)
                .execute(&mut conn)?;

            Ok::<(), anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    /// Update geolocation for a location
    pub async fn update_geolocation(
        &self,
        location_id: Uuid,
        new_geolocation: Point,
    ) -> Result<bool> {
        use crate::schema::locations::dsl::*;
        use chrono::Utc;

        let pool = self.pool.clone();

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let rows = diesel::update(locations.filter(id.eq(location_id)))
                .set((geolocation.eq(&new_geolocation), updated_at.eq(Utc::now())))
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(rows)
        })
        .await??;

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

            let location_results: Vec<LocationForGeocoding> =
                diesel::sql_query(&raw_query).load::<LocationForGeocoding>(&mut conn)?;

            Ok::<Vec<LocationForGeocoding>, anyhow::Error>(location_results)
        })
        .await??;

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

    /// Find or create a location by address (atomic operation)
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
        use crate::schema::locations::dsl::locations as locations_table;

        let pool = self.pool.clone();

        // Clone values for the closure
        let param_street1 = street1.clone();
        let param_street2 = street2.clone();
        let param_city = city.clone();
        let param_state = state.clone();
        let param_zip_code = zip_code.clone();
        let param_region_code = region_code.clone();
        let param_county_mail_code = county_mail_code.clone();
        let param_country_mail_code = country_mail_code.clone();
        let param_geolocation = geolocation;

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // First, try to insert the new location using ON CONFLICT DO NOTHING
            let new_location = Location::new(
                param_street1.clone(),
                param_street2.clone(),
                param_city.clone(),
                param_state.clone(),
                param_zip_code.clone(),
                param_region_code.clone(),
                param_county_mail_code.clone(),
                param_country_mail_code.clone(),
                param_geolocation,
            );

            let new_location_model: NewLocationModel = new_location.into();

            // Use INSERT ... ON CONFLICT DO NOTHING for atomic upsert
            diesel::insert_into(locations_table)
                .values(&new_location_model)
                .on_conflict_do_nothing()
                .execute(&mut conn)?;

            // Now select the location (either the one we just created or the existing one)
            // We need to match the exact COALESCE logic from the unique constraint
            let search_street1 = param_street1.as_deref().unwrap_or("");
            let search_street2 = param_street2.as_deref().unwrap_or("");
            let search_city = param_city.as_deref().unwrap_or("");
            let search_state = param_state.as_deref().unwrap_or("");
            let search_zip_code = param_zip_code.as_deref().unwrap_or("");
            let search_country = param_country_mail_code.as_deref().unwrap_or("US");

            // Use SQL to match the exact same COALESCE logic as the unique constraint
            let location_model = diesel::sql_query(
                "SELECT id, street1, street2, city, state, zip_code, region_code, county_mail_code, country_mail_code, geolocation, created_at, updated_at
                 FROM locations
                 WHERE COALESCE(street1, '') = $1
                   AND COALESCE(street2, '') = $2
                   AND COALESCE(city, '') = $3
                   AND COALESCE(state, '') = $4
                   AND COALESCE(zip_code, '') = $5
                   AND COALESCE(country_mail_code, 'US') = $6"
            )
            .bind::<diesel::sql_types::Text, _>(search_street1)
            .bind::<diesel::sql_types::Text, _>(search_street2)
            .bind::<diesel::sql_types::Text, _>(search_city)
            .bind::<diesel::sql_types::Text, _>(search_state)
            .bind::<diesel::sql_types::Text, _>(search_zip_code)
            .bind::<diesel::sql_types::Text, _>(search_country)
            .get_result::<LocationModel>(&mut conn)?;

            Ok::<Location, anyhow::Error>(location_model.into())
        })
        .await??;

        Ok(result)
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
