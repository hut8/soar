use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub is_admin: bool,
    pub club_id: Option<Uuid>,
    pub email_verified: bool,
    #[serde(skip_serializing)]
    pub password_reset_token: Option<String>,
    #[serde(skip_serializing)]
    pub password_reset_expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing)]
    pub email_verification_token: Option<String>,
    #[serde(skip_serializing)]
    pub email_verification_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub settings: JsonValue,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password: String,
    pub club_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserRequest {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub is_admin: Option<bool>,
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
    pub is_admin: bool,
    pub club_id: Option<Uuid>,
    pub email_verified: bool,
    pub settings: JsonValue,
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

#[derive(Debug, Deserialize)]
pub struct EmailVerificationRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct EmailVerificationConfirm {
    pub token: String,
}

impl User {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    pub fn to_user_info(&self) -> UserInfo {
        UserInfo {
            id: self.id,
            first_name: self.first_name.clone(),
            last_name: self.last_name.clone(),
            email: self.email.clone(),
            is_admin: self.is_admin,
            club_id: self.club_id,
            email_verified: self.email_verified,
            settings: self.settings.clone(),
        }
    }
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        user.to_user_info()
    }
}
