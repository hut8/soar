use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use ts_rs::TS;
use uuid::Uuid;

use crate::users::User;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct UserView {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>, // Nullable
    pub is_admin: bool,
    pub club_id: Option<Uuid>,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[ts(type = "Record<string, unknown>")]
    pub settings: JsonValue,
    // Pilot qualification fields
    pub is_licensed: bool,
    pub is_instructor: bool,
    pub is_tow_pilot: bool,
    pub is_examiner: bool,
    // Derived fields
    pub can_login: bool,
    pub is_pilot: bool,
}

impl From<User> for UserView {
    fn from(user: User) -> Self {
        let can_login = user.can_login();
        let is_pilot = user.is_pilot();

        Self {
            id: user.id,
            first_name: user.first_name,
            last_name: user.last_name,
            email: user.email,
            is_admin: user.is_admin,
            club_id: user.club_id,
            email_verified: user.email_verified,
            created_at: user.created_at,
            updated_at: user.updated_at,
            settings: user.settings,
            is_licensed: user.is_licensed,
            is_instructor: user.is_instructor,
            is_tow_pilot: user.is_tow_pilot,
            is_examiner: user.is_examiner,
            can_login,
            is_pilot,
        }
    }
}

impl UserView {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }
}
