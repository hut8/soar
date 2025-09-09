use anyhow::Result;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Json},
    routing::{delete, get, post, put},
};
use include_dir::{Dir, include_dir};
use mime_guess::from_path;
use serde::Deserialize;
use sqlx::{postgres::PgPool, types::Uuid};
use tower_http::trace::TraceLayer;
use tracing::{error, info};

use crate::airports_repo::AirportsRepository;
use crate::auth::{AdminUser, AuthUser, JwtService, get_jwt_secret};
use crate::clubs_repo::ClubsRepository;
use crate::email::EmailService;
use crate::fixes_repo::FixesRepository;
use crate::flights_repo::FlightsRepository;
use crate::users::{
    CreateUserRequest, EmailVerificationConfirm, LoginRequest, LoginResponse, 
    PasswordResetConfirm, PasswordResetRequest, UpdateUserRequest, UserInfo,
};
use crate::users_repo::UsersRepository;

// Embed web assets into the binary
static ASSETS: Dir<'_> = include_dir!("web/build");

// App state for sharing database pool
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    q: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    radius: Option<f64>,
    limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct FixQuery {
    aircraft_id: Option<String>,
    start_time: Option<String>, // ISO datetime
    end_time: Option<String>,   // ISO datetime
    limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct FlightQuery {
    aircraft_id: Option<String>,
    start_date: Option<String>, // ISO date
    in_progress: Option<bool>,  // Filter for flights in progress
    limit: Option<i64>,
}

async fn handle_static_file(uri: Uri, _state: State<AppState>) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Try to find the exact file
    if let Some(file) = ASSETS.get_file(path) {
        let mime_type = from_path(path).first_or_octet_stream();
        let mut headers = HeaderMap::new();
        headers.insert("content-type", mime_type.as_ref().parse().unwrap());
        return (StatusCode::OK, headers, file.contents()).into_response();
    }

    // For HTML requests (based on Accept header or file extension), serve index.html for SPA
    if (path.is_empty() || path.ends_with(".html") || path.ends_with('/'))
        && let Some(index_file) = ASSETS.get_file("index.html")
    {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "text/html".parse().unwrap());
        return (StatusCode::OK, headers, index_file.contents()).into_response();
    }

    // If no file found and it's not an API route, try serving index.html for SPA routing
    if !path.starts_with("api/")
        && let Some(index_file) = ASSETS.get_file("index.html")
    {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "text/html".parse().unwrap());
        return (StatusCode::OK, headers, index_file.contents()).into_response();
    }

    (StatusCode::NOT_FOUND, "Not Found").into_response()
}

async fn search_airports(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let airports_repo = AirportsRepository::new(state.pool);

    // Check if geographic search parameters are provided
    if let (Some(lat), Some(lng), Some(radius)) = (params.latitude, params.longitude, params.radius)
    {
        // Validate radius
        if radius <= 0.0 || radius > 1000.0 {
            return (
                StatusCode::BAD_REQUEST,
                "Radius must be between 0 and 1000 kilometers",
            )
                .into_response();
        }

        // Validate latitude
        if !(-90.0..=90.0).contains(&lat) {
            return (
                StatusCode::BAD_REQUEST,
                "Latitude must be between -90 and 90 degrees",
            )
                .into_response();
        }

        // Validate longitude
        if !(-180.0..=180.0).contains(&lng) {
            return (
                StatusCode::BAD_REQUEST,
                "Longitude must be between -180 and 180 degrees",
            )
                .into_response();
        }

        match airports_repo
            .search_nearby(lat, lng, radius, params.limit)
            .await
        {
            Ok(airports) => Json(airports).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error searching airports by location: {}", e),
            )
                .into_response(),
        }
    } else if let Some(query) = params.q {
        // Text-based search
        if query.trim().is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                "Query parameter 'q' cannot be empty",
            )
                .into_response();
        }

        match airports_repo.fuzzy_search(&query, params.limit).await {
            Ok(airports) => Json(airports).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error searching airports: {}", e),
            )
                .into_response(),
        }
    } else if params.latitude.is_some() || params.longitude.is_some() || params.radius.is_some() {
        // Some geographic parameters provided but not all
        (
            StatusCode::BAD_REQUEST,
            "Geographic search requires all three parameters: latitude, longitude, and radius",
        )
            .into_response()
    } else {
        // No search parameters provided
        (
            StatusCode::BAD_REQUEST,
            "Either 'q' for text search or 'latitude', 'longitude', and 'radius' for geographic search must be provided",
        )
            .into_response()
    }
}

async fn search_clubs(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let clubs_repo = ClubsRepository::new(state.pool);

    // Check if geographic search parameters are provided
    if let (Some(lat), Some(lng), Some(radius)) = (params.latitude, params.longitude, params.radius)
    {
        // Validate radius
        if radius <= 0.0 || radius > 1000.0 {
            return (
                StatusCode::BAD_REQUEST,
                "Radius must be between 0 and 1000 kilometers",
            )
                .into_response();
        }

        // Validate latitude
        if !(-90.0..=90.0).contains(&lat) {
            return (
                StatusCode::BAD_REQUEST,
                "Latitude must be between -90 and 90 degrees",
            )
                .into_response();
        }

        // Validate longitude
        if !(-180.0..=180.0).contains(&lng) {
            return (
                StatusCode::BAD_REQUEST,
                "Longitude must be between -180 and 180 degrees",
            )
                .into_response();
        }

        match clubs_repo
            .search_nearby_soaring(lat, lng, radius, params.limit)
            .await
        {
            Ok(clubs) => Json(clubs).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error searching clubs by location: {}", e),
            )
                .into_response(),
        }
    } else if let Some(query) = params.q {
        // Text-based search
        if query.trim().is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                "Query parameter 'q' cannot be empty",
            )
                .into_response();
        }

        match clubs_repo.fuzzy_search_soaring(&query, params.limit).await {
            Ok(clubs) => Json(clubs).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error searching clubs: {}", e),
            )
                .into_response(),
        }
    } else if params.latitude.is_some() || params.longitude.is_some() || params.radius.is_some() {
        // Some geographic parameters provided but not all
        (
            StatusCode::BAD_REQUEST,
            "Geographic search requires all three parameters: latitude, longitude, and radius",
        )
            .into_response()
    } else {
        // No search parameters provided
        (
            StatusCode::BAD_REQUEST,
            "Either 'q' for text search or 'latitude', 'longitude', and 'radius' for geographic search must be provided",
        )
            .into_response()
    }
}

async fn search_fixes(
    State(state): State<AppState>,
    Query(params): Query<FixQuery>,
) -> impl IntoResponse {
    use chrono::{DateTime, Utc};

    let fixes_repo = FixesRepository::new(state.pool);

    // Parse datetime strings if provided
    let start_time = if let Some(start_str) = params.start_time {
        match start_str.parse::<DateTime<Utc>>() {
            Ok(dt) => Some(dt),
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    "Invalid start_time format. Use ISO 8601 format.",
                )
                    .into_response();
            }
        }
    } else {
        None
    };

    let end_time = if let Some(end_str) = params.end_time {
        match end_str.parse::<DateTime<Utc>>() {
            Ok(dt) => Some(dt),
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    "Invalid end_time format. Use ISO 8601 format.",
                )
                    .into_response();
            }
        }
    } else {
        None
    };

    // Determine query strategy
    if let Some(aircraft_id) = params.aircraft_id {
        // Aircraft-specific query
        match fixes_repo
            .get_fixes_for_aircraft(&aircraft_id, params.limit)
            .await
        {
            Ok(fixes) => Json(fixes).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error fetching fixes for aircraft {}: {}", aircraft_id, e),
            )
                .into_response(),
        }
    } else if let (Some(start), Some(end)) = (start_time, end_time) {
        // Time range query
        match fixes_repo
            .get_fixes_in_time_range(start, end, params.limit)
            .await
        {
            Ok(fixes) => Json(fixes).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error fetching fixes in time range: {}", e),
            )
                .into_response(),
        }
    } else {
        // Recent fixes query
        match fixes_repo
            .get_recent_fixes(params.limit.unwrap_or(100))
            .await
        {
            Ok(fixes) => Json(fixes).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error fetching recent fixes: {}", e),
            )
                .into_response(),
        }
    }
}

async fn search_flights(
    State(state): State<AppState>,
    Query(params): Query<FlightQuery>,
) -> impl IntoResponse {
    use chrono::NaiveDate;

    let flights_repo = FlightsRepository::new(state.pool);

    // Handle in-progress filter
    if params.in_progress == Some(true) {
        match flights_repo.get_flights_in_progress().await {
            Ok(flights) => return Json(flights).into_response(),
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error fetching flights in progress: {}", e),
                )
                    .into_response();
            }
        }
    }

    // Aircraft-specific query
    if let Some(aircraft_id) = params.aircraft_id {
        match flights_repo.get_flights_for_aircraft(&aircraft_id).await {
            Ok(flights) => Json(flights).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error fetching flights for aircraft {}: {}", aircraft_id, e),
            )
                .into_response(),
        }
    } else if let Some(date_str) = params.start_date {
        // Date-specific query
        match date_str.parse::<NaiveDate>() {
            Ok(date) => match flights_repo.get_flights_for_date(date).await {
                Ok(flights) => Json(flights).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error fetching flights for date {}: {}", date, e),
                )
                    .into_response(),
            },
            Err(_) => (
                StatusCode::BAD_REQUEST,
                "Invalid start_date format. Use YYYY-MM-DD format.",
            )
                .into_response(),
        }
    } else {
        // Default: recent flights
        match flights_repo.get_flights_in_progress().await {
            Ok(flights) => {
                // Also get recently completed flights if we need more
                let limit = params.limit.unwrap_or(50) as usize;
                if flights.len() < limit {
                    // This is a simplified approach - in a real system you'd want a more sophisticated query
                    Json(flights).into_response()
                } else {
                    Json(flights).into_response()
                }
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error fetching recent flights: {}", e),
            )
                .into_response(),
        }
    }
}

// User authentication endpoints

async fn register_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool.clone());

    // Check if user already exists
    if let Ok(Some(_)) = users_repo.get_by_email(&payload.email).await {
        return (StatusCode::CONFLICT, "User with this email already exists").into_response();
    }

    // Create user
    match users_repo.create_user(&payload).await {
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
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to send email verification",
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

                    (StatusCode::CREATED, "User created. Please check your email to verify your account.").into_response()
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
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create user").into_response()
        }
    }
}

async fn login_user(
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
                        if let Ok(email_service) = EmailService::new() {
                            if let Err(e) = email_service
                                .send_email_verification(&user.email, &user.full_name(), &token)
                                .await
                            {
                                error!("Failed to send email verification: {}", e);
                            }
                        }
                        return (
                            StatusCode::FORBIDDEN,
                            "Email not verified. A new verification email has been sent to your email address.",
                        )
                            .into_response();
                    }
                    Err(e) => {
                        error!("Failed to generate email verification token: {}", e);
                        return (
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
                                user: user.to_user_info(),
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
            (StatusCode::INTERNAL_SERVER_ERROR, "Authentication failed").into_response()
        }
    }
}

async fn get_current_user(auth_user: AuthUser) -> impl IntoResponse {
    Json(auth_user.0.to_user_info())
}

async fn verify_email(
    State(state): State<AppState>,
    Json(payload): Json<EmailVerificationConfirm>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    match users_repo.get_by_verification_token(&payload.token).await {
        Ok(Some(user)) => {
            match users_repo.verify_user_email(user.id).await {
                Ok(true) => (StatusCode::OK, "Email verified successfully").into_response(),
                Ok(false) => (StatusCode::NOT_FOUND, "User not found").into_response(),
                Err(e) => {
                    error!("Failed to verify email: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to verify email",
                    )
                        .into_response()
                }
            }
        }
        Ok(None) => (StatusCode::BAD_REQUEST, "Invalid or expired verification token").into_response(),
        Err(e) => {
            error!("Database error during email verification: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to verify email",
            )
                .into_response()
        }
    }
}

async fn request_password_reset(
    State(state): State<AppState>,
    Json(payload): Json<PasswordResetRequest>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    match users_repo.get_by_email(&payload.email).await {
        Ok(Some(user)) => {
            match users_repo.set_password_reset_token(user.id).await {
                Ok(token) => {
                    // Send password reset email
                    if let Ok(email_service) = EmailService::new() {
                        if let Err(e) = email_service
                            .send_password_reset_email(&user.email, &user.full_name(), &token)
                            .await
                        {
                            error!("Failed to send password reset email: {}", e);
                            return (
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

                    (StatusCode::OK, "Password reset email sent").into_response()
                }
                Err(e) => {
                    error!("Failed to generate password reset token: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to generate password reset token",
                    )
                        .into_response()
                }
            }
        }
        Ok(None) => {
            // Don't reveal if email exists or not for security
            (StatusCode::OK, "Password reset email sent").into_response()
        }
        Err(e) => {
            error!("Database error during password reset: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to process password reset request",
            )
                .into_response()
        }
    }
}

async fn confirm_password_reset(
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
        Ok(None) => (StatusCode::BAD_REQUEST, "Invalid or expired token").into_response(),
        Err(e) => {
            error!("Database error during password reset confirmation: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to reset password",
            )
                .into_response()
        }
    }
}

// User management endpoints (admin only)

async fn get_all_users(
    _admin_user: AdminUser,
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    match users_repo.get_all(params.limit).await {
        Ok(users) => {
            let user_infos: Vec<UserInfo> = users.into_iter().map(|u| u.to_user_info()).collect();
            Json(user_infos).into_response()
        }
        Err(e) => {
            error!("Failed to get users: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get users").into_response()
        }
    }
}

async fn get_user_by_id(
    _admin_user: AdminUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    match users_repo.get_by_id(user_id).await {
        Ok(Some(user)) => Json(user.to_user_info()).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "User not found").into_response(),
        Err(e) => {
            error!("Failed to get user: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get user").into_response()
        }
    }
}

async fn update_user_by_id(
    _admin_user: AdminUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdateUserRequest>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    match users_repo.update_user(user_id, &payload).await {
        Ok(Some(user)) => Json(user.to_user_info()).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "User not found").into_response(),
        Err(e) => {
            error!("Failed to update user: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to update user").into_response()
        }
    }
}

async fn delete_user_by_id(
    _admin_user: AdminUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    match users_repo.delete_user(user_id).await {
        Ok(true) => (StatusCode::NO_CONTENT, "").into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "User not found").into_response(),
        Err(e) => {
            error!("Failed to delete user: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete user").into_response()
        }
    }
}

async fn get_users_by_club(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(club_id): Path<Uuid>,
) -> impl IntoResponse {
    let users_repo = UsersRepository::new(state.pool);

    // Check if user is admin or belongs to the same club
    if !auth_user.0.is_admin() && auth_user.0.club_id != Some(club_id) {
        return (StatusCode::FORBIDDEN, "Insufficient permissions").into_response();
    }

    match users_repo.get_by_club_id(club_id).await {
        Ok(users) => {
            let user_infos: Vec<UserInfo> = users.into_iter().map(|u| u.to_user_info()).collect();
            Json(user_infos).into_response()
        }
        Err(e) => {
            error!("Failed to get users by club: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get users by club",
            )
                .into_response()
        }
    }
}

pub async fn start_web_server(interface: String, port: u16, pool: PgPool) -> Result<()> {
    info!("Starting web server on {}:{}", interface, port);

    let app_state = AppState { pool };

    // Build the Axum application
    let app = Router::new()
        // Existing API routes
        .route("/airports", get(search_airports))
        .route("/clubs", get(search_clubs))
        .route("/fixes", get(search_fixes))
        .route("/flights", get(search_flights))
        // Authentication routes
        .route("/auth/register", post(register_user))
        .route("/auth/login", post(login_user))
        .route("/auth/me", get(get_current_user))
        .route("/auth/verify-email", post(verify_email))
        .route("/auth/password-reset/request", post(request_password_reset))
        .route("/auth/password-reset/confirm", post(confirm_password_reset))
        // User management routes (admin only)
        .route("/users", get(get_all_users))
        .route("/users/:id", get(get_user_by_id))
        .route("/users/:id", put(update_user_by_id))
        .route("/users/:id", delete(delete_user_by_id))
        .route("/clubs/:id/users", get(get_users_by_club))
        .fallback(handle_static_file)
        .with_state(app_state)
        .layer(TraceLayer::new_for_http());

    // Create the listener
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", interface, port)).await?;

    info!("Web server listening on http://{}:{}", interface, port);

    // Start the server
    axum::serve(listener, app).await?;

    Ok(())
}
