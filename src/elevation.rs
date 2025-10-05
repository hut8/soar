use anyhow::{Context, Result, bail};
use directories::BaseDirs;
use gdal::{Dataset, raster::ResampleAlg};
use metrics::histogram;
use std::{env, path::PathBuf, time::Instant};

use crate::tile_downloader::TileDownloader;

/// Database for elevation data using Copernicus DEM tiles
#[derive(Clone)]
pub struct ElevationDB {
    storage_path: PathBuf,
    /// Manages tile downloads with deduplication
    tile_downloader: TileDownloader,
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

        // Try GLO-30 first, automatically fall back to GLO-90 if unavailable
        let path = self
            .tile_downloader
            .ensure_cached_with_fallback(lat, lon)
            .await?;

        let ds = Dataset::open(&path)?;
        let band = ds.rasterband(1)?;
        let (raster_width, raster_height) = band.size();

        // GeoTransform: [origin_x, pixel_w, 0, origin_y, 0, pixel_h(negative)]
        let gt = ds.geo_transform()?;
        let px = (lon - gt[0]) / gt[1];
        let py = (lat - gt[3]) / gt[5]; // gt[5] is negative, so this works

        // Clamp pixel coordinates to ensure we can read a 2x2 window for bilinear interpolation
        // For a raster of size (width, height), valid starting positions for a 2x2 window
        // are (0 to width-2, 0 to height-2) to avoid reading out of bounds
        let px_clamped = px.floor().max(0.0).min((raster_width - 2) as f64) as isize;
        let py_clamped = py.floor().max(0.0).min((raster_height - 2) as f64) as isize;

        // Bilinear resample from a 2x2 window into a single value
        let out = band.read_as::<f64>(
            (px_clamped, py_clamped),
            (2, 2),
            (1, 1),
            Some(ResampleAlg::Bilinear),
        )?;

        // Handle NoData (Copernicus uses -32767 for nodata)
        let result = if let Some(nd) = band.no_data_value()
            && out[(0, 0)] == nd
        {
            Ok(None)
        } else {
            Ok(Some(out[(0, 0)]))
        };

        // Record metric for elevation lookup duration
        histogram!("elevation_lookup_duration_seconds").record(start.elapsed().as_secs_f64());

        result
    }
}
