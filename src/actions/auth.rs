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
use crate::users::CompletePilotRegistrationRequest;

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
                    let email = user
                        .email
                        .as_ref()
                        .expect("User must have email for registration");
                    match EmailService::new() {
                        Ok(email_service) => {
                            if let Err(e) = email_service
                                .send_email_verification(email, &user.full_name(), &token)
                                .await
                            {
                                error!(error = %e, "Failed to send email verification");
                                return json_error(
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    "Failed to send email verification",
                                )
                                .into_response();
                            }

                            // Send admin notification about new user signup (non-blocking)
                            let first_name = user.first_name.clone();
                            let last_name = user.last_name.clone();
                            let user_email = email.clone();
                            let club_id = user.club_id;
                            tokio::spawn(async move {
                                if let Ok(admin_email_service) = EmailService::new()
                                    && let Err(e) = admin_email_service
                                        .send_user_signup_notification(
                                            &first_name,
                                            &last_name,
                                            &user_email,
                                            club_id,
                                        )
                                        .await
                                {
                                    // Log but don't fail - admin notification is not critical
                                    tracing::warn!(
                                        "Failed to send admin signup notification: {}",
                                        e
                                    );
                                }
                            });
                        }
                        Err(e) => {
                            error!(error = %e, "Email service initialization failed");
                            return json_error(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Email service not configured",
                            )
                            .into_response();
                        }
                    }

                    Json(serde_json::json!({
                        "message": "User created. Please check your email to verify your account."
                    }))
                    .into_response()
                }
                Err(e) => {
                    error!(error = %e, "Failed to generate email verification token");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to generate email verification token",
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            error!(error = %e, "Failed to create user");
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
                let email = user.email.as_ref().expect("User must have email to login");
                match users_repo.set_email_verification_token(user.id).await {
                    Ok(token) => {
                        // Send new email verification email
                        if let Ok(email_service) = EmailService::new()
                            && let Err(e) = email_service
                                .send_email_verification(email, &user.full_name(), &token)
                                .await
                        {
                            error!(error = %e, "Failed to send email verification");
                        }
                        return json_error(
                            StatusCode::FORBIDDEN,
                            "Email not verified. A new verification email has been sent to your email address.",
                        ).into_response();
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to generate email verification token");
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
                            error!(error = %e, "Failed to generate token");
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to generate authentication token",
                            )
                                .into_response()
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "JWT secret not configured");
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
            error!(error = %e, "Authentication error");
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
            Ok(true) => {
                // Get the updated user with verified email
                let verified_user = match users_repo.get_by_id(user.id).await {
                    Ok(Some(user)) => user,
                    Ok(None) => {
                        return json_error(StatusCode::NOT_FOUND, "User not found").into_response();
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to get user after verification");
                        return json_error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Failed to verify email",
                        )
                        .into_response();
                    }
                };

                // Generate JWT token for immediate login
                match get_jwt_secret() {
                    Ok(secret) => {
                        let jwt_service = JwtService::new(&secret);
                        match jwt_service.generate_token(&verified_user) {
                            Ok(token) => {
                                let response = LoginResponse {
                                    token,
                                    user: UserView::from(verified_user),
                                };
                                Json(response).into_response()
                            }
                            Err(e) => {
                                error!(error = %e, "Failed to generate token");
                                json_error(
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    "Failed to generate authentication token",
                                )
                                .into_response()
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "JWT secret not configured");
                        json_error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Authentication configuration error",
                        )
                        .into_response()
                    }
                }
            }
            Ok(false) => (StatusCode::NOT_FOUND, "User not found").into_response(),
            Err(e) => {
                error!(error = %e, "Failed to verify email");
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
            error!(error = %e, "Database error during email verification");
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
        Ok(Some(user)) => {
            let email = user
                .email
                .as_ref()
                .expect("User must have email for password reset");
            match users_repo.set_password_reset_token(user.id).await {
                Ok(token) => {
                    if let Ok(email_service) = EmailService::new() {
                        if let Err(e) = email_service
                            .send_password_reset_email(email, &user.full_name(), &token)
                            .await
                        {
                            error!(error = %e, "Failed to send password reset email");
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
                    error!(error = %e, "Failed to generate password reset token");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to generate password reset token",
                    )
                        .into_response()
                }
            }
        }
        Ok(None) => {
            // Don't reveal if user exists or not for security
            json_error(StatusCode::OK, "Password reset email sent").into_response()
        }
        Err(e) => {
            error!(error = %e, "Database error during password reset request");
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
                Ok(true) => StatusCode::NO_CONTENT.into_response(),
                Ok(false) => (StatusCode::NOT_FOUND, "User not found").into_response(),
                Err(e) => {
                    error!(error = %e, "Failed to update password");
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
            error!(error = %e, "Database error during password reset");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to reset password",
            )
                .into_response()
        }
    }
}

/// Complete pilot registration after receiving invitation
/// This endpoint is used when a pilot receives an invitation email and sets their password
pub async fn complete_pilot_registration(
    State(state): State<AppState>,
    Json(payload): Json<CompletePilotRegistrationRequest>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    // Get user by verification token
    let user = match users_repo.get_by_verification_token(&payload.token).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return json_error(
                StatusCode::BAD_REQUEST,
                "Invalid or expired registration token",
            )
            .into_response();
        }
        Err(e) => {
            error!(error = %e, "Database error during pilot registration");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to complete registration",
            )
            .into_response();
        }
    };

    // Set password and verify email
    match users_repo
        .set_password_and_verify_email(user.id, &payload.password)
        .await
    {
        Ok(true) => {
            // Get updated user
            let updated_user = match users_repo.get_by_id(user.id).await {
                Ok(Some(user)) => user,
                Ok(None) => {
                    return json_error(StatusCode::NOT_FOUND, "User not found").into_response();
                }
                Err(e) => {
                    error!(error = %e, "Failed to get user after registration");
                    return json_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to complete registration",
                    )
                    .into_response();
                }
            };

            // Generate JWT token for immediate login
            match get_jwt_secret() {
                Ok(secret) => {
                    let jwt_service = JwtService::new(&secret);
                    match jwt_service.generate_token(&updated_user) {
                        Ok(token) => {
                            let response = LoginResponse {
                                token,
                                user: UserView::from(updated_user),
                            };
                            Json(response).into_response()
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to generate token");
                            json_error(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to generate authentication token",
                            )
                            .into_response()
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "JWT secret not configured");
                    json_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Authentication configuration error",
                    )
                    .into_response()
                }
            }
        }
        Ok(false) => json_error(StatusCode::NOT_FOUND, "User not found").into_response(),
        Err(e) => {
            error!(error = %e, "Failed to set password and verify email");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to complete registration",
            )
            .into_response()
        }
    }
}
