use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status values for a club join request
pub const STATUS_PENDING: &str = "pending";
pub const STATUS_APPROVED: &str = "approved";
pub const STATUS_REJECTED: &str = "rejected";

/// API model for club join requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClubJoinRequest {
    pub id: Uuid,
    pub user_id: Uuid,
    pub club_id: Uuid,
    pub status: String,
    pub message: Option<String>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Populated from joined user data when listing requests
    pub user_first_name: Option<String>,
    pub user_last_name: Option<String>,
}

/// Diesel model for the club_join_requests table
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::club_join_requests)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ClubJoinRequestModel {
    pub id: Uuid,
    pub user_id: Uuid,
    pub club_id: Uuid,
    pub status: String,
    pub message: Option<String>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insert model for new join requests
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::club_join_requests)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewClubJoinRequest {
    pub user_id: Uuid,
    pub club_id: Uuid,
    pub status: String,
    pub message: Option<String>,
}

impl From<ClubJoinRequestModel> for ClubJoinRequest {
    fn from(model: ClubJoinRequestModel) -> Self {
        Self {
            id: model.id,
            user_id: model.user_id,
            club_id: model.club_id,
            status: model.status,
            message: model.message,
            reviewed_by: model.reviewed_by,
            reviewed_at: model.reviewed_at,
            created_at: model.created_at,
            updated_at: model.updated_at,
            user_first_name: None,
            user_last_name: None,
        }
    }
}
