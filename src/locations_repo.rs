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
            // Use COALESCE to match the expression-based unique index
            use diesel::sql_types::Text;

            // Use raw SQL with COALESCE to match the unique index exactly
            let location_model: LocationModel = diesel::sql_query(
                r#"
                SELECT id, street1, street2, city, state, zip_code, region_code, country_mail_code,
                       geolocation, created_at, updated_at
                FROM locations
                WHERE COALESCE(street1, '') = COALESCE($1, '')
                  AND COALESCE(street2, '') = COALESCE($2, '')
                  AND COALESCE(city, '') = COALESCE($3, '')
                  AND COALESCE(state, '') = COALESCE($4, '')
                  AND COALESCE(zip_code, '') = COALESCE($5, '')
                  AND COALESCE(country_mail_code, '') = COALESCE($6, '')
                LIMIT 1
                "#,
            )
            .bind::<diesel::sql_types::Nullable<Text>, _>(&param_street1)
            .bind::<diesel::sql_types::Nullable<Text>, _>(&param_street2)
            .bind::<diesel::sql_types::Nullable<Text>, _>(&param_city)
            .bind::<diesel::sql_types::Nullable<Text>, _>(&param_state)
            .bind::<diesel::sql_types::Nullable<Text>, _>(&param_zip_code)
            .bind::<diesel::sql_types::Nullable<Text>, _>(&param_country_mail_code)
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
