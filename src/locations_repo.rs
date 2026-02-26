use anyhow::Result;
use chrono::TimeZone;
use diesel::prelude::*;
use uuid::Uuid;

use crate::locations::{Location, LocationModel, NewLocationModel, Point};
use crate::web::PgPool;

/// Parameters for finding or creating a location
#[derive(Debug, Clone)]
pub struct LocationParams {
    pub street1: Option<String>,
    pub street2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip_code: Option<String>,
    pub country_code: Option<String>,
    pub geolocation: Option<Point>,
}

#[derive(Clone)]
pub struct LocationsRepository {
    pool: PgPool,
}

impl LocationsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a location by its ID
    pub async fn get_by_id(&self, location_id: Uuid) -> Result<Option<Location>> {
        use crate::schema::locations::dsl::*;

        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let location_model = locations
                .filter(id.eq(location_id))
                .first::<LocationModel>(&mut conn)
                .optional()?;

            Ok::<Option<LocationModel>, anyhow::Error>(location_model)
        })
        .await??;

        Ok(result.map(|model| model.into()))
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

    /// Mark a location as having had geocoding attempted (regardless of success/failure)
    /// This prevents re-attempting geocoding on addresses that are known to fail
    pub async fn mark_geocode_attempted(&self, location_id: Uuid) -> Result<bool> {
        use crate::schema::locations::dsl::*;
        use chrono::Utc;

        let pool = self.pool.clone();

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let rows = diesel::update(locations.filter(id.eq(location_id)))
                .set(geocode_attempted_at.eq(Some(Utc::now())))
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
            // Only select locations that:
            // - Have no geolocation yet
            // - Have never had geocoding attempted (to avoid re-trying known failures)
            // - Have at least some address data
            // - Are linked to a soaring club
            let location_models: Vec<LocationModel> = locations
                .filter(geolocation.is_null())
                .filter(geocode_attempted_at.is_null())
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
    pub async fn find_or_create(&self, params: LocationParams) -> Result<Location> {
        use crate::schema::locations::dsl::*;

        let pool = self.pool.clone();

        // Clone and normalize values for the closure
        let param_street1 = params.street1;
        let param_street2 = params.street2;
        let param_city = params.city;
        let param_state = params.state;
        let param_zip_code = params.zip_code;
        let param_country_code = params.country_code.map(|c| c.to_uppercase());
        let param_geolocation = params.geolocation;

        let new_location = Location::new(
            param_street1.clone(),
            param_street2.clone(),
            param_city.clone(),
            param_state.clone(),
            param_zip_code.clone(),
            param_country_code.clone(),
            param_geolocation,
        );

        let new_location_model: NewLocationModel = new_location.into();
        let generated_id = new_location_model.id;

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Try to insert with ON CONFLICT DO NOTHING (no table bloat from no-op updates)
            let rows_affected = diesel::insert_into(locations)
                .values(&new_location_model)
                .on_conflict_do_nothing()
                .execute(&mut conn)?;

            let location_model: LocationModel = if rows_affected > 0 {
                // We inserted successfully - fetch by our generated ID (fast PK lookup)
                locations
                    .filter(id.eq(generated_id))
                    .select(LocationModel::as_select())
                    .first(&mut conn)?
            } else {
                // Conflict occurred - find existing location by address fields
                // Use IS NOT DISTINCT FROM for NULL-safe equality matching
                diesel::sql_query(
                    r#"
                    SELECT id, street1, street2, city, state, zip_code, country_code,
                           geolocation, created_at, updated_at, geocode_attempted_at
                    FROM locations
                    WHERE street1 IS NOT DISTINCT FROM $1
                      AND street2 IS NOT DISTINCT FROM $2
                      AND city IS NOT DISTINCT FROM $3
                      AND state IS NOT DISTINCT FROM $4
                      AND zip_code IS NOT DISTINCT FROM $5
                      AND country_code IS NOT DISTINCT FROM $6
                    LIMIT 1
                    "#,
                )
                .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&param_street1)
                .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&param_street2)
                .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&param_city)
                .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&param_state)
                .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&param_zip_code)
                .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(
                    &param_country_code,
                )
                .get_result::<LocationModel>(&mut conn)?
            };

            Ok::<Location, anyhow::Error>(location_model.into())
        })
        .await??;

        Ok(result)
    }

    /// Count unreferenced locations (locations not used by any other table)
    /// This helps identify potentially orphaned or problematic location records
    pub async fn count_unreferenced_locations(&self) -> Result<i64> {
        let pool = self.pool.clone();

        let count = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let result: i64 = diesel::sql_query(
                r#"
                SELECT COUNT(*) as count
                FROM locations l
                WHERE NOT EXISTS (SELECT 1 FROM aircraft_registrations WHERE location_id = l.id)
                  AND NOT EXISTS (SELECT 1 FROM airports WHERE location_id = l.id)
                  AND NOT EXISTS (SELECT 1 FROM clubs WHERE location_id = l.id)
                  AND NOT EXISTS (SELECT 1 FROM flights WHERE end_location_id = l.id)
                  AND NOT EXISTS (SELECT 1 FROM flights WHERE start_location_id = l.id)
                "#,
            )
            .get_result::<CountQueryResult>(&mut conn)?
            .count;

            Ok::<i64, anyhow::Error>(result)
        })
        .await??;

        Ok(count)
    }

    /// Count unreferenced locations created within a date range
    /// Returns the number of locations not referenced by any other table
    /// that were created between start_date and end_date (inclusive)
    pub async fn count_unreferenced_locations_in_range(
        &self,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<i64> {
        let pool = self.pool.clone();

        let count = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Convert NaiveDates to DateTime<Utc> for comparison
            let start_datetime = chrono::Utc
                .from_local_datetime(&start_date.and_hms_opt(0, 0, 0).unwrap())
                .single()
                .ok_or_else(|| anyhow::anyhow!("Failed to create start datetime"))?;
            let end_datetime = chrono::Utc
                .from_local_datetime(&end_date.and_hms_opt(23, 59, 59).unwrap())
                .single()
                .ok_or_else(|| anyhow::anyhow!("Failed to create end datetime"))?;

            let result: i64 = diesel::sql_query(
                r#"
                SELECT COUNT(*) as count
                FROM locations l
                WHERE l.created_at >= $1 AND l.created_at <= $2
                  AND NOT EXISTS (SELECT 1 FROM aircraft_registrations WHERE location_id = l.id)
                  AND NOT EXISTS (SELECT 1 FROM airports WHERE location_id = l.id)
                  AND NOT EXISTS (SELECT 1 FROM clubs WHERE location_id = l.id)
                  AND NOT EXISTS (SELECT 1 FROM flights WHERE end_location_id = l.id)
                  AND NOT EXISTS (SELECT 1 FROM flights WHERE start_location_id = l.id)
                "#,
            )
            .bind::<diesel::sql_types::Timestamptz, _>(start_datetime)
            .bind::<diesel::sql_types::Timestamptz, _>(end_datetime)
            .get_result::<CountQueryResult>(&mut conn)?
            .count;

            Ok::<i64, anyhow::Error>(result)
        })
        .await??;

        Ok(count)
    }

    /// Get paginated list of unreferenced locations
    /// Returns locations that are not referenced by any other table
    pub async fn get_unreferenced_locations(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Location>> {
        use crate::schema::aircraft_registrations::dsl as ar_dsl;
        use crate::schema::airports::dsl as airports_dsl;
        use crate::schema::clubs::dsl as clubs_dsl;
        use crate::schema::flights::dsl as flights_dsl;
        use crate::schema::locations::dsl::*;
        use diesel::dsl::{exists, not};

        let pool = self.pool.clone();

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Build NOT EXISTS subqueries for each referencing table
            let ar_exists =
                ar_dsl::aircraft_registrations.filter(ar_dsl::location_id.eq(id.nullable()));
            let airports_exists =
                airports_dsl::airports.filter(airports_dsl::location_id.eq(id.nullable()));
            let clubs_exists = clubs_dsl::clubs.filter(clubs_dsl::location_id.eq(id.nullable()));
            let flights_end_exists =
                flights_dsl::flights.filter(flights_dsl::end_location_id.eq(id.nullable()));
            let flights_start_exists =
                flights_dsl::flights.filter(flights_dsl::start_location_id.eq(id.nullable()));

            let location_models: Vec<LocationModel> = locations
                .filter(not(exists(ar_exists)))
                .filter(not(exists(airports_exists)))
                .filter(not(exists(clubs_exists)))
                .filter(not(exists(flights_end_exists)))
                .filter(not(exists(flights_start_exists)))
                .order(created_at.desc())
                .limit(limit)
                .offset(offset)
                .select(LocationModel::as_select())
                .load(&mut conn)?;

            Ok::<Vec<LocationModel>, anyhow::Error>(location_models)
        })
        .await??;

        Ok(results.into_iter().map(|model| model.into()).collect())
    }
}

// Helper struct for COUNT queries
#[derive(QueryableByName)]
struct CountQueryResult {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
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
            country_code: Some("US".to_string()),
            geolocation: Some(Point::new(34.0522, -118.2437)),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            geocode_attempted_at: None,
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
