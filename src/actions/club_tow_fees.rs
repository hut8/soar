use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::club_tow_fees::{ClubTowFee, NewClubTowFee, UpdateClubTowFee};
use crate::club_tow_fees_repo::ClubTowFeesRepository;
use crate::web::AppState;

use super::{DataListResponse, DataResponse, json_error};

/// Request body for creating a new tow fee tier
#[derive(Debug, Deserialize)]
pub struct CreateTowFeeRequest {
    /// Maximum altitude in feet AGL for this tier. Omit or set to null for the fallback tier (anything above highest altitude)
    pub max_altitude: Option<i32>,
    /// Cost for this tow tier
    pub cost: BigDecimal,
}

/// Request body for updating a tow fee tier
#[derive(Debug, Deserialize)]
pub struct UpdateTowFeeRequest {
    /// Maximum altitude in feet AGL for this tier
    pub max_altitude: Option<i32>,
    /// Cost for this tow tier
    pub cost: Option<BigDecimal>,
}

/// Response model for tow fees
#[derive(Debug, Serialize)]
pub struct TowFeeView {
    pub id: String,
    pub club_id: String,
    pub max_altitude: Option<i32>,
    pub cost: BigDecimal,
    pub modified_by: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ClubTowFee> for TowFeeView {
    fn from(fee: ClubTowFee) -> Self {
        Self {
            id: fee.id.to_string(),
            club_id: fee.club_id.to_string(),
            max_altitude: fee.max_altitude,
            cost: fee.cost,
            modified_by: fee.modified_by.to_string(),
            created_at: fee.created_at.to_rfc3339(),
            updated_at: fee.updated_at.to_rfc3339(),
        }
    }
}

/// GET /clubs/{club_id}/tow-fees
/// Get all tow fee tiers for a club, ordered by altitude
pub async fn get_club_tow_fees(
    State(state): State<AppState>,
    Path(club_id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = ClubTowFeesRepository::new(state.pool);

    match repo.get_by_club_id(club_id).await {
        Ok(fees) => {
            let fee_views: Vec<TowFeeView> = fees.into_iter().map(TowFeeView::from).collect();
            Json(DataListResponse { data: fee_views }).into_response()
        }
        Err(e) => {
            error!("Failed to get tow fees for club {}: {}", club_id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get tow fees").into_response()
        }
    }
}

/// POST /clubs/{club_id}/tow-fees
/// Create a new tow fee tier for a club
pub async fn create_club_tow_fee(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(club_id): Path<Uuid>,
    Json(request): Json<CreateTowFeeRequest>,
) -> impl IntoResponse {
    let user_id = auth_user.0.id;

    // Validate cost is non-negative
    if request.cost < 0 {
        return json_error(StatusCode::BAD_REQUEST, "Cost must be non-negative").into_response();
    }

    // Validate altitude if provided
    if let Some(alt) = request.max_altitude
        && alt <= 0
    {
        return json_error(StatusCode::BAD_REQUEST, "Altitude must be greater than 0")
            .into_response();
    }

    let repo = ClubTowFeesRepository::new(state.pool);

    // Check if trying to create a second fallback tier (NULL max_altitude)
    if request.max_altitude.is_none() {
        match repo.has_fallback_tier(club_id).await {
            Ok(true) => {
                return json_error(
                    StatusCode::CONFLICT,
                    "Club already has a fallback tier (NULL max_altitude). Delete the existing one first.",
                )
                .into_response();
            }
            Ok(false) => {}
            Err(e) => {
                error!("Failed to check fallback tier for club {}: {}", club_id, e);
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to validate fallback tier",
                )
                .into_response();
            }
        }
    }

    let new_fee = NewClubTowFee {
        club_id,
        max_altitude: request.max_altitude,
        cost: request.cost,
        modified_by: user_id,
    };

    match repo.create(new_fee).await {
        Ok(fee) => (
            StatusCode::CREATED,
            Json(DataResponse {
                data: TowFeeView::from(fee),
            }),
        )
            .into_response(),
        Err(e) => {
            error!("Failed to create tow fee for club {}: {}", club_id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create tow fee",
            )
            .into_response()
        }
    }
}

/// PUT /clubs/{club_id}/tow-fees/{fee_id}
/// Update an existing tow fee tier
pub async fn update_club_tow_fee(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path((club_id, fee_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateTowFeeRequest>,
) -> impl IntoResponse {
    let user_id = auth_user.0.id;

    // Validate cost if provided
    if let Some(ref cost) = request.cost
        && cost < 0
    {
        return json_error(StatusCode::BAD_REQUEST, "Cost must be non-negative").into_response();
    }

    // Validate altitude if provided
    if let Some(alt) = request.max_altitude
        && alt <= 0
    {
        return json_error(StatusCode::BAD_REQUEST, "Altitude must be greater than 0")
            .into_response();
    }

    let repo = ClubTowFeesRepository::new(state.pool.clone());

    // Verify the fee exists and belongs to this club, and get current values
    let existing_fee = match repo.get_by_id(fee_id).await {
        Ok(Some(fee)) => {
            if fee.club_id != club_id {
                return json_error(StatusCode::NOT_FOUND, "Tow fee not found for this club")
                    .into_response();
            }
            fee
        }
        Ok(None) => {
            return json_error(StatusCode::NOT_FOUND, "Tow fee not found").into_response();
        }
        Err(e) => {
            error!("Failed to get tow fee {}: {}", fee_id, e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get tow fee")
                .into_response();
        }
    };

    // Use provided values or keep existing ones
    let update_fee = UpdateClubTowFee {
        max_altitude: request.max_altitude,
        cost: request.cost.unwrap_or(existing_fee.cost),
        modified_by: user_id,
    };

    match repo.update(fee_id, update_fee).await {
        Ok(fee) => Json(DataResponse {
            data: TowFeeView::from(fee),
        })
        .into_response(),
        Err(e) => {
            error!("Failed to update tow fee {}: {}", fee_id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update tow fee",
            )
            .into_response()
        }
    }
}

/// DELETE /clubs/{club_id}/tow-fees/{fee_id}
/// Delete a tow fee tier
pub async fn delete_club_tow_fee(
    _auth_user: AuthUser,
    State(state): State<AppState>,
    Path((club_id, fee_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let repo = ClubTowFeesRepository::new(state.pool.clone());

    // Verify the fee exists and belongs to this club
    match repo.get_by_id(fee_id).await {
        Ok(Some(existing_fee)) => {
            if existing_fee.club_id != club_id {
                return json_error(StatusCode::NOT_FOUND, "Tow fee not found for this club")
                    .into_response();
            }
        }
        Ok(None) => {
            return json_error(StatusCode::NOT_FOUND, "Tow fee not found").into_response();
        }
        Err(e) => {
            error!("Failed to get tow fee {}: {}", fee_id, e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get tow fee")
                .into_response();
        }
    }

    match repo.delete(fee_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "Tow fee not found").into_response(),
        Err(e) => {
            error!("Failed to delete tow fee {}: {}", fee_id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to delete tow fee",
            )
            .into_response()
        }
    }
}
