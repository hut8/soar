use anyhow::{Context, Result, bail};
use directories::BaseDirs;
use gdal::{Dataset, raster::ResampleAlg};
use std::{fs, path::PathBuf};

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

fn cache_path(name: &str) -> Result<PathBuf> {
    let base = BaseDirs::new().context("no home directory")?;
    let dir = base
        .cache_dir()
        .join("elevation")
        .join("copernicus-dem-30m");
    fs::create_dir_all(&dir)?;
    Ok(dir.join(format!("{name}.tif")))
}

fn ensure_tile_cached(name: &str) -> Result<PathBuf> {
    let path = cache_path(name)?;
    if !path.exists() {
        let url = tile_url_glo30(name);
        let bytes = reqwest::blocking::get(&url)
            .and_then(|r| r.error_for_status())
            .with_context(|| format!("GET {url}"))?
            .bytes()?;
        fs::write(&path, &bytes).with_context(|| format!("write {:?}", path))?;
    }
    Ok(path)
}

/// Returns elevation in meters relative to EGM2008 (orthometric).
pub fn elevation_egm2008(lat: f64, lon: f64) -> Result<Option<f64>> {
    // Ocean tiles don't exist. You can choose to return 0.0 or None there.
    if !lat.is_finite() || !lon.is_finite() {
        bail!("bad coord");
    }

    let name = tile_name(lat, lon);
    let path = match ensure_tile_cached(&name) {
        Ok(p) => p,
        // If a 30m tile is not public (rare), you could fall back to 90m here by
        // building the 90m URL (resolution "30" in the name) and retrying.
        Err(e) => bail!("missing GLO-30 tile (consider fallback to GLO-90/NASADEM): {e}"),
    };

    let ds = Dataset::open(&path)?;
    let band = ds.rasterband(1)?;

    // GeoTransform: [origin_x, pixel_w, 0, origin_y, 0, pixel_h(negative)]
    let gt = ds.geo_transform()?;
    let px = (lon - gt[0]) / gt[1];
    let py = (lat - gt[3]) / gt[5]; // gt[5] is negative, so this works

    // Bilinear resample from a 1x1 window into a single value
    let out = band.read_as::<f64>(
        (px.floor() as isize, py.floor() as isize),
        (2, 2),
        (1, 1),
        Some(ResampleAlg::Bilinear),
    )?;

    // Handle NoData (Copernicus uses -32767 for nodata)
    if let Some(nd) = band.no_data_value() {
        if out[(0, 0)] == nd {
            return Ok(None);
        }
    }
    Ok(Some(out[(0, 0)]))
}
