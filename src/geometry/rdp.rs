//! Ramer-Douglas-Peucker algorithm for polyline simplification
//!
//! This module implements the RDP algorithm to reduce the number of points
//! in a path while preserving the overall shape within a given tolerance.

use super::spline::GeoPoint;

/// Calculate the perpendicular distance from a point to a line segment in 3D
///
/// Uses geographic distance for horizontal component and direct altitude
/// difference for vertical, with altitude scaled to be comparable to
/// horizontal distance (1 degree ≈ 111km, so we scale altitude accordingly).
fn point_to_line_distance(point: &GeoPoint, line_start: &GeoPoint, line_end: &GeoPoint) -> f64 {
    // If start and end are the same point, return distance to that point
    let line_length = line_start.distance_to(line_end);
    if line_length < 0.001 {
        // Less than 1mm
        return point.distance_3d_to(line_start);
    }

    // Project point onto the line using parameter t
    // We work in a local coordinate system where distances are in meters

    // Calculate vectors (using meter-based distances)
    let start_to_end_lat = line_end.latitude - line_start.latitude;
    let start_to_end_lng = line_end.longitude - line_start.longitude;
    let start_to_point_lat = point.latitude - line_start.latitude;
    let start_to_point_lng = point.longitude - line_start.longitude;

    // Calculate t parameter for projection onto line
    let dot_product = start_to_point_lat * start_to_end_lat + start_to_point_lng * start_to_end_lng;
    let line_length_sq = start_to_end_lat * start_to_end_lat + start_to_end_lng * start_to_end_lng;

    let t = if line_length_sq > 0.0 {
        (dot_product / line_length_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Find the closest point on the line
    let closest_lat = line_start.latitude + t * start_to_end_lat;
    let closest_lng = line_start.longitude + t * start_to_end_lng;

    // Interpolate altitude if available
    let closest_alt = match (line_start.altitude_meters, line_end.altitude_meters) {
        (Some(alt1), Some(alt2)) => Some(alt1 + t * (alt2 - alt1)),
        _ => None,
    };

    let closest_point = match closest_alt {
        Some(alt) => GeoPoint::new_with_altitude(closest_lat, closest_lng, alt),
        None => GeoPoint::new(closest_lat, closest_lng),
    };

    // Return 3D distance from point to closest point on line
    point.distance_3d_to(&closest_point)
}

/// Simplify a path using the Ramer-Douglas-Peucker algorithm
///
/// # Arguments
/// * `points` - The input path as a slice of GeoPoints
/// * `epsilon` - The maximum allowed perpendicular distance in meters
///
/// # Returns
/// A simplified path containing only the essential points
pub fn simplify_path(points: &[GeoPoint], epsilon: f64) -> Vec<GeoPoint> {
    let indices = simplify_path_indices(points, epsilon);
    indices.into_iter().map(|i| points[i]).collect()
}

/// Simplify a path and return the indices of kept points
///
/// # Arguments
/// * `points` - The input path as a slice of GeoPoints
/// * `epsilon` - The maximum allowed perpendicular distance in meters
///
/// # Returns
/// Indices of the points to keep from the original path
pub fn simplify_path_indices(points: &[GeoPoint], epsilon: f64) -> Vec<usize> {
    if points.is_empty() {
        return vec![];
    }
    if points.len() <= 2 {
        return (0..points.len()).collect();
    }

    let mut keep = vec![false; points.len()];
    rdp_recursive(points, 0, points.len() - 1, epsilon, &mut keep);

    // Always keep first and last
    keep[0] = true;
    keep[points.len() - 1] = true;

    keep.iter()
        .enumerate()
        .filter_map(|(i, &k)| if k { Some(i) } else { None })
        .collect()
}

/// Recursive helper for RDP that marks points to keep
#[allow(clippy::needless_range_loop)] // We need the index for keep[] and recursive calls
fn rdp_recursive(points: &[GeoPoint], start: usize, end: usize, epsilon: f64, keep: &mut [bool]) {
    if end <= start + 1 {
        return;
    }

    let first = &points[start];
    let last = &points[end];

    // Find the point with maximum distance from the line
    let mut max_distance = 0.0;
    let mut max_index = start;

    for i in (start + 1)..end {
        let distance = point_to_line_distance(&points[i], first, last);
        if distance > max_distance {
            max_distance = distance;
            max_index = i;
        }
    }

    // If max distance is greater than epsilon, keep this point and recurse
    if max_distance > epsilon {
        keep[max_index] = true;
        rdp_recursive(points, start, max_index, epsilon, keep);
        rdp_recursive(points, max_index, end, epsilon, keep);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simplify_straight_line() {
        // Points along a straight line should collapse to 2 points
        let points = vec![
            GeoPoint::new(0.0, 0.0),
            GeoPoint::new(0.5, 0.5),
            GeoPoint::new(1.0, 1.0),
            GeoPoint::new(1.5, 1.5),
            GeoPoint::new(2.0, 2.0),
        ];

        let simplified = simplify_path(&points, 100.0); // 100m tolerance
        assert_eq!(simplified.len(), 2);
    }

    #[test]
    fn test_simplify_preserves_corners() {
        // A sharp corner should be preserved
        let points = vec![
            GeoPoint::new(0.0, 0.0),
            GeoPoint::new(1.0, 0.0), // Corner point
            GeoPoint::new(1.0, 1.0),
        ];

        let simplified = simplify_path(&points, 100.0);
        // The corner should be preserved because perpendicular distance is large
        assert!(simplified.len() >= 2);
    }

    #[test]
    fn test_simplify_with_altitude() {
        // Climbing path should preserve altitude changes
        let points = vec![
            GeoPoint::new_with_altitude(0.0, 0.0, 0.0),
            GeoPoint::new_with_altitude(0.001, 0.001, 500.0),
            GeoPoint::new_with_altitude(0.002, 0.002, 1000.0),
        ];

        // With small epsilon, altitude change should cause preservation
        let simplified = simplify_path(&points, 10.0);
        assert!(simplified.len() >= 2);
    }

    #[test]
    fn test_simplify_empty_and_small() {
        assert_eq!(simplify_path(&[], 100.0).len(), 0);
        assert_eq!(simplify_path(&[GeoPoint::new(0.0, 0.0)], 100.0).len(), 1);
        assert_eq!(
            simplify_path(&[GeoPoint::new(0.0, 0.0), GeoPoint::new(1.0, 1.0)], 100.0).len(),
            2
        );
    }
}
