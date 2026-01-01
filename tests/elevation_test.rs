use soar::elevation::ElevationDB;
use std::path::PathBuf;

/// Helper function to create ElevationDB for tests
/// Uses test data path if available, otherwise falls back to system path
async fn create_test_elevation_db() -> ElevationDB {
    let test_data_path = PathBuf::from("tests/data/elevation");
    if test_data_path.exists() {
        ElevationDB::with_path(test_data_path)
            .await
            .expect("Failed to initialize ElevationDB with test data")
    } else {
        ElevationDB::new().expect("Failed to initialize ElevationDB")
    }
}

/// Test elevation lookup for a lower elevation location
/// Using Denver, Colorado (known as the "Mile High City")
/// Expected elevation: ~1,600m (5,280 ft)
/// This test uses a single tile stored in test data and will run in CI
#[tokio::test]
async fn test_elevation_denver() {
    // Denver, Colorado coordinates
    let lat = 39.7392;
    let lon = -104.9903;

    let elevation_db = create_test_elevation_db().await;

    let result = elevation_db.elevation_egm2008(lat, lon).await;
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
#[tokio::test]
async fn test_elevation_sea_level() {
    // New York City coordinates (Central Park)
    let lat = 40.7829;
    let lon = -73.9654;

    let elevation_db = create_test_elevation_db().await;
    let result = elevation_db.elevation_egm2008(lat, lon).await;
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
#[tokio::test]
async fn test_invalid_latitude() {
    let elevation_db = create_test_elevation_db().await;
    let result = elevation_db.elevation_egm2008(91.0, 0.0).await; // Latitude > 90
    assert!(
        result.is_err(),
        "Should fail for latitude > 90 (either at validation or tile fetch)"
    );
}

/// Test error handling for invalid longitude
/// Note: The current implementation only checks if coordinates are finite,
/// not if they're in valid range. Invalid coordinates will fail when fetching the tile.
#[tokio::test]
async fn test_invalid_longitude() {
    let elevation_db = create_test_elevation_db().await;
    let result = elevation_db.elevation_egm2008(0.0, 181.0).await; // Longitude > 180
    assert!(
        result.is_err(),
        "Should fail for longitude > 180 (either at validation or tile fetch)"
    );
}

/// Test error handling for NaN coordinates
#[tokio::test]
async fn test_nan_coordinates() {
    let elevation_db = create_test_elevation_db().await;
    let result = elevation_db.elevation_egm2008(f64::NAN, 0.0).await;
    assert!(result.is_err(), "Should fail for NaN latitude");
    assert!(result.unwrap_err().to_string().contains("bad coord"));

    let result = elevation_db.elevation_egm2008(0.0, f64::NAN).await;
    assert!(result.is_err(), "Should fail for NaN longitude");
    assert!(result.unwrap_err().to_string().contains("bad coord"));
}

/// Test error handling for infinite coordinates
#[tokio::test]
async fn test_infinite_coordinates() {
    let elevation_db = create_test_elevation_db().await;
    let result = elevation_db.elevation_egm2008(f64::INFINITY, 0.0).await;
    assert!(result.is_err(), "Should fail for infinite latitude");
    assert!(result.unwrap_err().to_string().contains("bad coord"));

    let result = elevation_db.elevation_egm2008(0.0, f64::INFINITY).await;
    assert!(result.is_err(), "Should fail for infinite longitude");
    assert!(result.unwrap_err().to_string().contains("bad coord"));
}

/// Test elevation lookup for ocean location
/// Ocean tiles may not exist or return NoData
#[tokio::test]
async fn test_elevation_ocean() {
    // Middle of Pacific Ocean
    let lat = 0.0;
    let lon = -160.0;

    let elevation_db = create_test_elevation_db().await;
    let result = elevation_db.elevation_egm2008(lat, lon).await;
    // Ocean tiles might not exist or might return None for NoData
    // Either case is acceptable
    if let Ok(Some(elev_m)) = result {
        // If tile exists and has elevation data, ocean depths can be quite deep
        // Mariana Trench is ~-11,000m, but most ocean is shallower
        assert!(
            elev_m > -12000.0 && elev_m < 100.0,
            "Ocean elevation {} is outside expected range",
            elev_m
        );
    }
    // If tile doesn't exist (Err) or returns None, that's acceptable for ocean areas
}

/// Test that repeated lookups use cache
/// This test verifies that the caching mechanism works by calling the same location twice
#[tokio::test]
async fn test_elevation_caching() {
    let lat = 40.7829;
    let lon = -73.9654;

    let elevation_db = create_test_elevation_db().await;

    // First call - will download and cache tile
    let result1 = elevation_db.elevation_egm2008(lat, lon).await;
    assert!(result1.is_ok(), "First elevation lookup should succeed");

    // Second call - should use cached tile
    let result2 = elevation_db.elevation_egm2008(lat, lon).await;
    assert!(result2.is_ok(), "Second elevation lookup should succeed");

    // Both should return the same value
    assert_eq!(
        result1.unwrap(),
        result2.unwrap(),
        "Cached lookup should return same value"
    );
}

/// Test elevation for negative latitude (Southern Hemisphere)
#[tokio::test]
async fn test_elevation_southern_hemisphere() {
    // Sydney, Australia
    let lat = -33.8688;
    let lon = 151.2093;

    let elevation_db = create_test_elevation_db().await;
    let result = elevation_db.elevation_egm2008(lat, lon).await;
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
#[tokio::test]
async fn test_elevation_tile_boundary() {
    // Test at exactly 45°N, 0°E (boundary between tiles)
    let lat = 45.0;
    let lon = 0.0;

    let elevation_db = create_test_elevation_db().await;
    let result = elevation_db.elevation_egm2008(lat, lon).await;
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
#[tokio::test]
async fn test_elevation_near_tile_boundary() {
    // Test near 45°N, 0°E but not exactly at boundary
    let lat = 45.1;
    let lon = 0.1;

    let elevation_db = create_test_elevation_db().await;
    let result = elevation_db.elevation_egm2008(lat, lon).await;
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
