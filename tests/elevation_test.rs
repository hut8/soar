use soar::elevation::elevation_egm2008;

/// Test elevation lookup for a known location
/// Using Mount Everest base camp in Nepal as a test location
/// Expected elevation: ~5,364m (17,598 ft)
#[test]
fn test_elevation_mount_everest_region() {
    // Coordinates near Mount Everest Base Camp (Nepal side)
    // Using coordinates that are safer (not at tile boundaries)
    let lat = 28.5;
    let lon = 86.5;

    let result = elevation_egm2008(lat, lon);
    assert!(result.is_ok(), "Elevation lookup should succeed");

    let elevation = result.unwrap();
    assert!(
        elevation.is_some(),
        "Elevation should be available for land"
    );

    let elev_m = elevation.unwrap();
    // The exact elevation depends on the precise coordinates and DEM resolution
    // Mount Everest region should have elevations above 3000m
    assert!(
        elev_m > 3000.0 && elev_m < 9000.0,
        "Elevation {} meters is outside expected range for Everest region",
        elev_m
    );
}

/// Test elevation lookup for a lower elevation location
/// Using Denver, Colorado (known as the "Mile High City")
/// Expected elevation: ~1,600m (5,280 ft)
#[test]
fn test_elevation_denver() {
    // Denver, Colorado coordinates
    let lat = 39.7392;
    let lon = -104.9903;

    let result = elevation_egm2008(lat, lon);
    assert!(result.is_ok(), "Elevation lookup should succeed");

    let elevation = result.unwrap();
    assert!(
        elevation.is_some(),
        "Elevation should be available for Denver"
    );

    let elev_m = elevation.unwrap();
    // Denver is around 1,600m elevation
    assert!(
        elev_m > 1400.0 && elev_m < 1800.0,
        "Elevation {} meters is outside expected range for Denver (~1,600m)",
        elev_m
    );
}

/// Test elevation lookup for sea level location
/// Using New York City (near sea level)
/// Expected elevation: close to 0m
#[test]
fn test_elevation_sea_level() {
    // New York City coordinates (Central Park)
    let lat = 40.7829;
    let lon = -73.9654;

    let result = elevation_egm2008(lat, lon);
    assert!(result.is_ok(), "Elevation lookup should succeed");

    let elevation = result.unwrap();
    assert!(
        elevation.is_some(),
        "Elevation should be available for NYC land area"
    );

    let elev_m = elevation.unwrap();
    // NYC is near sea level, should be between -10m and 100m
    assert!(
        elev_m > -10.0 && elev_m < 100.0,
        "Elevation {} meters is outside expected range for NYC sea level area",
        elev_m
    );
}

/// Test error handling for invalid latitude
/// Note: The current implementation only checks if coordinates are finite,
/// not if they're in valid range. Invalid coordinates will fail when fetching the tile.
#[test]
fn test_invalid_latitude() {
    let result = elevation_egm2008(91.0, 0.0); // Latitude > 90
    assert!(
        result.is_err(),
        "Should fail for latitude > 90 (either at validation or tile fetch)"
    );
    // Will fail with "missing GLO-30 tile" error, not "bad coord"
}

/// Test error handling for invalid longitude
/// Note: The current implementation only checks if coordinates are finite,
/// not if they're in valid range. Invalid coordinates will fail when fetching the tile.
#[test]
fn test_invalid_longitude() {
    let result = elevation_egm2008(0.0, 181.0); // Longitude > 180
    assert!(
        result.is_err(),
        "Should fail for longitude > 180 (either at validation or tile fetch)"
    );
    // Will fail with "missing GLO-30 tile" error, not "bad coord"
}

/// Test error handling for NaN coordinates
#[test]
fn test_nan_coordinates() {
    let result = elevation_egm2008(f64::NAN, 0.0);
    assert!(result.is_err(), "Should fail for NaN latitude");
    assert!(result.unwrap_err().to_string().contains("bad coord"));

    let result = elevation_egm2008(0.0, f64::NAN);
    assert!(result.is_err(), "Should fail for NaN longitude");
    assert!(result.unwrap_err().to_string().contains("bad coord"));
}

/// Test error handling for infinite coordinates
#[test]
fn test_infinite_coordinates() {
    let result = elevation_egm2008(f64::INFINITY, 0.0);
    assert!(result.is_err(), "Should fail for infinite latitude");
    assert!(result.unwrap_err().to_string().contains("bad coord"));

    let result = elevation_egm2008(0.0, f64::INFINITY);
    assert!(result.is_err(), "Should fail for infinite longitude");
    assert!(result.unwrap_err().to_string().contains("bad coord"));
}

/// Test elevation lookup for ocean location
/// Ocean tiles may not exist or return NoData
#[test]
fn test_elevation_ocean() {
    // Middle of Pacific Ocean
    let lat = 0.0;
    let lon = -160.0;

    let result = elevation_egm2008(lat, lon);
    // Ocean tiles might not exist or might return None for NoData
    // Either case is acceptable
    if let Ok(Some(elev_m)) = result {
        // If tile exists and has elevation data, it should be close to sea level
        assert!(
            elev_m > -1000.0 && elev_m < 100.0,
            "Ocean elevation {} is outside expected range",
            elev_m
        );
    }
    // If tile doesn't exist (Err) or returns None, that's acceptable for ocean areas
}

/// Test that repeated lookups use cache
/// This test verifies that the caching mechanism works by calling the same location twice
#[test]
fn test_elevation_caching() {
    let lat = 40.7829;
    let lon = -73.9654;

    // First call - will download and cache tile
    let result1 = elevation_egm2008(lat, lon);
    assert!(result1.is_ok(), "First elevation lookup should succeed");

    // Second call - should use cached tile
    let result2 = elevation_egm2008(lat, lon);
    assert!(result2.is_ok(), "Second elevation lookup should succeed");

    // Both should return the same value
    assert_eq!(
        result1.unwrap(),
        result2.unwrap(),
        "Cached lookup should return same value"
    );
}

/// Test elevation for negative latitude (Southern Hemisphere)
#[test]
fn test_elevation_southern_hemisphere() {
    // Sydney, Australia
    let lat = -33.8688;
    let lon = 151.2093;

    let result = elevation_egm2008(lat, lon);
    assert!(result.is_ok(), "Elevation lookup should succeed");

    let elevation = result.unwrap();
    assert!(
        elevation.is_some(),
        "Elevation should be available for Sydney"
    );

    let elev_m = elevation.unwrap();
    // Sydney is near sea level
    assert!(
        elev_m > -10.0 && elev_m < 200.0,
        "Elevation {} meters is outside expected range for Sydney",
        elev_m
    );
}

/// Test elevation for boundary coordinates (near tile edges)
/// This tests coordinates exactly at tile boundaries (integer lat/lon)
#[test]
fn test_elevation_tile_boundary() {
    // Test at exactly 45°N, 0°E (boundary between tiles)
    let lat = 45.0;
    let lon = 0.0;

    let result = elevation_egm2008(lat, lon);
    assert!(
        result.is_ok(),
        "Elevation lookup at tile boundary should succeed: {:?}",
        result.err()
    );

    let elevation = result.unwrap();
    // Should have some elevation data for this land location (southern France)
    assert!(
        elevation.is_some(),
        "Should have elevation data at boundary"
    );
}

/// Test elevation near (but not exactly at) tile boundary
#[test]
fn test_elevation_near_tile_boundary() {
    // Test near 45°N, 0°E but not exactly at boundary
    let lat = 45.1;
    let lon = 0.1;

    let result = elevation_egm2008(lat, lon);
    assert!(
        result.is_ok(),
        "Elevation lookup near tile boundary should succeed: {:?}",
        result.err()
    );

    let elevation = result.unwrap();
    // Should have some elevation data for this land location (southern France)
    assert!(
        elevation.is_some(),
        "Should have elevation data near boundary"
    );
}

/// Test elevation for Death Valley (below sea level)
/// Expected elevation: ~-86m (-282 ft) at Badwater Basin
#[test]
fn test_elevation_death_valley() {
    // Death Valley, California (Badwater Basin - lowest point in North America)
    let lat = 36.2295;
    let lon = -116.8295;

    let result = elevation_egm2008(lat, lon);
    assert!(result.is_ok(), "Elevation lookup should succeed");

    let elevation = result.unwrap();
    assert!(
        elevation.is_some(),
        "Elevation should be available for Death Valley"
    );

    let elev_m = elevation.unwrap();
    // Death Valley is below sea level, around -86m
    assert!(
        elev_m > -150.0 && elev_m < 0.0,
        "Elevation {} meters is outside expected range for Death Valley (below sea level)",
        elev_m
    );
}

/// Test elevation for Grand Canyon
/// Expected elevation: ~2,100m (6,900 ft) at South Rim
#[test]
fn test_elevation_grand_canyon() {
    // Grand Canyon South Rim, Arizona
    let lat = 36.0544;
    let lon = -112.1401;

    let result = elevation_egm2008(lat, lon);
    assert!(result.is_ok(), "Elevation lookup should succeed");

    let elevation = result.unwrap();
    assert!(
        elevation.is_some(),
        "Elevation should be available for Grand Canyon"
    );

    let elev_m = elevation.unwrap();
    // Grand Canyon South Rim is around 2,100m elevation
    assert!(
        elev_m > 1900.0 && elev_m < 2300.0,
        "Elevation {} meters is outside expected range for Grand Canyon South Rim (~2,100m)",
        elev_m
    );
}

/// Test elevation for Mexico City (high elevation city)
/// Expected elevation: ~2,240m (7,350 ft)
#[test]
fn test_elevation_mexico_city() {
    // Mexico City, Mexico (Zócalo square)
    let lat = 19.4326;
    let lon = -99.1332;

    let result = elevation_egm2008(lat, lon);
    assert!(result.is_ok(), "Elevation lookup should succeed");

    let elevation = result.unwrap();
    assert!(
        elevation.is_some(),
        "Elevation should be available for Mexico City"
    );

    let elev_m = elevation.unwrap();
    // Mexico City elevation varies widely across the city, from ~2,200m to ~2,600m
    assert!(
        elev_m > 1800.0 && elev_m < 2700.0,
        "Elevation {} meters is outside expected range for Mexico City (~2,240m average)",
        elev_m
    );
}

/// Test elevation for Tokyo (sea level, different hemisphere from NYC)
/// Expected elevation: close to 0m
#[test]
fn test_elevation_tokyo() {
    // Tokyo, Japan (Tokyo Tower area)
    let lat = 35.6586;
    let lon = 139.7454;

    let result = elevation_egm2008(lat, lon);
    assert!(result.is_ok(), "Elevation lookup should succeed");

    let elevation = result.unwrap();
    assert!(
        elevation.is_some(),
        "Elevation should be available for Tokyo"
    );

    let elev_m = elevation.unwrap();
    // Tokyo is near sea level
    assert!(
        elev_m > -10.0 && elev_m < 100.0,
        "Elevation {} meters is outside expected range for Tokyo (near sea level)",
        elev_m
    );
}
