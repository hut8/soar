use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use tracing::error;
use uuid::Uuid;

use crate::actions::{DataListResponse, DataResponse, json_error};
use crate::auth::AuthUser;
use crate::push_subscriptions::{PushSubscriptionRequest, VapidPublicKeyResponse};
use crate::push_subscriptions_repo::PushSubscriptionsRepository;
use crate::web::AppState;

/// GET /data/push/vapid-key - Return the VAPID public key for browser subscription
pub async fn get_vapid_key() -> impl IntoResponse {
    match std::env::var("VAPID_PUBLIC_KEY") {
        Ok(key) => Json(DataResponse {
            data: VapidPublicKeyResponse { public_key: key },
        })
        .into_response(),
        Err(_) => json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Push notifications are not configured",
        )
        .into_response(),
    }
}

/// POST /data/push/subscribe - Create or update a push subscription
pub async fn subscribe(
    auth_user: AuthUser,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<PushSubscriptionRequest>,
) -> impl IntoResponse {
    if req.endpoint.is_empty() || req.p256dh_key.is_empty() || req.auth_key.is_empty() {
        return json_error(
            StatusCode::BAD_REQUEST,
            "endpoint, p256dh_key, and auth_key are required",
        )
        .into_response();
    }

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let repo = PushSubscriptionsRepository::new(state.pool);
    let user_id = auth_user.0.id;

    match repo.upsert(user_id, &req, user_agent).await {
        Ok(sub) => Json(DataResponse { data: sub }).into_response(),
        Err(e) => {
            error!(error = %e, "Failed to save push subscription");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to save push subscription",
            )
            .into_response()
        }
    }
}

/// GET /data/push/subscriptions - List current user's push subscriptions
pub async fn list_subscriptions(
    auth_user: AuthUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let repo = PushSubscriptionsRepository::new(state.pool);
    let user_id = auth_user.0.id;

    match repo.get_by_user(user_id).await {
        Ok(subs) => Json(DataListResponse { data: subs }).into_response(),
        Err(e) => {
            error!(error = %e, "Failed to list push subscriptions");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list push subscriptions",
            )
            .into_response()
        }
    }
}

/// DELETE /data/push/subscriptions/{id} - Remove a push subscription
pub async fn delete_subscription(
    auth_user: AuthUser,
    State(state): State<AppState>,
    axum::extract::Path(subscription_id): axum::extract::Path<Uuid>,
) -> impl IntoResponse {
    let repo = PushSubscriptionsRepository::new(state.pool);
    let user_id = auth_user.0.id;

    match repo.delete(user_id, subscription_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => {
            json_error(StatusCode::NOT_FOUND, "Push subscription not found").into_response()
        }
        Err(e) => {
            error!(error = %e, "Failed to delete push subscription");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to delete push subscription",
            )
            .into_response()
        }
    }
}
