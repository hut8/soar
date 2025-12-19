use anyhow::Result;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use rand::Rng;
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::schema::users;
use crate::users::{CreateUserRequest, UpdateUserRequest, User};

type PgPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserRecord {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,         // Nullable
    pub password_hash: Option<String>, // Nullable
    pub is_admin: bool,
    pub club_id: Option<Uuid>,
    pub email_verified: bool,
    pub password_reset_token: Option<String>,
    pub password_reset_expires_at: Option<DateTime<Utc>>,
    pub email_verification_token: Option<String>,
    pub email_verification_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub settings: JsonValue,
    pub is_licensed: bool,
    pub is_instructor: bool,
    pub is_tow_pilot: bool,
    pub is_examiner: bool,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewUser {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub password_hash: Option<String>,
    pub is_admin: bool,
    pub club_id: Option<Uuid>,
    pub is_licensed: bool,
    pub is_instructor: bool,
    pub is_tow_pilot: bool,
    pub is_examiner: bool,
}

impl From<UserRecord> for User {
    fn from(record: UserRecord) -> Self {
        User {
            id: record.id,
            first_name: record.first_name,
            last_name: record.last_name,
            email: record.email,
            password_hash: record.password_hash,
            is_admin: record.is_admin,
            club_id: record.club_id,
            email_verified: record.email_verified,
            password_reset_token: record.password_reset_token,
            password_reset_expires_at: record.password_reset_expires_at,
            email_verification_token: record.email_verification_token,
            email_verification_expires_at: record.email_verification_expires_at,
            created_at: record.created_at,
            updated_at: record.updated_at,
            settings: record.settings,
            is_licensed: record.is_licensed,
            is_instructor: record.is_instructor,
            is_tow_pilot: record.is_tow_pilot,
            is_examiner: record.is_examiner,
            deleted_at: record.deleted_at,
        }
    }
}

pub struct UsersRepository {
    pool: PgPool,
}

impl UsersRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get user by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<Option<User>> {
            let mut conn = pool.get()?;
            let user = users::table
                .filter(users::id.eq(id))
                .filter(users::deleted_at.is_null()) // Exclude soft-deleted users
                .first::<UserRecord>(&mut conn)
                .optional()?;
            Ok(user.map(|record| record.into()))
        })
        .await?
    }

    /// Get user by email
    pub async fn get_by_email(&self, email: &str) -> Result<Option<User>> {
        let pool = self.pool.clone();
        let email = email.to_string();
        tokio::task::spawn_blocking(move || -> Result<Option<User>> {
            let mut conn = pool.get()?;
            let user = users::table
                .filter(users::email.eq(&email))
                .first::<UserRecord>(&mut conn)
                .optional()?;
            Ok(user.map(|record| record.into()))
        })
        .await?
    }

    /// Get user by password reset token
    pub async fn get_by_reset_token(&self, token: &str) -> Result<Option<User>> {
        let pool = self.pool.clone();
        let token = token.to_string();
        tokio::task::spawn_blocking(move || -> Result<Option<User>> {
            let mut conn = pool.get()?;
            let user = users::table
                .filter(users::password_reset_token.eq(&token))
                .filter(users::password_reset_expires_at.gt(Utc::now()))
                .first::<UserRecord>(&mut conn)
                .optional()?;
            Ok(user.map(|record| record.into()))
        })
        .await?
    }

    /// Get all users (admin only)
    pub async fn get_all(&self, limit: Option<i64>) -> Result<Vec<User>> {
        let pool = self.pool.clone();
        let limit = limit.unwrap_or(100);
        tokio::task::spawn_blocking(move || -> Result<Vec<User>> {
            let mut conn = pool.get()?;
            let users = users::table
                .order(users::created_at.desc())
                .limit(limit)
                .load::<UserRecord>(&mut conn)?;
            Ok(users.into_iter().map(|record| record.into()).collect())
        })
        .await?
    }

    /// Get users by club ID
    pub async fn get_by_club_id(&self, club_id: Uuid) -> Result<Vec<User>> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<User>> {
            let mut conn = pool.get()?;
            let users = users::table
                .filter(users::club_id.eq(club_id))
                .order((users::last_name.asc(), users::first_name.asc()))
                .load::<UserRecord>(&mut conn)?;
            Ok(users.into_iter().map(|record| record.into()).collect())
        })
        .await?
    }

    /// Create a new user
    pub async fn create_user(&self, request: &CreateUserRequest) -> Result<User> {
        // Hash password
        let password_hash = self.hash_password(&request.password)?;
        let user_id = Uuid::now_v7();

        let pool = self.pool.clone();
        let new_user = NewUser {
            id: user_id,
            first_name: request.first_name.clone(),
            last_name: request.last_name.clone(),
            email: Some(request.email.clone()),
            password_hash: Some(password_hash),
            is_admin: false,
            club_id: request.club_id,
            is_licensed: false,
            is_instructor: false,
            is_tow_pilot: false,
            is_examiner: false,
        };

        tokio::task::spawn_blocking(move || -> Result<User> {
            let mut conn = pool.get()?;
            let user = diesel::insert_into(users::table)
                .values(&new_user)
                .get_result::<UserRecord>(&mut conn)?;
            Ok(user.into())
        })
        .await?
    }

    /// Update user
    pub async fn update_user(
        &self,
        user_id: Uuid,
        request: &UpdateUserRequest,
    ) -> Result<Option<User>> {
        let pool = self.pool.clone();
        let request_clone = request.clone();

        tokio::task::spawn_blocking(move || -> Result<Option<User>> {
            let mut conn = pool.get()?;
            let now = Utc::now();

            // Check if any field needs updating
            let has_updates = request_clone.first_name.is_some()
                || request_clone.last_name.is_some()
                || request_clone.email.is_some()
                || request_clone.is_admin.is_some()
                || request_clone.club_id.is_some()
                || request_clone.email_verified.is_some();

            if !has_updates {
                // No updates to make, just return current user
                let user = users::table
                    .filter(users::id.eq(user_id))
                    .first::<UserRecord>(&mut conn)
                    .optional()?;
                return Ok(user.map(|record| record.into()));
            }

            // Fetch the current user to merge with updates
            let current_user = users::table
                .filter(users::id.eq(user_id))
                .first::<UserRecord>(&mut conn)
                .optional()?;

            if let Some(current) = current_user {
                let rows_affected = diesel::update(users::table.filter(users::id.eq(user_id)))
                    .set((
                        users::first_name
                            .eq(request_clone.first_name.unwrap_or(current.first_name)),
                        users::last_name.eq(request_clone.last_name.unwrap_or(current.last_name)),
                        users::email.eq(request_clone.email.or(current.email)),
                        users::is_admin.eq(request_clone.is_admin.unwrap_or(current.is_admin)),
                        users::club_id.eq(request_clone.club_id.or(current.club_id)),
                        users::email_verified.eq(request_clone
                            .email_verified
                            .unwrap_or(current.email_verified)),
                        users::updated_at.eq(now),
                    ))
                    .execute(&mut conn)?;

                if rows_affected > 0 {
                    let updated_user = users::table
                        .filter(users::id.eq(user_id))
                        .first::<UserRecord>(&mut conn)
                        .optional()?;
                    Ok(updated_user.map(|record| record.into()))
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        })
        .await?
    }

    /// Delete user
    pub async fn delete_user(&self, user_id: Uuid) -> Result<bool> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;
            let rows_affected =
                diesel::delete(users::table.filter(users::id.eq(user_id))).execute(&mut conn)?;
            Ok(rows_affected > 0)
        })
        .await?
    }

    /// Verify user password
    pub async fn verify_password(&self, email: &str, password: &str) -> Result<Option<User>> {
        let user = self.get_by_email(email).await?;

        if let Some(user) = user {
            if let Some(password_hash) = &user.password_hash {
                if self.verify_password_hash(password_hash, password)? {
                    Ok(Some(user))
                } else {
                    Ok(None)
                }
            } else {
                // User has no password (pilot-only account)
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Update user password
    pub async fn update_password(&self, user_id: Uuid, new_password: &str) -> Result<bool> {
        let password_hash = self.hash_password(new_password)?;
        let pool = self.pool.clone();
        let now = Utc::now();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;
            let rows_affected = diesel::update(users::table.filter(users::id.eq(user_id)))
                .set((
                    users::password_hash.eq(password_hash),
                    users::password_reset_token.eq(None::<String>),
                    users::password_reset_expires_at.eq(None::<DateTime<Utc>>),
                    users::updated_at.eq(now),
                ))
                .execute(&mut conn)?;
            Ok(rows_affected > 0)
        })
        .await?
    }

    /// Set password reset token
    pub async fn set_password_reset_token(&self, user_id: Uuid) -> Result<String> {
        let token = self.generate_reset_token();
        let expires_at = Utc::now() + chrono::Duration::hours(1); // Token expires in 1 hour
        let pool = self.pool.clone();
        let token_clone = token.clone();
        let now = Utc::now();

        tokio::task::spawn_blocking(move || -> Result<String> {
            let mut conn = pool.get()?;
            diesel::update(users::table.filter(users::id.eq(user_id)))
                .set((
                    users::password_reset_token.eq(Some(token_clone.clone())),
                    users::password_reset_expires_at.eq(Some(expires_at)),
                    users::updated_at.eq(now),
                ))
                .execute(&mut conn)?;
            Ok(token_clone)
        })
        .await?
    }

    /// Verify email
    pub async fn verify_email(&self, user_id: Uuid) -> Result<bool> {
        let pool = self.pool.clone();
        let now = Utc::now();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;
            let rows_affected = diesel::update(users::table.filter(users::id.eq(user_id)))
                .set((users::email_verified.eq(true), users::updated_at.eq(now)))
                .execute(&mut conn)?;
            Ok(rows_affected > 0)
        })
        .await?
    }

    /// Hash password using Argon2
    fn hash_password(&self, password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;

        Ok(password_hash.to_string())
    }

    /// Verify password against hash
    fn verify_password_hash(&self, hash: &str, password: &str) -> Result<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| anyhow::anyhow!("Failed to parse password hash: {}", e))?;

        let argon2 = Argon2::default();

        Ok(argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    /// Generate a random password reset token
    fn generate_reset_token(&self) -> String {
        let mut rng = rand::rng();
        let token: String = (0..32)
            .map(|_| rng.sample(rand::distr::Alphanumeric) as char)
            .collect();
        token
    }

    /// Set email verification token
    pub async fn set_email_verification_token(&self, user_id: Uuid) -> Result<String> {
        let token = self.generate_verification_token();
        let expires_at = Utc::now() + chrono::Duration::hours(24); // Token expires in 24 hours
        let pool = self.pool.clone();
        let token_clone = token.clone();
        let now = Utc::now();

        tokio::task::spawn_blocking(move || -> Result<String> {
            let mut conn = pool.get()?;
            diesel::update(users::table.filter(users::id.eq(user_id)))
                .set((
                    users::email_verification_token.eq(Some(token_clone.clone())),
                    users::email_verification_expires_at.eq(Some(expires_at)),
                    users::updated_at.eq(now),
                ))
                .execute(&mut conn)?;
            Ok(token_clone)
        })
        .await?
    }

    /// Get user by email verification token
    pub async fn get_by_verification_token(&self, token: &str) -> Result<Option<User>> {
        let pool = self.pool.clone();
        let token = token.to_string();
        tokio::task::spawn_blocking(move || -> Result<Option<User>> {
            let mut conn = pool.get()?;
            let user = users::table
                .filter(users::email_verification_token.eq(&token))
                .filter(users::email_verification_expires_at.gt(Utc::now()))
                .first::<UserRecord>(&mut conn)
                .optional()?;
            Ok(user.map(|record| record.into()))
        })
        .await?
    }

    /// Mark user's email as verified
    pub async fn verify_user_email(&self, user_id: Uuid) -> Result<bool> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;
            let rows_affected = diesel::update(users::table.filter(users::id.eq(user_id)))
                .set((
                    users::email_verified.eq(true),
                    users::email_verification_token.eq(None::<String>),
                    users::email_verification_expires_at.eq(None::<DateTime<Utc>>),
                ))
                .execute(&mut conn)?;
            Ok(rows_affected > 0)
        })
        .await?
    }

    /// Generate a random email verification token
    fn generate_verification_token(&self) -> String {
        let mut rng = rand::rng();
        let token: String = (0..32)
            .map(|_| rng.sample(rand::distr::Alphanumeric) as char)
            .collect();
        token
    }

    /// Update user settings
    pub async fn update_user_settings(&self, user_id: Uuid, settings: JsonValue) -> Result<bool> {
        let pool = self.pool.clone();
        let now = Utc::now();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;
            let rows_affected = diesel::update(users::table.filter(users::id.eq(user_id)))
                .set((users::settings.eq(settings), users::updated_at.eq(now)))
                .execute(&mut conn)?;
            Ok(rows_affected > 0)
        })
        .await?
    }

    // === NEW METHODS FOR PILOT MANAGEMENT ===

    /// Create a pilot without login capability (no email/password)
    pub async fn create_pilot(&self, pilot: User) -> Result<User> {
        // Validate that this is a pilot-only user (no email/password)
        if pilot.email.is_some() || pilot.password_hash.is_some() {
            return Err(anyhow::anyhow!(
                "Use create_user for users with login capability"
            ));
        }

        let pool = self.pool.clone();
        let new_pilot = NewUser {
            id: pilot.id,
            first_name: pilot.first_name.clone(),
            last_name: pilot.last_name.clone(),
            email: None,
            password_hash: None,
            is_admin: false,
            club_id: pilot.club_id,
            is_licensed: pilot.is_licensed,
            is_instructor: pilot.is_instructor,
            is_tow_pilot: pilot.is_tow_pilot,
            is_examiner: pilot.is_examiner,
        };

        tokio::task::spawn_blocking(move || -> Result<User> {
            let mut conn = pool.get()?;
            let user = diesel::insert_into(users::table)
                .values(&new_pilot)
                .get_result::<UserRecord>(&mut conn)?;
            Ok(user.into())
        })
        .await?
    }

    /// Get all pilots (users with pilot qualifications) for a club
    /// Excludes soft-deleted users
    pub async fn get_pilots_by_club(&self, club_id: Uuid) -> Result<Vec<User>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<User>> {
            let mut conn = pool.get()?;
            let pilots = users::table
                .filter(users::club_id.eq(Some(club_id)))
                .filter(users::deleted_at.is_null())
                .filter(
                    users::is_licensed
                        .eq(true)
                        .or(users::is_instructor.eq(true))
                        .or(users::is_tow_pilot.eq(true))
                        .or(users::is_examiner.eq(true)),
                )
                .order((users::last_name.asc(), users::first_name.asc()))
                .load::<UserRecord>(&mut conn)?;
            Ok(pilots.into_iter().map(|r| r.into()).collect())
        })
        .await?
    }

    /// Soft delete a user
    pub async fn soft_delete_user(&self, user_id: Uuid) -> Result<bool> {
        let pool = self.pool.clone();
        let now = Utc::now();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;
            let rows = diesel::update(users::table.filter(users::id.eq(user_id)))
                .set(users::deleted_at.eq(Some(now)))
                .execute(&mut conn)?;
            Ok(rows > 0)
        })
        .await?
    }

    /// Set email and generate verification token for a user (for invitation flow)
    pub async fn set_email_and_generate_token(&self, user_id: Uuid, email: &str) -> Result<String> {
        let token = self.generate_verification_token();
        let expires_at = Utc::now() + chrono::Duration::hours(72); // 3 days for invitation
        let pool = self.pool.clone();
        let email = email.to_string();
        let token_clone = token.clone();
        let now = Utc::now();

        tokio::task::spawn_blocking(move || -> Result<String> {
            let mut conn = pool.get()?;
            diesel::update(users::table.filter(users::id.eq(user_id)))
                .set((
                    users::email.eq(Some(email)),
                    users::email_verification_token.eq(Some(token_clone.clone())),
                    users::email_verification_expires_at.eq(Some(expires_at)),
                    users::updated_at.eq(now),
                ))
                .execute(&mut conn)?;
            Ok(token_clone)
        })
        .await?
    }

    /// Set password and verify email (for completing pilot registration)
    pub async fn set_password_and_verify_email(
        &self,
        user_id: Uuid,
        password: &str,
    ) -> Result<bool> {
        let password_hash = self.hash_password(password)?;
        let pool = self.pool.clone();
        let now = Utc::now();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;
            let rows = diesel::update(users::table.filter(users::id.eq(user_id)))
                .set((
                    users::password_hash.eq(Some(password_hash)),
                    users::email_verified.eq(true),
                    users::email_verification_token.eq(None::<String>),
                    users::email_verification_expires_at.eq(None::<DateTime<Utc>>),
                    users::updated_at.eq(now),
                ))
                .execute(&mut conn)?;
            Ok(rows > 0)
        })
        .await?
    }
}
