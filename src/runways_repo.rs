use anyhow::Result;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::upsert::excluded;
use num_traits::{FromPrimitive, ToPrimitive};
use tracing::info;

use crate::runways::Runway;
use crate::schema::runways;

pub type DieselPgPool = Pool<ConnectionManager<PgConnection>>;
pub type DieselPgPooledConnection = PooledConnection<ConnectionManager<PgConnection>>;

/// Helper function to convert Option<f64> to Option<BigDecimal>
fn f64_to_bigdecimal(value: Option<f64>) -> Option<BigDecimal> {
    value.and_then(BigDecimal::from_f64)
}

/// Helper function to convert Option<BigDecimal> to Option<f64>
fn bigdecimal_to_f64(value: Option<BigDecimal>) -> Option<f64> {
    value.and_then(|v| v.to_f64())
}

/// Diesel model for the runways table - used for database operations
#[derive(Debug, Clone, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = runways)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RunwayModel {
    pub id: i32,
    pub airport_ref: i32,
    pub airport_ident: String,
    pub length_ft: Option<i32>,
    pub width_ft: Option<i32>,
    pub surface: Option<String>,
    pub lighted: bool,
    pub closed: bool,
    pub le_ident: Option<String>,
    pub le_latitude_deg: Option<BigDecimal>,
    pub le_longitude_deg: Option<BigDecimal>,
    pub le_elevation_ft: Option<i32>,
    pub le_heading_degt: Option<BigDecimal>,
    pub le_displaced_threshold_ft: Option<i32>,
    pub he_ident: Option<String>,
    pub he_latitude_deg: Option<BigDecimal>,
    pub he_longitude_deg: Option<BigDecimal>,
    pub he_elevation_ft: Option<i32>,
    pub he_heading_degt: Option<BigDecimal>,
    pub he_displaced_threshold_ft: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insert model for new runways
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = runways)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewRunwayModel {
    pub id: i32,
    pub airport_ref: i32,
    pub airport_ident: String,
    pub length_ft: Option<i32>,
    pub width_ft: Option<i32>,
    pub surface: Option<String>,
    pub lighted: bool,
    pub closed: bool,
    pub le_ident: Option<String>,
    pub le_latitude_deg: Option<BigDecimal>,
    pub le_longitude_deg: Option<BigDecimal>,
    pub le_elevation_ft: Option<i32>,
    pub le_heading_degt: Option<BigDecimal>,
    pub le_displaced_threshold_ft: Option<i32>,
    pub he_ident: Option<String>,
    pub he_latitude_deg: Option<BigDecimal>,
    pub he_longitude_deg: Option<BigDecimal>,
    pub he_elevation_ft: Option<i32>,
    pub he_heading_degt: Option<BigDecimal>,
    pub he_displaced_threshold_ft: Option<i32>,
}

/// Conversion from Runway (API model) to RunwayModel (database model)
impl From<Runway> for RunwayModel {
    fn from(runway: Runway) -> Self {
        Self {
            id: runway.id,
            airport_ref: runway.airport_ref,
            airport_ident: runway.airport_ident,
            length_ft: runway.length_ft,
            width_ft: runway.width_ft,
            surface: runway.surface,
            lighted: runway.lighted,
            closed: runway.closed,
            le_ident: runway.le_ident,
            le_latitude_deg: f64_to_bigdecimal(runway.le_latitude_deg),
            le_longitude_deg: f64_to_bigdecimal(runway.le_longitude_deg),
            le_elevation_ft: runway.le_elevation_ft,
            le_heading_degt: f64_to_bigdecimal(runway.le_heading_degt),
            le_displaced_threshold_ft: runway.le_displaced_threshold_ft,
            he_ident: runway.he_ident,
            he_latitude_deg: f64_to_bigdecimal(runway.he_latitude_deg),
            he_longitude_deg: f64_to_bigdecimal(runway.he_longitude_deg),
            he_elevation_ft: runway.he_elevation_ft,
            he_heading_degt: f64_to_bigdecimal(runway.he_heading_degt),
            he_displaced_threshold_ft: runway.he_displaced_threshold_ft,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// Conversion from Runway (API model) to NewRunwayModel (insert model)
impl From<Runway> for NewRunwayModel {
    fn from(runway: Runway) -> Self {
        Self {
            id: runway.id,
            airport_ref: runway.airport_ref,
            airport_ident: runway.airport_ident,
            length_ft: runway.length_ft,
            width_ft: runway.width_ft,
            surface: runway.surface,
            lighted: runway.lighted,
            closed: runway.closed,
            le_ident: runway.le_ident,
            le_latitude_deg: f64_to_bigdecimal(runway.le_latitude_deg),
            le_longitude_deg: f64_to_bigdecimal(runway.le_longitude_deg),
            le_elevation_ft: runway.le_elevation_ft,
            le_heading_degt: f64_to_bigdecimal(runway.le_heading_degt),
            le_displaced_threshold_ft: runway.le_displaced_threshold_ft,
            he_ident: runway.he_ident,
            he_latitude_deg: f64_to_bigdecimal(runway.he_latitude_deg),
            he_longitude_deg: f64_to_bigdecimal(runway.he_longitude_deg),
            he_elevation_ft: runway.he_elevation_ft,
            he_heading_degt: f64_to_bigdecimal(runway.he_heading_degt),
            he_displaced_threshold_ft: runway.he_displaced_threshold_ft,
        }
    }
}

/// Conversion from RunwayModel (database model) to Runway (API model)
impl From<RunwayModel> for Runway {
    fn from(model: RunwayModel) -> Self {
        Self {
            id: model.id,
            airport_ref: model.airport_ref,
            airport_ident: model.airport_ident,
            length_ft: model.length_ft,
            width_ft: model.width_ft,
            surface: model.surface,
            lighted: model.lighted,
            closed: model.closed,
            le_ident: model.le_ident,
            le_latitude_deg: bigdecimal_to_f64(model.le_latitude_deg),
            le_longitude_deg: bigdecimal_to_f64(model.le_longitude_deg),
            le_elevation_ft: model.le_elevation_ft,
            le_heading_degt: bigdecimal_to_f64(model.le_heading_degt),
            le_displaced_threshold_ft: model.le_displaced_threshold_ft,
            he_ident: model.he_ident,
            he_latitude_deg: bigdecimal_to_f64(model.he_latitude_deg),
            he_longitude_deg: bigdecimal_to_f64(model.he_longitude_deg),
            he_elevation_ft: model.he_elevation_ft,
            he_heading_degt: bigdecimal_to_f64(model.he_heading_degt),
            he_displaced_threshold_ft: model.he_displaced_threshold_ft,
        }
    }
}

#[derive(Clone)]
pub struct RunwaysRepository {
    pool: DieselPgPool,
}

impl RunwaysRepository {
    pub fn new(pool: DieselPgPool) -> Self {
        Self { pool }
    }

    /// Upsert runways into the database
    /// This will insert new runways or update existing ones based on the primary key (id)
    /// Processes runways in batches for better performance
    pub async fn upsert_runways<I>(&self, runways: I) -> Result<usize>
    where
        I: IntoIterator<Item = Runway>,
    {
        let runways_vec: Vec<Runway> = runways.into_iter().collect();
        let runway_models: Vec<NewRunwayModel> =
            runways_vec.into_iter().map(NewRunwayModel::from).collect();

        // Process in batches of 1000 to balance performance and parameter limits
        const BATCH_SIZE: usize = 1000;
        let total_runways = runway_models.len();
        let mut total_upserted = 0;

        for (batch_num, batch) in runway_models.chunks(BATCH_SIZE).enumerate() {
            let pool = self.pool.clone();
            let batch_vec = batch.to_vec();

            let batch_result = tokio::task::spawn_blocking(move || {
                let mut conn = pool.get()?;

                let upserted_count = diesel::insert_into(runways::table)
                    .values(&batch_vec)
                    .on_conflict(runways::id)
                    .do_update()
                    .set((
                        runways::airport_ref.eq(excluded(runways::airport_ref)),
                        runways::airport_ident.eq(excluded(runways::airport_ident)),
                        runways::length_ft.eq(excluded(runways::length_ft)),
                        runways::width_ft.eq(excluded(runways::width_ft)),
                        runways::surface.eq(excluded(runways::surface)),
                        runways::lighted.eq(excluded(runways::lighted)),
                        runways::closed.eq(excluded(runways::closed)),
                        runways::le_ident.eq(excluded(runways::le_ident)),
                        runways::le_latitude_deg.eq(excluded(runways::le_latitude_deg)),
                        runways::le_longitude_deg.eq(excluded(runways::le_longitude_deg)),
                        runways::le_elevation_ft.eq(excluded(runways::le_elevation_ft)),
                        runways::le_heading_degt.eq(excluded(runways::le_heading_degt)),
                        runways::le_displaced_threshold_ft
                            .eq(excluded(runways::le_displaced_threshold_ft)),
                        runways::he_ident.eq(excluded(runways::he_ident)),
                        runways::he_latitude_deg.eq(excluded(runways::he_latitude_deg)),
                        runways::he_longitude_deg.eq(excluded(runways::he_longitude_deg)),
                        runways::he_elevation_ft.eq(excluded(runways::he_elevation_ft)),
                        runways::he_heading_degt.eq(excluded(runways::he_heading_degt)),
                        runways::he_displaced_threshold_ft
                            .eq(excluded(runways::he_displaced_threshold_ft)),
                        runways::updated_at.eq(diesel::dsl::now),
                    ))
                    .execute(&mut conn)?;

                Ok::<usize, anyhow::Error>(upserted_count)
            })
            .await??;

            total_upserted += batch_result;

            // Log progress for large batches
            if total_runways > BATCH_SIZE {
                info!(
                    "Processed batch {} of {}: {} runways ({}/{} total)",
                    batch_num + 1,
                    total_runways.div_ceil(BATCH_SIZE),
                    batch_result,
                    total_upserted,
                    total_runways
                );
            }
        }

        info!("Successfully upserted {} runways in total", total_upserted);
        Ok(total_upserted)
    }

    /// Get the total count of runways in the database
    pub async fn get_runway_count(&self) -> Result<i64> {
        use crate::schema::runways::dsl::*;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let count: i64 = runways.count().get_result(&mut conn)?;
            Ok::<i64, anyhow::Error>(count)
        })
        .await??;

        Ok(result)
    }

    /// Get a runway by its ID
    pub async fn get_runway_by_id(&self, runway_id: i32) -> Result<Option<Runway>> {
        use crate::schema::runways::dsl::*;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let runway_model: Option<RunwayModel> = runways
                .filter(id.eq(runway_id))
                .select(RunwayModel::as_select())
                .first::<RunwayModel>(&mut conn)
                .optional()?;

            Ok::<Option<RunwayModel>, anyhow::Error>(runway_model)
        })
        .await??;

        Ok(result.map(Runway::from))
    }

    /// Get all runways for a specific airport by airport ID
    pub async fn get_runways_by_airport_id(&self, airport_id: i32) -> Result<Vec<Runway>> {
        use crate::schema::runways::dsl::*;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let runway_models: Vec<RunwayModel> = runways
                .filter(airport_ref.eq(airport_id))
                .order(id.asc())
                .select(RunwayModel::as_select())
                .load::<RunwayModel>(&mut conn)?;

            Ok::<Vec<RunwayModel>, anyhow::Error>(runway_models)
        })
        .await??;

        Ok(result.into_iter().map(Runway::from).collect())
    }

    /// Get all runways for a specific airport by airport identifier
    pub async fn get_runways_by_airport_ident(
        &self,
        airport_ident_param: &str,
    ) -> Result<Vec<Runway>> {
        use crate::schema::runways::dsl::*;

        let pool = self.pool.clone();
        let airport_ident_param = airport_ident_param.to_string();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let runway_models: Vec<RunwayModel> = runways
                .filter(airport_ident.eq(airport_ident_param))
                .order(id.asc())
                .select(RunwayModel::as_select())
                .load::<RunwayModel>(&mut conn)?;

            Ok::<Vec<RunwayModel>, anyhow::Error>(runway_models)
        })
        .await??;

        Ok(result.into_iter().map(Runway::from).collect())
    }

    /// Get runways within a bounding box using PostGIS spatial functions
    /// Returns runways where either endpoint is within the bounding box
    pub async fn get_runways_in_bbox(
        &self,
        west: f64,
        south: f64,
        east: f64,
        north: f64,
        limit: Option<i64>,
    ) -> Result<Vec<Runway>> {
        let pool = self.pool.clone();
        let limit = limit.unwrap_or(500);

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Use raw SQL with PostGIS to find runways in bounding box
            // A runway is included if either endpoint is within the box
            let sql = r#"
                SELECT DISTINCT ON (id)
                    id, airport_ref, airport_ident, length_ft, width_ft, surface,
                    lighted, closed, le_ident, le_latitude_deg, le_longitude_deg,
                    le_elevation_ft, le_heading_degt, le_displaced_threshold_ft,
                    he_ident, he_latitude_deg, he_longitude_deg, he_elevation_ft,
                    he_heading_degt, he_displaced_threshold_ft
                FROM runways
                WHERE (
                    le_location IS NOT NULL AND ST_Intersects(
                        le_location,
                        ST_MakeEnvelope($1, $2, $3, $4, 4326)::geography
                    )
                ) OR (
                    he_location IS NOT NULL AND ST_Intersects(
                        he_location,
                        ST_MakeEnvelope($1, $2, $3, $4, 4326)::geography
                    )
                )
                ORDER BY id
                LIMIT $5
            "#;

            #[derive(Debug, QueryableByName)]
            struct RunwayBboxResult {
                #[diesel(sql_type = diesel::sql_types::Integer)]
                id: i32,
                #[diesel(sql_type = diesel::sql_types::Integer)]
                airport_ref: i32,
                #[diesel(sql_type = diesel::sql_types::Text)]
                airport_ident: String,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
                length_ft: Option<i32>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
                width_ft: Option<i32>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                surface: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Bool)]
                lighted: bool,
                #[diesel(sql_type = diesel::sql_types::Bool)]
                closed: bool,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                le_ident: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
                le_latitude_deg: Option<BigDecimal>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
                le_longitude_deg: Option<BigDecimal>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
                le_elevation_ft: Option<i32>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
                le_heading_degt: Option<BigDecimal>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
                le_displaced_threshold_ft: Option<i32>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                he_ident: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
                he_latitude_deg: Option<BigDecimal>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
                he_longitude_deg: Option<BigDecimal>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
                he_elevation_ft: Option<i32>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
                he_heading_degt: Option<BigDecimal>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
                he_displaced_threshold_ft: Option<i32>,
            }

            let results: Vec<RunwayBboxResult> = diesel::sql_query(sql)
                .bind::<diesel::sql_types::Double, _>(west)
                .bind::<diesel::sql_types::Double, _>(south)
                .bind::<diesel::sql_types::Double, _>(east)
                .bind::<diesel::sql_types::Double, _>(north)
                .bind::<diesel::sql_types::BigInt, _>(limit)
                .load(&mut conn)?;

            Ok::<Vec<RunwayBboxResult>, anyhow::Error>(results)
        })
        .await??;

        // Convert to Runway structs
        let runways = result
            .into_iter()
            .map(|r| Runway {
                id: r.id,
                airport_ref: r.airport_ref,
                airport_ident: r.airport_ident,
                length_ft: r.length_ft,
                width_ft: r.width_ft,
                surface: r.surface,
                lighted: r.lighted,
                closed: r.closed,
                le_ident: r.le_ident,
                le_latitude_deg: bigdecimal_to_f64(r.le_latitude_deg),
                le_longitude_deg: bigdecimal_to_f64(r.le_longitude_deg),
                le_elevation_ft: r.le_elevation_ft,
                le_heading_degt: bigdecimal_to_f64(r.le_heading_degt),
                le_displaced_threshold_ft: r.le_displaced_threshold_ft,
                he_ident: r.he_ident,
                he_latitude_deg: bigdecimal_to_f64(r.he_latitude_deg),
                he_longitude_deg: bigdecimal_to_f64(r.he_longitude_deg),
                he_elevation_ft: r.he_elevation_ft,
                he_heading_degt: bigdecimal_to_f64(r.he_heading_degt),
                he_displaced_threshold_ft: r.he_displaced_threshold_ft,
            })
            .collect();

        Ok(runways)
    }

    /// Find nearest runway endpoints to a given point using PostGIS spatial functions
    /// Returns runway endpoints within the specified distance (in meters) ordered by distance
    /// If airport_ref is provided, only returns runways at that airport
    pub async fn find_nearest_runway_endpoints(
        &self,
        latitude: f64,
        longitude: f64,
        max_distance_meters: f64,
        limit: i64,
        airport_ref: Option<i32>,
    ) -> Result<Vec<(Runway, f64, String)>> {
        let pool = self.pool.clone();

        #[derive(Debug, QueryableByName)]
        struct RunwayEndpointResult {
            #[diesel(sql_type = diesel::sql_types::Integer)]
            id: i32,
            #[diesel(sql_type = diesel::sql_types::Integer)]
            airport_ref: i32,
            #[diesel(sql_type = diesel::sql_types::Text)]
            airport_ident: String,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
            length_ft: Option<i32>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
            width_ft: Option<i32>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            surface: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Bool)]
            lighted: bool,
            #[diesel(sql_type = diesel::sql_types::Bool)]
            closed: bool,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            le_ident: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
            le_latitude_deg: Option<BigDecimal>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
            le_longitude_deg: Option<BigDecimal>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
            le_elevation_ft: Option<i32>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
            le_heading_degt: Option<BigDecimal>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
            le_displaced_threshold_ft: Option<i32>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            he_ident: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
            he_latitude_deg: Option<BigDecimal>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
            he_longitude_deg: Option<BigDecimal>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
            he_elevation_ft: Option<i32>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
            he_heading_degt: Option<BigDecimal>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
            he_displaced_threshold_ft: Option<i32>,
            #[diesel(sql_type = diesel::sql_types::Double)]
            distance_meters: f64,
            #[diesel(sql_type = diesel::sql_types::Text)]
            endpoint_type: String,
        }

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Use raw SQL with PostGIS to find nearest runway endpoints
            // UNION two queries: one for low end, one for high end
            // Build SQL with optional airport filter
            let (sql, airport_filter) = if airport_ref.is_some() {
                (
                    r#"
                (
                    SELECT id, airport_ref, airport_ident, length_ft, width_ft, surface,
                           lighted, closed, le_ident, le_latitude_deg, le_longitude_deg,
                           le_elevation_ft, le_heading_degt, le_displaced_threshold_ft,
                           he_ident, he_latitude_deg, he_longitude_deg, he_elevation_ft,
                           he_heading_degt, he_displaced_threshold_ft,
                           ST_Distance(
                               le_location,
                               ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography
                           ) as distance_meters,
                           'low_end' as endpoint_type
                    FROM runways
                    WHERE le_location IS NOT NULL
                      AND airport_ref = $5
                      AND ST_DWithin(
                          le_location,
                          ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography,
                          $3
                      )
                )
                UNION ALL
                (
                    SELECT id, airport_ref, airport_ident, length_ft, width_ft, surface,
                           lighted, closed, le_ident, le_latitude_deg, le_longitude_deg,
                           le_elevation_ft, le_heading_degt, le_displaced_threshold_ft,
                           he_ident, he_latitude_deg, he_longitude_deg, he_elevation_ft,
                           he_heading_degt, he_displaced_threshold_ft,
                           ST_Distance(
                               he_location,
                               ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography
                           ) as distance_meters,
                           'high_end' as endpoint_type
                    FROM runways
                    WHERE he_location IS NOT NULL
                      AND airport_ref = $5
                      AND ST_DWithin(
                          he_location,
                          ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography,
                          $3
                      )
                )
                ORDER BY distance_meters
                LIMIT $4
            "#,
                    true,
                )
            } else {
                (
                    r#"
                (
                    SELECT id, airport_ref, airport_ident, length_ft, width_ft, surface,
                           lighted, closed, le_ident, le_latitude_deg, le_longitude_deg,
                           le_elevation_ft, le_heading_degt, le_displaced_threshold_ft,
                           he_ident, he_latitude_deg, he_longitude_deg, he_elevation_ft,
                           he_heading_degt, he_displaced_threshold_ft,
                           ST_Distance(
                               le_location,
                               ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography
                           ) as distance_meters,
                           'low_end' as endpoint_type
                    FROM runways
                    WHERE le_location IS NOT NULL
                      AND ST_DWithin(
                          le_location,
                          ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography,
                          $3
                      )
                )
                UNION ALL
                (
                    SELECT id, airport_ref, airport_ident, length_ft, width_ft, surface,
                           lighted, closed, le_ident, le_latitude_deg, le_longitude_deg,
                           le_elevation_ft, le_heading_degt, le_displaced_threshold_ft,
                           he_ident, he_latitude_deg, he_longitude_deg, he_elevation_ft,
                           he_heading_degt, he_displaced_threshold_ft,
                           ST_Distance(
                               he_location,
                               ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography
                           ) as distance_meters,
                           'high_end' as endpoint_type
                    FROM runways
                    WHERE he_location IS NOT NULL
                      AND ST_DWithin(
                          he_location,
                          ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography,
                          $3
                      )
                )
                ORDER BY distance_meters
                LIMIT $4
            "#,
                    false,
                )
            };

            let query = diesel::sql_query(sql)
                .bind::<diesel::sql_types::Double, _>(latitude)
                .bind::<diesel::sql_types::Double, _>(longitude)
                .bind::<diesel::sql_types::Double, _>(max_distance_meters)
                .bind::<diesel::sql_types::BigInt, _>(limit);

            let results: Vec<RunwayEndpointResult> = if airport_filter {
                query
                    .bind::<diesel::sql_types::Integer, _>(airport_ref.unwrap())
                    .load(&mut conn)?
            } else {
                query.load(&mut conn)?
            };

            Ok::<Vec<RunwayEndpointResult>, anyhow::Error>(results)
        })
        .await??;

        // Convert results to (Runway, distance, endpoint_type) tuples
        let runways_with_distance = result
            .into_iter()
            .map(|r| {
                let runway = Runway {
                    id: r.id,
                    airport_ref: r.airport_ref,
                    airport_ident: r.airport_ident,
                    length_ft: r.length_ft,
                    width_ft: r.width_ft,
                    surface: r.surface,
                    lighted: r.lighted,
                    closed: r.closed,
                    le_ident: r.le_ident,
                    le_latitude_deg: bigdecimal_to_f64(r.le_latitude_deg),
                    le_longitude_deg: bigdecimal_to_f64(r.le_longitude_deg),
                    le_elevation_ft: r.le_elevation_ft,
                    le_heading_degt: bigdecimal_to_f64(r.le_heading_degt),
                    le_displaced_threshold_ft: r.le_displaced_threshold_ft,
                    he_ident: r.he_ident,
                    he_latitude_deg: bigdecimal_to_f64(r.he_latitude_deg),
                    he_longitude_deg: bigdecimal_to_f64(r.he_longitude_deg),
                    he_elevation_ft: r.he_elevation_ft,
                    he_heading_degt: bigdecimal_to_f64(r.he_heading_degt),
                    he_displaced_threshold_ft: r.he_displaced_threshold_ft,
                };
                (runway, r.distance_meters, r.endpoint_type)
            })
            .collect();

        Ok(runways_with_distance)
    }
}

#[cfg(test)]
mod tests {
    use crate::runways::Runway;

    // Note: These tests would require a test database setup
    // For now, they're just structural examples

    fn create_test_runway() -> Runway {
        Runway {
            id: 269408,
            airport_ref: 6523,
            airport_ident: "00A".to_string(),
            length_ft: Some(80),
            width_ft: Some(80),
            surface: Some("ASPH-G".to_string()),
            lighted: true,
            closed: false,
            le_ident: Some("H1".to_string()),
            le_latitude_deg: None,
            le_longitude_deg: None,
            le_elevation_ft: None,
            le_heading_degt: None,
            le_displaced_threshold_ft: None,
            he_ident: None,
            he_latitude_deg: None,
            he_longitude_deg: None,
            he_elevation_ft: None,
            he_heading_degt: None,
            he_displaced_threshold_ft: None,
        }
    }

    #[test]
    fn test_runway_creation() {
        let runway = create_test_runway();
        assert_eq!(runway.id, 269408);
        assert_eq!(runway.airport_ref, 6523);
        assert_eq!(runway.airport_ident, "00A");
        assert_eq!(runway.length_ft, Some(80));
        assert_eq!(runway.width_ft, Some(80));
        assert_eq!(runway.surface, Some("ASPH-G".to_string()));
        assert!(runway.lighted);
        assert!(!runway.closed);
        assert_eq!(runway.le_ident, Some("H1".to_string()));
    }
}
