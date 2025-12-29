use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use tracing::error;
use uuid::Uuid;

use crate::aircraft_registrations_repo::AircraftRegistrationsRepository;
use crate::aircraft_repo::AircraftRepository;
use crate::faa::aircraft_model_repo::AircraftModelRepository;
use crate::web::AppState;

use super::views::{AircraftRegistrationView, club::AircraftModelView};
use super::{
    DataListResponse, DataResponse, PaginatedDataResponse, PaginationMetadata, json_error,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AircraftIssuesParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    50
}

pub async fn get_aircraft_registrations_by_club(
    State(state): State<AppState>,
    Path(club_id): Path<Uuid>,
) -> impl IntoResponse {
    let aircraft_repo = AircraftRegistrationsRepository::new(state.pool.clone());
    let aircraft_model_repo = AircraftModelRepository::new(state.pool.clone());
    let aircraft_repository = AircraftRepository::new(state.pool.clone());

    // First get all aircraft for this club
    let aircraft_list = match aircraft_repository.search_by_club_id(club_id).await {
        Ok(aircraft) => aircraft,
        Err(e) => {
            error!("Failed to get aircraft by club: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get aircraft by club",
            )
                .into_response();
        }
    };

    let mut aircraft_views: Vec<AircraftRegistrationView> = Vec::new();

    // For each aircraft, get its aircraft registration and model
    for aircraft in aircraft_list {
        // Get aircraft registration if it exists
        let aircraft_registration = match aircraft_repo
            .get_aircraft_registration_by_device_id(aircraft.id.unwrap())
            .await
        {
            Ok(Some(reg)) => reg,
            Ok(None) => continue, // No aircraft registration for this aircraft
            Err(e) => {
                error!(
                    "Failed to get aircraft registration for aircraft {}: {}",
                    aircraft.id.unwrap(),
                    e
                );
                continue;
            }
        };

        // Convert AircraftRegistrationModel to Aircraft, then to view
        let aircraft_domain: crate::aircraft_registrations::Aircraft =
            aircraft_registration.clone().into();
        let mut view = AircraftRegistrationView::from(aircraft_domain);
        view.club_id = Some(club_id);
        view.aircraft_type_ogn = aircraft.aircraft_type_ogn;

        // Get aircraft model if available
        match aircraft_model_repo
            .get_aircraft_model_by_key(
                &aircraft_registration.manufacturer_code,
                &aircraft_registration.model_code,
                &aircraft_registration.series_code,
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
                    aircraft_registration.registration_number, e
                );
                // Continue without model data
            }
        }

        aircraft_views.push(view);
    }

    Json(DataListResponse {
        data: aircraft_views,
    })
    .into_response()
}

/// Get aircraft registration for an aircraft by aircraft ID
pub async fn get_device_aircraft_registration(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let aircraft_repo = AircraftRegistrationsRepository::new(state.pool.clone());
    let aircraft_repository = AircraftRepository::new(state.pool.clone());

    // First try to query aircraft_registrations table for a record with the given aircraft_id
    match aircraft_repo
        .get_aircraft_registration_by_device_id(id)
        .await
    {
        Ok(Some(aircraft_registration)) => {
            return Json(DataResponse {
                data: AircraftRegistrationView::from(aircraft_registration),
            })
            .into_response();
        }
        Ok(None) => {
            // Fallback: try to find aircraft and then look up by registration number
            tracing::debug!(
                "No aircraft registration found by aircraft_id {}, trying registration lookup",
                id
            );
        }
        Err(e) => {
            tracing::error!(
                "Failed to get aircraft registration by aircraft_id {}: {}",
                id,
                e
            );
        }
    }

    // Fallback: Get aircraft and look up by registration number
    match aircraft_repository.get_aircraft_by_id(id).await {
        Ok(Some(aircraft)) => {
            // Check if aircraft has a registration
            if let Some(ref registration) = aircraft.registration {
                // Try to find aircraft registration by registration number
                match aircraft_repo
                    .get_aircraft_registration_model_by_n_number(registration)
                    .await
                {
                    Ok(Some(aircraft_model)) => Json(DataResponse {
                        data: AircraftRegistrationView::from(aircraft_model),
                    })
                    .into_response(),
                    Ok(None) => (StatusCode::NO_CONTENT).into_response(),
                    Err(e) => {
                        tracing::error!(
                            "Failed to get aircraft registration for aircraft {} by n-number {}: {}",
                            id,
                            registration,
                            e
                        );
                        json_error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Failed to get aircraft registration",
                        )
                        .into_response()
                    }
                }
            } else {
                // No registration available
                (StatusCode::NO_CONTENT).into_response()
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND).into_response(),
        Err(e) => {
            tracing::error!("Failed to get aircraft by ID {}: {}", id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get aircraft").into_response()
        }
    }
}

/// Get aircraft model for an aircraft by aircraft ID
/// This joins aircraft_registrations to aircraft_models using manufacturer_code, model_code, and series_code
pub async fn get_device_aircraft_model(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let aircraft_repo = AircraftRegistrationsRepository::new(state.pool.clone());
    let aircraft_model_repo = AircraftModelRepository::new(state.pool.clone());
    let aircraft_repository = AircraftRepository::new(state.pool.clone());

    // First try to get the aircraft registration for this aircraft by aircraft_id
    let aircraft_registration = match aircraft_repo
        .get_aircraft_registration_by_device_id(id)
        .await
    {
        Ok(Some(registration)) => registration,
        Ok(None) => {
            // Fallback: try to find aircraft and then look up by registration number
            tracing::debug!(
                "No aircraft registration found by aircraft_id {}, trying registration lookup",
                id
            );

            match aircraft_repository.get_aircraft_by_id(id).await {
                Ok(Some(aircraft)) => {
                    if let Some(ref registration) = aircraft.registration {
                        match aircraft_repo
                            .get_aircraft_registration_model_by_n_number(registration)
                            .await
                        {
                            Ok(Some(aircraft_model)) => aircraft_model,
                            Ok(None) => {
                                return (StatusCode::NO_CONTENT).into_response();
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to get aircraft registration for aircraft {} by n-number {}: {}",
                                    id,
                                    registration,
                                    e
                                );
                                return json_error(
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    "Failed to get aircraft registration",
                                )
                                .into_response();
                            }
                        }
                    } else {
                        // No registration available
                        return (StatusCode::NO_CONTENT).into_response();
                    }
                }
                Ok(None) => {
                    return (StatusCode::NOT_FOUND).into_response();
                }
                Err(e) => {
                    tracing::error!("Failed to get aircraft by ID {}: {}", id, e);
                    return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get aircraft")
                        .into_response();
                }
            }
        }
        Err(e) => {
            tracing::error!(
                "Failed to get aircraft registration for aircraft {}: {}",
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
        .get_aircraft_model_record_by_key(
            &aircraft_registration.manufacturer_code,
            &aircraft_registration.model_code,
            &aircraft_registration.series_code,
        )
        .await
    {
        Ok(Some(aircraft_model)) => Json(DataResponse {
            data: AircraftModelView::from(aircraft_model),
        })
        .into_response(),
        Ok(None) => (StatusCode::NO_CONTENT).into_response(),
        Err(e) => {
            tracing::error!(
                "Failed to get aircraft model for aircraft {} with codes {}-{}-{}: {}",
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

/// Get aircraft issues including duplicate aircraft addresses
pub async fn get_aircraft_issues(
    State(state): State<AppState>,
    Query(params): Query<AircraftIssuesParams>,
) -> impl IntoResponse {
    let aircraft_repo = AircraftRepository::new(state.pool.clone());

    // Ensure page is at least 1
    let page = params.page.max(1);
    // Ensure per_page is between 1 and 100
    let per_page = params.per_page.clamp(1, 100);

    match aircraft_repo
        .get_duplicate_aircraft_paginated(page, per_page)
        .await
    {
        Ok((duplicate_aircraft, total_count)) => {
            let total_pages = (total_count as f64 / per_page as f64).ceil() as i64;

            Json(PaginatedDataResponse {
                data: duplicate_aircraft,
                metadata: PaginationMetadata {
                    page,
                    total_pages,
                    total_count,
                },
            })
            .into_response()
        }
        Err(e) => {
            error!("Failed to get aircraft issues: {}", e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get aircraft issues",
            )
            .into_response()
        }
    }
}
