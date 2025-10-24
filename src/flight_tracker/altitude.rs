use crate::Fix;
use crate::elevation::ElevationDB;
use crate::fixes_repo::FixesRepository;
use tracing::{debug, trace};

/// Calculate altitude offset in feet between reported altitude and true MSL elevation
/// Returns the difference (reported_altitude_ft - true_elevation_ft)
/// Returns None if elevation lookup fails or fix has no altitude
pub(crate) async fn calculate_altitude_offset_ft(
    elevation_db: &ElevationDB,
    fix: &Fix,
) -> Option<i32> {
    // Get reported altitude from fix (in feet)
    let reported_altitude_ft = fix.altitude_msl_feet?;

    let lat = fix.latitude;
    let lon = fix.longitude;

    // Run blocking elevation lookup in a separate thread
    let elevation_result = elevation_db.elevation_at(lat, lon).await.ok()?;

    // Get true elevation at this location (in meters)
    match elevation_result {
        Some(elevation_m) => {
            // Convert elevation from meters to feet (1 meter = 3.28084 feet)
            let elevation_ft = elevation_m * 3.28084;
            // Calculate offset
            let offset = reported_altitude_ft as f64 - elevation_ft;

            debug!(
                "Altitude offset calculation: indicated={} ft, known_elevation={:.1} ft, offset={:.0} ft at ({:.6}, {:.6})",
                reported_altitude_ft, elevation_ft, offset, lat, lon
            );

            Some(offset.round() as i32)
        }
        None => {
            // No elevation data available (e.g., ocean)
            debug!(
                "No elevation data available for location ({}, {})",
                fix.latitude, fix.longitude
            );
            None
        }
    }
}

pub(crate) async fn calculate_altitude_agl(elevation_db: &ElevationDB, fix: &Fix) -> Option<i32> {
    // Get reported altitude from fix (in feet)
    let reported_altitude_ft = fix.altitude_msl_feet?;

    let lat = fix.latitude;
    let lon = fix.longitude;

    // Run blocking elevation lookup in a separate thread
    let elevation_result = elevation_db.elevation_at(lat, lon).await.ok()?;

    // Get true elevation at this location (in meters)
    match elevation_result {
        Some(elevation_m) => {
            // Convert elevation from meters to feet (1 meter = 3.28084 feet)
            let elevation_ft = elevation_m * 3.28084;
            // Calculate AGL (Above Ground Level)
            let agl = reported_altitude_ft as f64 - elevation_ft;

            Some(agl.round() as i32)
        }
        None => {
            // No elevation data available (e.g., ocean)
            None
        }
    }
}

/// Calculate altitude AGL and update the fix in the database asynchronously
/// This method is designed to be called in a background task after the fix is inserted
pub async fn calculate_and_update_agl_async(
    elevation_db: &ElevationDB,
    fix_id: uuid::Uuid,
    fix: &Fix,
    fixes_repo: FixesRepository,
) {
    match calculate_altitude_agl(elevation_db, fix).await {
        Some(agl) => {
            if let Err(e) = fixes_repo.update_altitude_agl(fix_id, agl).await {
                debug!("Failed to update altitude_agl for fix {}: {}", fix_id, e);
            }
        }
        None => {
            trace!(
                "No altitude or elevation data for fix {}, altitude_agl remains NULL",
                fix_id
            );
        }
    }
}
