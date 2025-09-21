use anyhow::Result;
use axum::{
    RequestPartsExt,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{users::User, users_repo::UsersRepository, web::AppState};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user ID
    pub email: String,
    pub is_admin: bool,
    pub club_id: Option<Uuid>,
    pub exp: i64, // expiration timestamp
    pub iat: i64, // issued at timestamp
}

impl Claims {
    pub fn new(user: &User) -> Self {
        let now = Utc::now();
        let exp = now + Duration::days(7); // Token expires in 7 days

        Self {
            sub: user.id.to_string(),
            email: user.email.clone(),
            is_admin: user.is_admin,
            club_id: user.club_id,
            exp: exp.timestamp(),
            iat: now.timestamp(),
        }
    }

    pub fn user_id(&self) -> Result<Uuid> {
        self.sub
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid user ID: {}", e))
    }
}

pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtService {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
        }
    }

    pub fn generate_token(&self, user: &User) -> Result<String> {
        let claims = Claims::new(user);
        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| anyhow::anyhow!("Failed to generate token: {}", e))
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        decode::<Claims>(token, &self.decoding_key, &Validation::default())
            .map(|data| data.claims)
            .map_err(|e| anyhow::anyhow!("Failed to verify token: {}", e))
    }
}

#[derive(Debug)]
pub struct AuthUser(pub User);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract the authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthError::MissingToken)?;

        // Get JWT secret from environment
        let jwt_secret = std::env::var("JWT_SECRET").map_err(|_| AuthError::MissingJwtSecret)?;

        // Create JWT service and verify token
        let jwt_service = JwtService::new(&jwt_secret);
        let claims = jwt_service
            .verify_token(bearer.token())
            .map_err(|_| AuthError::InvalidToken)?;

        // Get user from database
        let users_repo = UsersRepository::new(state.pool.clone());
        let user_id = claims.user_id().map_err(|_| AuthError::InvalidToken)?;

        let user = users_repo
            .get_by_id(user_id)
            .await
            .map_err(|_| AuthError::DatabaseError)?
            .ok_or(AuthError::UserNotFound)?;

        Ok(AuthUser(user))
    }
}

#[derive(Debug)]
pub struct AdminUser(pub User);

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let AuthUser(user) = AuthUser::from_request_parts(parts, state).await?;

        if !user.is_admin {
            return Err(AuthError::InsufficientPermissions);
        }

        Ok(AdminUser(user))
    }
}

#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
    MissingJwtSecret,
    DatabaseError,
    UserNotFound,
    InsufficientPermissions,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authorization token"),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid token"),
            AuthError::MissingJwtSecret => {
                (StatusCode::INTERNAL_SERVER_ERROR, "JWT configuration error")
            }
            AuthError::DatabaseError => (StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
            AuthError::UserNotFound => (StatusCode::UNAUTHORIZED, "User not found"),
            AuthError::InsufficientPermissions => {
                (StatusCode::FORBIDDEN, "Insufficient permissions")
            }
        };
        (status, error_message).into_response()
    }
}

pub fn get_jwt_secret() -> Result<String> {
    std::env::var("JWT_SECRET")
        .map_err(|_| anyhow::anyhow!("JWT_SECRET environment variable not set"))
}
