use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use diesel::sql_types;
use tracing::info;
use uuid::Uuid;

use crate::coverage::{CoverageHexFeature, NewReceiverCoverageH3, ReceiverCoverageH3};
use crate::web::PgPool;

/// Queryable result for raw SQL coverage queries
#[derive(QueryableByName, Debug)]
struct CoverageQueryResult {
    #[diesel(sql_type = sql_types::BigInt)]
    h3_index: i64,
    #[diesel(sql_type = sql_types::SmallInt)]
    resolution: i16,
    #[diesel(sql_type = sql_types::Uuid)]
    receiver_id: Uuid,
    #[diesel(sql_type = sql_types::Date)]
    date: NaiveDate,
    #[diesel(sql_type = sql_types::Integer)]
    fix_count: i32,
    #[diesel(sql_type = sql_types::Timestamptz)]
    first_seen_at: DateTime<Utc>,
    #[diesel(sql_type = sql_types::Timestamptz)]
    last_seen_at: DateTime<Utc>,
    #[diesel(sql_type = sql_types::Nullable<sql_types::Integer>)]
    min_altitude_msl_feet: Option<i32>,
    #[diesel(sql_type = sql_types::Nullable<sql_types::Integer>)]
    max_altitude_msl_feet: Option<i32>,
    #[diesel(sql_type = sql_types::Nullable<sql_types::Integer>)]
    avg_altitude_msl_feet: Option<i32>,
    #[diesel(sql_type = sql_types::Timestamptz)]
    updated_at: DateTime<Utc>,
}

impl From<CoverageQueryResult> for ReceiverCoverageH3 {
    fn from(result: CoverageQueryResult) -> Self {
        Self {
            h3_index: result.h3_index,
            resolution: result.resolution,
            receiver_id: result.receiver_id,
            date: result.date,
            fix_count: result.fix_count,
            first_seen_at: result.first_seen_at,
            last_seen_at: result.last_seen_at,
            min_altitude_msl_feet: result.min_altitude_msl_feet,
            max_altitude_msl_feet: result.max_altitude_msl_feet,
            avg_altitude_msl_feet: result.avg_altitude_msl_feet,
            updated_at: result.updated_at,
        }
    }
}

#[derive(Clone)]
pub struct CoverageRepository {
    pool: PgPool,
}

impl CoverageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get coverage hexes within bounding box for a given resolution and time range
    /// Filters by date range, optional receiver, and optional altitude range
    /// Uses h3_postgis extension for efficient spatial filtering
    #[allow(clippy::too_many_arguments)]
    pub async fn get_coverage_in_bbox(
        &self,
        resolution: i16,
        start_date: NaiveDate,
        end_date: NaiveDate,
        west: f64,
        south: f64,
        east: f64,
        north: f64,
        receiver_id: Option<Uuid>,
        min_altitude: Option<i32>,
        max_altitude: Option<i32>,
        limit: i64,
    ) -> Result<Vec<ReceiverCoverageH3>> {
        let pool = self.pool.clone();
        let limit = limit.min(10000); // Cap at 10k hexes

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Use h3_postgis to efficiently filter coverage data within bounding box
            // 1. Create bounding box as PostGIS geography
            // 2. Use h3_polygon_to_cells to get all H3 cells within the bbox
            // 3. Join with receiver_coverage_h3 to get coverage data for those cells

            // Build SQL - using format! is safe here as all parameters are validated API inputs
            let base_sql = format!(
                r#"
                WITH bbox AS (
                    SELECT ST_MakeEnvelope({}, {}, {}, {}, 4326)::geography AS geog
                ),
                cells AS (
                    SELECT h3_polygon_to_cells(bbox.geog, {}) AS h3_idx
                    FROM bbox
                )
                SELECT rch.h3_index, rch.resolution, rch.receiver_id, rch.date,
                       rch.fix_count, rch.first_seen_at, rch.last_seen_at,
                       rch.min_altitude_msl_feet, rch.max_altitude_msl_feet,
                       rch.avg_altitude_msl_feet, rch.updated_at
                FROM receiver_coverage_h3 rch
                INNER JOIN cells c ON rch.h3_index = c.h3_idx::bigint
                WHERE rch.resolution = {}
                  AND rch.date >= '{}'
                  AND rch.date <= '{}'
                "#,
                west, south, east, north, resolution, resolution, start_date, end_date
            );

            let mut conditions = Vec::new();

            if let Some(rid) = receiver_id {
                conditions.push(format!("rch.receiver_id = '{}'", rid));
            }

            if let Some(min_alt) = min_altitude {
                conditions.push(format!("rch.max_altitude_msl_feet >= {}", min_alt));
            }

            if let Some(max_alt) = max_altitude {
                conditions.push(format!("rch.min_altitude_msl_feet <= {}", max_alt));
            }

            let mut sql = base_sql;
            if !conditions.is_empty() {
                sql.push_str(" AND ");
                sql.push_str(&conditions.join(" AND "));
            }

            sql.push_str(&format!(" ORDER BY rch.fix_count DESC LIMIT {}", limit));

            // Execute raw SQL query
            let query_results: Vec<CoverageQueryResult> = diesel::sql_query(sql).load(&mut conn)?;

            let results: Vec<ReceiverCoverageH3> =
                query_results.into_iter().map(|r| r.into()).collect();

            info!(
                "Found {} coverage hexes (resolution {}) in bbox [{}, {}] to [{}, {}]",
                results.len(),
                resolution,
                south,
                west,
                north,
                east
            );

            Ok(results)
        })
        .await?
    }

    /// Get coverage hexes and convert to GeoJSON features
    #[allow(clippy::too_many_arguments)]
    pub async fn get_coverage_geojson(
        &self,
        resolution: i16,
        start_date: NaiveDate,
        end_date: NaiveDate,
        west: f64,
        south: f64,
        east: f64,
        north: f64,
        receiver_id: Option<Uuid>,
        min_altitude: Option<i32>,
        max_altitude: Option<i32>,
        limit: i64,
    ) -> Result<Vec<CoverageHexFeature>> {
        let coverages = self
            .get_coverage_in_bbox(
                resolution,
                start_date,
                end_date,
                west,
                south,
                east,
                north,
                receiver_id,
                min_altitude,
                max_altitude,
                limit,
            )
            .await?;

        // Convert to GeoJSON features
        let features: Result<Vec<CoverageHexFeature>> = coverages
            .into_iter()
            .map(CoverageHexFeature::from_coverage)
            .collect();

        features
    }

    /// Upsert coverage data in batches (used by aggregation command)
    pub async fn upsert_coverage_batch(
        &self,
        coverages: Vec<NewReceiverCoverageH3>,
    ) -> Result<usize> {
        let pool = self.pool.clone();
        let total_count = coverages.len();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Process in chunks of 5000 to avoid parameter limits and improve performance
            const CHUNK_SIZE: usize = 5000;

            for chunk in coverages.chunks(CHUNK_SIZE) {
                // Build arrays for bulk insert using UNNEST
                let h3_indexes: Vec<i64> = chunk.iter().map(|c| c.h3_index).collect();
                let resolutions: Vec<i16> = chunk.iter().map(|c| c.resolution).collect();
                let receiver_ids: Vec<Uuid> = chunk.iter().map(|c| c.receiver_id).collect();
                let dates: Vec<NaiveDate> = chunk.iter().map(|c| c.date).collect();
                let fix_counts: Vec<i32> = chunk.iter().map(|c| c.fix_count).collect();
                let first_seen_ats: Vec<_> = chunk.iter().map(|c| c.first_seen_at).collect();
                let last_seen_ats: Vec<_> = chunk.iter().map(|c| c.last_seen_at).collect();
                let min_altitudes: Vec<Option<i32>> =
                    chunk.iter().map(|c| c.min_altitude_msl_feet).collect();
                let max_altitudes: Vec<Option<i32>> =
                    chunk.iter().map(|c| c.max_altitude_msl_feet).collect();
                let avg_altitudes: Vec<Option<i32>> =
                    chunk.iter().map(|c| c.avg_altitude_msl_feet).collect();

                // Use UNNEST for bulk insert - much faster than individual inserts
                diesel::sql_query(
                    r#"
                    INSERT INTO receiver_coverage_h3 (
                        h3_index, resolution, receiver_id, date,
                        fix_count, first_seen_at, last_seen_at,
                        min_altitude_msl_feet, max_altitude_msl_feet, avg_altitude_msl_feet
                    )
                    SELECT * FROM UNNEST(
                        $1::bigint[], $2::smallint[], $3::uuid[], $4::date[],
                        $5::integer[], $6::timestamptz[], $7::timestamptz[],
                        $8::integer[], $9::integer[], $10::integer[]
                    )
                    ON CONFLICT (h3_index, resolution, receiver_id, date) DO UPDATE SET
                        fix_count = receiver_coverage_h3.fix_count + EXCLUDED.fix_count,
                        first_seen_at = LEAST(receiver_coverage_h3.first_seen_at, EXCLUDED.first_seen_at),
                        last_seen_at = GREATEST(receiver_coverage_h3.last_seen_at, EXCLUDED.last_seen_at),
                        min_altitude_msl_feet = LEAST(receiver_coverage_h3.min_altitude_msl_feet, EXCLUDED.min_altitude_msl_feet),
                        max_altitude_msl_feet = GREATEST(receiver_coverage_h3.max_altitude_msl_feet, EXCLUDED.max_altitude_msl_feet),
                        avg_altitude_msl_feet = (
                            (COALESCE(receiver_coverage_h3.avg_altitude_msl_feet, 0) * receiver_coverage_h3.fix_count +
                             COALESCE(EXCLUDED.avg_altitude_msl_feet, 0) * EXCLUDED.fix_count) /
                            (receiver_coverage_h3.fix_count + EXCLUDED.fix_count)
                        )::int,
                        updated_at = NOW()
                    "#,
                )
                .bind::<sql_types::Array<sql_types::BigInt>, _>(h3_indexes)
                .bind::<sql_types::Array<sql_types::SmallInt>, _>(resolutions)
                .bind::<sql_types::Array<sql_types::Uuid>, _>(receiver_ids)
                .bind::<sql_types::Array<sql_types::Date>, _>(dates)
                .bind::<sql_types::Array<sql_types::Integer>, _>(fix_counts)
                .bind::<sql_types::Array<sql_types::Timestamptz>, _>(first_seen_ats)
                .bind::<sql_types::Array<sql_types::Timestamptz>, _>(last_seen_ats)
                .bind::<sql_types::Array<sql_types::Nullable<sql_types::Integer>>, _>(
                    min_altitudes,
                )
                .bind::<sql_types::Array<sql_types::Nullable<sql_types::Integer>>, _>(
                    max_altitudes,
                )
                .bind::<sql_types::Array<sql_types::Nullable<sql_types::Integer>>, _>(
                    avg_altitudes,
                )
                .execute(&mut conn)?;
            }

            info!(
                "Upserted {} coverage records ({} chunks of max {})",
                total_count,
                total_count.div_ceil(CHUNK_SIZE),
                CHUNK_SIZE
            );

            Ok(total_count)
        })
        .await?
    }
}
