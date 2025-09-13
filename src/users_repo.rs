use anyhow::Result;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::Utc;
use rand::Rng;
use sqlx::PgPool;
use uuid::Uuid;

use crate::users::{AccessLevel, CreateUserRequest, UpdateUserRequest, User};

pub struct UsersRepository {
    pool: PgPool,
}

impl UsersRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get user by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let result = sqlx::query!(
            r#"
            SELECT id, first_name, last_name, email, password_hash,
                   access_level as "access_level!: AccessLevel", club_id, email_verified,
                   password_reset_token, password_reset_expires_at,
                   email_verification_token, email_verification_expires_at,
                   created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|row| User {
            id: row.id,
            first_name: row.first_name,
            last_name: row.last_name,
            email: row.email,
            password_hash: row.password_hash,
            access_level: row.access_level,
            club_id: row.club_id,
            email_verified: row.email_verified.unwrap_or(false),
            password_reset_token: row.password_reset_token,
            password_reset_expires_at: row.password_reset_expires_at,
            email_verification_token: row.email_verification_token,
            email_verification_expires_at: row.email_verification_expires_at,
            created_at: row.created_at.unwrap_or_else(Utc::now),
            updated_at: row.updated_at.unwrap_or_else(Utc::now),
        }))
    }

    /// Get user by email
    pub async fn get_by_email(&self, email: &str) -> Result<Option<User>> {
        let result = sqlx::query!(
            r#"
            SELECT id, first_name, last_name, email, password_hash,
                   access_level as "access_level!: AccessLevel", club_id, email_verified,
                   password_reset_token, password_reset_expires_at,
                   email_verification_token, email_verification_expires_at,
                   created_at, updated_at
            FROM users
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|row| User {
            id: row.id,
            first_name: row.first_name,
            last_name: row.last_name,
            email: row.email,
            password_hash: row.password_hash,
            access_level: row.access_level,
            club_id: row.club_id,
            email_verified: row.email_verified.unwrap_or(false),
            password_reset_token: row.password_reset_token,
            password_reset_expires_at: row.password_reset_expires_at,
            email_verification_token: row.email_verification_token,
            email_verification_expires_at: row.email_verification_expires_at,
            created_at: row.created_at.unwrap_or_else(Utc::now),
            updated_at: row.updated_at.unwrap_or_else(Utc::now),
        }))
    }

    /// Get user by password reset token
    pub async fn get_by_reset_token(&self, token: &str) -> Result<Option<User>> {
        let result = sqlx::query!(
            r#"
            SELECT id, first_name, last_name, email, password_hash,
                   access_level as "access_level!: AccessLevel", club_id, email_verified,
                   password_reset_token, password_reset_expires_at,
                   email_verification_token, email_verification_expires_at,
                   created_at, updated_at
            FROM users
            WHERE password_reset_token = $1
            AND password_reset_expires_at > NOW()
            "#,
            token
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|row| User {
            id: row.id,
            first_name: row.first_name,
            last_name: row.last_name,
            email: row.email,
            password_hash: row.password_hash,
            access_level: row.access_level,
            club_id: row.club_id,
            email_verified: row.email_verified.unwrap_or(false),
            password_reset_token: row.password_reset_token,
            password_reset_expires_at: row.password_reset_expires_at,
            email_verification_token: row.email_verification_token,
            email_verification_expires_at: row.email_verification_expires_at,
            created_at: row.created_at.unwrap_or_else(Utc::now),
            updated_at: row.updated_at.unwrap_or_else(Utc::now),
        }))
    }

    /// Get all users (admin only)
    pub async fn get_all(&self, limit: Option<i64>) -> Result<Vec<User>> {
        let limit = limit.unwrap_or(100);

        let results = sqlx::query!(
            r#"
            SELECT id, first_name, last_name, email, password_hash,
                   access_level as "access_level!: AccessLevel", club_id, email_verified,
                   password_reset_token, password_reset_expires_at,
                   email_verification_token, email_verification_expires_at,
                   created_at, updated_at
            FROM users
            ORDER BY created_at DESC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(results
            .into_iter()
            .map(|row| User {
                id: row.id,
                first_name: row.first_name,
                last_name: row.last_name,
                email: row.email,
                password_hash: row.password_hash,
                access_level: row.access_level,
                club_id: row.club_id,
                email_verified: row.email_verified.unwrap_or(false),
                password_reset_token: row.password_reset_token,
                password_reset_expires_at: row.password_reset_expires_at,
                email_verification_token: row.email_verification_token,
                email_verification_expires_at: row.email_verification_expires_at,
                created_at: row.created_at.unwrap_or_else(Utc::now),
                updated_at: row.updated_at.unwrap_or_else(Utc::now),
            })
            .collect())
    }

    /// Get users by club ID
    pub async fn get_by_club_id(&self, club_id: Uuid) -> Result<Vec<User>> {
        let results = sqlx::query!(
            r#"
            SELECT id, first_name, last_name, email, password_hash,
                   access_level as "access_level!: AccessLevel", club_id, email_verified,
                   password_reset_token, password_reset_expires_at,
                   email_verification_token, email_verification_expires_at,
                   created_at, updated_at
            FROM users
            WHERE club_id = $1
            ORDER BY last_name, first_name
            "#,
            club_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(results
            .into_iter()
            .map(|row| User {
                id: row.id,
                first_name: row.first_name,
                last_name: row.last_name,
                email: row.email,
                password_hash: row.password_hash,
                access_level: row.access_level,
                club_id: row.club_id,
                email_verified: row.email_verified.unwrap_or(false),
                password_reset_token: row.password_reset_token,
                password_reset_expires_at: row.password_reset_expires_at,
                email_verification_token: row.email_verification_token,
                email_verification_expires_at: row.email_verification_expires_at,
                created_at: row.created_at.unwrap_or_else(Utc::now),
                updated_at: row.updated_at.unwrap_or_else(Utc::now),
            })
            .collect())
    }

    /// Create a new user
    pub async fn create_user(&self, request: &CreateUserRequest) -> Result<User> {
        // Hash password
        let password_hash = self.hash_password(&request.password)?;
        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let row = sqlx::query!(
            r#"
            INSERT INTO users (
                id, first_name, last_name, email, password_hash,
                access_level, club_id, email_verified, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, first_name, last_name, email, password_hash,
                      access_level as "access_level!: AccessLevel", club_id, email_verified,
                      password_reset_token, password_reset_expires_at,
                      email_verification_token, email_verification_expires_at,
                      created_at, updated_at
            "#,
            user_id,
            request.first_name,
            request.last_name,
            request.email,
            password_hash,
            AccessLevel::Standard as AccessLevel,
            request.club_id,
            false, // email_verified defaults to false
            now,
            now
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(User {
            id: row.id,
            first_name: row.first_name,
            last_name: row.last_name,
            email: row.email,
            password_hash: row.password_hash,
            access_level: row.access_level,
            club_id: row.club_id,
            email_verified: row.email_verified.unwrap_or(false),
            password_reset_token: row.password_reset_token,
            password_reset_expires_at: row.password_reset_expires_at,
            email_verification_token: row.email_verification_token,
            email_verification_expires_at: row.email_verification_expires_at,
            created_at: row.created_at.unwrap_or_else(Utc::now),
            updated_at: row.updated_at.unwrap_or_else(Utc::now),
        })
    }

    /// Update user
    pub async fn update_user(
        &self,
        user_id: Uuid,
        request: &UpdateUserRequest,
    ) -> Result<Option<User>> {
        // Build dynamic update query
        let mut query_builder = sqlx::QueryBuilder::new("UPDATE users SET updated_at = NOW()");
        let mut has_updates = false;

        if let Some(first_name) = &request.first_name {
            query_builder.push(", first_name = ");
            query_builder.push_bind(first_name);
            has_updates = true;
        }

        if let Some(last_name) = &request.last_name {
            query_builder.push(", last_name = ");
            query_builder.push_bind(last_name);
            has_updates = true;
        }

        if let Some(email) = &request.email {
            query_builder.push(", email = ");
            query_builder.push_bind(email);
            has_updates = true;
        }

        if let Some(access_level) = &request.access_level {
            query_builder.push(", access_level = ");
            query_builder.push_bind(access_level);
            has_updates = true;
        }

        if let Some(club_id) = &request.club_id {
            query_builder.push(", club_id = ");
            query_builder.push_bind(club_id);
            has_updates = true;
        }

        if let Some(email_verified) = &request.email_verified {
            query_builder.push(", email_verified = ");
            query_builder.push_bind(email_verified);
            has_updates = true;
        }

        if !has_updates {
            // No updates to make, just return current user
            return self.get_by_id(user_id).await;
        }

        query_builder.push(" WHERE id = ");
        query_builder.push_bind(user_id);

        let update_query = query_builder.build();
        let rows_affected = update_query.execute(&self.pool).await?.rows_affected();

        if rows_affected > 0 {
            self.get_by_id(user_id).await
        } else {
            Ok(None)
        }
    }

    /// Delete user
    pub async fn delete_user(&self, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
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

        let result = sqlx::query!(
            r#"
            UPDATE users 
            SET password_hash = $2, 
                password_reset_token = NULL,
                password_reset_expires_at = NULL,
                updated_at = NOW()
            WHERE id = $1
            "#,
            user_id,
            password_hash
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Set password reset token
    pub async fn set_password_reset_token(&self, user_id: Uuid) -> Result<String> {
        let token = self.generate_reset_token();
        let expires_at = Utc::now() + chrono::Duration::hours(1); // Token expires in 1 hour

        sqlx::query!(
            r#"
            UPDATE users 
            SET password_reset_token = $2,
                password_reset_expires_at = $3,
                updated_at = NOW()
            WHERE id = $1
            "#,
            user_id,
            token,
            expires_at
        )
        .execute(&self.pool)
        .await?;

        Ok(token)
    }

    /// Clear password reset token
    pub async fn clear_password_reset_token(&self, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            UPDATE users 
            SET password_reset_token = NULL,
                password_reset_expires_at = NULL,
                updated_at = NOW()
            WHERE id = $1
            "#,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Verify email
    pub async fn verify_email(&self, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            UPDATE users 
            SET email_verified = true,
                updated_at = NOW()
            WHERE id = $1
            "#,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
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

        sqlx::query!(
            r#"
            UPDATE users 
            SET email_verification_token = $2,
                email_verification_expires_at = $3,
                updated_at = NOW()
            WHERE id = $1
            "#,
            user_id,
            token,
            expires_at
        )
        .execute(&self.pool)
        .await?;

        Ok(token)
    }

    /// Get user by email verification token
    pub async fn get_by_verification_token(&self, token: &str) -> Result<Option<User>> {
        let result = sqlx::query!(
            r#"
            SELECT id, first_name, last_name, email, password_hash,
                   access_level as "access_level!: AccessLevel", club_id, email_verified,
                   password_reset_token, password_reset_expires_at,
                   email_verification_token, email_verification_expires_at,
                   created_at, updated_at
            FROM users
            WHERE email_verification_token = $1
            AND email_verification_expires_at > NOW()
            "#,
            token
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|row| User {
            id: row.id,
            first_name: row.first_name,
            last_name: row.last_name,
            email: row.email,
            password_hash: row.password_hash,
            access_level: row.access_level,
            club_id: row.club_id,
            email_verified: row.email_verified.unwrap_or(false),
            password_reset_token: row.password_reset_token,
            password_reset_expires_at: row.password_reset_expires_at,
            email_verification_token: row.email_verification_token,
            email_verification_expires_at: row.email_verification_expires_at,
            created_at: row.created_at.unwrap_or_else(Utc::now),
            updated_at: row.updated_at.unwrap_or_else(Utc::now),
        }))
    }

    /// Mark user's email as verified
    pub async fn verify_user_email(&self, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            UPDATE users 
            SET email_verified = true,
                email_verification_token = NULL,
                email_verification_expires_at = NULL,
                updated_at = NOW()
            WHERE id = $1
            "#,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
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
