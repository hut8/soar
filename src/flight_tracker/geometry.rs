/// Calculate distance between two points using Haversine formula
/// Returns distance in meters
pub(crate) fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_M: f64 = 6_371_000.0;

    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_M * c
}

/// Calculate the angular difference between two headings in degrees
/// Returns the smallest angle between the two headings (0-180 degrees)
pub(crate) fn angular_difference(angle1: f64, angle2: f64) -> f64 {
    let diff = (angle1 - angle2).abs() % 360.0;
    if diff > 180.0 { 360.0 - diff } else { diff }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine_distance() {
        // Test distance between two known points
        let lat1 = 40.7128; // New York
        let lon1 = -74.0060;
        let lat2 = 40.7489; // Times Square
        let lon2 = -73.9857;

        let distance = haversine_distance(lat1, lon1, lat2, lon2);
        // Should be approximately 4.5km
        assert!(distance > 4000.0 && distance < 5000.0);
    }
}
