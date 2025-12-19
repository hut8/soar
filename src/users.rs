use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>, // Nullable - None indicates user cannot log in
    #[serde(skip_serializing)]
    pub password_hash: Option<String>, // Nullable - None indicates user cannot log in
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
    // Pilot qualification fields
    pub is_licensed: bool,
    pub is_instructor: bool,
    pub is_tow_pilot: bool,
    pub is_examiner: bool,
    pub deleted_at: Option<DateTime<Utc>>, // Soft delete support
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
    pub email: Option<String>, // Nullable
    pub is_admin: bool,
    pub club_id: Option<Uuid>,
    pub email_verified: bool,
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

// New request types for unified user/pilot management

#[derive(Debug, Deserialize)]
pub struct CreatePilotRequest {
    pub first_name: String,
    pub last_name: String,
    pub is_licensed: bool,
    pub is_instructor: bool,
    pub is_tow_pilot: bool,
    pub is_examiner: bool,
    pub club_id: Option<Uuid>,
    pub email: Option<String>, // Optional - for sending invitation
}

#[derive(Debug, Deserialize)]
pub struct SendInvitationRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct CompletePilotRegistrationRequest {
    pub token: String,
    pub password: String,
}

impl User {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    /// Check if this user can log in (has email and password)
    pub fn can_login(&self) -> bool {
        self.email.is_some() && self.password_hash.is_some()
    }

    /// Check if this user is a pilot (has any qualification)
    pub fn is_pilot(&self) -> bool {
        self.is_licensed || self.is_instructor || self.is_tow_pilot || self.is_examiner
    }

    /// Factory method to create a new pilot-only user (no login capability)
    pub fn new_pilot(
        first_name: String,
        last_name: String,
        is_licensed: bool,
        is_instructor: bool,
        is_tow_pilot: bool,
        is_examiner: bool,
        club_id: Option<Uuid>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            first_name,
            last_name,
            email: None,         // No email = cannot login
            password_hash: None, // No password = cannot login
            email_verified: false,
            is_admin: false,
            club_id,
            is_licensed,
            is_instructor,
            is_tow_pilot,
            is_examiner,
            deleted_at: None,
            password_reset_token: None,
            password_reset_expires_at: None,
            email_verification_token: None,
            email_verification_expires_at: None,
            created_at: now,
            updated_at: now,
            settings: serde_json::json!({}),
        }
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
            is_licensed: self.is_licensed,
            is_instructor: self.is_instructor,
            is_tow_pilot: self.is_tow_pilot,
            is_examiner: self.is_examiner,
            can_login: self.can_login(),
            is_pilot: self.is_pilot(),
        }
    }
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        user.to_user_info()
    }
}
