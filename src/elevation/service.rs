use anyhow::{Context, Result, bail};
use metrics::{counter, gauge, histogram};
use moka::future::Cache;
use std::{env, path::PathBuf, sync::Arc, time::Instant};
use tracing::{debug, info, warn};

use super::hgt::HGT;
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
/// - On-demand S3 download for missing tiles (optional)
#[derive(Clone)]
pub struct ElevationService {
    storage_path: PathBuf,
    /// Concurrent cache for elevation results: (rounded_lat, rounded_lon) -> elevation_meters
    /// 250,000 entries ≈ 14MB of memory, provides excellent hit rate for multi-aircraft operations
    /// Uses moka for lock-free concurrent access across multiple workers
    elevation_cache: Cache<CacheKey, Option<i16>>,
    /// Concurrent cache for HGT tiles: (lat_floor, lon_floor) -> HGT
    /// 500 entries ≈ 12.5GB for 1-arcsec tiles (25MB each), less for 3-arcsec
    /// Uses moka for lock-free concurrent access across multiple workers
    tile_cache: Cache<(i32, i32), Arc<HGT>>,
    /// Optional S3 client for on-demand tile downloads
    s3_client: Option<Arc<aws_sdk_s3::Client>>,
    /// S3 bucket name for elevation tiles (e.g., "elevation-tiles-prod")
    s3_bucket: Option<String>,
    /// S3 key prefix for tiles (e.g., "skadi")
    s3_prefix: Option<String>,
}

impl ElevationService {
    /// Create a new ElevationService using ELEVATION_DATA_PATH env var
    /// Defaults to /var/soar/elevation if not specified
    ///
    /// If S3 download is enabled (ELEVATION_S3_BUCKET env var set), missing tiles will be
    /// downloaded from S3 on-demand. The directory will be created if it doesn't exist.
    pub fn new() -> Result<Self> {
        let storage_path =
            env::var("ELEVATION_DATA_PATH").unwrap_or_else(|_| "/var/soar/elevation".to_string());
        let storage_path = PathBuf::from(storage_path);

        // Check if S3 download is enabled
        let s3_enabled = env::var("ELEVATION_S3_BUCKET").is_ok();

        if !s3_enabled && !storage_path.exists() {
            bail!(
                "Elevation data directory does not exist: {:?} (and S3 download is not enabled)",
                storage_path
            );
        }

        // Create directory if it doesn't exist (needed for S3 downloads or writable path)
        if !storage_path.exists() {
            info!("Creating elevation data directory: {:?}", storage_path);
            std::fs::create_dir_all(&storage_path).with_context(|| {
                format!("Failed to create elevation directory: {:?}", storage_path)
            })?;
        }

        Ok(Self {
            storage_path,
            elevation_cache: Cache::builder().max_capacity(250_000).build(),
            tile_cache: Cache::builder().max_capacity(500).build(),
            s3_client: None,
            s3_bucket: None,
            s3_prefix: None,
        })
    }

    /// Create a new ElevationService with S3 download capability
    ///
    /// Environment variables:
    /// - ELEVATION_DATA_PATH: Local storage path (default: /var/soar/elevation)
    /// - ELEVATION_S3_BUCKET: S3 bucket name (default: "elevation-tiles-prod")
    /// - ELEVATION_S3_PREFIX: S3 key prefix (default: "skadi")
    /// - ELEVATION_S3_REGION: AWS region (default: "us-east-1")
    ///
    /// S3 is always enabled with a default bucket for automatic tile downloads
    pub async fn new_with_s3() -> Result<Self> {
        let storage_path =
            env::var("ELEVATION_DATA_PATH").unwrap_or_else(|_| "/var/soar/elevation".to_string());
        let storage_path = PathBuf::from(storage_path);

        // Create directory if it doesn't exist
        if !storage_path.exists() {
            info!("Creating elevation data directory: {:?}", storage_path);
            std::fs::create_dir_all(&storage_path).with_context(|| {
                format!("Failed to create elevation directory: {:?}", storage_path)
            })?;
        }

        // Get S3 configuration from environment with defaults
        let s3_bucket =
            env::var("ELEVATION_S3_BUCKET").unwrap_or_else(|_| "elevation-tiles-prod".to_string());
        let s3_prefix = env::var("ELEVATION_S3_PREFIX").unwrap_or_else(|_| "skadi".to_string());

        info!(
            "Initializing S3 client for elevation tile downloads from bucket: {}/{}",
            s3_bucket, s3_prefix
        );

        // Load AWS config from environment
        let config = aws_config::load_from_env().await;
        let client = aws_sdk_s3::Client::new(&config);

        Ok(Self {
            storage_path,
            elevation_cache: Cache::builder().max_capacity(250_000).build(),
            tile_cache: Cache::builder().max_capacity(500).build(),
            s3_client: Some(Arc::new(client)),
            s3_bucket: Some(s3_bucket),
            s3_prefix: Some(s3_prefix),
        })
    }

    /// Create a new ElevationService with an explicit storage path
    /// S3 downloads are enabled with default bucket for missing tiles
    pub async fn with_path(storage_path: PathBuf) -> Result<Self> {
        if !storage_path.exists() {
            bail!(
                "Elevation data directory does not exist: {:?}",
                storage_path
            );
        }

        // Get S3 configuration from environment with defaults
        let s3_bucket =
            env::var("ELEVATION_S3_BUCKET").unwrap_or_else(|_| "elevation-tiles-prod".to_string());
        let s3_prefix = env::var("ELEVATION_S3_PREFIX").unwrap_or_else(|_| "skadi".to_string());

        info!(
            "Initializing S3 client for elevation tile downloads from bucket: {}/{}",
            s3_bucket, s3_prefix
        );

        // Load AWS config from environment
        let config = aws_config::load_from_env().await;
        let client = aws_sdk_s3::Client::new(&config);

        Ok(Self {
            storage_path,
            elevation_cache: Cache::builder().max_capacity(250_000).build(),
            tile_cache: Cache::builder().max_capacity(500).build(),
            s3_client: Some(Arc::new(client)),
            s3_bucket: Some(s3_bucket),
            s3_prefix: Some(s3_prefix),
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
            s3_client: None,
            s3_bucket: None,
            s3_prefix: None,
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
            counter!("elevation_cache_hits_total").increment(1);
            gauge!("elevation_cache_entries").set(self.elevation_cache.entry_count() as f64);
            histogram!("elevation_lookup_duration_seconds").record(start.elapsed().as_secs_f64());
            return Ok(cached_elevation);
        }

        // Elevation cache miss - need to look up from HGT tile
        counter!("elevation_cache_misses_total").increment(1);

        // Determine which tile we need
        let lat_floor = lat.floor() as i32;
        let lon_floor = lon.floor() as i32;
        let tile_key = (lat_floor, lon_floor);

        // Check if we have this tile cached, or load it
        let hgt_tile = if let Some(cached) = self.tile_cache.get(&tile_key).await {
            counter!("elevation_tile_cache_hits_total").increment(1);
            cached
        } else {
            // Tile cache miss - need to load from disk
            counter!("elevation_tile_cache_misses_total").increment(1);

            let tile_load_start = Instant::now();

            // Build tile path
            let tile_path = self.get_tile_path(lat_floor, lon_floor);

            // Check if tile exists locally
            if !tile_path.exists() {
                // Try to download from S3 if configured
                match self.download_tile_from_s3(lat_floor, lon_floor).await {
                    Ok(true) => {
                        debug!("Successfully downloaded tile from S3, will load it");
                        // Continue to load the tile below
                    }
                    Ok(false) => {
                        // Tile doesn't exist in S3 either (likely ocean)
                        debug!(
                            "Tile not available locally or in S3 (likely ocean): lat={}, lon={}",
                            lat_floor, lon_floor
                        );
                        self.elevation_cache.insert(cache_key, None).await;
                        gauge!("elevation_cache_entries")
                            .set(self.elevation_cache.entry_count() as f64);
                        histogram!("elevation_lookup_duration_seconds")
                            .record(start.elapsed().as_secs_f64());
                        return Ok(None);
                    }
                    Err(e) => {
                        // S3 download failed - treat as missing tile
                        warn!(
                            "Failed to download tile from S3: {} (lat={}, lon={})",
                            e, lat_floor, lon_floor
                        );
                        self.elevation_cache.insert(cache_key, None).await;
                        gauge!("elevation_cache_entries")
                            .set(self.elevation_cache.entry_count() as f64);
                        histogram!("elevation_lookup_duration_seconds")
                            .record(start.elapsed().as_secs_f64());
                        return Ok(None);
                    }
                }
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

    /// Build the S3 key for a tile
    /// Format: skadi/N45/N45E009.hgt.gz
    fn get_s3_key(&self, lat_floor: i32, lon_floor: i32) -> String {
        let lat_prefix = if lat_floor < 0 { "S" } else { "N" };
        let lon_prefix = if lon_floor < 0 { "W" } else { "E" };
        let lat_abs = lat_floor.abs();
        let lon_abs = lon_floor.abs();

        let lat_dir = format!("{}{:02}", lat_prefix, lat_abs);
        let filename = format!("{}{}{:03}.hgt.gz", lat_dir, lon_prefix, lon_abs);

        let prefix = self.s3_prefix.as_deref().unwrap_or("skadi");
        format!("{}/{}/{}", prefix, lat_dir, filename)
    }

    /// Download a tile from S3 if S3 client is configured
    /// Returns true if downloaded successfully, false if tile doesn't exist in S3
    async fn download_tile_from_s3(&self, lat_floor: i32, lon_floor: i32) -> Result<bool> {
        let s3_client = match &self.s3_client {
            Some(client) => client,
            None => return Ok(false), // S3 not configured
        };

        let bucket = match &self.s3_bucket {
            Some(b) => b,
            None => return Ok(false),
        };

        let s3_key = self.get_s3_key(lat_floor, lon_floor);
        let tile_path = self.get_tile_path(lat_floor, lon_floor);

        info!(
            "Downloading elevation tile from S3: s3://{}/{}",
            bucket, s3_key
        );
        counter!("elevation_s3_download_attempts_total").increment(1);

        // Download from S3
        let download_start = Instant::now();
        let result = s3_client
            .get_object()
            .bucket(bucket)
            .key(&s3_key)
            .send()
            .await;

        let object = match result {
            Ok(obj) => obj,
            Err(e) => {
                // Check if it's a 404 (tile doesn't exist in S3 either - likely ocean)
                if e.to_string().contains("404") || e.to_string().contains("NoSuchKey") {
                    debug!("Tile not found in S3: {}", s3_key);
                    counter!("elevation_s3_tile_not_found_total").increment(1);
                    return Ok(false);
                }
                // Other error - propagate
                counter!("elevation_s3_download_errors_total").increment(1);
                return Err(e.into());
            }
        };

        // Create parent directory if needed
        if let Some(parent) = tile_path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }

        // Stream data to file
        let body = object.body;
        let mut file = tokio::fs::File::create(&tile_path)
            .await
            .with_context(|| format!("Failed to create file: {:?}", tile_path))?;

        use tokio::io::AsyncWriteExt;

        // Use AWS SDK ByteStream's collect method instead
        let bytes = body.collect().await?.into_bytes();
        file.write_all(&bytes).await?;

        file.flush().await?;
        drop(file);

        histogram!("elevation_s3_download_duration_seconds")
            .record(download_start.elapsed().as_secs_f64());
        counter!("elevation_s3_downloads_success_total").increment(1);

        info!("Successfully downloaded elevation tile: {:?}", tile_path);
        Ok(true)
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
            s3_client: None,
            s3_bucket: None,
            s3_prefix: None,
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
