use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde_json::json;
use tracing::error;

use crate::aircraft_repo::AircraftRepository;
use crate::airports_repo::AirportsRepository;
use crate::auth::AuthUser;
use crate::flights::{FlightState, haversine_distance};
use crate::flights_repo::FlightsRepository;
use crate::tracker::{
    NearbyAircraftInfo, TrackerAircraftInfo, TrackerFixRequest, TrackerFixResponse,
    TrackerFlightInfo,
};
use crate::user_fixes_repo::UserFixesRepository;
use crate::web::AppState;

use super::json_error;

/// POST /data/tracker - Record a tracker fix and return contextual flight info
pub async fn create_tracker_fix(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Json(request): Json<TrackerFixRequest>,
) -> impl IntoResponse {
    let user_id = auth_user.0.id;

    // Validate coordinates
    if request.latitude < -90.0 || request.latitude > 90.0 {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Latitude must be between -90 and 90",
        )
        .into_response();
    }
    if request.longitude < -180.0 || request.longitude > 180.0 {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Longitude must be between -180 and 180",
        )
        .into_response();
    }
    if let Some(heading) = request.heading
        && !(0.0..=360.0).contains(&heading)
    {
        return json_error(StatusCode::BAD_REQUEST, "Heading must be between 0 and 360")
            .into_response();
    }

    // Pack sensor data into raw JSONB
    let raw = json!({
        "source": "android_tracker",
        "altitude_meters": request.altitude_meters,
        "speed_mps": request.speed_mps,
        "accuracy_meters": request.accuracy_meters,
        "pressure_hpa": request.pressure_hpa,
        "accel_x": request.accel_x,
        "accel_y": request.accel_y,
        "accel_z": request.accel_z,
        "vertical_speed_mps": request.vertical_speed_mps,
    });

    // Store the fix
    let user_fixes_repo = UserFixesRepository::new(state.pool.clone());
    let fix = match user_fixes_repo
        .create(
            user_id,
            request.latitude,
            request.longitude,
            request.heading,
            Some(raw),
        )
        .await
    {
        Ok(fix) => fix,
        Err(e) => {
            error!(error = %e, "Failed to create tracker fix");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to record location",
            )
            .into_response();
        }
    };

    // Look up ground elevation and compute AGL
    let (ground_elevation_meters, altitude_agl_feet) =
        if let Some(ref elevation_db) = state.elevation_db {
            match elevation_db
                .elevation(request.latitude, request.longitude)
                .await
            {
                Ok(Some(elev_m)) => {
                    let ground_elev = elev_m as f64;
                    let agl = request.altitude_meters.map(|alt_m| {
                        let alt_ft = alt_m * 3.28084;
                        let ground_ft = ground_elev * 3.28084;
                        (alt_ft - ground_ft).round() as i32
                    });
                    (Some(ground_elev), agl)
                }
                Ok(None) => (None, None),
                Err(e) => {
                    tracing::warn!(error = %e, "Elevation lookup failed");
                    (None, None)
                }
            }
        } else {
            (None, None)
        };

    // Compute magnetic declination
    let magnetic_declination_degrees = if let Some(ref magnetic_service) = state.magnetic_service {
        let altitude_m = request.altitude_meters.unwrap_or(0.0);
        match magnetic_service
            .declination(request.latitude, request.longitude, altitude_m, None)
            .await
        {
            Ok(decl) => Some(decl),
            Err(e) => {
                tracing::warn!(error = %e, "Magnetic declination lookup failed");
                None
            }
        }
    } else {
        None
    };

    let aircraft_repo = AircraftRepository::new(state.pool.clone());

    // Run candidate and nearby queries in parallel
    let candidates_fut = aircraft_repo.find_candidate_aircraft(request.latitude, request.longitude);
    let nearby_fut =
        aircraft_repo.find_nearby_aircraft(request.latitude, request.longitude, 185_200.0, 50);

    let (candidates_result, nearby_result) = tokio::join!(candidates_fut, nearby_fut);

    // Process candidates for matching with extrapolation
    let matched_aircraft = match candidates_result {
        Ok(candidates) => find_best_match(&request, &candidates),
        Err(e) => {
            error!(error = %e, "Failed to query candidate aircraft");
            None
        }
    };

    // Look up flight if we matched an aircraft
    let flight = if let Some(ref matched) = matched_aircraft {
        lookup_flight(state.pool.clone(), matched.id).await
    } else {
        None
    };

    // Build nearby aircraft list, filtering out rows with missing coordinates
    let nearby_aircraft = match nearby_result {
        Ok(rows) => rows
            .into_iter()
            .filter_map(|row| {
                let lat = row.latitude?;
                let lon = row.longitude?;
                let (altitude_feet, ground_speed_knots, climb_fpm, track_degrees) =
                    extract_fix_data(&row.current_fix);
                Some(NearbyAircraftInfo {
                    id: row.id,
                    registration: row.registration,
                    aircraft_model: row.aircraft_model,
                    latitude: lat,
                    longitude: lon,
                    altitude_feet,
                    ground_speed_knots,
                    distance_meters: row.distance_meters.unwrap_or(0.0),
                    last_fix_at: row.last_fix_at,
                    climb_fpm,
                    track_degrees,
                })
            })
            .collect(),
        Err(e) => {
            error!(error = %e, "Failed to query nearby aircraft");
            Vec::new()
        }
    };

    let response = TrackerFixResponse {
        fix_id: fix.id,
        timestamp: fix.timestamp,
        matched_aircraft,
        flight,
        nearby_aircraft,
        ground_elevation_meters,
        altitude_agl_feet,
        magnetic_declination_degrees,
    };

    (StatusCode::CREATED, Json(response)).into_response()
}

/// Find the best matching aircraft using extrapolation-based matching.
///
/// For each candidate within 2km, extrapolate its position forward based on
/// speed and heading, then pick the closest extrapolated position within 200m.
fn find_best_match(
    request: &TrackerFixRequest,
    candidates: &[crate::aircraft_repo::CandidateAircraftRow],
) -> Option<TrackerAircraftInfo> {
    let now = chrono::Utc::now();
    let mut best: Option<(f64, &crate::aircraft_repo::CandidateAircraftRow)> = None;

    for candidate in candidates {
        let Some(lat) = candidate.latitude else {
            continue;
        };
        let Some(lon) = candidate.longitude else {
            continue;
        };

        // Extrapolate position if we have speed and heading from current_fix
        let (extrap_lat, extrap_lon) = if let Some(ref fix_json) = candidate.current_fix {
            // current_fix is serialized from Fix with #[serde(rename_all = "camelCase")]
            let speed_knots = fix_json
                .get("groundSpeedKnots")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let track_deg = fix_json
                .get("trackDegrees")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            if let Some(last_fix) = candidate.last_fix_at {
                let elapsed_s = (now - last_fix).num_milliseconds() as f64 / 1000.0;
                if elapsed_s > 0.0 && elapsed_s < 300.0 && speed_knots > 5.0 {
                    let speed_mps = speed_knots * 0.514444;
                    let distance_m = speed_mps * elapsed_s;
                    extrapolate_position(lat, lon, track_deg, distance_m)
                } else {
                    (lat, lon)
                }
            } else {
                (lat, lon)
            }
        } else {
            (lat, lon)
        };

        let dist = haversine_distance(request.latitude, request.longitude, extrap_lat, extrap_lon);

        if dist <= 200.0 && (best.is_none() || dist < best.unwrap().0) {
            best = Some((dist, candidate));
        }
    }

    best.map(|(dist, c)| TrackerAircraftInfo {
        id: c.id,
        registration: c.registration.clone(),
        aircraft_model: c.aircraft_model.clone(),
        competition_number: c.competition_number.clone(),
        distance_meters: dist,
    })
}

/// Extrapolate a position forward given a heading and distance.
/// Uses simple great-circle offset approximation.
fn extrapolate_position(lat: f64, lon: f64, heading_deg: f64, distance_m: f64) -> (f64, f64) {
    const EARTH_RADIUS_M: f64 = 6_371_000.0;

    let lat_rad = lat.to_radians();
    let heading_rad = heading_deg.to_radians();
    let angular_dist = distance_m / EARTH_RADIUS_M;

    let new_lat_rad = (lat_rad.sin() * angular_dist.cos()
        + lat_rad.cos() * angular_dist.sin() * heading_rad.cos())
    .asin();

    let new_lon_rad = lon.to_radians()
        + (heading_rad.sin() * angular_dist.sin() * lat_rad.cos())
            .atan2(angular_dist.cos() - lat_rad.sin() * new_lat_rad.sin());

    (new_lat_rad.to_degrees(), new_lon_rad.to_degrees())
}

/// Look up the active flight for a matched aircraft and optionally resolve the departure airport.
async fn lookup_flight(
    pool: crate::web::PgPool,
    aircraft_id: uuid::Uuid,
) -> Option<TrackerFlightInfo> {
    let flights_repo = FlightsRepository::new(pool.clone());

    // Look for active flight with 30-minute max age
    let (flight_id, _callsign) = match flights_repo
        .get_active_flight_for_aircraft(aircraft_id, chrono::Duration::minutes(30))
        .await
    {
        Ok(Some(result)) => result,
        Ok(None) => return None,
        Err(e) => {
            error!(error = %e, "Failed to look up active flight");
            return None;
        }
    };

    // Get the full flight record for additional details
    let flight = match flights_repo.get_flight_by_id(flight_id).await {
        Ok(Some(f)) => f,
        Ok(None) => return None,
        Err(e) => {
            error!(error = %e, "Failed to get flight details");
            return None;
        }
    };

    // Resolve departure airport ident if present
    let departure_airport = if let Some(airport_id) = flight.departure_airport_id {
        let airports_repo = AirportsRepository::new(pool.clone());
        match airports_repo.get_airport_by_id(airport_id).await {
            Ok(Some(airport)) => Some(airport.ident),
            _ => None,
        }
    } else {
        None
    };

    // Resolve tow aircraft info if present
    let (towed_by_registration, towed_by_aircraft_model) =
        if let Some(tow_aircraft_id) = flight.towed_by_aircraft_id {
            let aircraft_repo = AircraftRepository::new(pool);
            match aircraft_repo.get_aircraft_by_id(tow_aircraft_id).await {
                Ok(Some(aircraft)) => (aircraft.registration, Some(aircraft.aircraft_model)),
                _ => (None, None),
            }
        } else {
            (None, None)
        };

    let duration_seconds = flight
        .takeoff_time
        .map(|t| (chrono::Utc::now() - t).num_seconds());

    Some(TrackerFlightInfo {
        id: flight.id,
        state: FlightState::Active,
        takeoff_time: flight.takeoff_time,
        departure_airport,
        duration_seconds,
        towed_by_registration,
        towed_by_aircraft_model,
        tow_release_time: flight.tow_release_time,
        tow_release_altitude_msl_ft: flight.tow_release_altitude_msl_ft,
    })
}

/// Extract altitude (feet), ground speed (knots), climb rate (fpm), and track (degrees)
/// from the current_fix JSONB.
/// The Fix struct uses `#[serde(rename_all = "camelCase")]`, so keys are camelCase.
fn extract_fix_data(
    current_fix: &Option<serde_json::Value>,
) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
    match current_fix {
        Some(fix) => {
            let altitude_feet = fix.get("altitudeMslFeet").and_then(|v| v.as_f64());
            let ground_speed_knots = fix.get("groundSpeedKnots").and_then(|v| v.as_f64());
            let climb_fpm = fix.get("climbFpm").and_then(|v| v.as_f64());
            let track_degrees = fix.get("trackDegrees").and_then(|v| v.as_f64());
            (altitude_feet, ground_speed_knots, climb_fpm, track_degrees)
        }
        None => (None, None, None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aircraft_repo::CandidateAircraftRow;
    use serde_json::json;

    fn make_request(lat: f64, lon: f64) -> TrackerFixRequest {
        TrackerFixRequest {
            latitude: lat,
            longitude: lon,
            heading: None,
            altitude_meters: None,
            speed_mps: None,
            accuracy_meters: None,
            pressure_hpa: None,
            accel_x: None,
            accel_y: None,
            accel_z: None,
            vertical_speed_mps: None,
        }
    }

    fn make_candidate(
        lat: f64,
        lon: f64,
        speed_knots: Option<f64>,
        track_deg: Option<f64>,
        last_fix_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> CandidateAircraftRow {
        let current_fix = match (speed_knots, track_deg) {
            (Some(speed), Some(track)) => Some(json!({
                "groundSpeedKnots": speed,
                "trackDegrees": track,
            })),
            _ => None,
        };

        CandidateAircraftRow {
            id: uuid::Uuid::new_v4(),
            registration: Some("N12345".to_string()),
            aircraft_model: "ASK-21".to_string(),
            competition_number: "AB".to_string(),
            latitude: Some(lat),
            longitude: Some(lon),
            current_fix,
            last_fix_at,
            distance_meters: Some(100.0),
        }
    }

    #[test]
    fn extrapolate_position_north() {
        // Moving due north at ~100m
        let (new_lat, new_lon) = extrapolate_position(42.0, -71.0, 0.0, 100.0);
        assert!(new_lat > 42.0, "should move north");
        assert!(
            (new_lon - (-71.0)).abs() < 0.0001,
            "longitude should stay roughly the same"
        );
    }

    #[test]
    fn extrapolate_position_east() {
        let (new_lat, new_lon) = extrapolate_position(42.0, -71.0, 90.0, 100.0);
        assert!(
            (new_lat - 42.0).abs() < 0.001,
            "latitude should stay roughly the same"
        );
        assert!(new_lon > -71.0, "should move east");
    }

    #[test]
    fn match_aircraft_at_same_position() {
        let request = make_request(42.0, -71.0);
        let candidates = vec![make_candidate(42.0, -71.0, None, None, None)];

        let result = find_best_match(&request, &candidates);
        assert!(result.is_some(), "should match aircraft at same position");
        assert!(result.unwrap().distance_meters < 1.0);
    }

    #[test]
    fn no_match_when_too_far() {
        let request = make_request(42.0, -71.0);
        // ~1km away - outside 200m match radius
        let candidates = vec![make_candidate(42.01, -71.0, None, None, None)];

        let result = find_best_match(&request, &candidates);
        assert!(result.is_none(), "should not match aircraft 1km away");
    }

    #[test]
    fn no_extrapolation_when_slow() {
        let request = make_request(42.0, -71.0);
        let candidates = vec![make_candidate(
            42.0,
            -71.0,
            Some(3.0), // below 5-knot threshold
            Some(90.0),
            Some(chrono::Utc::now() - chrono::Duration::seconds(10)),
        )];

        let result = find_best_match(&request, &candidates);
        assert!(result.is_some(), "should still match without extrapolation");
    }

    #[test]
    fn skip_candidate_with_null_coordinates() {
        let request = make_request(42.0, -71.0);
        let mut candidate = make_candidate(42.0, -71.0, None, None, None);
        candidate.latitude = None;

        let result = find_best_match(&request, &[candidate]);
        assert!(result.is_none(), "should skip candidate with null lat");
    }

    #[test]
    fn selects_closest_candidate() {
        let request = make_request(42.0, -71.0);
        let candidates = vec![
            make_candidate(42.0001, -71.0, None, None, None), // ~11m away
            make_candidate(42.0, -71.0, None, None, None),    // 0m away
            make_candidate(42.00005, -71.0, None, None, None), // ~5.5m away
        ];

        let result = find_best_match(&request, &candidates);
        assert!(result.is_some());
        assert!(
            result.unwrap().distance_meters < 1.0,
            "should select closest"
        );
    }
}
