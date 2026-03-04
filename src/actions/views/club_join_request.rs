use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::club_join_requests::ClubJoinRequest;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ClubJoinRequestView {
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

impl From<ClubJoinRequest> for ClubJoinRequestView {
    fn from(req: ClubJoinRequest) -> Self {
        Self {
            id: req.id,
            user_id: req.user_id,
            club_id: req.club_id,
            status: req.status,
            message: req.message,
            reviewed_by: req.reviewed_by,
            reviewed_at: req.reviewed_at,
            created_at: req.created_at,
            updated_at: req.updated_at,
        }
    }
}
