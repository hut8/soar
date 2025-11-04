use anyhow::Result;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use num_traits::ToPrimitive;
use uuid::Uuid;

use crate::clubs::{Club, ClubModel, NewClubModel};
use crate::locations::Point;
use crate::locations_repo::LocationsRepository;
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
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    home_base_airport_ident: Option<String>,
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
    country_mail_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
    longitude: Option<BigDecimal>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
    latitude: Option<BigDecimal>,
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
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub home_base_airport_ident: Option<String>,
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
    pub country_mail_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
    pub longitude: Option<BigDecimal>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
    pub latitude: Option<BigDecimal>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub created_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub updated_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float4>)]
    pub similarity_score: Option<f32>,
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
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub home_base_airport_ident: Option<String>,
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
    pub country_mail_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
    pub longitude: Option<BigDecimal>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
    pub latitude: Option<BigDecimal>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub created_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub updated_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float8>)]
    pub distance_meters: Option<f64>,
}

/// Helper function to convert Option<BigDecimal> to Option<f64>
fn bigdecimal_to_f64(value: Option<BigDecimal>) -> Option<f64> {
    value.and_then(|v| v.to_f64())
}

/// Helper function to determine if a club name indicates it's a soaring club
fn is_soaring_club(club_name: &str) -> bool {
    let name_lower = club_name.to_lowercase();
    name_lower.contains("soar") || name_lower.contains("glider")
}

impl From<ClubWithLocationAndDistance> for Club {
    fn from(cwld: ClubWithLocationAndDistance) -> Self {
        let base_location = if cwld.longitude.is_some() && cwld.latitude.is_some() {
            let lat = bigdecimal_to_f64(cwld.latitude);
            let lng = bigdecimal_to_f64(cwld.longitude);
            if let (Some(lat), Some(lng)) = (lat, lng) {
                Some(Point::new(lat, lng))
            } else {
                None
            }
        } else {
            None
        };

        Self {
            id: cwld.id,
            name: cwld.name,
            is_soaring: cwld.is_soaring,
            home_base_airport_id: cwld.home_base_airport_id,
            home_base_airport_ident: cwld.home_base_airport_ident,
            location_id: cwld.location_id,
            street1: cwld.street1,
            street2: cwld.street2,
            city: cwld.city,
            state: cwld.state,
            zip_code: cwld.zip_code,
            region_code: cwld.region_code,
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
            let lat = bigdecimal_to_f64(cwls.latitude);
            let lng = bigdecimal_to_f64(cwls.longitude);
            if let (Some(lat), Some(lng)) = (lat, lng) {
                Some(Point::new(lat, lng))
            } else {
                None
            }
        } else {
            None
        };

        Self {
            id: cwls.id,
            name: cwls.name,
            is_soaring: cwls.is_soaring,
            home_base_airport_id: cwls.home_base_airport_id,
            home_base_airport_ident: cwls.home_base_airport_ident,
            location_id: cwls.location_id,
            street1: cwls.street1,
            street2: cwls.street2,
            city: cwls.city,
            state: cwls.state,
            zip_code: cwls.zip_code,
            region_code: cwls.region_code,
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
            let lat = bigdecimal_to_f64(cwl.latitude);
            let lng = bigdecimal_to_f64(cwl.longitude);
            if let (Some(lat), Some(lng)) = (lat, lng) {
                Some(Point::new(lat, lng))
            } else {
                None
            }
        } else {
            None
        };

        Self {
            id: cwl.id,
            name: cwl.name,
            is_soaring: cwl.is_soaring,
            home_base_airport_id: cwl.home_base_airport_id,
            home_base_airport_ident: cwl.home_base_airport_ident,
            location_id: cwl.location_id,
            street1: cwl.street1,
            street2: cwl.street2,
            city: cwl.city,
            state: cwl.state,
            zip_code: cwl.zip_code,
            region_code: cwl.region_code,
            country_mail_code: cwl.country_mail_code,
            base_location,
            created_at: cwl.created_at,
            updated_at: cwl.updated_at,
        }
    }
}

/// Location parameters for club creation
#[derive(Debug, Clone)]
pub struct LocationParams {
    pub street1: Option<String>,
    pub street2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip_code: Option<String>,
    pub region_code: Option<String>,
    pub country_mail_code: Option<String>,
}

#[derive(Clone)]
pub struct ClubsRepository {
    pool: PgPool,
    locations_repo: LocationsRepository,
}

impl ClubsRepository {
    pub fn new(pool: PgPool) -> Self {
        let locations_repo = LocationsRepository::new(pool.clone());
        Self {
            pool,
            locations_repo,
        }
    }

    /// Find club by exact name match
    pub async fn find_by_name(&self, name: &str) -> Result<Option<Club>> {
        let pool = self.pool.clone();
        let name_upper = name.to_uppercase();

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let sql = r#"
                SELECT c.id, c.name, c.is_soaring, c.home_base_airport_id, a.ident as home_base_airport_ident, c.location_id,
                       l.street1, l.street2, l.city, l.state, l.zip_code, l.region_code,
                       l.country_mail_code,
                       CASE
                           WHEN l.geolocation IS NOT NULL THEN ST_X(l.geolocation::geometry)::NUMERIC
                           ELSE NULL::NUMERIC
                       END as longitude,
                       CASE
                           WHEN l.geolocation IS NOT NULL THEN ST_Y(l.geolocation::geometry)::NUMERIC
                           ELSE NULL::NUMERIC
                       END as latitude,
                       c.created_at, c.updated_at
                FROM clubs c
                LEFT JOIN locations l ON c.location_id = l.id
                LEFT JOIN airports a ON c.home_base_airport_id = a.id
                WHERE UPPER(c.name) = $1
                LIMIT 1
            "#;

            let club_opt: Option<ClubWithLocation> = diesel::sql_query(sql)
                .bind::<diesel::sql_types::Varchar, _>(&name_upper)
                .get_result::<ClubWithLocation>(&mut conn)
                .optional()?;

            Ok::<Option<ClubWithLocation>, anyhow::Error>(club_opt)
        })
        .await??;

        Ok(result.map(|cwl| cwl.into()))
    }

    /// Get club by ID
    pub async fn get_by_id(&self, club_id: Uuid) -> Result<Option<Club>> {
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Use raw SQL for complex join with locations table
            let sql = r#"
                SELECT c.id, c.name, c.is_soaring, c.home_base_airport_id, a.ident as home_base_airport_ident, c.location_id,
                       l.street1, l.street2, l.city, l.state, l.zip_code, l.region_code,
                       l.country_mail_code,
                       CASE
                           WHEN l.geolocation IS NOT NULL THEN ST_X(l.geolocation::geometry)::NUMERIC
                           ELSE NULL::NUMERIC
                       END as longitude,
                       CASE
                           WHEN l.geolocation IS NOT NULL THEN ST_Y(l.geolocation::geometry)::NUMERIC
                           ELSE NULL::NUMERIC
                       END as latitude,
                       c.created_at, c.updated_at
                FROM clubs c
                LEFT JOIN locations l ON c.location_id = l.id
                LEFT JOIN airports a ON c.home_base_airport_id = a.id
                WHERE c.id = $1
            "#;

            let club_opt: Option<ClubWithLocation> = diesel::sql_query(sql)
                .bind::<diesel::sql_types::Uuid, _>(club_id)
                .get_result::<ClubWithLocation>(&mut conn)
                .optional()?;

            Ok::<Option<ClubWithLocation>, anyhow::Error>(club_opt)
        })
        .await??;

        Ok(result.map(|cwl| cwl.into()))
    }

    /// Get all clubs
    pub async fn get_all(&self) -> Result<Vec<Club>> {
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Use raw SQL for complex join with locations table
            let sql = r#"
                SELECT c.id, c.name, c.is_soaring, c.home_base_airport_id, a.ident as home_base_airport_ident, c.location_id,
                       l.street1, l.street2, l.city, l.state, l.zip_code, l.region_code,
                       l.country_mail_code,
                       CASE
                           WHEN l.geolocation IS NOT NULL THEN ST_X(l.geolocation::geometry)::NUMERIC
                           ELSE NULL::NUMERIC
                       END as longitude,
                       CASE
                           WHEN l.geolocation IS NOT NULL THEN ST_Y(l.geolocation::geometry)::NUMERIC
                           ELSE NULL::NUMERIC
                       END as latitude,
                       c.created_at, c.updated_at
                FROM clubs c
                LEFT JOIN locations l ON c.location_id = l.id
                LEFT JOIN airports a ON c.home_base_airport_id = a.id
                WHERE c.is_soaring = true
                ORDER BY c.name
            "#;

            let clubs: Vec<ClubWithLocation> =
                diesel::sql_query(sql).load::<ClubWithLocation>(&mut conn)?;

            Ok::<Vec<ClubWithLocation>, anyhow::Error>(clubs)
        })
        .await??;

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
                SELECT c.id, c.name, c.is_soaring, c.home_base_airport_id, a.ident as home_base_airport_ident, c.location_id,
                       l.street1, l.street2, l.city, l.state, l.zip_code, l.region_code,
                       l.country_mail_code,
                       CASE
                           WHEN l.geolocation IS NOT NULL THEN ST_X(l.geolocation::geometry)::NUMERIC
                           ELSE NULL::NUMERIC
                       END as longitude,
                       CASE
                           WHEN l.geolocation IS NOT NULL THEN ST_Y(l.geolocation::geometry)::NUMERIC
                           ELSE NULL::NUMERIC
                       END as latitude,
                       c.created_at, c.updated_at,
                       SIMILARITY(UPPER(c.name), $1) as similarity_score
                FROM clubs c
                LEFT JOIN locations l ON c.location_id = l.id
                LEFT JOIN airports a ON c.home_base_airport_id = a.id
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
        })
        .await??;

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
                SELECT c.id, c.name, c.is_soaring, c.home_base_airport_id, a.ident as home_base_airport_ident, c.location_id,
                       l.street1, l.street2, l.city, l.state, l.zip_code, l.region_code,
                       l.country_mail_code,
                       NULL::float8 as longitude, NULL::float8 as latitude,
                       c.created_at, c.updated_at,
                       ST_Distance(ST_SetSRID(l.geolocation::geometry, 4326)::geography, ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography) as distance_meters
                FROM clubs c
                INNER JOIN locations l ON c.location_id = l.id
                LEFT JOIN airports a ON c.home_base_airport_id = a.id
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
                       l.country_mail_code,
                       (l.geolocation)[0]::numeric as longitude,
                       (l.geolocation)[1]::numeric as latitude,
                       c.created_at, c.updated_at
                FROM clubs c
                LEFT JOIN locations l ON c.location_id = l.id
                WHERE c.is_soaring = true
                AND c.home_base_airport_id IS NULL
                AND l.geolocation IS NOT NULL
                ORDER BY c.name
            "#;

            let clubs: Vec<ClubWithLocation> =
                diesel::sql_query(sql).load::<ClubWithLocation>(&mut conn)?;

            Ok::<Vec<ClubWithLocation>, anyhow::Error>(clubs)
        })
        .await??;

        Ok(result.into_iter().map(|cwl| cwl.into()).collect())
    }

    /// Get clubs based at a specific airport
    pub async fn get_clubs_by_airport(&self, airport_id: i32) -> Result<Vec<Club>> {
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let sql = r#"
                SELECT c.id, c.name, c.is_soaring, c.home_base_airport_id, c.location_id,
                       l.street1, l.street2, l.city, l.state, l.zip_code, l.region_code,
                       l.country_mail_code,
                       (l.geolocation)[0]::numeric as longitude,
                       (l.geolocation)[1]::numeric as latitude,
                       c.created_at, c.updated_at
                FROM clubs c
                LEFT JOIN locations l ON c.location_id = l.id
                WHERE c.home_base_airport_id = $1
                ORDER BY c.name
            "#;

            let clubs: Vec<ClubWithLocation> = diesel::sql_query(sql)
                .bind::<diesel::sql_types::Integer, _>(airport_id)
                .load::<ClubWithLocation>(&mut conn)?;

            Ok::<Vec<ClubWithLocation>, anyhow::Error>(clubs)
        })
        .await??;

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

    /// Insert a new club with minimal data (for aircraft registration club creation)
    pub async fn create_simple_club(&self, club_name: &str) -> Result<Club> {
        use crate::schema::clubs::dsl::*;

        let club_id = Uuid::now_v7();
        let new_club = NewClubModel {
            id: club_id,
            name: club_name.to_string(),
            is_soaring: Some(is_soaring_club(club_name)),
            home_base_airport_id: None,
            location_id: None, // Will be populated later if needed
        };

        let pool = self.pool.clone();
        let created_club = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Use RETURNING to get the inserted record directly
            let inserted_club: ClubModel = diesel::insert_into(clubs)
                .values(&new_club)
                .get_result(&mut conn)?;

            Ok::<Club, anyhow::Error>(inserted_club.into())
        })
        .await??;

        Ok(created_club)
    }

    /// Create a new club with location data
    pub async fn create_club_with_location(
        &self,
        club_name: &str,
        location_params: LocationParams,
    ) -> Result<Club> {
        use crate::schema::clubs::dsl::*;

        // Use LocationsRepository to find or create the location
        let location = self
            .locations_repo
            .find_or_create(
                location_params.street1,
                location_params.street2,
                location_params.city,
                location_params.state,
                location_params.zip_code,
                location_params.region_code,
                location_params.country_mail_code,
                None, // geolocation will be set by triggers if coordinates are available
            )
            .await?;

        // Create the club with the location_id
        let club_id = Uuid::now_v7();
        let new_club = NewClubModel {
            id: club_id,
            name: club_name.to_string(),
            is_soaring: Some(is_soaring_club(club_name)),
            home_base_airport_id: None,
            location_id: Some(location.id),
        };

        let pool = self.pool.clone();
        let created_club = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Insert the club
            let inserted_club: ClubModel = diesel::insert_into(clubs)
                .values(&new_club)
                .get_result(&mut conn)?;

            Ok::<Club, anyhow::Error>(inserted_club.into())
        })
        .await??;

        Ok(created_club)
    }

    /// Find a club by name, or create it if it doesn't exist
    /// Takes location data to create a proper location for new clubs
    pub async fn find_or_create_club(
        &self,
        club_name: &str,
        location_params: LocationParams,
    ) -> Result<Club> {
        // First try to find existing club
        if let Some(existing_club) = self.find_by_name(club_name).await? {
            Ok(existing_club)
        } else {
            // Create new club with location data
            self.create_club_with_location(club_name, location_params)
                .await
        }
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
    use diesel::r2d2::{ConnectionManager, Pool};

    // Note: These tests would require a test database setup
    // For now, they're just structural examples

    fn create_test_club() -> Club {
        Club {
            id: Uuid::now_v7(),
            name: "Adirondack Soaring Club".to_string(),
            is_soaring: Some(true),
            home_base_airport_id: None,
            home_base_airport_ident: None,
            location_id: None,
            street1: Some("123 Mountain Rd".to_string()),
            street2: None,
            city: Some("Milton".to_string()),
            state: Some("NY".to_string()),
            zip_code: Some("12946".to_string()),
            region_code: None,
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

    #[test]
    fn test_is_soaring_club_detection() {
        // Test soaring-related names
        assert!(is_soaring_club("Adirondack Soaring Club"));
        assert!(is_soaring_club("Valley Soaring Association"));
        assert!(is_soaring_club("SOARING CLUB OF WESTERN PA"));
        assert!(is_soaring_club("Central Valley Glider Club"));
        assert!(is_soaring_club("GLIDER CLUB"));
        assert!(is_soaring_club("Mountain Gliders"));

        // Test non-soaring names
        assert!(!is_soaring_club("Flying Club"));
        assert!(!is_soaring_club("Cessna Pilots Association"));
        assert!(!is_soaring_club("EAA Chapter 123"));
        assert!(!is_soaring_club("Private Aircraft Owners"));
        assert!(!is_soaring_club("Aviation Club"));
    }

    // Helper function to create a test database pool (for integration tests)
    fn create_test_pool() -> Result<PgPool> {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost/soar_test".to_string());
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder().build(manager)?;
        Ok(pool)
    }

    #[tokio::test]
    async fn test_fuzzy_search_with_null_geolocation() {
        // Skip test if we can't connect to test database
        if let Ok(pool) = create_test_pool() {
            let repo = ClubsRepository::new(pool);

            // This test ensures that fuzzy search doesn't fail with f64 decoding errors
            // when some clubs have NULL geolocation data
            let result = repo.fuzzy_search("test", Some(10)).await;

            // The search should complete without errors, even if no clubs are found
            match result {
                Ok(_clubs) => {
                    // Success - no f64 decoding error occurred
                    // Test passes if we reach here without panicking
                }
                Err(e) => {
                    // If there's an error, it shouldn't be about f64 decoding
                    let error_msg = e.to_string();
                    assert!(
                        !error_msg.contains("f64") && !error_msg.contains("double"),
                        "Unexpected f64 decoding error: {}",
                        error_msg
                    );
                }
            }
        } else {
            println!("Skipping test - no test database connection");
        }
    }

    #[tokio::test]
    async fn test_get_all_with_null_geolocation() {
        // Skip test if we can't connect to test database
        if let Ok(pool) = create_test_pool() {
            let repo = ClubsRepository::new(pool);

            // This test ensures that get_all doesn't fail with f64 decoding errors
            // when some clubs have NULL geolocation data
            let result = repo.get_all().await;

            // The search should complete without errors, even if no clubs are found
            match result {
                Ok(_clubs) => {
                    // Success - no f64 decoding error occurred
                    // Test passes if we reach here without panicking
                }
                Err(e) => {
                    // If there's an error, it shouldn't be about f64 decoding
                    let error_msg = e.to_string();
                    assert!(
                        !error_msg.contains("f64") && !error_msg.contains("double"),
                        "Unexpected f64 decoding error: {}",
                        error_msg
                    );
                }
            }
        } else {
            println!("Skipping test - no test database connection");
        }
    }
}
