use anyhow::Result;
use diesel::prelude::*;
use uuid::Uuid;

use crate::locations::{Location, LocationModel, NewLocationModel, Point};
use crate::web::PgPool;

#[derive(Clone)]
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
        use crate::schema::clubs::dsl as clubs_dsl;
        use crate::schema::locations::dsl::*;
        use diesel::dsl::exists;

        let pool = self.pool.clone();
        let query_limit = limit.unwrap_or(100);

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Build the EXISTS subquery
            // Note: location_id in clubs is Nullable<Uuid>, so we need to match it with id.nullable()
            let soaring_club_exists = clubs_dsl::clubs
                .filter(clubs_dsl::location_id.eq(id.nullable()))
                .filter(clubs_dsl::is_soaring.eq(true));

            // Build the main query using Diesel's query builder
            let location_models: Vec<LocationModel> = locations
                .filter(geolocation.is_null())
                .filter(
                    street1
                        .is_not_null()
                        .or(city.is_not_null())
                        .or(state.is_not_null()),
                )
                .filter(exists(soaring_club_exists))
                .order(created_at.asc())
                .limit(query_limit)
                .select(LocationModel::as_select())
                .load(&mut conn)?;

            Ok::<Vec<LocationModel>, anyhow::Error>(location_models)
        })
        .await??;

        Ok(results.into_iter().map(|model| model.into()).collect())
    }

    /// Find or create a location by geolocation only (for receivers)
    /// This creates a minimal location record with just coordinates
    pub async fn find_or_create_by_geolocation(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<Location> {
        // Use the general find_or_create with only geolocation
        self.find_or_create(
            None, // street1
            None, // street2
            None, // city
            None, // state
            None, // zip_code
            None, // region_code
            None, // country_mail_code
            Some(Point::new(latitude, longitude)),
        )
        .await
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
            // Use direct nullable comparisons matching the simplified unique index
            use crate::schema::locations::dsl::*;

            // Use direct nullable comparisons - no COALESCE needed
            let location_model = locations
                .filter(street1.eq(&param_street1))
                .filter(street2.eq(&param_street2))
                .filter(city.eq(&param_city))
                .filter(state.eq(&param_state))
                .filter(zip_code.eq(&param_zip_code))
                .filter(country_mail_code.eq(&param_country_mail_code))
                .select(LocationModel::as_select())
                .first::<LocationModel>(&mut conn)?;

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
            id: Uuid::now_v7(),
            street1: Some("123 Main St".to_string()),
            street2: Some("Suite 100".to_string()),
            city: Some("Anytown".to_string()),
            state: Some("CA".to_string()),
            zip_code: Some("12345".to_string()),
            region_code: Some("4".to_string()),
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
