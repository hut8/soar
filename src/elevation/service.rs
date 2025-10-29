use anyhow::{Context, Result, bail};
use metrics::{counter, gauge, histogram};
use moka::future::Cache;
use std::{env, path::PathBuf, sync::Arc, time::Instant};

use super::hgt::HGT;
use crate::Fix;
use uuid::Uuid;

/// Task for calculating elevation/AGL for a fix
/// Sent through a separate bounded channel to prevent blocking main processing
pub struct ElevationTask {
    pub fix_id: Uuid,
    pub fix: Fix,
}

/// Task for batch updating AGL values in database
/// Sent from elevation workers to the batch database writer
pub struct AglDatabaseTask {
    pub fix_id: Uuid,
    pub altitude_agl_feet: Option<i32>,
}

/// Round coordinates to ~100m grid (0.001 degrees ≈ 111m)
/// This creates a cache key that groups nearby lookups together
fn round_coord_for_cache(coord: f64) -> i32 {
    (coord * 1000.0).round() as i32
}

/// Cache key for elevation lookups: (lat_millidegrees, lon_millidegrees)
type CacheKey = (i32, i32);

/// High-performance elevation service using HGT (SRTM) tiles
///
/// This implementation is optimized for concurrent access with:
/// - Lock-free caching using moka for both elevation results and HGT tiles
/// - Bilinear interpolation for accurate elevation values
/// - Memory-efficient design that keeps tiles in memory for fast access
#[derive(Clone)]
pub struct ElevationService {
    storage_path: PathBuf,
    /// Concurrent cache for elevation results: (rounded_lat, rounded_lon) -> elevation_meters
    /// 500,000 entries ≈ 28MB of memory, provides excellent hit rate for multi-aircraft operations
    /// Uses moka for lock-free concurrent access across multiple workers
    elevation_cache: Cache<CacheKey, Option<i16>>,
    /// Concurrent cache for HGT tiles: (lat_floor, lon_floor) -> HGT
    /// 1000 entries ≈ 25GB for 1-arcsec tiles (25MB each), less for 3-arcsec
    /// Uses moka for lock-free concurrent access across multiple workers
    tile_cache: Cache<(i32, i32), Arc<HGT>>,
}

impl ElevationService {
    /// Create a new ElevationService using ELEVATION_DATA_PATH env var
    /// Defaults to /var/soar/elevation if not specified
    pub fn new() -> Result<Self> {
        let storage_path =
            env::var("ELEVATION_DATA_PATH").unwrap_or_else(|_| "/var/soar/elevation".to_string());
        let storage_path = PathBuf::from(storage_path);

        if !storage_path.exists() {
            bail!(
                "Elevation data directory does not exist: {:?}",
                storage_path
            );
        }

        Ok(Self {
            storage_path,
            elevation_cache: Cache::builder().max_capacity(500_000).build(),
            tile_cache: Cache::builder().max_capacity(1000).build(),
        })
    }

    /// Create a new ElevationService with an explicit storage path
    pub fn with_path(storage_path: PathBuf) -> Result<Self> {
        if !storage_path.exists() {
            bail!(
                "Elevation data directory does not exist: {:?}",
                storage_path
            );
        }

        Ok(Self {
            storage_path,
            elevation_cache: Cache::builder().max_capacity(500_000).build(),
            tile_cache: Cache::builder().max_capacity(1000).build(),
        })
    }

    /// Create a new ElevationService with custom cache sizes for specialized workloads
    /// Used by AGL backfill which needs a larger tile cache to avoid blocking real-time processing
    /// Defaults to /var/soar/elevation if ELEVATION_DATA_PATH is not specified
    pub fn with_custom_cache_sizes(
        elevation_cache_size: u64,
        tile_cache_size: u64,
    ) -> Result<Self> {
        let storage_path =
            env::var("ELEVATION_DATA_PATH").unwrap_or_else(|_| "/var/soar/elevation".to_string());
        let storage_path = PathBuf::from(storage_path);

        if !storage_path.exists() {
            bail!(
                "Elevation data directory does not exist: {:?}",
                storage_path
            );
        }

        Ok(Self {
            storage_path,
            elevation_cache: Cache::builder().max_capacity(elevation_cache_size).build(),
            tile_cache: Cache::builder().max_capacity(tile_cache_size).build(),
        })
    }

    /// Get the storage path for this ElevationService
    pub fn storage_path(&self) -> &PathBuf {
        &self.storage_path
    }

    /// Returns elevation in meters at the given lat/lng coordinates
    /// Uses SRTM/HGT format tiles with bilinear interpolation
    ///
    /// Note: This returns i16 (matching HGT format) while the old implementation returned f64.
    /// For most use cases, converting i16 to f64 is sufficient.
    pub async fn elevation(&self, lat: f64, lon: f64) -> Result<Option<i16>> {
        let start = Instant::now();

        // Validate coordinates
        if !lat.is_finite() || !lon.is_finite() {
            bail!("bad coord");
        }

        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            bail!("Coordinates out of range: lat={}, lon={}", lat, lon);
        }

        // Create cache key by rounding coordinates to ~100m grid
        let cache_key = (round_coord_for_cache(lat), round_coord_for_cache(lon));

        // Check elevation cache first (fastest path)
        if let Some(cached_elevation) = self.elevation_cache.get(&cache_key).await {
            counter!("elevation_cache_hits").increment(1);
            gauge!("elevation_cache_entries").set(self.elevation_cache.entry_count() as f64);
            histogram!("elevation_lookup_duration_seconds").record(start.elapsed().as_secs_f64());
            return Ok(cached_elevation);
        }

        // Elevation cache miss - need to look up from HGT tile
        counter!("elevation_cache_misses").increment(1);

        // Determine which tile we need
        let lat_floor = lat.floor() as i32;
        let lon_floor = lon.floor() as i32;
        let tile_key = (lat_floor, lon_floor);

        // Check if we have this tile cached, or load it
        let hgt_tile = if let Some(cached) = self.tile_cache.get(&tile_key).await {
            counter!("elevation_tile_cache_hits").increment(1);
            cached
        } else {
            // Tile cache miss - need to load from disk
            counter!("elevation_tile_cache_misses").increment(1);

            let tile_load_start = Instant::now();

            // Build tile path
            let tile_path = self.get_tile_path(lat_floor, lon_floor);

            // Check if tile exists
            if !tile_path.exists() {
                // Ocean/missing tile - cache None result
                self.elevation_cache.insert(cache_key, None).await;
                gauge!("elevation_cache_entries").set(self.elevation_cache.entry_count() as f64);
                histogram!("elevation_lookup_duration_seconds")
                    .record(start.elapsed().as_secs_f64());
                return Ok(None);
            }

            // Load and decompress tile
            let hgt = HGT::from_file(&tile_path, (lat_floor as f64, lon_floor as f64))
                .await
                .with_context(|| format!("Failed to load HGT tile: {:?}", tile_path))?;

            let hgt = Arc::new(hgt);

            // Record tile load time
            histogram!("elevation_tile_load_duration_seconds")
                .record(tile_load_start.elapsed().as_secs_f64());

            // Cache the tile
            self.tile_cache.insert(tile_key, hgt.clone()).await;
            gauge!("elevation_tile_cache_entries").set(self.tile_cache.entry_count() as f64);

            hgt
        };

        // Get elevation from tile using bilinear interpolation
        let interpolation_start = Instant::now();
        let elevation = hgt_tile.get_elevation(lat, lon).ok();

        // Record interpolation time
        histogram!("elevation_interpolation_duration_seconds")
            .record(interpolation_start.elapsed().as_secs_f64());

        // Store in elevation cache for future lookups
        self.elevation_cache.insert(cache_key, elevation).await;
        gauge!("elevation_cache_entries").set(self.elevation_cache.entry_count() as f64);

        // Record total elevation lookup duration
        histogram!("elevation_lookup_duration_seconds").record(start.elapsed().as_secs_f64());

        Ok(elevation)
    }

    /// Build the file path for a tile at the given lat/lon floor coordinates
    /// Format: /var/soar/elevation/N45/N45E009.hgt.gz
    fn get_tile_path(&self, lat_floor: i32, lon_floor: i32) -> PathBuf {
        let lat_prefix = if lat_floor < 0 { "S" } else { "N" };
        let lon_prefix = if lon_floor < 0 { "W" } else { "E" };
        let lat_abs = lat_floor.abs();
        let lon_abs = lon_floor.abs();

        let lat_dir = format!("{}{:02}", lat_prefix, lat_abs);
        let filename = format!("{}{}{:03}.hgt.gz", lat_dir, lon_prefix, lon_abs);

        self.storage_path.join(&lat_dir).join(filename)
    }

    /// Legacy compatibility method - returns elevation as f64
    /// This method exists for backwards compatibility with the old ElevationDB API
    pub async fn elevation_egm2008(&self, lat: f64, lon: f64) -> Result<Option<f64>> {
        Ok(self.elevation(lat, lon).await?.map(|e| e as f64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_tile_path() {
        // Create a test service with a dummy path (directory doesn't need to exist for this test)
        let service = ElevationService {
            storage_path: PathBuf::from("/var/soar/elevation"),
            elevation_cache: Cache::builder().max_capacity(100).build(),
            tile_cache: Cache::builder().max_capacity(10).build(),
        };

        // Northern hemisphere, eastern
        let path = service.get_tile_path(45, 9);
        assert_eq!(
            path,
            PathBuf::from("/var/soar/elevation/N45/N45E009.hgt.gz")
        );

        // Southern hemisphere, western
        let path = service.get_tile_path(-45, -9);
        assert_eq!(
            path,
            PathBuf::from("/var/soar/elevation/S45/S45W009.hgt.gz")
        );

        // Equator
        let path = service.get_tile_path(0, 0);
        assert_eq!(
            path,
            PathBuf::from("/var/soar/elevation/N00/N00E000.hgt.gz")
        );

        // Large coordinates
        let path = service.get_tile_path(90, 180);
        assert_eq!(
            path,
            PathBuf::from("/var/soar/elevation/N90/N90E180.hgt.gz")
        );
    }

    #[test]
    fn test_round_coord_for_cache() {
        // Test rounding to ~100m grid
        assert_eq!(round_coord_for_cache(45.1234), 45123);
        assert_eq!(round_coord_for_cache(45.1235), 45124);
        assert_eq!(round_coord_for_cache(-45.1234), -45123);
        assert_eq!(round_coord_for_cache(0.0), 0);
    }
}
