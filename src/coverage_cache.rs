use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::NaiveDate;
use moka::future::Cache;
use uuid::Uuid;

use crate::coverage::CoverageHexFeature;
use crate::coverage_repo::CoverageRepository;

/// Cache key for coverage queries
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
struct CoverageKey {
    resolution: i16,
    start_date: NaiveDate,
    end_date: NaiveDate,
    west: String, // f64 as string to avoid float equality issues
    south: String,
    east: String,
    north: String,
    receiver_id: Option<Uuid>,
    min_altitude: Option<i32>,
    max_altitude: Option<i32>,
    limit: i64,
}

impl CoverageKey {
    #[allow(clippy::too_many_arguments)]
    fn new(
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
    ) -> Self {
        Self {
            resolution,
            start_date,
            end_date,
            west: format!("{:.6}", west),
            south: format!("{:.6}", south),
            east: format!("{:.6}", east),
            north: format!("{:.6}", north),
            receiver_id,
            min_altitude,
            max_altitude,
            limit,
        }
    }
}

/// Cached coverage service with 60-second TTL
#[derive(Clone)]
pub struct CoverageCache {
    repo: CoverageRepository,
    cache: Cache<CoverageKey, Vec<CoverageHexFeature>>,
}

impl CoverageCache {
    pub fn new(repo: CoverageRepository) -> Self {
        let ttl = Duration::from_secs(60);
        let max_capacity = 100;

        Self {
            repo,
            cache: Cache::builder()
                .max_capacity(max_capacity)
                .time_to_live(ttl)
                .build(),
        }
    }

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
        let start = Instant::now();
        let key = CoverageKey::new(
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
        );

        if let Some(cached) = self.cache.get(&key).await {
            metrics::counter!("coverage.cache.hit").increment(1);
            let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
            metrics::histogram!("coverage.query.hexes_ms").record(duration_ms);
            return Ok(cached);
        }

        metrics::counter!("coverage.cache.miss").increment(1);
        let result = self
            .repo
            .get_coverage_geojson(
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

        self.cache.insert(key, result.clone()).await;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        metrics::histogram!("coverage.query.hexes_ms").record(duration_ms);
        Ok(result)
    }
}
