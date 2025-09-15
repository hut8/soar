use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::clubs::{Club, NewClubModel};
use crate::locations::Point;
// use crate::locations_repo::LocationsRepository;
use crate::web::PgPool;

#[derive(QueryableByName, Debug)]
struct ClubWithLocation {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Bool>)]
    is_soaring: Option<bool>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
    home_base_airport_id: Option<i32>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
    location_id: Option<Uuid>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    street1: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    street2: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    city: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    state: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    zip_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    region_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    county_mail_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    country_mail_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float8>)]
    longitude: Option<f64>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float8>)]
    latitude: Option<f64>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    created_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    updated_at: DateTime<Utc>,
}

#[derive(QueryableByName, Debug)]
pub struct ClubWithLocationAndSimilarity {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Bool>)]
    pub is_soaring: Option<bool>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
    pub home_base_airport_id: Option<i32>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
    pub location_id: Option<Uuid>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub street1: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub street2: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub city: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub state: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub zip_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub region_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub county_mail_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub country_mail_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float8>)]
    pub longitude: Option<f64>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float8>)]
    pub latitude: Option<f64>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub created_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub updated_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float8>)]
    pub similarity_score: Option<f64>,
}

#[derive(QueryableByName, Debug)]
pub struct ClubWithLocationAndDistance {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Bool>)]
    pub is_soaring: Option<bool>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
    pub home_base_airport_id: Option<i32>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
    pub location_id: Option<Uuid>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub street1: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub street2: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub city: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub state: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub zip_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub region_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub county_mail_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub country_mail_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float8>)]
    pub longitude: Option<f64>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float8>)]
    pub latitude: Option<f64>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub created_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub updated_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float8>)]
    pub distance_meters: Option<f64>,
}

impl From<ClubWithLocationAndDistance> for Club {
    fn from(cwld: ClubWithLocationAndDistance) -> Self {
        let base_location = if cwld.longitude.is_some() && cwld.latitude.is_some() {
            Some(Point::new(cwld.latitude.unwrap(), cwld.longitude.unwrap()))
        } else {
            None
        };

        Self {
            id: cwld.id,
            name: cwld.name,
            is_soaring: cwld.is_soaring,
            home_base_airport_id: cwld.home_base_airport_id,
            location_id: cwld.location_id,
            street1: cwld.street1,
            street2: cwld.street2,
            city: cwld.city,
            state: cwld.state,
            zip_code: cwld.zip_code,
            region_code: cwld.region_code,
            county_mail_code: cwld.county_mail_code,
            country_mail_code: cwld.country_mail_code,
            base_location,
            created_at: cwld.created_at,
            updated_at: cwld.updated_at,
        }
    }
}

impl From<ClubWithLocationAndSimilarity> for Club {
    fn from(cwls: ClubWithLocationAndSimilarity) -> Self {
        let base_location = if cwls.longitude.is_some() && cwls.latitude.is_some() {
            Some(Point::new(cwls.latitude.unwrap(), cwls.longitude.unwrap()))
        } else {
            None
        };

        Self {
            id: cwls.id,
            name: cwls.name,
            is_soaring: cwls.is_soaring,
            home_base_airport_id: cwls.home_base_airport_id,
            location_id: cwls.location_id,
            street1: cwls.street1,
            street2: cwls.street2,
            city: cwls.city,
            state: cwls.state,
            zip_code: cwls.zip_code,
            region_code: cwls.region_code,
            county_mail_code: cwls.county_mail_code,
            country_mail_code: cwls.country_mail_code,
            base_location,
            created_at: cwls.created_at,
            updated_at: cwls.updated_at,
        }
    }
}

impl From<ClubWithLocation> for Club {
    fn from(cwl: ClubWithLocation) -> Self {
        let base_location = if cwl.longitude.is_some() && cwl.latitude.is_some() {
            Some(Point::new(cwl.latitude.unwrap(), cwl.longitude.unwrap()))
        } else {
            None
        };

        Self {
            id: cwl.id,
            name: cwl.name,
            is_soaring: cwl.is_soaring,
            home_base_airport_id: cwl.home_base_airport_id,
            location_id: cwl.location_id,
            street1: cwl.street1,
            street2: cwl.street2,
            city: cwl.city,
            state: cwl.state,
            zip_code: cwl.zip_code,
            region_code: cwl.region_code,
            county_mail_code: cwl.county_mail_code,
            country_mail_code: cwl.country_mail_code,
            base_location,
            created_at: cwl.created_at,
            updated_at: cwl.updated_at,
        }
    }
}

pub struct ClubsRepository {
    pool: PgPool,
    // locations_repo: LocationsRepository,
}

impl ClubsRepository {
    pub fn new(pool: PgPool) -> Self {
        // Note: LocationsRepository will need to be migrated to Diesel as well
        // let locations_repo = LocationsRepository::new(pool.clone());
        Self {
            pool,
            // locations_repo,
        }
    }

    /// Get club by ID
    pub async fn get_by_id(&self, club_id: Uuid) -> Result<Option<Club>> {
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Use raw SQL for complex join with locations table
            let sql = r#"
                SELECT c.id, c.name, c.is_soaring, c.home_base_airport_id, c.location_id,
                       l.street1, l.street2, l.city, l.state, l.zip_code, l.region_code,
                       l.county_mail_code, l.country_mail_code,
                       ST_X(l.geolocation::geometry) as longitude, ST_Y(l.geolocation::geometry) as latitude,
                       c.created_at, c.updated_at
                FROM clubs c
                LEFT JOIN locations l ON c.location_id = l.id
                WHERE c.id = $1
            "#;

            let club_opt: Option<ClubWithLocation> = diesel::sql_query(sql)
                .bind::<diesel::sql_types::Uuid, _>(club_id)
                .get_result::<ClubWithLocation>(&mut conn)
                .optional()?;

            Ok::<Option<ClubWithLocation>, anyhow::Error>(club_opt)
        }).await??;

        Ok(result.map(|cwl| cwl.into()))
    }

    /// Get all clubs
    pub async fn get_all(&self) -> Result<Vec<Club>> {
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Use raw SQL for complex join with locations table
            let sql = r#"
                SELECT c.id, c.name, c.is_soaring, c.home_base_airport_id, c.location_id,
                       l.street1, l.street2, l.city, l.state, l.zip_code, l.region_code,
                       l.county_mail_code, l.country_mail_code,
                       ST_X(l.geolocation::geometry) as longitude, ST_Y(l.geolocation::geometry) as latitude,
                       c.created_at, c.updated_at
                FROM clubs c
                LEFT JOIN locations l ON c.location_id = l.id
                WHERE c.is_soaring = true
                ORDER BY c.name
            "#;

            let clubs: Vec<ClubWithLocation> = diesel::sql_query(sql)
                .load::<ClubWithLocation>(&mut conn)?;

            Ok::<Vec<ClubWithLocation>, anyhow::Error>(clubs)
        }).await??;

        Ok(result.into_iter().map(|cwl| cwl.into()).collect())
    }

    /// Fuzzy search clubs by name using trigram similarity
    /// Returns clubs ordered by similarity score (best matches first)
    pub async fn fuzzy_search(&self, query: &str, limit: Option<i64>) -> Result<Vec<Club>> {
        let query_upper = query.to_uppercase();
        let search_limit = limit.unwrap_or(20);

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let sql = r#"
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
            "#;

            let clubs: Vec<ClubWithLocationAndSimilarity> = diesel::sql_query(sql)
                .bind::<diesel::sql_types::Varchar, _>(&query_upper)
                .bind::<diesel::sql_types::BigInt, _>(search_limit)
                .load::<ClubWithLocationAndSimilarity>(&mut conn)?;

            Ok::<Vec<ClubWithLocationAndSimilarity>, anyhow::Error>(clubs)
        }).await??;

        Ok(result.into_iter().map(|cwls| cwls.into()).collect())
    }

    /// Fuzzy search soaring clubs only by name using trigram similarity
    /// Returns soaring clubs (is_soaring=true) ordered by similarity score (best matches first)
    pub async fn fuzzy_search_soaring(&self, query: &str, limit: Option<i64>) -> Result<Vec<Club>> {
        // This method has the same implementation as fuzzy_search since both filter by is_soaring = true
        self.fuzzy_search(query, limit).await
    }

    /// Search soaring clubs within a radius of a given point using PostGIS
    /// Returns soaring clubs (is_soaring=true) within the specified radius (in kilometers)
    pub async fn search_nearby_soaring(
        &self,
        latitude: f64,
        longitude: f64,
        radius_km: f64,
        limit: Option<i64>,
    ) -> Result<Vec<Club>> {
        let search_limit = limit.unwrap_or(20);
        let radius_m = radius_km * 1000.0; // Convert km to meters for PostGIS

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let sql = r#"
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
            "#;

            let clubs: Vec<ClubWithLocationAndDistance> = diesel::sql_query(sql)
                .bind::<diesel::sql_types::Float8, _>(latitude)
                .bind::<diesel::sql_types::Float8, _>(longitude)
                .bind::<diesel::sql_types::Float8, _>(radius_m)
                .bind::<diesel::sql_types::BigInt, _>(search_limit)
                .load::<ClubWithLocationAndDistance>(&mut conn)?;

            Ok::<Vec<ClubWithLocationAndDistance>, anyhow::Error>(clubs)
        }).await??;

        Ok(result.into_iter().map(|cwld| cwld.into()).collect())
    }

    /// Get soaring clubs that don't have a home base airport ID set
    pub async fn get_soaring_clubs_without_home_base(&self) -> Result<Vec<Club>> {
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let sql = r#"
                SELECT c.id, c.name, c.is_soaring, c.home_base_airport_id, c.location_id,
                       l.street1, l.street2, l.city, l.state, l.zip_code, l.region_code,
                       l.county_mail_code, l.country_mail_code,
                       ST_X(l.geolocation::geometry) as longitude, ST_Y(l.geolocation::geometry) as latitude,
                       c.created_at, c.updated_at
                FROM clubs c
                LEFT JOIN locations l ON c.location_id = l.id
                WHERE c.is_soaring = true
                AND c.home_base_airport_id IS NULL
                AND l.geolocation IS NOT NULL
                ORDER BY c.name
            "#;

            let clubs: Vec<ClubWithLocation> = diesel::sql_query(sql)
                .load::<ClubWithLocation>(&mut conn)?;

            Ok::<Vec<ClubWithLocation>, anyhow::Error>(clubs)
        }).await??;

        Ok(result.into_iter().map(|cwl| cwl.into()).collect())
    }

    /// Update the home base airport ID for a club
    pub async fn update_home_base_airport(&self, club_id: Uuid, airport_id: i32) -> Result<bool> {
        use crate::schema::clubs::dsl::*;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let updated_count = diesel::update(clubs.filter(id.eq(club_id)))
                .set((
                    home_base_airport_id.eq(Some(airport_id)),
                    updated_at.eq(diesel::dsl::now),
                ))
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(updated_count)
        })
        .await??;

        Ok(result > 0)
    }

    /// Insert a new club
    pub async fn insert(&self, club: &Club) -> Result<()> {
        use crate::schema::clubs::dsl::*;

        // First create location if we have address data
        let final_location_id = if let Some(existing_location_id) = club.location_id {
            existing_location_id
        } else {
            // TODO: Re-enable location creation when LocationsRepository is migrated to Diesel
            // For now, clubs without location_id will not be inserted
            return Err(anyhow::anyhow!(
                "Club must have location_id (locations repository not yet migrated to Diesel)"
            ));
        };

        let new_club = NewClubModel {
            id: club.id,
            name: club.name.clone(),
            is_soaring: club.is_soaring,
            home_base_airport_id: club.home_base_airport_id,
            location_id: Some(final_location_id),
        };

        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            diesel::insert_into(clubs)
                .values(&new_club)
                .execute(&mut conn)?;

            Ok::<(), anyhow::Error>(())
        })
        .await??;

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
            city: Some("Milton".to_string()),
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
        assert_eq!(club.city, Some("Milton".to_string()));
        assert_eq!(club.state, Some("NY".to_string()));
    }
}
