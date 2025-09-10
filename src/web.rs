use anyhow::Result;
use axum::{
    Router,
    http::{HeaderMap, StatusCode, Uri},
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use include_dir::{Dir, include_dir};
use mime_guess::from_path;
use sqlx::postgres::PgPool;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::actions;

// Embed web assets into the binary
static ASSETS: Dir<'_> = include_dir!("web/build");

// App state for sharing database pool
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

async fn handle_static_file(uri: Uri, _state: axum::extract::State<AppState>) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Handle root path
    if (path.is_empty() || path == "index.html")
        && let Some(index_file) = ASSETS.get_file("index.html")
    {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "text/html".parse().unwrap());
        headers.insert(
            "cache-control",
            "public, max-age=31536000, immutable".parse().unwrap(),
        );
        return (StatusCode::OK, headers, index_file.contents()).into_response();
    }

    // Try to find the file in embedded assets
    if let Some(file) = ASSETS.get_file(path) {
        let mut headers = HeaderMap::new();

        // Set content type based on file extension
        let content_type = from_path(path).first_or_octet_stream();
        headers.insert("content-type", content_type.as_ref().parse().unwrap());

        // Set cache control headers for static assets
        if path.starts_with("_app/") || path.starts_with("assets/") {
            headers.insert(
                "cache-control",
                "public, max-age=31536000, immutable".parse().unwrap(),
            );
        } else {
            headers.insert("cache-control", "public, max-age=3600".parse().unwrap());
        }

        return (StatusCode::OK, headers, file.contents()).into_response();
    }

    // For client-side routing, serve index.html for paths that don't exist
    if !path.contains('.') && path != "favicon.ico"
        && let Some(index_file) = ASSETS.get_file("index.html") {
            let mut headers = HeaderMap::new();
            headers.insert("content-type", "text/html".parse().unwrap());
            headers.insert("cache-control", "public, max-age=3600".parse().unwrap());
            return (StatusCode::OK, headers, index_file.contents()).into_response();
        }

    (StatusCode::NOT_FOUND, "Not Found").into_response()
}

// All individual action functions have been moved to the actions module

pub async fn start_web_server(interface: String, port: u16, pool: PgPool) -> Result<()> {
    info!("Starting web server on {}:{}", interface, port);

    let app_state = AppState { pool };

    // Build the Axum application
    let app = Router::new()
        // Search and data routes
        .route("/airports", get(actions::search_airports))
        .route("/clubs", get(actions::search_clubs))
        .route("/fixes", get(actions::search_fixes))
        .route("/fixes/live", get(actions::fixes_live_websocket))
        .route("/flights", get(actions::search_flights))
        // Aircraft routes
        .route("/clubs/:id/aircraft", get(actions::get_aircraft_by_club))
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
        .route("/users/:id", get(actions::get_user_by_id))
        .route("/users/:id", put(actions::update_user_by_id))
        .route("/users/:id", delete(actions::delete_user_by_id))
        .route("/clubs/:id/users", get(actions::get_users_by_club))
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
