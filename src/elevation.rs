use anyhow::{Context, Result, bail};
use directories::BaseDirs;
use gdal::{Dataset, raster::ResampleAlg};
use lru::LruCache;
use metrics::{counter, gauge, histogram};
use std::{env, num::NonZeroUsize, path::PathBuf, sync::Arc, time::Instant};
use tokio::sync::Mutex;

use crate::tile_downloader::TileDownloader;

/// Round coordinates to ~100m grid (0.001 degrees ≈ 111m)
/// This creates a cache key that groups nearby lookups together
fn round_coord_for_cache(coord: f64) -> i32 {
    (coord * 1000.0).round() as i32
}

/// Cache key for elevation lookups: (lat_millidegrees, lon_millidegrees)
type CacheKey = (i32, i32);

/// Cached GDAL Dataset with metadata
/// Keeps the TIFF file open and parsed to avoid repeated Dataset::open() calls
/// SAFETY: GDAL Dataset is not thread-safe, so we wrap it in a Mutex to ensure
/// exclusive access during reads.
struct CachedDataset {
    dataset: Mutex<Dataset>,
    geo_transform: [f64; 6],
    raster_width: usize,
    raster_height: usize,
    no_data_value: Option<f64>,
}

impl CachedDataset {
    fn new(dataset: Dataset) -> Result<Self> {
        let band = dataset.rasterband(1)?;
        let (raster_width, raster_height) = band.size();
        let geo_transform = dataset.geo_transform()?;
        let no_data_value = band.no_data_value();

        Ok(Self {
            dataset: Mutex::new(dataset),
            geo_transform,
            raster_width,
            raster_height,
            no_data_value,
        })
    }
}

// SAFETY: CachedDataset can be safely shared between threads because:
// 1. The Dataset is protected by a Mutex ensuring exclusive access
// 2. All other fields (geo_transform, dimensions, no_data_value) are immutable primitives
unsafe impl Send for CachedDataset {}
unsafe impl Sync for CachedDataset {}

/// Database for elevation data using Copernicus DEM tiles
#[derive(Clone)]
pub struct ElevationDB {
    storage_path: PathBuf,
    /// Manages tile downloads with deduplication
    tile_downloader: TileDownloader,
    /// LRU cache for elevation results: (rounded_lat, rounded_lon) -> elevation_meters
    /// 500,000 entries ≈ 28MB of memory, provides excellent hit rate for multi-aircraft operations
    elevation_cache: Arc<Mutex<LruCache<CacheKey, Option<f64>>>>,
    /// LRU cache for open GDAL Datasets: tile_path -> CachedDataset
    /// 100 entries ≈ 1-2GB memory (10-20MB per TIFF), eliminates repeated Dataset::open() calls
    dataset_cache: Arc<Mutex<LruCache<PathBuf, Arc<CachedDataset>>>>,
}

impl ElevationDB {
    /// Create a new ElevationDB using ELEVATION_DATA_PATH env var or default cache directory
    pub fn new() -> Result<Self> {
        let storage_path = match env::var("ELEVATION_DATA_PATH") {
            Ok(path) => PathBuf::from(path),
            Err(_) => {
                let base = BaseDirs::new().context("no home directory")?;
                base.cache_dir()
                    .join("elevation")
                    .join("copernicus-dem-30m")
            }
        };

        std::fs::create_dir_all(&storage_path).with_context(|| {
            format!(
                "Failed to create elevation storage directory: {:?}",
                storage_path
            )
        })?;

        Ok(Self {
            tile_downloader: TileDownloader::new(storage_path.clone()),
            storage_path,
            elevation_cache: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(500_000).unwrap(),
            ))),
            dataset_cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap()))),
        })
    }

    /// Create a new ElevationDB with an explicit storage path
    pub fn with_path(storage_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&storage_path).with_context(|| {
            format!(
                "Failed to create elevation storage directory: {:?}",
                storage_path
            )
        })?;
        Ok(Self {
            tile_downloader: TileDownloader::new(storage_path.clone()),
            storage_path,
            elevation_cache: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(500_000).unwrap(),
            ))),
            dataset_cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap()))),
        })
    }

    /// Get the storage path for this ElevationDB
    pub fn storage_path(&self) -> &PathBuf {
        &self.storage_path
    }

    /// Returns elevation in meters relative to EGM2008 (orthometric).
    pub async fn elevation_egm2008(&self, lat: f64, lon: f64) -> Result<Option<f64>> {
        let start = Instant::now();

        // Ocean tiles don't exist. You can choose to return 0.0 or None there.
        if !lat.is_finite() || !lon.is_finite() {
            bail!("bad coord");
        }

        // Create cache key by rounding coordinates to ~100m grid
        let cache_key = (round_coord_for_cache(lat), round_coord_for_cache(lon));

        // Check elevation cache first (fastest path)
        {
            let mut cache = self.elevation_cache.lock().await;
            if let Some(cached_elevation) = cache.get(&cache_key) {
                counter!("elevation_cache_hits").increment(1);

                // Copy the elevation value before releasing the cache lock
                let elevation = *cached_elevation;

                // Update cache size metric (each entry ≈ 56 bytes: 8 for key + 16 for value + 32 LRU overhead)
                let size_mb = (cache.len() * 56) as f64 / 1_048_576.0;
                gauge!("elevation_cache_size_mb").set(size_mb);

                histogram!("elevation_lookup_duration_seconds")
                    .record(start.elapsed().as_secs_f64());
                return Ok(elevation);
            }
        }

        // Elevation cache miss - need to look up from GDAL dataset
        counter!("elevation_cache_misses").increment(1);

        // Try GLO-30 first, automatically fall back to GLO-90 if unavailable
        let path = self
            .tile_downloader
            .ensure_cached_with_fallback(lat, lon)
            .await?;

        // Check if we have this dataset cached, or load it
        let dataset_open_start = Instant::now();
        let cached_ds = {
            let mut ds_cache = self.dataset_cache.lock().await;

            if let Some(cached) = ds_cache.get(&path) {
                counter!("elevation_dataset_cache_hits").increment(1);
                cached.clone()
            } else {
                // Dataset cache miss - need to open the file
                counter!("elevation_dataset_cache_misses").increment(1);

                let ds = Dataset::open(&path)?;
                let cached = Arc::new(CachedDataset::new(ds)?);
                ds_cache.put(path.clone(), cached.clone());

                // Record dataset open time
                histogram!("elevation_dataset_open_duration_seconds")
                    .record(dataset_open_start.elapsed().as_secs_f64());

                // Update dataset cache size metric
                gauge!("elevation_dataset_cache_size").set(ds_cache.len() as f64);

                cached
            }
        }; // Release dataset cache lock

        // Perform GDAL read operation using the cached dataset
        let gdal_read_start = Instant::now();
        let elevation = {
            let gt = &cached_ds.geo_transform;
            let px = (lon - gt[0]) / gt[1];
            let py = (lat - gt[3]) / gt[5]; // gt[5] is negative, so this works

            // Clamp pixel coordinates to ensure we can read a 2x2 window for bilinear interpolation
            // For a raster of size (width, height), valid starting positions for a 2x2 window
            // are (0 to width-2, 0 to height-2) to avoid reading out of bounds
            let px_clamped = px.floor().max(0.0).min((cached_ds.raster_width - 2) as f64) as isize;
            let py_clamped = py
                .floor()
                .max(0.0)
                .min((cached_ds.raster_height - 2) as f64) as isize;

            // Lock the dataset for reading (GDAL is not thread-safe)
            let dataset = cached_ds.dataset.lock().await;
            let band = dataset.rasterband(1)?;

            // Bilinear resample from a 2x2 window into a single value
            let out = band.read_as::<f64>(
                (px_clamped, py_clamped),
                (2, 2),
                (1, 1),
                Some(ResampleAlg::Bilinear),
            )?;

            // Handle NoData (Copernicus uses -32767 for nodata)
            if let Some(nd) = cached_ds.no_data_value
                && out[(0, 0)] == nd
            {
                None
            } else {
                Some(out[(0, 0)])
            }
        };

        // Record GDAL read time
        histogram!("elevation_gdal_read_duration_seconds")
            .record(gdal_read_start.elapsed().as_secs_f64());

        // Store in elevation cache for future lookups
        {
            let mut cache = self.elevation_cache.lock().await;
            cache.put(cache_key, elevation);

            // Update cache size metric (each entry ≈ 56 bytes: 8 for key + 16 for value + 32 LRU overhead)
            let size_mb = (cache.len() * 56) as f64 / 1_048_576.0;
            gauge!("elevation_cache_size_mb").set(size_mb);
        }

        // Record total elevation lookup duration
        histogram!("elevation_lookup_duration_seconds").record(start.elapsed().as_secs_f64());

        Ok(elevation)
    }
}
