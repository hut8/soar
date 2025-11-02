use anyhow::Result;
use axum::{
    Router,
    body::Body,
    extract::{MatchedPath, Path},
    http::{HeaderMap, Request, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use include_dir::{Dir, include_dir};
use metrics_exporter_prometheus::PrometheusHandle;
use mime_guess::from_path;
use std::sync::OnceLock;
use std::time::Instant;
use uuid::Uuid;

use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};

use crate::actions;
use crate::live_fixes::LiveFixService;

// Embed web assets into the binary
static ASSETS: Dir<'_> = include_dir!("web/build");

// Embed robots.txt into the binary
static ROBOTS_TXT: &str = include_str!("../static/robots.txt");

// Global Prometheus metrics handle
static METRICS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

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

async fn handle_static_file(uri: Uri, request: Request<Body>) -> Response {
    let path = uri.path().trim_start_matches('/');
    let compilation_timestamp = get_compilation_timestamp();

    // Handle sitemap files (sitemap-1.xml, sitemap-2.xml, etc.)
    if path.starts_with("sitemap-") && path.ends_with(".xml") {
        return sitemap_file(Path(path.to_string())).await.into_response();
    }

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
    let request_id = Uuid::now_v7().to_string()[..8].to_string();
    let start_time = Instant::now();

    // Skip logging for metrics endpoint to reduce noise
    let should_log = path != "/data/metrics";

    // Format query parameters for logging
    let query_params_formatted = if should_log {
        format_query_params(query_string)
    } else {
        String::new()
    };

    if should_log {
        info!(
            "Started {} {} [{}{}]",
            method, path, request_id, query_params_formatted
        );
    }

    let response = next.run(request).await;
    let duration = start_time.elapsed();
    let status = response.status();

    if should_log {
        info!(
            "Completed {} {} [{}{}] {} in {:.2}ms",
            method,
            path,
            request_id,
            query_params_formatted,
            status.as_u16(),
            duration.as_secs_f64() * 1000.0
        );
    }

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

// Middleware to record metrics for API endpoints
async fn metrics_middleware(request: Request<Body>, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().clone();

    // Extract the matched route pattern (e.g., "/data/devices/{id}" instead of "/data/devices/123")
    let raw_path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|matched_path| matched_path.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string());

    // Normalize paths to avoid creating metrics for each individual request
    let path = if raw_path.starts_with("/_app/")
        || raw_path.starts_with("/assets/")
        || raw_path.starts_with("/_immutable/")
        || raw_path.ends_with(".js")
        || raw_path.ends_with(".css")
        || raw_path.ends_with(".woff")
        || raw_path.ends_with(".woff2")
        || raw_path.ends_with(".ttf")
        || raw_path.ends_with(".svg")
        || raw_path.ends_with(".png")
        || raw_path.ends_with(".jpg")
        || raw_path.ends_with(".jpeg")
        || raw_path.ends_with(".ico")
        || raw_path.ends_with(".webp")
    {
        // Static assets (JS, CSS, fonts, images)
        "/static_files".to_string()
    } else if raw_path.starts_with("/data/") {
        // API routes - keep the matched path pattern (e.g., "/data/devices/{id}")
        raw_path
    } else if raw_path == "/robots.txt" || raw_path.starts_with("/sitemap") {
        // Keep robots.txt and sitemap routes as-is
        raw_path
    } else {
        // Everything else is served by the fallback handler (client-side routing via index.html)
        // This includes /, /devices/uuid, /flights/uuid, etc.
        "/[...fallback]".to_string()
    };

    let response = next.run(request).await;
    let status = response.status();
    let duration = start.elapsed();

    // Exclude the metrics endpoint itself from being recorded
    if path != "/data/metrics" {
        let method_str = method.to_string();
        let status_str = status.as_u16().to_string();

        // Record request duration histogram with labels for endpoint, method, and status
        metrics::histogram!(
            "http_request_duration_seconds",
            "method" => method_str.clone(),
            "endpoint" => path.clone(),
            "status" => status_str.clone()
        )
        .record(duration.as_secs_f64());

        // Record request count by endpoint, method, and status
        metrics::counter!(
            "http_requests_total",
            "method" => method_str,
            "endpoint" => path,
            "status" => status_str
        )
        .increment(1);
    }

    response
}

// Handler for robots.txt - serves embedded content
async fn robots_txt() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "text/plain".parse().unwrap());
    // Cache robots.txt for 24 hours
    headers.insert("cache-control", "public, max-age=86400".parse().unwrap());

    (StatusCode::OK, headers, ROBOTS_TXT)
}

// Handler for Prometheus metrics endpoint
async fn metrics_handler() -> impl IntoResponse {
    let handle = METRICS_HANDLE
        .get()
        .expect("Metrics handle not initialized");

    let mut headers = HeaderMap::new();
    headers.insert("content-type", "text/plain; version=0.0.4".parse().unwrap());

    (StatusCode::OK, headers, handle.render())
}

// Handler for sitemap files - serves files from disk
async fn sitemap_file(Path(filename): Path<String>) -> impl IntoResponse {
    // Get sitemap root from environment variable
    let sitemap_root =
        std::env::var("SITEMAP_ROOT").unwrap_or_else(|_| "/var/soar/sitemap".to_string());

    // Validate filename to prevent directory traversal
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return (
            StatusCode::BAD_REQUEST,
            HeaderMap::new(),
            "Invalid filename",
        )
            .into_response();
    }

    // Only allow XML files
    if !filename.ends_with(".xml") {
        return (StatusCode::NOT_FOUND, HeaderMap::new(), "Not found").into_response();
    }

    // Build full path
    let file_path = std::path::Path::new(&sitemap_root).join(&filename);

    // Try to read the file
    match tokio::fs::read_to_string(&file_path).await {
        Ok(content) => {
            let mut headers = HeaderMap::new();
            headers.insert("content-type", "application/xml".parse().unwrap());
            // Cache sitemaps for 6 hours
            headers.insert("cache-control", "public, max-age=21600".parse().unwrap());

            (StatusCode::OK, headers, content).into_response()
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            HeaderMap::new(),
            "Sitemap file not found",
        )
            .into_response(),
    }
}

pub async fn start_web_server(interface: String, port: u16, pool: PgPool) -> Result<()> {
    sentry::configure_scope(|scope| {
        scope.set_tag("operation", "web-server");
    });

    // Initialize Prometheus metrics exporter
    let metrics_handle = crate::metrics::init_metrics();
    METRICS_HANDLE
        .set(metrics_handle)
        .expect("Metrics handle already initialized");
    info!("Prometheus metrics exporter initialized");

    // Initialize WebSocket metrics to zero so they're always exported to Prometheus
    // even when there are no active connections
    metrics::gauge!("websocket_connections").set(0.0);
    metrics::gauge!("websocket_active_subscriptions").set(0.0);
    metrics::gauge!("websocket_queue_depth").set(0.0);
    metrics::counter!("websocket_messages_sent").absolute(0);
    metrics::counter!("websocket_send_errors").absolute(0);
    metrics::counter!("websocket_serialization_errors").absolute(0);
    metrics::counter!("websocket_device_subscribes").absolute(0);
    metrics::counter!("websocket_device_unsubscribes").absolute(0);
    metrics::counter!("websocket_area_subscribes").absolute(0);
    metrics::counter!("websocket_area_unsubscribes").absolute(0);

    // Start process metrics background task
    tokio::spawn(crate::metrics::process_metrics_task());

    info!("Starting web server on {}:{}", interface, port);

    // Initialize live fix service if NATS_URL is configured
    let live_fix_service = match std::env::var("NATS_URL") {
        Ok(nats_url) => {
            info!("NATS_URL found, initializing live fix service");
            match LiveFixService::new(&nats_url).await {
                Ok(service) => {
                    info!("Live fix service initialized successfully with on-demand subscriptions");
                    Some(service)
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
        // Metrics endpoint
        .route("/metrics", get(metrics_handler))
        // Search and data routes
        .route("/airports", get(actions::search_airports))
        .route("/airports/{id}", get(actions::get_airport_by_id))
        .route("/airports/{id}/flights", get(actions::get_airport_flights))
        .route("/airports/{id}/clubs", get(actions::get_clubs_by_airport))
        .route("/clubs", get(actions::search_clubs))
        .route("/clubs/{id}", get(actions::get_club_by_id))
        .route("/fixes", get(actions::search_fixes))
        .route("/fixes/live", get(actions::fixes_live_websocket))
        .route("/flights", get(actions::search_flights))
        .route("/flights/{id}", get(actions::get_flight_by_id))
        .route("/flights/{id}/kml", get(actions::get_flight_kml))
        .route("/flights/{id}/fixes", get(actions::get_flight_fixes))
        .route("/flights/{id}/nearby", get(actions::get_nearby_flights))
        // Pilot routes
        .route("/pilots/{id}", get(actions::get_pilot_by_id))
        .route("/clubs/{id}/pilots", get(actions::get_pilots_by_club))
        .route("/flights/{id}/pilots", get(actions::get_pilots_for_flight))
        .route("/flights/{id}/pilots", post(actions::link_pilot_to_flight))
        .route(
            "/flights/{flight_id}/pilots/{pilot_id}",
            delete(actions::unlink_pilot_from_flight),
        )
        // Aircraft routes
        .route("/clubs/{id}/aircraft", get(actions::get_aircraft_by_club))
        .route("/clubs/{id}/devices", get(actions::get_devices_by_club))
        // Device routes
        .route("/devices", get(actions::search_devices))
        .route("/devices/{id}", get(actions::get_device_by_id))
        .route("/devices/{id}/fixes", get(actions::get_device_fixes))
        .route("/devices/{id}/flights", get(actions::get_device_flights))
        .route("/devices/{id}/club", put(actions::update_device_club))
        .route(
            "/devices/{id}/aircraft/registration",
            get(actions::aircraft::get_device_aircraft_registration),
        )
        .route(
            "/devices/{id}/aircraft/model",
            get(actions::aircraft::get_device_aircraft_model),
        )
        // Receiver routes
        .route("/receivers", get(actions::search_receivers))
        .route("/receivers/{id}", get(actions::get_receiver_by_id))
        .route("/receivers/{id}/fixes", get(actions::get_receiver_fixes))
        .route(
            "/receivers/{id}/statuses",
            get(actions::get_receiver_statuses),
        )
        .route(
            "/receivers/{id}/statistics",
            get(actions::get_receiver_statistics),
        )
        .route(
            "/receivers/{id}/raw-messages",
            get(actions::get_receiver_raw_messages),
        )
        .route(
            "/receivers/{id}/fix-counts-by-aprs-type",
            get(actions::get_receiver_fix_counts_by_aprs_type),
        )
        // APRS messages routes
        .route("/aprs-messages", post(actions::get_aprs_messages_bulk))
        .route("/aprs-messages/{id}", get(actions::get_aprs_message))
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
        // User settings routes
        .route("/user/settings", get(actions::get_user_settings))
        .route("/user/settings", put(actions::update_user_settings))
        .with_state(app_state.clone());

    // Build the main Axum application
    let app = Router::new()
        .nest("/data", api_router)
        .route("/robots.txt", get(robots_txt))
        .route(
            "/sitemap.xml",
            get(|| async move { sitemap_file(Path("sitemap.xml".to_string())).await }),
        )
        // Note: numbered sitemap files (sitemap-1.xml, etc.) are handled in the fallback handler
        .fallback(handle_static_file)
        .with_state(app_state.clone())
        .layer(middleware::from_fn(metrics_middleware))
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
