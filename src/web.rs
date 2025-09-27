use anyhow::Result;
use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Request, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use include_dir::{Dir, include_dir};
use mime_guess::from_path;
use std::time::Instant;
use uuid::Uuid;

use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};

use crate::actions;
use crate::live_fixes::LiveFixService;

// Embed web assets into the binary
static ASSETS: Dir<'_> = include_dir!("web/build");

// Helper function to format compilation timestamp for HTTP headers
fn get_compilation_timestamp() -> String {
    // For now, use a simple approach - generate timestamp at first call
    use std::sync::OnceLock;
    static TIMESTAMP: OnceLock<String> = OnceLock::new();

    TIMESTAMP
        .get_or_init(|| {
            use chrono::{TimeZone, Utc};
            use std::time::{SystemTime, UNIX_EPOCH};

            // Get current time (this will be consistent for the binary)
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            // Format as HTTP date
            let datetime = Utc.timestamp_opt(now as i64, 0).single().unwrap();
            datetime.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
        })
        .clone()
}

pub type PgPool = Pool<ConnectionManager<PgConnection>>;

// App state for sharing database pool and services
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,                             // Diesel pool for all operations
    pub live_fix_service: Option<LiveFixService>, // Live fix service for WebSocket subscriptions
}

async fn handle_static_file(uri: Uri, request: Request<Body>) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    let compilation_timestamp = get_compilation_timestamp();

    // Check if client sent If-Modified-Since header
    let if_modified_since = request
        .headers()
        .get("if-modified-since")
        .and_then(|h| h.to_str().ok());

    // If client has the same timestamp, return 304 Not Modified
    if let Some(client_timestamp) = if_modified_since
        && client_timestamp == compilation_timestamp
    {
        let mut headers = HeaderMap::new();
        headers.insert("last-modified", compilation_timestamp.parse().unwrap());
        return (StatusCode::NOT_MODIFIED, headers, "").into_response();
    }

    // Handle root path
    if (path.is_empty() || path == "index.html")
        && let Some(index_file) = ASSETS.get_file("index.html")
    {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "text/html".parse().unwrap());
        headers.insert("last-modified", compilation_timestamp.parse().unwrap());
        // HTML files should be revalidated
        headers.insert(
            "cache-control",
            "public, max-age=0, must-revalidate".parse().unwrap(),
        );
        return (StatusCode::OK, headers, index_file.contents()).into_response();
    }

    // Try to find the file in embedded assets
    if let Some(file) = ASSETS.get_file(path) {
        let mut headers = HeaderMap::new();

        // Set content type based on file extension
        let content_type = from_path(path).first_or_octet_stream();
        headers.insert("content-type", content_type.as_ref().parse().unwrap());
        headers.insert("last-modified", compilation_timestamp.parse().unwrap());

        // Set cache control headers for static assets
        if path.starts_with("_app/") || path.starts_with("assets/") {
            // Hashed assets can be cached forever, but still include Last-Modified for completeness
            headers.insert(
                "cache-control",
                "public, max-age=31536000, immutable".parse().unwrap(),
            );
        } else {
            // Other static files should be revalidated
            headers.insert(
                "cache-control",
                "public, max-age=3600, must-revalidate".parse().unwrap(),
            );
        }

        return (StatusCode::OK, headers, file.contents()).into_response();
    }

    // For client-side routing, serve index.html for paths that don't exist
    if !path.contains('.')
        && path != "favicon.ico"
        && let Some(index_file) = ASSETS.get_file("index.html")
    {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "text/html".parse().unwrap());
        headers.insert("last-modified", compilation_timestamp.parse().unwrap());
        headers.insert(
            "cache-control",
            "public, max-age=0, must-revalidate".parse().unwrap(),
        );
        return (StatusCode::OK, headers, index_file.contents()).into_response();
    }

    (StatusCode::NOT_FOUND, "Not Found").into_response()
}

// Helper function to format query parameters for logging
fn format_query_params(query_string: &str) -> String {
    if query_string.is_empty() {
        return String::new();
    }

    let query_params: Vec<String> = query_string
        .split('&')
        .map(|param| {
            if let Some(eq_pos) = param.find('=') {
                let (key, value) = param.split_at(eq_pos);
                format!("{}={}", key, &value[1..]) // Skip the '=' character
            } else {
                param.to_string()
            }
        })
        .collect();

    let formatted_params = query_params.join(" ");
    if formatted_params.is_empty() {
        String::new()
    } else {
        format!(" {}", formatted_params)
    }
}

// Middleware for request logging with correlation ID
async fn request_logging_middleware(request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let query_string = request.uri().query().unwrap_or("");
    let request_id = Uuid::new_v4().to_string()[..8].to_string();
    let start_time = Instant::now();

    // Format query parameters for logging
    let query_params_formatted = format_query_params(query_string);

    info!("Started {} {} [{}{}]", method, path, request_id, query_params_formatted);

    let response = next.run(request).await;
    let duration = start_time.elapsed();
    let status = response.status();

    info!(
        "Completed {} {} [{}{}] {} in {:.2}ms",
        method,
        path,
        request_id,
        query_params_formatted,
        status.as_u16(),
        duration.as_secs_f64() * 1000.0
    );

    response
}

// Middleware to capture HTTP errors to Sentry
async fn sentry_error_middleware(request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();

    let response = next.run(request).await;

    // Capture HTTP 5xx errors to Sentry
    if response.status().is_server_error() {
        let status = response.status();
        error!("HTTP {} error on {} {}", status.as_u16(), method, uri);

        sentry::configure_scope(|scope| {
            scope.set_tag("http.method", method.as_str());
            scope.set_tag("http.url", uri.to_string());
            scope.set_tag("http.status_code", status.as_u16().to_string());
        });

        sentry::capture_message(
            &format!("HTTP {} error on {} {}", status.as_u16(), method, uri),
            sentry::Level::Error,
        );
    }

    response
}

pub async fn start_web_server(interface: String, port: u16, pool: PgPool) -> Result<()> {
    sentry::configure_scope(|scope| {
        scope.set_tag("operation", "web-server");
    });
    info!("Starting web server on {}:{}", interface, port);

    // Initialize live fix service if NATS_URL is configured
    let live_fix_service = match std::env::var("NATS_URL") {
        Ok(nats_url) => {
            info!("NATS_URL found, initializing live fix service");
            match LiveFixService::new(&nats_url).await {
                Ok(service) => {
                    if let Err(e) = service.start_listening().await {
                        error!("Failed to start live fix service listener: {}", e);
                        None
                    } else {
                        info!("Live fix service initialized successfully");
                        Some(service)
                    }
                }
                Err(e) => {
                    error!("Failed to create live fix service: {}", e);
                    None
                }
            }
        }
        Err(_) => {
            warn!("NATS_URL not configured, live fixes will not be available");
            None
        }
    };

    let app_state = AppState {
        pool,
        live_fix_service,
    };

    // Create CORS layer that allows all origins and methods
    let cors_layer = CorsLayer::permissive();

    // Create API sub-router rooted at "/data"
    let api_router = Router::new()
        // Search and data routes
        .route("/airports", get(actions::search_airports))
        .route("/airports/{id}", get(actions::get_airport_by_id))
        .route("/clubs", get(actions::search_clubs))
        .route("/clubs/{id}", get(actions::get_club_by_id))
        .route("/fixes", get(actions::search_fixes))
        .route("/fixes/live", get(actions::fixes_live_websocket))
        .route("/flights", get(actions::search_flights))
        // Aircraft routes
        .route("/clubs/{id}/aircraft", get(actions::get_aircraft_by_club))
        // Device routes
        .route("/devices", get(actions::search_devices))
        .route("/devices/{id}", get(actions::get_device_by_id))
        .route("/devices/{id}/fixes", get(actions::get_device_fixes))
        // Authentication routes
        .route("/auth/register", post(actions::register_user))
        .route("/auth/login", post(actions::login_user))
        .route("/auth/me", get(actions::get_current_user))
        .route("/auth/verify-email", post(actions::verify_email))
        .route(
            "/auth/password-reset/request",
            post(actions::request_password_reset),
        )
        .route(
            "/auth/password-reset/confirm",
            post(actions::confirm_password_reset),
        )
        // User management routes
        .route("/users", get(actions::get_all_users))
        .route("/users/{id}", get(actions::get_user_by_id))
        .route("/users/{id}", put(actions::update_user_by_id))
        .route("/users/{id}", delete(actions::delete_user_by_id))
        .route("/users/set-club", put(actions::set_user_club))
        .route("/clubs/{id}/users", get(actions::get_users_by_club))
        .with_state(app_state.clone());

    // Build the main Axum application
    let app = Router::new()
        .nest("/data", api_router)
        .fallback(handle_static_file)
        .with_state(app_state.clone())
        .layer(middleware::from_fn(request_logging_middleware))
        .layer(middleware::from_fn(sentry_error_middleware))
        .layer(cors_layer);

    // Create the listener
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", interface, port)).await?;
    info!("Web server listening on http://{}:{}", interface, port);

    // Start the server
    axum::serve(listener, app).await?;

    Ok(())
}
