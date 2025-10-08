use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tracing::{debug, info};

/// Copernicus DEM resolution options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// GLO-30: 30m resolution (10 arc-seconds)
    Glo30,
    /// GLO-90: 90m resolution (30 arc-seconds)
    Glo90,
}

impl Resolution {
    /// Returns the resolution code used in tile names
    /// GLO-30 uses "10" (arc-seconds), GLO-90 uses "30"
    pub fn resolution_code(&self) -> &str {
        match self {
            Resolution::Glo30 => "10",
            Resolution::Glo90 => "30",
        }
    }

    /// Returns the S3 bucket name for this resolution
    pub fn bucket_name(&self) -> &str {
        match self {
            Resolution::Glo30 => "copernicus-dem-30m",
            Resolution::Glo90 => "copernicus-dem-90m",
        }
    }

    /// Returns the subdirectory name for storing tiles of this resolution
    pub fn subdirectory(&self) -> &str {
        match self {
            Resolution::Glo30 => "glo30",
            Resolution::Glo90 => "glo90",
        }
    }
}

/// Manages concurrent tile downloads with deduplication
/// Ensures only one download happens per tile even with concurrent requests
#[derive(Clone)]
pub struct TileDownloader {
    storage_path: PathBuf,
    /// Track in-progress downloads: tile_name -> Notify
    /// When a download completes, all waiters are notified
    in_progress: Arc<Mutex<HashMap<String, Arc<Notify>>>>,
}

impl TileDownloader {
    /// Create a new TileDownloader with the given storage directory
    pub fn new(storage_path: PathBuf) -> Self {
        Self {
            storage_path,
            in_progress: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get the full path where a tile should be cached
    /// Checks both shared cache (/var/soar/elevation) and instance-specific cache
    /// Returns the first existing path, or the instance-specific path for downloads
    fn tile_path(&self, tile_name: &str, resolution: Resolution) -> PathBuf {
        let shared_cache = PathBuf::from("/var/soar/elevation")
            .join(resolution.subdirectory())
            .join(format!("{}.tif", tile_name));

        // If tile exists in shared cache, use it
        if shared_cache.exists() {
            return shared_cache;
        }

        // Otherwise use instance-specific path (for checking and downloading)
        self.storage_path
            .join(resolution.subdirectory())
            .join(format!("{}.tif", tile_name))
    }

    /// Ensure a tile is cached locally, downloading if necessary
    /// If multiple concurrent requests ask for the same tile, only one download occurs
    /// and the others wait for it to complete
    pub async fn ensure_cached(
        &self,
        tile_name: &str,
        resolution: Resolution,
        lat: f64,
        lon: f64,
    ) -> Result<PathBuf> {
        let path = self.tile_path(tile_name, resolution);

        // Fast path: tile already cached
        if path.exists() {
            return Ok(path);
        }

        // Create a unique key for in-progress tracking (tile_name + resolution)
        let progress_key = format!("{}:{}", tile_name, resolution.subdirectory());

        // Check if someone else is already downloading this tile
        let (should_download, notify) = {
            let mut map = self.in_progress.lock().await;
            if let Some(existing_notify) = map.get(&progress_key) {
                // Someone else is downloading, we'll wait
                (false, existing_notify.clone())
            } else {
                // We are first, we'll download
                let notify = Arc::new(Notify::new());
                map.insert(progress_key.clone(), notify.clone());
                (true, notify)
            }
        }; // Lock released here

        if should_download {
            // We are responsible for downloading
            info!(
                "Downloading elevation tile {} for coordinates ({:.4}, {:.4})",
                tile_name, lat, lon
            );
            let url = tile_url(tile_name, resolution);

            // Ensure the subdirectory exists before downloading
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {:?}", parent))?;
            }

            let result = self.download_tile(&path, &url).await;

            // Notify all waiters (whether success or failure)
            notify.notify_waiters();

            // Clean up the in-progress entry
            self.in_progress.lock().await.remove(&progress_key);

            // Return the result
            result?;
            Ok(path)
        } else {
            // Wait for the downloader to finish
            notify.notified().await;

            // Double-check that the file now exists
            if path.exists() {
                Ok(path)
            } else {
                bail!("Tile download completed but file not found at {:?}", path)
            }
        }
    }

    /// Ensure a tile is cached, trying GLO-30 first, then falling back to GLO-90
    pub async fn ensure_cached_with_fallback(&self, lat: f64, lon: f64) -> Result<PathBuf> {
        let name_glo30 = tile_name(Resolution::Glo30, lat, lon);

        // Try GLO-30 first
        match self
            .ensure_cached(&name_glo30, Resolution::Glo30, lat, lon)
            .await
        {
            Ok(path) => Ok(path),
            Err(e) => {
                // GLO-30 failed, try GLO-90
                debug!("GLO-30 tile not available ({}), falling back to GLO-90", e);
                let name_glo90 = tile_name(Resolution::Glo90, lat, lon);
                self.ensure_cached(&name_glo90, Resolution::Glo90, lat, lon)
                    .await
                    .with_context(|| {
                        format!(
                            "Both GLO-30 and GLO-90 tiles unavailable for ({}, {})",
                            lat, lon
                        )
                    })
            }
        }
    }

    /// Download a tile from the given URL to the specified path
    async fn download_tile(&self, path: &Path, url: &str) -> Result<()> {
        info!("Downloading elevation tile from {url}");

        let bytes = reqwest::get(url)
            .await
            .and_then(|r| r.error_for_status())
            .with_context(|| format!("GET {url}"))?
            .bytes()
            .await
            .with_context(|| format!("read body {url}"))?;

        fs::write(path, &bytes).with_context(|| format!("write {:?}", path))?;

        Ok(())
    }
}

/// Generate tile name for given coordinates and resolution
pub fn tile_name(resolution: Resolution, lat: f64, lon: f64) -> String {
    let ns = if lat >= 0.0 { 'N' } else { 'S' };
    let ew = if lon >= 0.0 { 'E' } else { 'W' };
    let latd = lat.floor().abs() as i32;
    let lond = lon.floor().abs() as i32;
    // Copernicus naming uses 2-digit lat, 3-digit lon, and "_00" minutes.
    format!(
        "Copernicus_DSM_COG_{}_{}{:02}_00_{}{:03}_00_DEM",
        resolution.resolution_code(),
        ns,
        latd,
        ew,
        lond
    )
}

/// Generate the URL for a Copernicus DEM tile
fn tile_url(name: &str, resolution: Resolution) -> String {
    format!(
        "https://{}.s3.amazonaws.com/{}/{}.tif",
        resolution.bucket_name(),
        name,
        name
    )
}
