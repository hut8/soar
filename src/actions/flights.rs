use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::actions::json_error;
use crate::actions::views::{AirportInfo, DeviceInfo, FlightView};
use crate::airports_repo::AirportsRepository;
use crate::device_repo::DeviceRepository;
use crate::devices::Device;
use crate::fixes::FixWithRawPacket;
use crate::fixes_repo::FixesRepository;
use crate::flights::Flight;
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
    pub device: Option<Device>,
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

/// Get a flight by its UUID
pub async fn get_flight_by_id(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool.clone());
    let airports_repo = AirportsRepository::new(state.pool.clone());
    let device_repo = DeviceRepository::new(state.pool.clone());
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

            // Look up device information
            let (device, device_info) = if let Some(device_id) = flight.device_id {
                match device_repo.get_device_by_uuid(device_id).await {
                    Ok(Some(dev)) => (
                        Some(dev.clone()),
                        Some(DeviceInfo {
                            aircraft_model: Some(dev.aircraft_model),
                            registration: Some(dev.registration),
                            aircraft_type_ogn: dev.aircraft_type_ogn,
                        }),
                    ),
                    _ => (None, None),
                }
            } else {
                (None, None)
            };

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

            // Get previous and next flights for navigation in a single query (if device_id is present)
            let (previous_flight_id, next_flight_id) = if let Some(device_id) = flight.device_id {
                flights_repo
                    .get_adjacent_flights_for_device(id, device_id, flight.takeoff_time)
                    .await
                    .unwrap_or((None, None))
            } else {
                (None, None)
            };

            let flight_view = FlightView::from_flight_full(
                flight,
                departure_airport,
                arrival_airport,
                device_info,
                None,
                None,
                None,
                previous_flight_id,
                next_flight_id,
            );
            Json(FlightResponse {
                flight: flight_view,
                device,
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

/// Get KML file for a flight
pub async fn get_flight_kml(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool.clone());
    let fixes_repo = FixesRepository::new(state.pool.clone());

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

    // Generate KML
    match flight.make_kml(&fixes_repo).await {
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
    let start_time = flight.takeoff_time.unwrap_or(flight.created_at);
    let end_time = flight.landing_time.unwrap_or_else(chrono::Utc::now);

    match fixes_repo
        .get_fixes_for_aircraft_with_time_range(
            &flight.device_id.unwrap_or(Uuid::nil()),
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
            &flight.device_id.unwrap_or(Uuid::nil()),
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
    let device_repo = DeviceRepository::new(state.pool.clone());

    let completed = params.completed.unwrap_or(false);
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    if completed {
        // Get completed flights with device info and total count
        let total_count_result = flights_repo.get_completed_flights_count().await;
        let flights_result = flights_repo.get_completed_flights(limit, offset).await;

        match (total_count_result, flights_result) {
            (Ok(total_count), Ok(flights)) => {
                let mut flight_views = Vec::new();

                for flight in flights {
                    // Look up device information if device_id is present
                    let device_info = if let Some(device_id) = flight.device_id {
                        match device_repo.get_device_by_uuid(device_id).await {
                            Ok(Some(device)) => Some(DeviceInfo {
                                aircraft_model: Some(device.aircraft_model),
                                registration: Some(device.registration),
                                aircraft_type_ogn: device.aircraft_type_ogn,
                            }),
                            _ => None,
                        }
                    } else {
                        None
                    };

                    let flight_view = FlightView::from_flight(flight, None, None, device_info);
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
                    // Look up device information if device_id is present
                    let device_info = if let Some(device_id) = flight.device_id {
                        match device_repo.get_device_by_uuid(device_id).await {
                            Ok(Some(device)) => Some(DeviceInfo {
                                aircraft_model: Some(device.aircraft_model),
                                registration: Some(device.registration),
                                aircraft_type_ogn: device.aircraft_type_ogn,
                            }),
                            _ => None,
                        }
                    } else {
                        None
                    };

                    // Get latest altitude and timestamp information for active flights
                    let (latest_altitude_msl, latest_altitude_agl, latest_fix_timestamp) =
                        if let Some(device_id) = flight.device_id {
                            let start_time = flight.takeoff_time.unwrap_or(flight.created_at);
                            match fixes_repo
                                .get_latest_fix_for_device(device_id, start_time)
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
                        device_info,
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
    let device_repo = DeviceRepository::new(state.pool);

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

                let (device, device_info) = if let Some(device_id) = flight.device_id {
                    match device_repo.get_device_by_uuid(device_id).await {
                        Ok(Some(dev)) => (
                            Some(dev.clone()),
                            Some(DeviceInfo {
                                aircraft_model: Some(dev.aircraft_model),
                                registration: Some(dev.registration),
                                aircraft_type_ogn: dev.aircraft_type_ogn,
                            }),
                        ),
                        _ => (None, None),
                    }
                } else {
                    (None, None)
                };

                let flight_view = FlightView::from_flight(
                    flight,
                    departure_airport,
                    arrival_airport,
                    device_info,
                );
                flight_responses.push(FlightResponse {
                    flight: flight_view,
                    device,
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
    let device_repo = DeviceRepository::new(state.pool);

    match flights_repo.get_nearby_flights(id).await {
        Ok(flights) => {
            // Build flight views with device info
            let mut flight_views = Vec::new();

            for flight in flights {
                // Look up device information if device_id is present
                let device_info = if let Some(device_id) = flight.device_id {
                    match device_repo.get_device_by_uuid(device_id).await {
                        Ok(Some(device)) => Some(DeviceInfo {
                            aircraft_model: Some(device.aircraft_model),
                            registration: Some(device.registration),
                            aircraft_type_ogn: device.aircraft_type_ogn,
                        }),
                        _ => None,
                    }
                } else {
                    None
                };

                let flight_view = FlightView::from_flight(flight, None, None, device_info);
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
