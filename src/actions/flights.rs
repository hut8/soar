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
use crate::fixes::Fix;
use crate::fixes_repo::FixesRepository;
use crate::flights::Flight;
use crate::flights_repo::FlightsRepository;
use crate::web::AppState;

#[derive(Debug, Deserialize)]
pub struct FlightsQueryParams {
    pub club_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub completed: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct FlightResponse {
    pub flight: FlightView,
    pub device: Option<Device>,
}

#[derive(Debug, Serialize)]
pub struct FlightFixesResponse {
    pub fixes: Vec<Fix>,
    pub count: usize,
}

/// Get a flight by its UUID
pub async fn get_flight_by_id(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool.clone());
    let airports_repo = AirportsRepository::new(state.pool.clone());
    let device_repo = DeviceRepository::new(state.pool.clone());

    match flights_repo.get_flight_by_id(id).await {
        Ok(Some(flight)) => {
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

            let flight_view =
                FlightView::from_flight(flight, departure_airport, arrival_airport, device_info);
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

    if completed {
        // Get completed flights with device info
        match flights_repo.get_completed_flights(limit).await {
            Ok(flights) => {
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
                tracing::error!("Failed to get completed flights: {}", e);
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to get completed flights",
                )
                .into_response()
            }
        }
    } else {
        // Get flights in progress
        if let Some(_club_id) = params.club_id {
            // Club-based flight search would require joining with aircraft_registrations
            // For now, just return flights in progress with limit
            match flights_repo.get_flights_in_progress(limit).await {
                Ok(flights) => {
                    let flight_views: Vec<FlightView> =
                        flights.into_iter().map(|f| f.into()).collect();
                    Json(flight_views).into_response()
                }
                Err(e) => {
                    tracing::error!("Failed to get flights by club ID: {}", e);
                    json_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to get flights by club ID",
                    )
                    .into_response()
                }
            }
        } else {
            match flights_repo.get_flights_in_progress(limit).await {
                Ok(flights) => {
                    let flight_views: Vec<FlightView> =
                        flights.into_iter().map(|f| f.into()).collect();
                    Json(flight_views).into_response()
                }
                Err(e) => {
                    tracing::error!("Failed to get recent flights: {}", e);
                    json_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to get recent flights",
                    )
                    .into_response()
                }
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
