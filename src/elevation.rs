use anyhow::{Context, Result, bail};
use directories::BaseDirs;
use gdal::{Dataset, raster::ResampleAlg};
use std::{env, fs, path::PathBuf};
use tracing::debug;

/// Database for elevation data using Copernicus DEM tiles
#[derive(Clone)]
pub struct ElevationDB {
    storage_path: PathBuf,
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

        fs::create_dir_all(&storage_path).with_context(|| {
            format!(
                "Failed to create elevation storage directory: {:?}",
                storage_path
            )
        })?;

        Ok(Self { storage_path })
    }

    /// Create a new ElevationDB with an explicit storage path
    pub fn with_path(storage_path: PathBuf) -> Result<Self> {
        fs::create_dir_all(&storage_path).with_context(|| {
            format!(
                "Failed to create elevation storage directory: {:?}",
                storage_path
            )
        })?;
        Ok(Self { storage_path })
    }

    /// Get the storage path for this ElevationDB
    pub fn storage_path(&self) -> &PathBuf {
        &self.storage_path
    }

    /// Get the cache path for a specific tile
    fn cache_path(&self, name: &str) -> PathBuf {
        self.storage_path.join(format!("{name}.tif"))
    }

    /// Ensure a tile is cached locally, downloading if necessary
    async fn ensure_tile_cached(&self, name: &str) -> Result<PathBuf> {
        let path = self.cache_path(name);
        if !path.exists() {
            let url = tile_url_glo30(name);
            debug!("downloading elevation tile from {url}");
            let bytes = reqwest::get(&url)
                .await
                .and_then(|r| r.error_for_status())
                .with_context(|| format!("GET {url}"))?
                .bytes()
                .await
                .with_context(|| format!("read body {url}"))?;
            fs::write(&path, &bytes).with_context(|| format!("write {:?}", path))?;
        }
        Ok(path)
    }

    /// Returns elevation in meters relative to EGM2008 (orthometric).
    pub async fn elevation_egm2008(&self, lat: f64, lon: f64) -> Result<Option<f64>> {
        // Ocean tiles don't exist. You can choose to return 0.0 or None there.
        if !lat.is_finite() || !lon.is_finite() {
            bail!("bad coord");
        }

        let name = tile_name(lat, lon);
        let path = match self.ensure_tile_cached(&name).await {
            Ok(p) => p,
            // If a 30m tile is not public (rare), you could fall back to 90m here by
            // building the 90m URL (resolution "30" in the name) and retrying.
            Err(e) => bail!("missing GLO-30 tile (consider fallback to GLO-90/NASADEM): {e}"),
        };

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
        if let Some(nd) = band.no_data_value()
            && out[(0, 0)] == nd
        {
            return Ok(None);
        }
        Ok(Some(out[(0, 0)]))
    }
}

fn tile_name(lat: f64, lon: f64) -> String {
    let ns = if lat >= 0.0 { 'N' } else { 'S' };
    let ew = if lon >= 0.0 { 'E' } else { 'W' };
    let latd = lat.floor().abs() as i32;
    let lond = lon.floor().abs() as i32;
    // Copernicus naming uses 2-digit lat, 3-digit lon, and "_00" minutes.
    format!(
        "Copernicus_DSM_COG_10_{}{:02}_00_{}{:03}_00_DEM",
        ns, latd, ew, lond
    )
}

fn tile_url_glo30(name: &str) -> String {
    format!(
        "https://copernicus-dem-30m.s3.amazonaws.com/{0}/{0}.tif",
        name
    )
}
