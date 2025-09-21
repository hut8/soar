use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use tracing::error;

use crate::auth::{AuthUser, JwtService, get_jwt_secret};
use crate::email::EmailService;
use crate::users_repo::UsersRepository;
use crate::web::AppState;

use super::{
    json_error,
    views::{
        CreateUserRequest, EmailVerificationConfirm, LoginRequest, LoginResponse,
        PasswordResetConfirm, PasswordResetRequest, UserView,
    },
};

pub async fn register_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool.clone());

    // Check if user already exists
    if let Ok(Some(_)) = users_repo.get_by_email(&payload.email).await {
        return json_error(StatusCode::CONFLICT, "User with this email already exists")
            .into_response();
    }

    // Convert view request to domain request
    let domain_request = crate::users::CreateUserRequest {
        first_name: payload.first_name,
        last_name: payload.last_name,
        email: payload.email,
        password: payload.password,
        club_id: payload.club_id,
    };

    // Create user
    match users_repo.create_user(&domain_request).await {
        Ok(user) => {
            // Generate and send email verification token
            match users_repo.set_email_verification_token(user.id).await {
                Ok(token) => {
                    // Send email verification email
                    if let Ok(email_service) = EmailService::new() {
                        if let Err(e) = email_service
                            .send_email_verification(&user.email, &user.full_name(), &token)
                            .await
                        {
                            error!("Failed to send email verification: {}", e);
                            return json_error(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to send email verification",
                            )
                            .into_response();
                        }
                    } else {
                        return json_error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Email service not configured",
                        )
                        .into_response();
                    }

                    Json(serde_json::json!({
                        "message": "User created. Please check your email to verify your account."
                    }))
                    .into_response()
                }
                Err(e) => {
                    error!("Failed to generate email verification token: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to generate email verification token",
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            error!("Failed to create user: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create user").into_response()
        }
    }
}

pub async fn login_user(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    match users_repo
        .verify_password(&payload.email, &payload.password)
        .await
    {
        Ok(Some(user)) => {
            // Check if email is verified
            if !user.email_verified {
                // Generate new verification token and resend email
                match users_repo.set_email_verification_token(user.id).await {
                    Ok(token) => {
                        // Send new email verification email
                        if let Ok(email_service) = EmailService::new()
                            && let Err(e) = email_service
                                .send_email_verification(&user.email, &user.full_name(), &token)
                                .await
                        {
                            error!("Failed to send email verification: {}", e);
                        }
                        return json_error(
                            StatusCode::FORBIDDEN,
                            "Email not verified. A new verification email has been sent to your email address.",
                        ).into_response();
                    }
                    Err(e) => {
                        error!("Failed to generate email verification token: {}", e);
                        return json_error(
                            StatusCode::FORBIDDEN,
                            "Email not verified. Please contact support.",
                        )
                        .into_response();
                    }
                }
            }

            // Generate JWT token
            match get_jwt_secret() {
                Ok(secret) => {
                    let jwt_service = JwtService::new(&secret);
                    match jwt_service.generate_token(&user) {
                        Ok(token) => {
                            let response = LoginResponse {
                                token,
                                user: UserView::from(user),
                            };
                            Json(response).into_response()
                        }
                        Err(e) => {
                            error!("Failed to generate token: {}", e);
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to generate authentication token",
                            )
                                .into_response()
                        }
                    }
                }
                Err(e) => {
                    error!("JWT secret not configured: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Authentication configuration error",
                    )
                        .into_response()
                }
            }
        }
        Ok(None) => (StatusCode::UNAUTHORIZED, "Invalid credentials").into_response(),
        Err(e) => {
            error!("Authentication error: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Authentication failed").into_response()
        }
    }
}

pub async fn get_current_user(auth_user: AuthUser) -> impl IntoResponse {
    Json(UserView::from(auth_user.0))
}

pub async fn verify_email(
    State(state): State<AppState>,
    Json(payload): Json<EmailVerificationConfirm>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    match users_repo.get_by_verification_token(&payload.token).await {
        Ok(Some(user)) => match users_repo.verify_user_email(user.id).await {
            Ok(true) => Json(serde_json::json!({
                "message": "Email verified successfully"
            }))
            .into_response(),
            Ok(false) => (StatusCode::NOT_FOUND, "User not found").into_response(),
            Err(e) => {
                error!("Failed to verify email: {}", e);
                json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to verify email")
                    .into_response()
            }
        },
        Ok(None) => (
            StatusCode::BAD_REQUEST,
            "Invalid or expired verification token",
        )
            .into_response(),
        Err(e) => {
            error!("Database error during email verification: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to verify email").into_response()
        }
    }
}

pub async fn request_password_reset(
    State(state): State<AppState>,
    Json(payload): Json<PasswordResetRequest>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    match users_repo.get_by_email(&payload.email).await {
        Ok(Some(user)) => match users_repo.set_password_reset_token(user.id).await {
            Ok(token) => {
                if let Ok(email_service) = EmailService::new() {
                    if let Err(e) = email_service
                        .send_password_reset_email(&user.email, &user.full_name(), &token)
                        .await
                    {
                        error!("Failed to send password reset email: {}", e);
                        return json_error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Failed to send password reset email",
                        )
                        .into_response();
                    }
                } else {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Email service not configured",
                    )
                        .into_response();
                }

                json_error(StatusCode::OK, "Password reset email sent").into_response()
            }
            Err(e) => {
                error!("Failed to generate password reset token: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to generate password reset token",
                )
                    .into_response()
            }
        },
        Ok(None) => {
            // Don't reveal if user exists or not for security
            json_error(StatusCode::OK, "Password reset email sent").into_response()
        }
        Err(e) => {
            error!("Database error during password reset request: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to request password reset",
            )
                .into_response()
        }
    }
}

pub async fn confirm_password_reset(
    State(state): State<AppState>,
    Json(payload): Json<PasswordResetConfirm>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    match users_repo.get_by_reset_token(&payload.token).await {
        Ok(Some(user)) => {
            match users_repo
                .update_password(user.id, &payload.new_password)
                .await
            {
                Ok(true) => (StatusCode::OK, "Password updated successfully").into_response(),
                Ok(false) => (StatusCode::NOT_FOUND, "User not found").into_response(),
                Err(e) => {
                    error!("Failed to update password: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to update password",
                    )
                        .into_response()
                }
            }
        }
        Ok(None) => (StatusCode::BAD_REQUEST, "Invalid or expired reset token").into_response(),
        Err(e) => {
            error!("Database error during password reset: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to reset password",
            )
                .into_response()
        }
    }
}
