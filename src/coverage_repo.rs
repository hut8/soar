use anyhow::Result;
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::sql_types;
use tracing::info;
use uuid::Uuid;

use crate::coverage::{CoverageHexFeature, NewReceiverCoverageH3, ReceiverCoverageH3};
use crate::schema::receiver_coverage_h3;
use crate::web::PgPool;

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
        use h3o::{CellIndex, LatLng, Resolution};

        let pool = self.pool.clone();
        let limit = limit.min(10000); // Cap at 10k hexes

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Convert bounding box to H3 index range
            // Get H3 indexes for bbox corners to determine query range
            let res = Resolution::try_from(resolution as u8)?;
            let sw = LatLng::new(south, west)?;
            let ne = LatLng::new(north, east)?;
            let sw_h3: u64 = sw.to_cell(res).into();
            let ne_h3: u64 = ne.to_cell(res).into();

            // Get min/max H3 index range for the bounding box
            // Note: H3 indexes are spatially ordered but not perfectly,
            // so we need to over-fetch and filter by actual geometry later
            let min_h3 = sw_h3.min(ne_h3) as i64;
            let max_h3 = sw_h3.max(ne_h3) as i64;

            // Build query with filters
            let mut query = receiver_coverage_h3::table
                .filter(receiver_coverage_h3::resolution.eq(resolution))
                .filter(receiver_coverage_h3::date.ge(start_date))
                .filter(receiver_coverage_h3::date.le(end_date))
                .filter(receiver_coverage_h3::h3_index.ge(min_h3))
                .filter(receiver_coverage_h3::h3_index.le(max_h3))
                .into_boxed();

            if let Some(rid) = receiver_id {
                query = query.filter(receiver_coverage_h3::receiver_id.eq(rid));
            }

            if let Some(min_alt) = min_altitude {
                query = query.filter(receiver_coverage_h3::max_altitude_msl_feet.ge(min_alt));
            }

            if let Some(max_alt) = max_altitude {
                query = query.filter(receiver_coverage_h3::min_altitude_msl_feet.le(max_alt));
            }

            let results = query.limit(limit).load::<ReceiverCoverageH3>(&mut conn)?;

            // Filter results to only include hexes actually within bounding box
            // (H3 range query over-fetches)
            let filtered: Vec<ReceiverCoverageH3> = results
                .into_iter()
                .filter(|coverage| {
                    if let Ok(cell) = CellIndex::try_from(coverage.h3_index as u64) {
                        let center = LatLng::from(cell);
                        center.lat() >= south
                            && center.lat() <= north
                            && center.lng() >= west
                            && center.lng() <= east
                    } else {
                        false
                    }
                })
                .collect();

            info!(
                "Found {} coverage hexes (resolution {}) in bbox [{}, {}] to [{}, {}]",
                filtered.len(),
                resolution,
                south,
                west,
                north,
                east
            );

            Ok(filtered)
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

    /// Upsert coverage data (used by aggregation command)
    pub async fn upsert_coverage_batch(
        &self,
        coverages: Vec<NewReceiverCoverageH3>,
    ) -> Result<usize> {
        let pool = self.pool.clone();
        let count = coverages.len();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Use raw SQL for efficient UPSERT
            // Diesel doesn't handle ON CONFLICT well for composite keys
            for coverage in coverages {
                diesel::sql_query(
                    r#"
                    INSERT INTO receiver_coverage_h3 (
                        h3_index, resolution, receiver_id, date,
                        fix_count, first_seen_at, last_seen_at,
                        min_altitude_msl_feet, max_altitude_msl_feet, avg_altitude_msl_feet
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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
                .bind::<sql_types::BigInt, _>(coverage.h3_index)
                .bind::<sql_types::SmallInt, _>(coverage.resolution)
                .bind::<sql_types::Uuid, _>(coverage.receiver_id)
                .bind::<sql_types::Date, _>(coverage.date)
                .bind::<sql_types::Integer, _>(coverage.fix_count)
                .bind::<sql_types::Timestamptz, _>(coverage.first_seen_at)
                .bind::<sql_types::Timestamptz, _>(coverage.last_seen_at)
                .bind::<sql_types::Nullable<sql_types::Integer>, _>(
                    coverage.min_altitude_msl_feet,
                )
                .bind::<sql_types::Nullable<sql_types::Integer>, _>(
                    coverage.max_altitude_msl_feet,
                )
                .bind::<sql_types::Nullable<sql_types::Integer>, _>(
                    coverage.avg_altitude_msl_feet,
                )
                .execute(&mut conn)?;
            }

            Ok(count)
        })
        .await?
    }
}
