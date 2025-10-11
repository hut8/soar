use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use tracing::error;
use uuid::Uuid;

use crate::aircraft_registrations_repo::AircraftRegistrationsRepository;
use crate::device_repo::DeviceRepository;
use crate::faa::aircraft_model_repo::AircraftModelRepository;
use crate::web::AppState;

use super::json_error;
use super::views::{AircraftView, club::AircraftModelView};

pub async fn get_aircraft_by_club(
    State(state): State<AppState>,
    Path(club_id): Path<Uuid>,
) -> impl IntoResponse {
    let aircraft_repo = AircraftRegistrationsRepository::new(state.pool.clone());
    let aircraft_model_repo = AircraftModelRepository::new(state.pool.clone());
    let device_repo = DeviceRepository::new(state.pool.clone());

    // First get all devices for this club
    let devices = match device_repo.search_by_club_id(club_id).await {
        Ok(devices) => devices,
        Err(e) => {
            error!("Failed to get devices by club: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get devices by club",
            )
                .into_response();
        }
    };

    let mut aircraft_views: Vec<AircraftView> = Vec::new();

    // For each device, get its aircraft registration and model
    for device in devices {
        // Get aircraft registration if it exists
        let aircraft = match aircraft_repo
            .get_aircraft_registration_by_device_id(device.id.unwrap())
            .await
        {
            Ok(Some(reg)) => reg,
            Ok(None) => continue, // No aircraft registration for this device
            Err(e) => {
                error!(
                    "Failed to get aircraft registration for device {}: {}",
                    device.id.unwrap(),
                    e
                );
                continue;
            }
        };

        // Convert AircraftRegistrationModel to Aircraft, then to view
        let aircraft_domain: crate::aircraft_registrations::Aircraft = aircraft.clone().into();
        let mut view = AircraftView::from(aircraft_domain);
        view.club_id = Some(club_id);
        view.aircraft_type_ogn = device.aircraft_type_ogn;

        // Get aircraft model if available
        match aircraft_model_repo
            .get_aircraft_model_by_key(
                &aircraft.manufacturer_code,
                &aircraft.model_code,
                &aircraft.series_code,
            )
            .await
        {
            Ok(Some(model)) => {
                // Convert AircraftModel to AircraftModelRecord
                let model_record: crate::faa::aircraft_model_repo::AircraftModelRecord =
                    model.into();
                view.model = Some(AircraftModelView::from(model_record));
            }
            Ok(None) => {
                // No model found, leave as None
            }
            Err(e) => {
                error!(
                    "Failed to get aircraft model for {}: {}",
                    aircraft.registration_number, e
                );
                // Continue without model data
            }
        }

        aircraft_views.push(view);
    }

    Json(aircraft_views).into_response()
}

/// Get aircraft registration for a device by device ID
pub async fn get_device_aircraft_registration(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let aircraft_repo = AircraftRegistrationsRepository::new(state.pool.clone());
    let device_repo = DeviceRepository::new(state.pool.clone());

    // First try to query aircraft_registrations table for a record with the given device_id
    match aircraft_repo
        .get_aircraft_registration_by_device_id(id)
        .await
    {
        Ok(Some(aircraft_registration)) => {
            return Json(aircraft_registration).into_response();
        }
        Ok(None) => {
            // Fallback: try to find device and then look up by registration number
            tracing::debug!(
                "No aircraft registration found by device_id {}, trying registration lookup",
                id
            );
        }
        Err(e) => {
            tracing::error!(
                "Failed to get aircraft registration by device_id {}: {}",
                id,
                e
            );
        }
    }

    // Fallback: Get device and look up aircraft by registration number
    match device_repo.get_device_by_uuid(id).await {
        Ok(Some(device)) => {
            // Try to find aircraft registration by registration number
            match aircraft_repo
                .get_aircraft_registration_model_by_n_number(&device.registration)
                .await
            {
                Ok(Some(aircraft_model)) => Json(aircraft_model).into_response(),
                Ok(None) => (StatusCode::NOT_FOUND).into_response(),
                Err(e) => {
                    tracing::error!(
                        "Failed to get aircraft registration for device {} by n-number {}: {}",
                        id,
                        device.registration,
                        e
                    );
                    json_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to get aircraft registration",
                    )
                    .into_response()
                }
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND).into_response(),
        Err(e) => {
            tracing::error!("Failed to get device by ID {}: {}", id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get device").into_response()
        }
    }
}

/// Get aircraft model for a device by device ID
/// This joins aircraft_registrations to aircraft_models using manufacturer_code, model_code, and series_code
pub async fn get_device_aircraft_model(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let aircraft_repo = AircraftRegistrationsRepository::new(state.pool.clone());
    let aircraft_model_repo = AircraftModelRepository::new(state.pool.clone());
    let device_repo = DeviceRepository::new(state.pool.clone());

    // First try to get the aircraft registration for this device by device_id
    let aircraft_registration = match aircraft_repo
        .get_aircraft_registration_by_device_id(id)
        .await
    {
        Ok(Some(registration)) => registration,
        Ok(None) => {
            // Fallback: try to find device and then look up by registration number
            tracing::debug!(
                "No aircraft registration found by device_id {}, trying registration lookup",
                id
            );

            match device_repo.get_device_by_uuid(id).await {
                Ok(Some(device)) => {
                    match aircraft_repo
                        .get_aircraft_registration_model_by_n_number(&device.registration)
                        .await
                    {
                        Ok(Some(aircraft_model)) => aircraft_model,
                        Ok(None) => {
                            return (StatusCode::NOT_FOUND).into_response();
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to get aircraft registration for device {} by n-number {}: {}",
                                id,
                                device.registration,
                                e
                            );
                            return json_error(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to get aircraft registration",
                            )
                            .into_response();
                        }
                    }
                }
                Ok(None) => {
                    return (StatusCode::NOT_FOUND).into_response();
                }
                Err(e) => {
                    tracing::error!("Failed to get device by ID {}: {}", id, e);
                    return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get device")
                        .into_response();
                }
            }
        }
        Err(e) => {
            tracing::error!(
                "Failed to get aircraft registration for device {}: {}",
                id,
                e
            );
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get aircraft registration",
            )
            .into_response();
        }
    };

    // Now get the aircraft model using the codes from the registration
    match aircraft_model_repo
        .get_aircraft_model_by_key(
            &aircraft_registration.manufacturer_code,
            &aircraft_registration.model_code,
            &aircraft_registration.series_code,
        )
        .await
    {
        Ok(Some(aircraft_model)) => Json(aircraft_model).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND).into_response(),
        Err(e) => {
            tracing::error!(
                "Failed to get aircraft model for device {} with codes {}-{}-{}: {}",
                id,
                aircraft_registration.manufacturer_code,
                aircraft_registration.model_code,
                aircraft_registration.series_code,
                e
            );
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get aircraft model",
            )
            .into_response()
        }
    }
}
