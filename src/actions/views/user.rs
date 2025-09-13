use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::users::{AccessLevel, User};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserView {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub access_level: AccessLevel,
    pub club_id: Option<Uuid>,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<User> for UserView {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            first_name: user.first_name,
            last_name: user.last_name,
            email: user.email,
            access_level: user.access_level,
            club_id: user.club_id,
            email_verified: user.email_verified,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

impl UserView {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    pub fn is_admin(&self) -> bool {
        matches!(self.access_level, AccessLevel::Admin)
    }
}
