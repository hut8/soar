use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use tracing::{error, info};
use ts_rs::TS;
use uuid::Uuid;

use crate::aircraft_repo::AircraftRepository;
use crate::auth::{AdminUser, AuthUser};
use crate::clubs::{CLUB_STATUS_APPROVED, CLUB_STATUS_PENDING, CLUB_STATUS_REJECTED};
use crate::clubs_repo::{ClubsRepository, is_soaring_club};
use crate::flights_repo::FlightsRepository;
use crate::locations_repo::LocationParams;
use crate::notifications;
use crate::users::UpdateUserRequest;
use crate::users_repo::UsersRepository;
use crate::web::AppState;

use super::{
    DataListResponse, DataResponse, json_error,
    views::{AircraftInfo, ClubView, FlightView},
};

#[derive(Debug, Deserialize)]
pub struct ClubSearchParams {
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub radius: Option<f64>,
    pub active: Option<bool>,
    pub days: Option<i32>,
}

pub async fn get_club_by_id(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let clubs_repo = ClubsRepository::new(state.pool);

    match clubs_repo.get_by_id(id).await {
        Ok(Some(club)) => Json(DataResponse {
            data: ClubView::from(club),
        })
        .into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Club not found").into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get club by ID");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get club by ID",
            )
            .into_response()
        }
    }
}

pub async fn search_clubs(
    State(state): State<AppState>,
    Query(params): Query<ClubSearchParams>,
) -> impl IntoResponse {
    let clubs_repo = ClubsRepository::new(state.pool);

    // Check if active clubs search is requested
    if params.active == Some(true) {
        let days = params.days.unwrap_or(7).clamp(1, 90);
        match clubs_repo.get_recently_active(days, params.limit).await {
            Ok(clubs) => {
                let club_views: Vec<ClubView> = clubs.into_iter().map(ClubView::from).collect();
                return Json(DataListResponse { data: club_views }).into_response();
            }
            Err(e) => {
                error!(error = %e, "Failed to get recently active clubs");
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to get recently active clubs",
                )
                .into_response();
            }
        }
    }

    // Check if geographic search parameters are provided
    if let (Some(lat), Some(lng)) = (params.latitude, params.longitude) {
        let radius = params.radius.unwrap_or(50.0); // Default to 50km radius if not specified

        // Validate radius
        if radius <= 0.0 || radius > 1000.0 {
            return json_error(
                StatusCode::BAD_REQUEST,
                "Radius must be between 0 and 1000 kilometers",
            )
            .into_response();
        }

        // Validate latitude
        if !(-90.0..=90.0).contains(&lat) {
            return json_error(
                StatusCode::BAD_REQUEST,
                "Latitude must be between -90 and 90 degrees",
            )
            .into_response();
        }

        // Validate longitude
        if !(-180.0..=180.0).contains(&lng) {
            return json_error(
                StatusCode::BAD_REQUEST,
                "Longitude must be between -180 and 180 degrees",
            )
            .into_response();
        }

        match clubs_repo
            .search_nearby_soaring(lat, lng, radius, params.limit)
            .await
        {
            Ok(clubs) => {
                let club_views: Vec<ClubView> = clubs.into_iter().map(ClubView::from).collect();
                Json(DataListResponse { data: club_views }).into_response()
            }
            Err(e) => {
                error!(error = %e, "Failed to search nearby clubs");
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to search nearby clubs",
                )
                .into_response()
            }
        }
    } else if let Some(query) = params.q {
        // Text-based search
        if query.trim().is_empty() {
            return json_error(
                StatusCode::BAD_REQUEST,
                "Query parameter 'q' cannot be empty",
            )
            .into_response();
        }

        match clubs_repo.fuzzy_search_soaring(&query, params.limit).await {
            Ok(clubs) => {
                let club_views: Vec<ClubView> = clubs.into_iter().map(ClubView::from).collect();
                Json(DataListResponse { data: club_views }).into_response()
            }
            Err(e) => {
                error!(error = %e, "Failed to search clubs");
                json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to search clubs")
                    .into_response()
            }
        }
    } else if params.latitude.is_some() || params.longitude.is_some() || params.radius.is_some() {
        // Some geographic parameters provided but not latitude and longitude
        json_error(
            StatusCode::BAD_REQUEST,
            "Geographic search requires at least latitude and longitude parameters",
        )
        .into_response()
    } else {
        // No search parameters provided - return all clubs
        match clubs_repo.get_all().await {
            Ok(clubs) => {
                let club_views: Vec<ClubView> = clubs.into_iter().map(ClubView::from).collect();
                Json(DataListResponse { data: club_views }).into_response()
            }
            Err(e) => {
                error!(error = %e, "Failed to get clubs");
                json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get clubs").into_response()
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ClubFlightsQueryParams {
    pub date: Option<String>, // YYYYMMDD format
    pub completed: Option<bool>,
}

/// Get flights for a specific club with optional date and completion filters
pub async fn get_club_flights(
    Path(club_id): Path<Uuid>,
    State(state): State<AppState>,
    Query(params): Query<ClubFlightsQueryParams>,
) -> impl IntoResponse {
    let flights_repo = FlightsRepository::new(state.pool.clone());
    let aircraft_repo = AircraftRepository::new(state.pool);

    // Parse date if provided
    let date = if let Some(date_str) = params.date {
        match chrono::NaiveDate::parse_from_str(&date_str, "%Y%m%d") {
            Ok(d) => Some(d),
            Err(_) => {
                return json_error(
                    StatusCode::BAD_REQUEST,
                    "Invalid date format. Use YYYYMMDD (e.g., 20250102)",
                )
                .into_response();
            }
        }
    } else {
        None
    };

    // Get flights for the club
    match flights_repo
        .get_flights_by_club(club_id, date, params.completed)
        .await
    {
        Ok(flights) => {
            let mut flight_views = Vec::new();

            for flight in flights {
                // Look up aircraft information if aircraft_id is present
                let aircraft_info = if let Some(aircraft_id) = flight.aircraft_id {
                    match aircraft_repo.get_aircraft_by_id(aircraft_id).await {
                        Ok(Some(aircraft)) => Some(AircraftInfo {
                            aircraft_model: Some(aircraft.aircraft_model),
                            registration: aircraft.registration,
                            aircraft_category: aircraft.aircraft_category,
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

            Json(DataListResponse { data: flight_views }).into_response()
        }
        Err(e) => {
            error!(club_id = %club_id, error = %e, "Failed to get flights for club");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get club flights",
            )
            .into_response()
        }
    }
}

/// Check that an optional string doesn't exceed max_len. Returns Err with field name if it does.
fn check_len(s: &Option<String>, max_len: usize, field: &str) -> Result<(), String> {
    if let Some(v) = s
        && v.len() > max_len
    {
        return Err(format!(
            "{} is too long (max {} characters)",
            field, max_len
        ));
    }
    Ok(())
}

/// Check that a double-option string doesn't exceed max_len.
fn check_len_double(s: &Option<Option<String>>, max_len: usize, field: &str) -> Result<(), String> {
    if let Some(Some(v)) = s
        && v.len() > max_len
    {
        return Err(format!(
            "{} is too long (max {} characters)",
            field, max_len
        ));
    }
    Ok(())
}

// --- Manual club creation ---

#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct CreateClubRequest {
    pub name: String,
    pub home_base_airport_id: Option<i32>,
    pub is_soaring: Option<bool>,
    pub street1: Option<String>,
    pub street2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip_code: Option<String>,
    pub country_code: Option<String>,
}

/// Create a new club (any authenticated user)
pub async fn create_club(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateClubRequest>,
) -> impl IntoResponse {
    let user = &auth_user.0;

    // Validate and bound name length
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "Club name is required").into_response();
    }
    if name.len() > 255 {
        return json_error(StatusCode::BAD_REQUEST, "Club name is too long").into_response();
    }

    let clubs_repo = ClubsRepository::new(state.pool.clone());

    // Check for duplicate name
    match clubs_repo.find_by_name(&name).await {
        Ok(Some(_)) => {
            return json_error(StatusCode::CONFLICT, "A club with this name already exists")
                .into_response();
        }
        Err(e) => {
            error!(error = %e, "Failed to check club name");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to check club name",
            )
            .into_response();
        }
        Ok(None) => {}
    }

    // Validate address field lengths
    for result in [
        check_len(&payload.street1, 255, "Street address"),
        check_len(&payload.street2, 255, "Street address 2"),
        check_len(&payload.city, 255, "City"),
        check_len(&payload.state, 255, "State"),
        check_len(&payload.zip_code, 20, "ZIP code"),
        check_len(&payload.country_code, 2, "Country code"),
    ] {
        if let Err(msg) = result {
            return json_error(StatusCode::BAD_REQUEST, &msg).into_response();
        }
    }

    // Determine is_soaring
    let club_is_soaring = Some(payload.is_soaring.unwrap_or_else(|| is_soaring_club(&name)));

    // Build LocationParams if any address fields provided
    let location_params = {
        let has_address = payload.street1.is_some()
            || payload.street2.is_some()
            || payload.city.is_some()
            || payload.state.is_some()
            || payload.zip_code.is_some()
            || payload.country_code.is_some();

        if has_address {
            Some(LocationParams {
                street1: payload.street1,
                street2: payload.street2,
                city: payload.city,
                state: payload.state,
                zip_code: payload.zip_code,
                country_code: payload.country_code,
                geolocation: None,
            })
        } else {
            None
        }
    };

    // Create the club with pending status
    let club = match clubs_repo
        .create_club_manual(
            &name,
            club_is_soaring,
            payload.home_base_airport_id,
            location_params,
            CLUB_STATUS_PENDING,
            user.id,
        )
        .await
    {
        Ok(club) => club,
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("unique")
                || msg.contains("duplicate")
                || msg.contains("idx_clubs_name_unique")
            {
                return json_error(StatusCode::CONFLICT, "A club with this name already exists")
                    .into_response();
            }
            error!(error = %e, "Failed to create club");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create club")
                .into_response();
        }
    };

    // Auto-join: set user's club_id and make them club admin
    let users_repo = UsersRepository::new(state.pool.clone());
    let update_request = UpdateUserRequest {
        first_name: None,
        last_name: None,
        email: None,
        is_admin: None,
        is_club_admin: Some(true),
        club_id: Some(club.id),
        email_verified: None,
    };
    if let Err(e) = users_repo.update_user(user.id, &update_request).await {
        error!(error = %e, user_id = %user.id, "Failed to auto-join user to new club");
    }

    // Send admin notification email in the background
    let pool = state.pool.clone();
    let club_name = club.name.clone();
    let creator_name = user.full_name();
    tokio::spawn(async move {
        if let Err(e) =
            notifications::send_club_creation_notification(&pool, &club_name, &creator_name).await
        {
            error!(error = %e, "Failed to send club creation notification");
        }
    });

    info!(club_id = %club.id, club_name = %club.name, created_by = %user.id, "New club created (pending approval)");

    (
        StatusCode::CREATED,
        Json(DataResponse {
            data: ClubView::from(club),
        }),
    )
        .into_response()
}

/// Get pending clubs (admin only)
pub async fn get_pending_clubs(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let clubs_repo = ClubsRepository::new(state.pool);

    match clubs_repo.get_by_status(CLUB_STATUS_PENDING).await {
        Ok(clubs) => {
            let club_views: Vec<ClubView> = clubs.into_iter().map(ClubView::from).collect();
            Json(DataListResponse { data: club_views }).into_response()
        }
        Err(e) => {
            error!(error = %e, "Failed to get pending clubs");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get pending clubs",
            )
            .into_response()
        }
    }
}

/// Approve a pending club (admin only)
pub async fn approve_club(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let clubs_repo = ClubsRepository::new(state.pool);

    match clubs_repo
        .update_club_status(id, CLUB_STATUS_APPROVED)
        .await
    {
        Ok(Some(club)) => {
            info!(club_id = %id, "Club approved");
            Json(DataResponse {
                data: ClubView::from(club),
            })
            .into_response()
        }
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Club not found").into_response(),
        Err(e) => {
            error!(error = %e, "Failed to approve club");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to approve club").into_response()
        }
    }
}

/// Reject a pending club (admin only)
pub async fn reject_club(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let clubs_repo = ClubsRepository::new(state.pool.clone());

    // Get the club first to check status
    let club = match clubs_repo.get_by_id(id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return json_error(StatusCode::NOT_FOUND, "Club not found").into_response();
        }
        Err(e) => {
            error!(error = %e, "Failed to get club");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get club")
                .into_response();
        }
    };

    if club.status != CLUB_STATUS_PENDING {
        return json_error(StatusCode::BAD_REQUEST, "Club is not pending").into_response();
    }

    // Update status to rejected
    match clubs_repo
        .update_club_status(id, CLUB_STATUS_REJECTED)
        .await
    {
        Ok(Some(updated_club)) => {
            // Unset club_id for members of this club
            let users_repo = UsersRepository::new(state.pool.clone());
            if let Err(e) = users_repo.remove_all_members_from_club(id).await {
                error!(error = %e, club_id = %id, "Failed to remove members from rejected club");
            }

            info!(club_id = %id, "Club rejected");
            Json(DataResponse {
                data: ClubView::from(updated_club),
            })
            .into_response()
        }
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Club not found").into_response(),
        Err(e) => {
            error!(error = %e, "Failed to reject club");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to reject club").into_response()
        }
    }
}

// --- Club editing ---

/// Patch-style update request: `None` = field omitted (no change),
/// `Some(None)` = explicitly set to null (clear), `Some(Some(v))` = set to v.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateClubRequest {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub home_base_airport_id: Option<Option<i32>>,
    pub is_soaring: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub street1: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub street2: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub city: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub state: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub zip_code: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub country_code: Option<Option<String>>,
}

/// Deserialize a field that can be: absent (None), null (Some(None)), or present (Some(Some(v)))
fn deserialize_optional_field<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}

/// Update a club (club admins of that club or system admins)
pub async fn update_club(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateClubRequest>,
) -> impl IntoResponse {
    let user = &auth_user.0;

    // Authorization: club admin of this club or system admin
    let is_club_admin = user.is_club_admin && user.club_id == Some(id);
    if !user.is_admin && !is_club_admin {
        return json_error(
            StatusCode::FORBIDDEN,
            "You must be a club admin to edit this club",
        )
        .into_response();
    }

    // Validate name if provided
    if let Some(ref name) = payload.name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return json_error(StatusCode::BAD_REQUEST, "Club name cannot be empty")
                .into_response();
        }

        // Check for duplicate name (different club)
        let clubs_repo = ClubsRepository::new(state.pool.clone());
        match clubs_repo.find_by_name(trimmed).await {
            Ok(Some(existing)) if existing.id != id => {
                return json_error(StatusCode::CONFLICT, "A club with this name already exists")
                    .into_response();
            }
            Err(e) => {
                error!(error = %e, "Failed to check club name");
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to check club name",
                )
                .into_response();
            }
            _ => {}
        }
    }

    let clubs_repo = ClubsRepository::new(state.pool.clone());

    // Validate address field lengths
    for result in [
        check_len_double(&payload.street1, 255, "Street address"),
        check_len_double(&payload.street2, 255, "Street address 2"),
        check_len_double(&payload.city, 255, "City"),
        check_len_double(&payload.state, 255, "State"),
        check_len_double(&payload.zip_code, 20, "ZIP code"),
        check_len_double(&payload.country_code, 2, "Country code"),
    ] {
        if let Err(msg) = result {
            return json_error(StatusCode::BAD_REQUEST, &msg).into_response();
        }
    }

    // Flatten Option<Option<T>> fields:
    // None = not provided (no change), Some(None) = clear, Some(Some(v)) = set
    let airport_update = payload.home_base_airport_id; // Option<Option<i32>>

    // Build location params if any address fields were explicitly provided (even if null to clear)
    let location_params = {
        let has_address = payload.street1.is_some()
            || payload.street2.is_some()
            || payload.city.is_some()
            || payload.state.is_some()
            || payload.zip_code.is_some()
            || payload.country_code.is_some();

        if has_address {
            Some(LocationParams {
                street1: payload.street1.unwrap_or(None),
                street2: payload.street2.unwrap_or(None),
                city: payload.city.unwrap_or(None),
                state: payload.state.unwrap_or(None),
                zip_code: payload.zip_code.unwrap_or(None),
                country_code: payload.country_code.unwrap_or(None),
                geolocation: None,
            })
        } else {
            None
        }
    };

    match clubs_repo
        .update_club(
            id,
            payload.name.as_deref().map(|n| n.trim()),
            airport_update,
            payload.is_soaring,
            location_params,
        )
        .await
    {
        Ok(Some(club)) => {
            info!(club_id = %id, "Club updated");
            Json(DataResponse {
                data: ClubView::from(club),
            })
            .into_response()
        }
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Club not found").into_response(),
        Err(e) => {
            error!(error = %e, "Failed to update club");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to update club").into_response()
        }
    }
}
