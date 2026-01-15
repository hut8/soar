/// World Magnetic Model (WMM) integration
/// Uses the world_magnetic_model crate which implements NOAA's WMM
/// https://www.ngdc.noaa.gov/geomag/WMM/
///
/// The WMM is a geomagnetic field model that describes the Earth's magnetic field.
/// It is updated every 5 years and is used by navigation systems worldwide.

use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Utc};
use tracing::warn;
use world_magnetic_model::{time::Date as WmmDate, uom::si::angle::degree, uom::si::f32::*, uom::si::length::meter, GeomagneticField};

/// Calculate magnetic declination (variance) at a given location and time
///
/// # Arguments
/// * `latitude` - Latitude in degrees (-90 to +90)
/// * `longitude` - Longitude in degrees (-180 to +180)
/// * `altitude_meters` - Altitude above sea level in meters
/// * `timestamp` - Time for the calculation
///
/// # Returns
/// Magnetic declination in degrees (positive = east, negative = west)
///
/// # Errors
/// Returns an error if the geomagnetic field calculation fails (e.g., invalid coordinates or near poles)
pub fn calculate_declination(
    latitude: f64,
    longitude: f64,
    altitude_meters: f64,
    timestamp: DateTime<Utc>,
) -> Result<f64> {
    // Convert to WMM date
    let year = timestamp.year();
    let day_of_year = timestamp.ordinal();
    let wmm_date = WmmDate::from_ordinal_date(year, day_of_year as u16)
        .unwrap_or_else(|e| {
            warn!(
                "Failed to convert date {} (day {}) to WMM date: {:?}, using fallback date 2025-01-01",
                year, day_of_year, e
            );
            WmmDate::from_ordinal_date(2025, 1).unwrap()
        });

    // Create geomagnetic field calculation
    let field = GeomagneticField::new(
        Length::new::<meter>(altitude_meters as f32),
        Angle::new::<degree>(latitude as f32),
        Angle::new::<degree>(longitude as f32),
        wmm_date,
    )
    .with_context(|| format!(
        "Failed to calculate geomagnetic field at lat={}, lon={}, alt={}m",
        latitude, longitude, altitude_meters
    ))?;

    // Return declination in degrees
    Ok(field.declination().get::<degree>() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_declination_known_locations() {
        // Test a few known locations
        // Note: These are approximate values for validation
        let timestamp = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();

        // San Francisco, CA (known for eastward declination ~13°E)
        let dec = calculate_declination(37.77, -122.42, 0.0, timestamp).unwrap();
        assert!(
            dec > 10.0 && dec < 16.0,
            "SF declination should be ~13°E, got {}",
            dec
        );

        // New York, NY (known for westward declination ~12°W)
        let dec = calculate_declination(40.71, -74.01, 0.0, timestamp).unwrap();
        assert!(
            dec < -10.0 && dec > -15.0,
            "NYC declination should be ~12°W, got {}",
            dec
        );

        // London, UK (near-zero declination)
        let dec = calculate_declination(51.51, -0.13, 0.0, timestamp).unwrap();
        assert!(
            dec.abs() < 5.0,
            "London declination should be near 0°, got {}",
            dec
        );
    }

    #[test]
    fn test_coordinate_bounds() {
        // Test that function works at coordinate extremes
        let timestamp = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();

        let dec = calculate_declination(89.0, 0.0, 0.0, timestamp).unwrap();
        assert!(dec.is_finite());

        let dec = calculate_declination(-89.0, 179.0, 0.0, timestamp).unwrap();
        assert!(dec.is_finite());
    }

    #[test]
    fn test_altitude_effect() {
        // Magnetic field weakens with altitude, but declination angle
        // should not change dramatically
        let timestamp = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();

        let dec_sea = calculate_declination(40.0, -100.0, 0.0, timestamp).unwrap();
        let dec_high = calculate_declination(40.0, -100.0, 10000.0, timestamp).unwrap();

        // Difference should be small (less than 1 degree)
        assert!((dec_sea - dec_high).abs() < 1.0);
    }
}
