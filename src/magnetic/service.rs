use anyhow::Result;
use chrono::{DateTime, Datelike, Utc};
use metrics::{counter, gauge, histogram};
use moka::future::Cache;
use std::time::Instant;
use tracing::debug;

use super::wmm;

/// Round coordinates to ~10km grid using tenths of a degree (0.1° ≈ 11km at the equator).
/// This creates a cache key that groups nearby lookups together.
/// The `* 10.0` factor is intentional: we quantize to 0.1° resolution to balance cache hit rate
/// against spatial accuracy, and magnetic declination changes slowly enough that this is appropriate.
fn round_coord_for_cache(coord: f64) -> i32 {
    (coord * 10.0).round() as i32
}

/// Cache key for magnetic declination lookups: (lat_tenths, lon_tenths, year)
type CacheKey = (i32, i32, i32);

/// High-performance magnetic declination service with caching
///
/// This implementation is optimized for concurrent access with:
/// - Lock-free caching using moka for declination results
/// - WMM2025 (World Magnetic Model) for accurate magnetic declination
/// - Memory-efficient design that caches results for fast repeated lookups
#[derive(Clone)]
pub struct MagneticService {
    /// Concurrent cache for magnetic declination results
    /// Key: (rounded_lat, rounded_lon, year) -> declination_degrees
    /// 100,000 entries provides good coverage for typical flight operations
    /// Uses moka for lock-free concurrent access across multiple workers
    declination_cache: Cache<CacheKey, f64>,
}

impl MagneticService {
    /// Create a new MagneticService with default cache size
    pub fn new() -> Self {
        Self {
            declination_cache: Cache::builder().max_capacity(100_000).build(),
        }
    }

    /// Create a new MagneticService with custom cache size
    pub fn with_cache_size(cache_size: u64) -> Self {
        Self {
            declination_cache: Cache::builder().max_capacity(cache_size).build(),
        }
    }

    /// Get magnetic declination (variance) at a given location and time
    ///
    /// # Arguments
    /// * `latitude` - Latitude in degrees (-90 to +90)
    /// * `longitude` - Longitude in degrees (-180 to +180)
    /// * `altitude_meters` - Altitude above sea level in meters (typically 0-15000m for aviation)
    /// * `timestamp` - Time for the calculation (uses current time if None)
    ///
    /// # Returns
    /// Magnetic declination in degrees (positive = east, negative = west)
    ///
    /// # Example
    /// ```
    /// use soar::magnetic::MagneticService;
    /// use chrono::Utc;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let service = MagneticService::new();
    /// let declination = service.declination(37.77, -122.42, 0.0, None).await?;
    /// println!("Magnetic declination in SF: {:.2}°", declination);
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(skip(self), fields(lat = %latitude, lon = %longitude))]
    pub async fn declination(
        &self,
        latitude: f64,
        longitude: f64,
        altitude_meters: f64,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<f64> {
        let start = Instant::now();

        // Validate coordinates
        if !latitude.is_finite() || !longitude.is_finite() || !altitude_meters.is_finite() {
            anyhow::bail!("Invalid coordinates or altitude");
        }

        if !(-90.0..=90.0).contains(&latitude) || !(-180.0..=180.0).contains(&longitude) {
            anyhow::bail!(
                "Coordinates out of range: lat={}, lon={}",
                latitude,
                longitude
            );
        }

        // Get timestamp (use current time if not provided)
        let time = timestamp.unwrap_or_else(Utc::now);
        let year = time.year();

        // Create cache key by rounding coordinates and year
        let cache_key = (
            round_coord_for_cache(latitude),
            round_coord_for_cache(longitude),
            year,
        );

        // Check cache first (fastest path)
        if let Some(cached_declination) = self.declination_cache.get(&cache_key).await {
            counter!("magnetic_cache_hits_total").increment(1);
            gauge!("magnetic_cache_entries").set(self.declination_cache.entry_count() as f64);
            histogram!("magnetic_lookup_duration_seconds").record(start.elapsed().as_secs_f64());
            return Ok(cached_declination);
        }

        // Cache miss - need to calculate
        counter!("magnetic_cache_misses_total").increment(1);

        // Calculate magnetic declination using WMM
        let calculation_start = Instant::now();
        let declination = wmm::calculate_declination(latitude, longitude, altitude_meters, time)?;

        // Record calculation time
        histogram!("magnetic_calculation_duration_seconds")
            .record(calculation_start.elapsed().as_secs_f64());

        debug!(
            "Calculated magnetic declination at ({:.3}, {:.3}): {:.2}°",
            latitude, longitude, declination
        );

        // Store in cache for future lookups
        self.declination_cache.insert(cache_key, declination).await;
        gauge!("magnetic_cache_entries").set(self.declination_cache.entry_count() as f64);

        // Record total lookup duration
        histogram!("magnetic_lookup_duration_seconds").record(start.elapsed().as_secs_f64());

        Ok(declination)
    }

    /// Convert true heading to magnetic heading at a given location
    ///
    /// # Arguments
    /// * `true_heading` - True heading in degrees (0-360)
    /// * `latitude` - Latitude in degrees
    /// * `longitude` - Longitude in degrees
    /// * `altitude_meters` - Altitude above sea level in meters
    /// * `timestamp` - Time for the calculation (uses current time if None)
    ///
    /// # Returns
    /// Magnetic heading in degrees (0-360)
    ///
    /// # Example
    /// ```
    /// use soar::magnetic::MagneticService;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let service = MagneticService::new();
    /// // True heading of 90° (east)
    /// let mag_heading = service.true_to_magnetic(90.0, 37.77, -122.42, 0.0, None).await?;
    /// println!("Magnetic heading: {:.0}°", mag_heading);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn true_to_magnetic(
        &self,
        true_heading: f64,
        latitude: f64,
        longitude: f64,
        altitude_meters: f64,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<f64> {
        let declination = self
            .declination(latitude, longitude, altitude_meters, timestamp)
            .await?;

        // Magnetic heading = True heading - Declination
        // (eastward declination means magnetic north is east of true north)
        let mag_heading = true_heading - declination;

        // Normalize to 0-360 range
        let normalized = if mag_heading < 0.0 {
            mag_heading + 360.0
        } else if mag_heading >= 360.0 {
            mag_heading - 360.0
        } else {
            mag_heading
        };

        Ok(normalized)
    }

    /// Convert magnetic heading to true heading at a given location
    ///
    /// # Arguments
    /// * `magnetic_heading` - Magnetic heading in degrees (0-360)
    /// * `latitude` - Latitude in degrees
    /// * `longitude` - Longitude in degrees
    /// * `altitude_meters` - Altitude above sea level in meters
    /// * `timestamp` - Time for the calculation (uses current time if None)
    ///
    /// # Returns
    /// True heading in degrees (0-360)
    pub async fn magnetic_to_true(
        &self,
        magnetic_heading: f64,
        latitude: f64,
        longitude: f64,
        altitude_meters: f64,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<f64> {
        let declination = self
            .declination(latitude, longitude, altitude_meters, timestamp)
            .await?;

        // True heading = Magnetic heading + Declination
        let true_heading = magnetic_heading + declination;

        // Normalize to 0-360 range
        let normalized = if true_heading < 0.0 {
            true_heading + 360.0
        } else if true_heading >= 360.0 {
            true_heading - 360.0
        } else {
            true_heading
        };

        Ok(normalized)
    }
}

impl Default for MagneticService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_declination() {
        let service = MagneticService::new();

        // Test San Francisco (should have eastward declination ~13°E)
        let dec = service
            .declination(37.77, -122.42, 0.0, None)
            .await
            .unwrap();
        assert!(
            dec > 10.0 && dec < 16.0,
            "SF declination should be ~13°E, got {}",
            dec
        );

        // Test New York (should have westward declination ~12°W)
        let dec = service.declination(40.71, -74.01, 0.0, None).await.unwrap();
        assert!(
            dec < -10.0 && dec > -15.0,
            "NYC declination should be ~12°W, got {}",
            dec
        );
    }

    #[tokio::test]
    async fn test_true_to_magnetic() {
        let service = MagneticService::new();

        // In SF with ~13°E declination:
        // True heading 90° (east) -> Magnetic heading ~77° (90 - 13)
        let mag_heading = service
            .true_to_magnetic(90.0, 37.77, -122.42, 0.0, None)
            .await
            .unwrap();
        assert!(
            mag_heading > 70.0 && mag_heading < 85.0,
            "Expected ~77°, got {}",
            mag_heading
        );
    }

    #[tokio::test]
    async fn test_magnetic_to_true() {
        let service = MagneticService::new();

        // In SF with ~13°E declination:
        // Magnetic heading 77° -> True heading ~90° (77 + 13)
        let true_heading = service
            .magnetic_to_true(77.0, 37.77, -122.42, 0.0, None)
            .await
            .unwrap();
        assert!(
            true_heading > 85.0 && true_heading < 95.0,
            "Expected ~90°, got {}",
            true_heading
        );
    }

    #[tokio::test]
    async fn test_caching() {
        let service = MagneticService::new();

        // First call - cache miss
        let dec1 = service.declination(40.0, -100.0, 0.0, None).await.unwrap();

        // Second call with similar coordinates (within rounding threshold) - should hit cache
        let dec2 = service
            .declination(40.05, -100.05, 0.0, None)
            .await
            .unwrap();

        // Results should be close (same cached value)
        assert!(
            (dec1 - dec2).abs() < 0.1,
            "Cache should return same value for nearby coordinates: {} vs {}",
            dec1,
            dec2
        );
    }

    #[tokio::test]
    async fn test_heading_normalization() {
        let service = MagneticService::new();

        // Test wraparound at 360°
        // If true heading is 5° and declination is +10°E, magnetic heading should be 355° (5 - 10 + 360)
        let mag_heading = service
            .true_to_magnetic(5.0, 37.77, -122.42, 0.0, None)
            .await
            .unwrap();
        assert!(
            (0.0..360.0).contains(&mag_heading),
            "Heading should be normalized to 0-360"
        );
    }

    #[tokio::test]
    async fn test_invalid_coordinates() {
        let service = MagneticService::new();

        // Test invalid latitude
        let result = service.declination(100.0, 0.0, 0.0, None).await;
        assert!(result.is_err(), "Should reject invalid latitude");

        // Test invalid longitude
        let result = service.declination(0.0, 200.0, 0.0, None).await;
        assert!(result.is_err(), "Should reject invalid longitude");

        // Test NaN
        let result = service.declination(f64::NAN, 0.0, 0.0, None).await;
        assert!(result.is_err(), "Should reject NaN coordinates");
    }
}
