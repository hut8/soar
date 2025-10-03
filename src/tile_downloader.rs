use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tracing::debug;

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
    fn tile_path(&self, tile_name: &str) -> PathBuf {
        self.storage_path.join(format!("{}.tif", tile_name))
    }

    /// Ensure a tile is cached locally, downloading if necessary
    /// If multiple concurrent requests ask for the same tile, only one download occurs
    /// and the others wait for it to complete
    pub async fn ensure_cached(&self, tile_name: &str) -> Result<PathBuf> {
        let path = self.tile_path(tile_name);

        // Fast path: tile already cached
        if path.exists() {
            return Ok(path);
        }

        // Check if someone else is already downloading this tile
        let (should_download, notify) = {
            let mut map = self.in_progress.lock().await;
            if let Some(existing_notify) = map.get(tile_name) {
                // Someone else is downloading, we'll wait
                (false, existing_notify.clone())
            } else {
                // We are first, we'll download
                let notify = Arc::new(Notify::new());
                map.insert(tile_name.to_string(), notify.clone());
                (true, notify)
            }
        }; // Lock released here

        if should_download {
            // We are responsible for downloading
            let url = tile_url_glo30(tile_name);
            let result = self.download_tile(&path, &url).await;

            // Notify all waiters (whether success or failure)
            notify.notify_waiters();

            // Clean up the in-progress entry
            self.in_progress.lock().await.remove(tile_name);

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

    /// Download a tile from the given URL to the specified path
    async fn download_tile(&self, path: &Path, url: &str) -> Result<()> {
        debug!("downloading elevation tile from {url}");

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

/// Generate the URL for a Copernicus GLO-30 tile
fn tile_url_glo30(name: &str) -> String {
    format!(
        "https://copernicus-dem-30m.s3.amazonaws.com/{0}/{0}.tif",
        name
    )
}
