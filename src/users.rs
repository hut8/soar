use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "access_level", rename_all = "lowercase")]
pub enum AccessLevel {
    Standard,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub access_level: AccessLevel,
    pub club_id: Option<Uuid>,
    pub email_verified: bool,
    #[serde(skip_serializing)]
    pub password_reset_token: Option<String>,
    #[serde(skip_serializing)]
    pub password_reset_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password: String,
    pub club_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub access_level: Option<AccessLevel>,
    pub club_id: Option<Uuid>,
    pub email_verified: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub access_level: AccessLevel,
    pub club_id: Option<Uuid>,
    pub email_verified: bool,
}

#[derive(Debug, Deserialize)]
pub struct PasswordResetRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct PasswordResetConfirm {
    pub token: String,
    pub new_password: String,
}

impl User {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    pub fn is_admin(&self) -> bool {
        matches!(self.access_level, AccessLevel::Admin)
    }

    pub fn to_user_info(&self) -> UserInfo {
        UserInfo {
            id: self.id,
            first_name: self.first_name.clone(),
            last_name: self.last_name.clone(),
            email: self.email.clone(),
            access_level: self.access_level,
            club_id: self.club_id,
            email_verified: self.email_verified,
        }
    }
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        user.to_user_info()
    }
}
