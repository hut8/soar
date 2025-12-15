use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::NaiveDate;
use moka::future::Cache;
use uuid::Uuid;

use crate::analytics::*;
use crate::analytics_repo::AnalyticsRepository;

/// Cache keys for different analytics queries
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
enum CacheKey {
    DailyFlights(NaiveDate, NaiveDate),
    DurationDistribution,
    HourlyFlights(i32),
    AircraftOutliers(String), // threshold as string to avoid float equality issues
    TopAircraft(i32, i32),    // limit, period_days
    ClubAnalytics(NaiveDate, NaiveDate, Option<Uuid>),
    AirportActivity(NaiveDate, NaiveDate, i32),
    Summary,
}

/// Cached analytics service with 60-second TTL
#[derive(Clone)]
pub struct AnalyticsCache {
    repo: AnalyticsRepository,
    daily_flights_cache: Cache<CacheKey, Vec<FlightAnalyticsDaily>>,
    duration_cache: Cache<CacheKey, Vec<FlightDurationBucket>>,
    hourly_cache: Cache<CacheKey, Vec<FlightAnalyticsHourly>>,
    outliers_cache: Cache<CacheKey, Vec<AircraftOutlier>>,
    top_aircraft_cache: Cache<CacheKey, Vec<TopAircraft>>,
    club_cache: Cache<CacheKey, Vec<ClubAnalyticsDaily>>,
    airport_cache: Cache<CacheKey, Vec<AirportActivity>>,
    summary_cache: Cache<CacheKey, AnalyticsSummary>,
}

impl AnalyticsCache {
    pub fn new(repo: AnalyticsRepository) -> Self {
        let ttl = Duration::from_secs(60);
        let max_capacity = 100;

        Self {
            repo,
            daily_flights_cache: Cache::builder()
                .max_capacity(max_capacity)
                .time_to_live(ttl)
                .build(),
            duration_cache: Cache::builder().max_capacity(10).time_to_live(ttl).build(),
            hourly_cache: Cache::builder().max_capacity(20).time_to_live(ttl).build(),
            outliers_cache: Cache::builder().max_capacity(20).time_to_live(ttl).build(),
            top_aircraft_cache: Cache::builder().max_capacity(50).time_to_live(ttl).build(),
            club_cache: Cache::builder()
                .max_capacity(max_capacity)
                .time_to_live(ttl)
                .build(),
            airport_cache: Cache::builder()
                .max_capacity(max_capacity)
                .time_to_live(ttl)
                .build(),
            summary_cache: Cache::builder().max_capacity(1).time_to_live(ttl).build(),
        }
    }

    pub async fn get_daily_flights(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<FlightAnalyticsDaily>> {
        let start = Instant::now();
        let key = CacheKey::DailyFlights(start_date, end_date);

        if let Some(cached) = self.daily_flights_cache.get(&key).await {
            metrics::counter!("analytics.cache.hit").increment(1);
            let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
            metrics::histogram!("analytics.query.daily_flights_ms").record(duration_ms);
            return Ok(cached);
        }

        metrics::counter!("analytics.cache.miss").increment(1);
        let result = self.repo.get_daily_flights(start_date, end_date).await?;
        self.daily_flights_cache.insert(key, result.clone()).await;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        metrics::histogram!("analytics.query.daily_flights_ms").record(duration_ms);
        Ok(result)
    }

    pub async fn get_duration_distribution(&self) -> Result<Vec<FlightDurationBucket>> {
        let start = Instant::now();
        let key = CacheKey::DurationDistribution;

        if let Some(cached) = self.duration_cache.get(&key).await {
            metrics::counter!("analytics.cache.hit").increment(1);
            let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
            metrics::histogram!("analytics.query.duration_distribution_ms").record(duration_ms);
            return Ok(cached);
        }

        metrics::counter!("analytics.cache.miss").increment(1);
        let result = self.repo.get_duration_distribution().await?;
        self.duration_cache.insert(key, result.clone()).await;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        metrics::histogram!("analytics.query.duration_distribution_ms").record(duration_ms);
        Ok(result)
    }

    pub async fn get_hourly_flights(&self, hours: i32) -> Result<Vec<FlightAnalyticsHourly>> {
        let start = Instant::now();
        let key = CacheKey::HourlyFlights(hours);

        if let Some(cached) = self.hourly_cache.get(&key).await {
            metrics::counter!("analytics.cache.hit").increment(1);
            let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
            metrics::histogram!("analytics.query.hourly_flights_ms").record(duration_ms);
            return Ok(cached);
        }

        metrics::counter!("analytics.cache.miss").increment(1);
        let result = self.repo.get_hourly_flights(hours).await?;
        self.hourly_cache.insert(key, result.clone()).await;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        metrics::histogram!("analytics.query.hourly_flights_ms").record(duration_ms);
        Ok(result)
    }

    pub async fn get_device_outliers(&self, threshold: f64) -> Result<Vec<AircraftOutlier>> {
        let start = Instant::now();
        let key = CacheKey::AircraftOutliers(format!("{:.2}", threshold));

        if let Some(cached) = self.outliers_cache.get(&key).await {
            metrics::counter!("analytics.cache.hit").increment(1);
            let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
            metrics::histogram!("analytics.query.aircraft_outliers_ms").record(duration_ms);
            return Ok(cached);
        }

        metrics::counter!("analytics.cache.miss").increment(1);
        let result = self.repo.get_device_outliers(threshold).await?;
        self.outliers_cache.insert(key, result.clone()).await;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        metrics::histogram!("analytics.query.aircraft_outliers_ms").record(duration_ms);
        Ok(result)
    }

    pub async fn get_top_aircraft(&self, limit: i32, period_days: i32) -> Result<Vec<TopAircraft>> {
        let start = Instant::now();
        let key = CacheKey::TopAircraft(limit, period_days);

        if let Some(cached) = self.top_aircraft_cache.get(&key).await {
            metrics::counter!("analytics.cache.hit").increment(1);
            let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
            metrics::histogram!("analytics.query.top_aircraft_ms").record(duration_ms);
            return Ok(cached);
        }

        metrics::counter!("analytics.cache.miss").increment(1);
        let result = self.repo.get_top_aircraft(limit, period_days).await?;
        self.top_aircraft_cache.insert(key, result.clone()).await;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        metrics::histogram!("analytics.query.top_aircraft_ms").record(duration_ms);
        Ok(result)
    }

    pub async fn get_club_analytics(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        club_id: Option<Uuid>,
    ) -> Result<Vec<ClubAnalyticsDaily>> {
        let start = Instant::now();
        let key = CacheKey::ClubAnalytics(start_date, end_date, club_id);

        if let Some(cached) = self.club_cache.get(&key).await {
            metrics::counter!("analytics.cache.hit").increment(1);
            let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
            metrics::histogram!("analytics.query.club_analytics_ms").record(duration_ms);
            return Ok(cached);
        }

        metrics::counter!("analytics.cache.miss").increment(1);
        let result = self
            .repo
            .get_club_analytics(start_date, end_date, club_id)
            .await?;
        self.club_cache.insert(key, result.clone()).await;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        metrics::histogram!("analytics.query.club_analytics_ms").record(duration_ms);
        Ok(result)
    }

    pub async fn get_airport_activity(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        limit: i32,
    ) -> Result<Vec<AirportActivity>> {
        let start = Instant::now();
        let key = CacheKey::AirportActivity(start_date, end_date, limit);

        if let Some(cached) = self.airport_cache.get(&key).await {
            metrics::counter!("analytics.cache.hit").increment(1);
            let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
            metrics::histogram!("analytics.query.airport_activity_ms").record(duration_ms);
            return Ok(cached);
        }

        metrics::counter!("analytics.cache.miss").increment(1);
        let result = self
            .repo
            .get_airport_activity(start_date, end_date, limit)
            .await?;
        self.airport_cache.insert(key, result.clone()).await;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        metrics::histogram!("analytics.query.airport_activity_ms").record(duration_ms);
        Ok(result)
    }

    pub async fn get_summary(&self) -> Result<AnalyticsSummary> {
        let start = Instant::now();
        let key = CacheKey::Summary;

        if let Some(cached) = self.summary_cache.get(&key).await {
            metrics::counter!("analytics.cache.hit").increment(1);
            let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
            metrics::histogram!("analytics.query.summary_ms").record(duration_ms);
            return Ok(cached);
        }

        metrics::counter!("analytics.cache.miss").increment(1);
        let result = self.repo.get_summary().await?;
        self.summary_cache.insert(key, result.clone()).await;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        metrics::histogram!("analytics.query.summary_ms").record(duration_ms);
        Ok(result)
    }

    /// Invalidate all caches (useful for testing or manual refresh)
    pub async fn invalidate_all(&self) {
        self.daily_flights_cache.invalidate_all();
        self.duration_cache.invalidate_all();
        self.hourly_cache.invalidate_all();
        self.outliers_cache.invalidate_all();
        self.top_aircraft_cache.invalidate_all();
        self.club_cache.invalidate_all();
        self.airport_cache.invalidate_all();
        self.summary_cache.invalidate_all();
    }
}
