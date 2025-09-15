use anyhow::Result;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use rand::Rng;
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
    pub email: String,
    pub password_hash: String,
    pub is_admin: bool,
    pub club_id: Option<Uuid>,
    pub email_verified: bool,
    pub password_reset_token: Option<String>,
    pub password_reset_expires_at: Option<DateTime<Utc>>,
    pub email_verification_token: Option<String>,
    pub email_verification_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewUser {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password_hash: String,
    pub is_admin: bool,
    pub club_id: Option<Uuid>,
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
        let user_id = Uuid::new_v4();

        let pool = self.pool.clone();
        let new_user = NewUser {
            id: user_id,
            first_name: request.first_name.clone(),
            last_name: request.last_name.clone(),
            email: request.email.clone(),
            password_hash,
            is_admin: false,
            club_id: request.club_id,
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
                        users::email.eq(request_clone.email.unwrap_or(current.email)),
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
            if self.verify_password_hash(&user.password_hash, password)? {
                Ok(Some(user))
            } else {
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

    /// Clear password reset token
    pub async fn clear_password_reset_token(&self, user_id: Uuid) -> Result<bool> {
        let pool = self.pool.clone();
        let now = Utc::now();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;
            let rows_affected = diesel::update(users::table.filter(users::id.eq(user_id)))
                .set((
                    users::password_reset_token.eq(None::<String>),
                    users::password_reset_expires_at.eq(None::<DateTime<Utc>>),
                    users::updated_at.eq(now),
                ))
                .execute(&mut conn)?;
            Ok(rows_affected > 0)
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
        let mut rng = rand::thread_rng();
        let token: String = (0..32)
            .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
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
        let mut rng = rand::thread_rng();
        let token: String = (0..32)
            .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
            .collect();
        token
    }
}
