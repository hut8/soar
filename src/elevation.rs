use anyhow::{Context, Result};
use lru::LruCache;
use metrics::{counter, histogram};
use std::{num::NonZeroUsize, sync::Arc, time::Instant};
use tokio::sync::Mutex;

/// Round coordinates to ~100m grid (0.001 degrees ≈ 111m)
/// This creates a cache key that groups nearby lookups together
fn round_coord_for_cache(coord: f64) -> i32 {
    (coord * 1000.0).round() as i32
}

/// Cache key for elevation lookups: (lat_millidegrees, lon_millidegrees)
type CacheKey = (i32, i32);

/// Database for elevation data using HTTP elevation-service
#[derive(Clone)]
pub struct ElevationDB {
    client: reqwest::Client,
    base_url: String,
    /// LRU cache for elevation results: (rounded_lat, rounded_lon) -> elevation_meters
    /// 500,000 entries ≈ 28MB of memory, provides excellent hit rate for multi-aircraft operations
    elevation_cache: Arc<Mutex<LruCache<CacheKey, Option<i16>>>>,
}

impl ElevationDB {
    /// Create a new ElevationDB that queries the elevation-service HTTP API
    pub fn new() -> Result<Self> {
        let base_url = std::env::var("ELEVATION_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:5125".to_string());

        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()?,
            base_url,
            elevation_cache: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(500_000).unwrap(),
            ))),
        })
    }

    /// Query elevation from the elevation-service via HTTP
    /// Returns elevation in meters as a 16-bit signed integer, or None if not available
    async fn query_elevation_service(&self, lat: f64, lon: f64) -> Result<Option<i16>> {
        let start = Instant::now();

        let url = format!("{}/?lat={}&lng={}", self.base_url, lat, lon);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to elevation service")?;

        if !response.status().is_success() {
            counter!("elevation_service_errors_total").increment(1);
            return Ok(None);
        }

        let text = response
            .text()
            .await
            .context("Failed to read elevation service response")?;

        let elevation = text
            .trim()
            .parse::<i16>()
            .context("Failed to parse elevation as integer")?;

        histogram!("elevation_service_query_duration_seconds")
            .record(start.elapsed().as_secs_f64());
        counter!("elevation_service_queries_total").increment(1);

        Ok(Some(elevation))
    }

    /// Get elevation in meters for a given lat/lon
    /// Uses a cache to avoid repeated HTTP queries for nearby coordinates
    pub async fn elevation_at(&self, lat: f64, lon: f64) -> Result<Option<f64>> {
        // Validate coordinates
        if !lat.is_finite() || !lon.is_finite() {
            anyhow::bail!("bad coord: lat and lon must be finite values");
        }

        // Create cache key by rounding to ~100m grid
        let cache_key = (round_coord_for_cache(lat), round_coord_for_cache(lon));

        // Check cache first
        {
            let mut cache = self.elevation_cache.lock().await;
            if let Some(&elevation_opt) = cache.get(&cache_key) {
                counter!("elevation_cache_hits_total").increment(1);
                return Ok(elevation_opt.map(|e| e as f64));
            }
        }

        counter!("elevation_cache_misses_total").increment(1);

        // Query the elevation service
        let elevation_i16 = self.query_elevation_service(lat, lon).await?;

        // Store in cache
        {
            let mut cache = self.elevation_cache.lock().await;
            cache.put(cache_key, elevation_i16);
        }

        Ok(elevation_i16.map(|e| e as f64))
    }
}
