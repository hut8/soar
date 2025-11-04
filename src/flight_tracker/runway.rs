use crate::devices::Device;
use crate::fixes_repo::FixesRepository;
use crate::runways_repo::RunwaysRepository;
use chrono::{DateTime, Utc};
use tracing::{debug, warn};

use super::geometry::angular_difference;

/// Convert heading in degrees to runway identifier
/// e.g., 230° -> "23", 47° -> "05", 354° -> "35"
pub(crate) fn heading_to_runway_identifier(heading: f64) -> String {
    // Round to nearest 10 degrees and divide by 10
    let runway_number = ((heading / 10.0).round() as i32) % 36;
    // Handle 360° -> 36 -> 0
    let runway_number = if runway_number == 0 {
        36
    } else {
        runway_number
    };
    // Format with leading zero
    format!("{:02}", runway_number)
}

/// Check if an aircraft type uses runways
pub(crate) fn uses_runways(aircraft_type: &crate::ogn_aprs_aircraft::AircraftType) -> bool {
    use crate::ogn_aprs_aircraft::AircraftType;
    match aircraft_type {
        // Aircraft that use runways
        AircraftType::Glider => true,
        AircraftType::TowTug => true,
        AircraftType::RecipEngine => true,
        AircraftType::JetTurboprop => true,
        AircraftType::DropPlane => true,
        // Aircraft that don't use runways
        AircraftType::Paraglider => false,
        AircraftType::HangGlider => false,
        AircraftType::HelicopterGyro => false,
        AircraftType::Balloon => false,
        AircraftType::Airship => false,
        AircraftType::SkydiverParachute => false, // The parachute itself, not the plane
        AircraftType::Uav => false,
        AircraftType::StaticObstacle => false,
        AircraftType::Reserved => false,
        AircraftType::Unknown => true, // Default to true for unknown types
    }
}

/// Determine runway identifier based on aircraft course during takeoff/landing
/// First tries to match against nearby runways with coordinates from the database.
/// If no runways found or no good match, infers runway from aircraft heading.
/// Loads fixes from 20 seconds before to 20 seconds after the event time
///
/// Returns a tuple of (runway_identifier, was_inferred)
/// - runway_identifier: e.g., "14" or "32"
/// - was_inferred: true if inferred from heading, false if looked up in database
///
/// If airport_ref is provided, only searches for runways at that specific airport
pub(crate) async fn determine_runway_identifier(
    fixes_repo: &FixesRepository,
    runways_repo: &RunwaysRepository,
    device: &Device,
    event_time: DateTime<Utc>,
    latitude: f64,
    longitude: f64,
    airport_ref: Option<i32>,
) -> Option<(String, bool)> {
    // Extract device ID (devices from database always have an ID)
    let device_id = device
        .id
        .expect("Device fetched from database should have ID");

    // Get fixes from 20 seconds before to 20 seconds after the event
    let start_time = event_time - chrono::Duration::seconds(20);
    let end_time = event_time + chrono::Duration::seconds(20);

    // Check if this aircraft type uses runways (skip detection for helicopters, paragliders, etc.)
    if let Some(aircraft_type) = device.aircraft_type_ogn
        && !uses_runways(&aircraft_type)
    {
        debug!(
            "Device {} is type {:?} which doesn't use runways, skipping runway detection",
            device_id, aircraft_type
        );
        return None;
    }

    let fixes = match fixes_repo
        .get_fixes_for_aircraft_with_time_range(&device_id, start_time, end_time, None)
        .await
    {
        Ok(f) if !f.is_empty() => {
            debug!(
                "Found {} fixes for device {} between {} and {}",
                f.len(),
                device_id,
                start_time,
                end_time
            );
            f
        }
        Ok(_) => {
            debug!(
                "No fixes found for device {} between {} and {}",
                device_id, start_time, end_time
            );
            return None;
        }
        Err(e) => {
            warn!(
                "Error loading fixes for device {} during runway detection: {}",
                device_id, e
            );
            return None;
        }
    };

    // Calculate average course from fixes that have track_degrees
    let courses: Vec<f32> = fixes.iter().filter_map(|fix| fix.track_degrees).collect();

    if courses.is_empty() {
        debug!(
            "No track_degrees data in fixes for device {}, cannot determine runway",
            device_id
        );
        return None;
    }

    let avg_course = courses.iter().sum::<f32>() as f64 / courses.len() as f64;
    debug!(
        "Calculated average course {:.1}° from {} fixes for device {}",
        avg_course,
        courses.len(),
        device_id
    );

    // Try to find nearby runways (within 2km)
    // If we have an airport_ref, use it to filter runways to that airport only
    let nearby_runways = match runways_repo
        .find_nearest_runway_endpoints(latitude, longitude, 2000.0, 10, airport_ref)
        .await
    {
        Ok(runways) if !runways.is_empty() => {
            debug!(
                "Found {} nearby runway endpoints for device {} at ({}, {})",
                runways.len(),
                device_id,
                latitude,
                longitude
            );
            Some(runways)
        }
        Ok(_) => {
            debug!(
                "No nearby runways found for device {} at ({}, {}), will infer from heading",
                device_id, latitude, longitude
            );
            None
        }
        Err(e) => {
            warn!(
                "Error finding nearby runways for device {}: {}, will infer from heading",
                device_id, e
            );
            None
        }
    };

    // If we have nearby runways, try to match against them
    if let Some(runways) = nearby_runways {
        let mut best_match: Option<(String, f64)> = None;

        for (runway, _, endpoint_type) in runways {
            // Determine which end to check based on which is closer
            let (ident, heading) = match endpoint_type.as_str() {
                "low_end" => {
                    // Aircraft is near low end, check both ends to see which direction they're traveling
                    if let (Some(le_heading), Some(he_heading)) =
                        (runway.le_heading_degt, runway.he_heading_degt)
                    {
                        // Calculate angular difference for both directions
                        let le_diff = angular_difference(avg_course, le_heading);
                        let he_diff = angular_difference(avg_course, he_heading);

                        if le_diff < he_diff {
                            (runway.le_ident.clone(), le_heading)
                        } else {
                            (runway.he_ident.clone(), he_heading)
                        }
                    } else {
                        continue;
                    }
                }
                "high_end" => {
                    // Aircraft is near high end, check both ends to see which direction they're traveling
                    if let (Some(le_heading), Some(he_heading)) =
                        (runway.le_heading_degt, runway.he_heading_degt)
                    {
                        let le_diff = angular_difference(avg_course, le_heading);
                        let he_diff = angular_difference(avg_course, he_heading);

                        if he_diff < le_diff {
                            (runway.he_ident.clone(), he_heading)
                        } else {
                            (runway.le_ident.clone(), le_heading)
                        }
                    } else {
                        continue;
                    }
                }
                _ => continue,
            };

            if let Some(ident_str) = ident {
                let heading_diff = angular_difference(avg_course, heading);

                // Update best match if this is closer (or first match)
                match best_match {
                    None => best_match = Some((ident_str, heading_diff)),
                    Some((_, current_diff)) if heading_diff < current_diff => {
                        best_match = Some((ident_str, heading_diff));
                    }
                    _ => {}
                }
            }
        }

        // If we found a match and we're searching at a specific airport (airport_ref provided),
        // always use the best match from the database since we know runway data exists
        if let Some((ident, diff)) = best_match {
            if airport_ref.is_some() {
                // Airport-specific search: always use the closest runway from database
                debug!(
                    "Matched runway {} from database for device {} at airport {} (heading diff: {:.1}°)",
                    ident,
                    device_id,
                    airport_ref.unwrap(),
                    diff
                );
                return Some((ident, false)); // false = from database, not inferred
            } else if diff < 30.0 {
                // General search: only use if heading difference is reasonable
                debug!(
                    "Matched runway {} from database for device {} (heading diff: {:.1}°)",
                    ident, device_id, diff
                );
                return Some((ident, false)); // false = from database, not inferred
            } else {
                debug!(
                    "Best runway match {} has large heading diff ({:.1}°), will infer from heading instead",
                    ident, diff
                );
            }
        }
    }

    // Fallback: Infer runway from heading
    let inferred_runway = heading_to_runway_identifier(avg_course);
    debug!(
        "Inferred runway {} from heading {:.1}° for device {}",
        inferred_runway, avg_course, device_id
    );
    Some((inferred_runway, true)) // true = inferred from heading
}
