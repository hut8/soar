use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::actions::json_error;
use crate::actions::views::{AircraftInfo, AirportInfo, FlightView};
use crate::aircraft_repo::AircraftRepository;
use crate::airports_repo::AirportsRepository;
use crate::fixes::FixWithRawPacket;
use crate::fixes_repo::FixesRepository;
use crate::flights::{Flight, haversine_distance};
use crate::flights_repo::FlightsRepository;
use crate::geometry::spline::{GeoPoint, generate_spline_path};
use crate::web::AppState;

#[derive(Debug, Deserialize)]
pub struct FlightsQueryParams {
    pub club_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub completed: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct FlightResponse {
    pub flight: FlightView,
}

#[derive(Debug, Serialize)]
pub struct FlightFixesResponse {
    pub fixes: Vec<FixWithRawPacket>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct SplinePoint {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_meters: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct FlightSplinePathResponse {
    pub points: Vec<SplinePoint>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct FlightsListResponse {
    pub flights: Vec<FlightView>,
    pub total_count: i64,
}

#[derive(Debug, Serialize)]
pub struct FlightGap {
    /// Start time of the gap (timestamp of last fix before gap)
    pub gap_start: String,
    /// End time of the gap (timestamp of first fix after gap)
    pub gap_end: String,
    /// Duration of the gap in seconds
    pub duration_seconds: i64,
    /// Straight-line distance covered during the gap in meters
    pub distance_meters: f64,
    /// Callsign before the gap (if available)
    pub callsign_before: Option<String>,
    /// Callsign after the gap (if available)
    pub callsign_after: Option<String>,
    /// Squawk code before the gap (if available)
    pub squawk_before: Option<String>,
    /// Squawk code after the gap (if available)
    pub squawk_after: Option<String>,
    /// Climb rate (fpm) for the fix immediately before the gap
    pub climb_rate_before: Option<i32>,
    /// Climb rate (fpm) for the fix immediately after the gap
    pub climb_rate_after: Option<i32>,
    /// Average climb rate (fpm) for 10 fixes before the gap
    pub avg_climb_rate_10_before: Option<i32>,
    /// Average climb rate (fpm) for 10 fixes after the gap
    pub avg_climb_rate_10_after: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct FlightGapsResponse {
    pub gaps: Vec<FlightGap>,
    pub count: usize,
}

/// Get a flight by its UUID
pub async fn get_flight_by_id(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool.clone());
    let airports_repo = AirportsRepository::new(state.pool.clone());
    let fixes_repo = FixesRepository::new(state.pool.clone());

    match flights_repo.get_flight_by_id(id).await {
        Ok(Some(mut flight)) => {
            // Look up airport identifiers and country codes if airport IDs are present
            let departure_airport = if let Some(dep_id) = flight.departure_airport_id {
                airports_repo
                    .get_airport_by_id(dep_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|a| AirportInfo {
                        ident: Some(a.ident),
                        country: a.iso_country,
                    })
            } else {
                None
            };

            let arrival_airport = if let Some(arr_id) = flight.arrival_airport_id {
                airports_repo
                    .get_airport_by_id(arr_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|a| AirportInfo {
                        ident: Some(a.ident),
                        country: a.iso_country,
                    })
            } else {
                None
            };

            // Aircraft information is now fetched separately via /flights/{id}/device
            // We don't include it in the flight response anymore

            // For active flights, calculate distance metrics dynamically
            if flight.landing_time.is_none() {
                // Calculate total distance flown
                if let Ok(Some(total_distance)) = flight.total_distance(&fixes_repo, None).await {
                    flight.total_distance_meters = Some(total_distance);
                }

                // Calculate maximum displacement from takeoff point
                if let Ok(Some(max_displacement)) = flight
                    .maximum_displacement(&fixes_repo, &airports_repo, None)
                    .await
                {
                    flight.maximum_displacement_meters = Some(max_displacement);
                }
            }

            // Get previous and next flights for navigation in a single query (if aircraft_id is present)
            let (previous_flight_id, next_flight_id) = if let Some(aircraft_id) = flight.aircraft_id
            {
                flights_repo
                    .get_adjacent_flights_for_device(id, aircraft_id, flight.takeoff_time)
                    .await
                    .unwrap_or((None, None))
            } else {
                (None, None)
            };

            let flight_view = FlightView::from_flight_full(
                flight,
                departure_airport,
                arrival_airport,
                None, // No device info - fetch separately via /flights/{id}/device
                None,
                None,
                None,
                previous_flight_id,
                next_flight_id,
            );
            Json(FlightResponse {
                flight: flight_view,
            })
            .into_response()
        }
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Flight not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get flight by ID {}: {}", id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get flight").into_response()
        }
    }
}

/// Get aircraft information for a flight
pub async fn get_flight_device(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool.clone());
    let aircraft_repo = AircraftRepository::new(state.pool.clone());

    // First verify the flight exists and get its aircraft_id
    match flights_repo.get_flight_by_id(id).await {
        Ok(Some(flight)) => {
            if let Some(aircraft_id) = flight.aircraft_id {
                // Look up aircraft information
                match aircraft_repo.get_aircraft_by_id(aircraft_id).await {
                    Ok(Some(aircraft)) => Json(aircraft).into_response(),
                    Ok(None) => {
                        json_error(StatusCode::NOT_FOUND, "Aircraft not found").into_response()
                    }
                    Err(e) => {
                        tracing::error!("Failed to get aircraft {}: {}", aircraft_id, e);
                        json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get aircraft")
                            .into_response()
                    }
                }
            } else {
                json_error(StatusCode::NOT_FOUND, "Flight has no associated aircraft")
                    .into_response()
            }
        }
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Flight not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get flight by ID {}: {}", id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get flight").into_response()
        }
    }
}

/// Get KML file for a flight
pub async fn get_flight_kml(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool.clone());
    let fixes_repo = FixesRepository::new(state.pool.clone());
    let aircraft_repo = AircraftRepository::new(state.pool.clone());

    // First get the flight
    let flight = match flights_repo.get_flight_by_id(id).await {
        Ok(Some(flight)) => flight,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Flight not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get flight by ID {}: {}", id, e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get flight")
                .into_response();
        }
    };

    // Get aircraft info for better KML naming
    let aircraft = if let Some(aircraft_id) = flight.aircraft_id {
        aircraft_repo
            .get_aircraft_by_id(aircraft_id)
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    // Generate KML
    match flight.make_kml(&fixes_repo, aircraft.as_ref()).await {
        Ok(kml_content) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                "content-type",
                "application/vnd.google-earth.kml+xml".parse().unwrap(),
            );

            // Generate filename based on flight info
            let filename = generate_kml_filename(&flight);
            headers.insert(
                "content-disposition",
                format!("attachment; filename=\"{}\"", filename)
                    .parse()
                    .unwrap(),
            );

            (StatusCode::OK, headers, kml_content).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to generate KML for flight {}: {}", id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to generate KML").into_response()
        }
    }
}

/// Get fixes for a flight by flight ID
pub async fn get_flight_fixes(
    Path(id): Path<Uuid>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool.clone());
    let fixes_repo = FixesRepository::new(state.pool.clone());

    // First verify the flight exists
    let flight = match flights_repo.get_flight_by_id(id).await {
        Ok(Some(flight)) => flight,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Flight not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get flight by ID {}: {}", id, e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get flight")
                .into_response();
        }
    };

    // Get fixes for the flight based on device address and time range
    // If 'after' parameter is provided, use it as the start time (for polling updates)
    let start_time = if let Some(after_str) = params.get("after") {
        match chrono::DateTime::parse_from_rfc3339(after_str) {
            Ok(dt) => dt.with_timezone(&chrono::Utc),
            Err(e) => {
                tracing::warn!("Invalid 'after' parameter: {}, error: {}", after_str, e);
                flight.takeoff_time.unwrap_or(flight.created_at)
            }
        }
    } else {
        flight.takeoff_time.unwrap_or(flight.created_at)
    };

    let end_time = flight.landing_time.unwrap_or_else(chrono::Utc::now);

    match fixes_repo
        .get_fixes_for_aircraft_with_time_range(
            &flight.aircraft_id.unwrap_or(Uuid::nil()),
            start_time,
            end_time,
            None, // No limit
        )
        .await
    {
        Ok(fixes) => {
            let count = fixes.len();
            Json(FlightFixesResponse { fixes, count }).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get fixes for flight {}: {}", id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get flight fixes",
            )
            .into_response()
        }
    }
}

/// Get spline-interpolated path for a flight
/// Returns smoothed coordinates suitable for rendering polylines
pub async fn get_flight_spline_path(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool.clone());
    let fixes_repo = FixesRepository::new(state.pool.clone());

    // First verify the flight exists
    let flight = match flights_repo.get_flight_by_id(id).await {
        Ok(Some(flight)) => flight,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Flight not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get flight by ID {}: {}", id, e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get flight")
                .into_response();
        }
    };

    // Get fixes for the flight
    let start_time = flight.takeoff_time.unwrap_or(flight.created_at);
    let end_time = flight.landing_time.unwrap_or(flight.last_fix_at);

    let fixes = match fixes_repo
        .get_fixes_for_aircraft_with_time_range(
            &flight.aircraft_id.unwrap_or(Uuid::nil()),
            start_time,
            end_time,
            None,
        )
        .await
    {
        Ok(fixes) => fixes,
        Err(e) => {
            tracing::error!("Failed to get fixes for flight {}: {}", id, e);
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get flight fixes",
            )
            .into_response();
        }
    };

    if fixes.len() < 2 {
        // Not enough points for spline interpolation, return empty
        return Json(FlightSplinePathResponse {
            points: vec![],
            count: 0,
        })
        .into_response();
    }

    // Convert fixes to GeoPoints with altitude
    let fix_points: Vec<GeoPoint> = fixes
        .iter()
        .map(|fix| {
            let altitude_meters = fix.altitude_msl_feet.map(|alt| alt as f64 * 0.3048);
            if let Some(alt) = altitude_meters {
                GeoPoint::new_with_altitude(fix.latitude, fix.longitude, alt)
            } else {
                GeoPoint::new(fix.latitude, fix.longitude)
            }
        })
        .collect();

    // Generate spline path with 100m spacing
    let path = generate_spline_path(&fix_points, 100.0);

    // Convert to response format
    let points: Vec<SplinePoint> = path
        .iter()
        .map(|p| SplinePoint {
            latitude: p.latitude,
            longitude: p.longitude,
            altitude_meters: p.altitude_meters,
        })
        .collect();

    let count = points.len();
    Json(FlightSplinePathResponse { points, count }).into_response()
}

/// Generate an appropriate filename for the KML download
fn generate_kml_filename(flight: &Flight) -> String {
    let base_name = if let Some(takeoff_time) = flight.takeoff_time {
        format!(
            "flight-{}-{}",
            flight.device_address,
            takeoff_time.format("%Y%m%d-%H%M")
        )
    } else {
        format!("flight-{}-{}", flight.device_address, flight.id)
    };

    format!("{}.kml", base_name)
}

/// Search flights by club ID, or return recent flights (in progress or completed)
pub async fn search_flights(
    State(state): State<AppState>,
    Query(params): Query<FlightsQueryParams>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool.clone());
    let aircraft_repo = AircraftRepository::new(state.pool.clone());

    let completed = params.completed.unwrap_or(false);
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    if completed {
        // Get completed flights with aircraft info and total count
        let total_count_result = flights_repo.get_completed_flights_count().await;
        let flights_result = flights_repo.get_completed_flights(limit, offset).await;

        match (total_count_result, flights_result) {
            (Ok(total_count), Ok(flights)) => {
                let mut flight_views = Vec::new();

                for flight in flights {
                    // Look up aircraft information if aircraft_id is present
                    let aircraft_info = if let Some(aircraft_id) = flight.aircraft_id {
                        match aircraft_repo.get_aircraft_by_id(aircraft_id).await {
                            Ok(Some(aircraft)) => Some(AircraftInfo {
                                aircraft_model: Some(aircraft.aircraft_model),
                                registration: Some(aircraft.registration),
                                aircraft_type_ogn: aircraft.aircraft_type_ogn,
                                country_code: aircraft.country_code,
                            }),
                            _ => None,
                        }
                    } else {
                        None
                    };

                    let flight_view = FlightView::from_flight(flight, None, None, aircraft_info);
                    flight_views.push(flight_view);
                }

                Json(FlightsListResponse {
                    flights: flight_views,
                    total_count,
                })
                .into_response()
            }
            _ => {
                tracing::error!("Failed to get completed flights or count");
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to get completed flights",
                )
                .into_response()
            }
        }
    } else {
        // Get flights in progress with total count
        let total_count_result = flights_repo.get_flights_in_progress_count().await;
        let flights_result = flights_repo.get_flights_in_progress(limit, offset).await;
        let fixes_repo = FixesRepository::new(state.pool.clone());

        match (total_count_result, flights_result) {
            (Ok(total_count), Ok(flights)) => {
                let mut flight_views = Vec::new();

                for flight in flights {
                    // Look up aircraft information if aircraft_id is present
                    let aircraft_info = if let Some(aircraft_id) = flight.aircraft_id {
                        match aircraft_repo.get_aircraft_by_id(aircraft_id).await {
                            Ok(Some(aircraft)) => Some(AircraftInfo {
                                aircraft_model: Some(aircraft.aircraft_model),
                                registration: Some(aircraft.registration),
                                aircraft_type_ogn: aircraft.aircraft_type_ogn,
                                country_code: aircraft.country_code,
                            }),
                            _ => None,
                        }
                    } else {
                        None
                    };

                    // Get latest altitude and timestamp information for active flights
                    let (latest_altitude_msl, latest_altitude_agl, latest_fix_timestamp) =
                        if let Some(aircraft_id) = flight.aircraft_id {
                            let start_time = flight.takeoff_time.unwrap_or(flight.created_at);
                            match fixes_repo
                                .get_latest_fix_for_aircraft(aircraft_id, start_time)
                                .await
                            {
                                Ok(Some(fix)) => (
                                    fix.altitude_msl_feet,
                                    fix.altitude_agl_feet,
                                    Some(fix.timestamp),
                                ),
                                _ => (None, None, None),
                            }
                        } else {
                            (None, None, None)
                        };

                    let flight_view = FlightView::from_flight_with_altitude(
                        flight,
                        None,
                        None,
                        aircraft_info,
                        latest_altitude_msl,
                        latest_altitude_agl,
                        latest_fix_timestamp,
                    );
                    flight_views.push(flight_view);
                }

                Json(FlightsListResponse {
                    flights: flight_views,
                    total_count,
                })
                .into_response()
            }
            _ => {
                tracing::error!("Failed to get flights in progress or count");
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to get flights in progress",
                )
                .into_response()
            }
        }
    }
}

/// Get flights associated with an airport (departure or arrival) from the last 24 hours
pub async fn get_airport_flights(
    Path(airport_id): Path<i32>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool.clone());
    let airports_repo = AirportsRepository::new(state.pool.clone());
    let aircraft_repo = AircraftRepository::new(state.pool);

    // Calculate 24 hours ago
    let since = chrono::Utc::now() - chrono::Duration::hours(24);

    match flights_repo.get_flights_by_airport(airport_id, since).await {
        Ok(flights) => {
            // Build flight views with airport idents and device info
            let mut flight_responses = Vec::new();

            for flight in flights {
                let departure_airport = if let Some(dep_id) = flight.departure_airport_id {
                    airports_repo
                        .get_airport_by_id(dep_id)
                        .await
                        .ok()
                        .flatten()
                        .map(|a| AirportInfo {
                            ident: Some(a.ident),
                            country: a.iso_country,
                        })
                } else {
                    None
                };

                let arrival_airport = if let Some(arr_id) = flight.arrival_airport_id {
                    airports_repo
                        .get_airport_by_id(arr_id)
                        .await
                        .ok()
                        .flatten()
                        .map(|a| AirportInfo {
                            ident: Some(a.ident),
                            country: a.iso_country,
                        })
                } else {
                    None
                };

                // For list views, include aircraft info in FlightView for display purposes
                let aircraft_info = if let Some(aircraft_id) = flight.aircraft_id {
                    match aircraft_repo.get_aircraft_by_id(aircraft_id).await {
                        Ok(Some(aircraft)) => Some(AircraftInfo {
                            aircraft_model: Some(aircraft.aircraft_model),
                            registration: Some(aircraft.registration),
                            aircraft_type_ogn: aircraft.aircraft_type_ogn,
                            country_code: aircraft.country_code,
                        }),
                        _ => None,
                    }
                } else {
                    None
                };

                let flight_view = FlightView::from_flight(
                    flight,
                    departure_airport,
                    arrival_airport,
                    aircraft_info,
                );
                flight_responses.push(FlightResponse {
                    flight: flight_view,
                });
            }

            Json(flight_responses).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get flights for airport {}: {}", airport_id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get airport flights",
            )
            .into_response()
        }
    }
}

/// Get nearby flights that occurred during the same time period as a given flight
/// Returns flights without fixes for lightweight response
pub async fn get_nearby_flights(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool.clone());
    let aircraft_repo = AircraftRepository::new(state.pool);

    match flights_repo.get_nearby_flights(id).await {
        Ok(flights) => {
            // Build flight views with aircraft info
            let mut flight_views = Vec::new();

            for flight in flights {
                // Look up aircraft information if aircraft_id is present
                let aircraft_info = if let Some(aircraft_id) = flight.aircraft_id {
                    match aircraft_repo.get_aircraft_by_id(aircraft_id).await {
                        Ok(Some(aircraft)) => Some(AircraftInfo {
                            aircraft_model: Some(aircraft.aircraft_model),
                            registration: Some(aircraft.registration),
                            aircraft_type_ogn: aircraft.aircraft_type_ogn,
                            country_code: aircraft.country_code,
                        }),
                        _ => None,
                    }
                } else {
                    None
                };

                let flight_view = FlightView::from_flight(flight, None, None, aircraft_info);
                flight_views.push(flight_view);
            }

            Json(flight_views).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get nearby flights for flight {}: {}", id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get nearby flights",
            )
            .into_response()
        }
    }
}

/// Calculate average climb rate from a slice of fixes
fn calculate_avg_climb_rate(fixes: &[FixWithRawPacket]) -> Option<i32> {
    if fixes.len() < 2 {
        return None;
    }

    let first = &fixes[0];
    let last = &fixes[fixes.len() - 1];

    // Get altitude MSL for both fixes
    let first_alt = first.altitude_msl_feet?;
    let last_alt = last.altitude_msl_feet?;

    // Calculate time difference in minutes
    let time_diff = (last.timestamp - first.timestamp).num_seconds() as f64 / 60.0;

    if time_diff == 0.0 {
        return None;
    }

    // Calculate climb rate in feet per minute
    let climb_rate = (last_alt - first_alt) as f64 / time_diff;

    Some(climb_rate.round() as i32)
}

/// Get gaps in position fixes for a flight (5+ minutes between fixes)
/// This is useful for debugging flight detection and understanding tracking coverage
pub async fn get_flight_gaps(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool.clone());
    let fixes_repo = FixesRepository::new(state.pool.clone());

    // First verify the flight exists
    let flight = match flights_repo.get_flight_by_id(id).await {
        Ok(Some(flight)) => flight,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Flight not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get flight by ID {}: {}", id, e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get flight")
                .into_response();
        }
    };

    // Get all fixes for the flight in chronological order
    let start_time = flight.takeoff_time.unwrap_or(flight.created_at);
    let end_time = flight.landing_time.unwrap_or_else(chrono::Utc::now);

    let mut fixes = match fixes_repo
        .get_fixes_for_aircraft_with_time_range(
            &flight.aircraft_id.unwrap_or(Uuid::nil()),
            start_time,
            end_time,
            None,
        )
        .await
    {
        Ok(fixes) => fixes,
        Err(e) => {
            tracing::error!("Failed to get fixes for flight {}: {}", id, e);
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get flight fixes",
            )
            .into_response();
        }
    };

    // Reverse to get chronological order (earliest first)
    fixes.reverse();

    const GAP_THRESHOLD_SECONDS: i64 = 5 * 60; // 5 minutes
    const LOOKBACK_COUNT: usize = 10;

    let mut gaps = Vec::new();

    // Iterate through consecutive fixes to find gaps
    for i in 0..fixes.len().saturating_sub(1) {
        let current = &fixes[i];
        let next = &fixes[i + 1];

        let time_diff = (next.timestamp - current.timestamp).num_seconds();

        if time_diff >= GAP_THRESHOLD_SECONDS {
            // Calculate distance covered during the gap
            let distance_meters = haversine_distance(
                current.latitude,
                current.longitude,
                next.latitude,
                next.longitude,
            );

            // Get callsign before and after (only if different)
            let callsign_before = current.flight_number.clone();
            let callsign_after = next.flight_number.clone();

            // Get squawk codes
            let squawk_before = current.squawk.clone();
            let squawk_after = next.squawk.clone();

            // Climb rate for fixes immediately before and after
            let climb_rate_before = current.climb_fpm;
            let climb_rate_after = next.climb_fpm;

            // Calculate average climb rate for 10 fixes before the gap
            let start_lookback = i.saturating_sub(LOOKBACK_COUNT);
            let fixes_before = &fixes[start_lookback..=i];
            let avg_climb_rate_10_before = calculate_avg_climb_rate(fixes_before);

            // Calculate average climb rate for 10 fixes after the gap
            let end_lookback = (i + 1 + LOOKBACK_COUNT).min(fixes.len());
            let fixes_after = &fixes[(i + 1)..end_lookback];
            let avg_climb_rate_10_after = calculate_avg_climb_rate(fixes_after);

            gaps.push(FlightGap {
                gap_start: current.timestamp.to_rfc3339(),
                gap_end: next.timestamp.to_rfc3339(),
                duration_seconds: time_diff,
                distance_meters,
                callsign_before,
                callsign_after,
                squawk_before,
                squawk_after,
                climb_rate_before,
                climb_rate_after,
                avg_climb_rate_10_before,
                avg_climb_rate_10_after,
            });
        }
    }

    let count = gaps.len();
    Json(FlightGapsResponse { gaps, count }).into_response()
}
