use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use serde::Serialize;
use stripe::{
    Account, AccountLink, AccountLinkType, AccountType, CreateAccount, CreateAccountLink, Event,
    EventObject, LoginLink, Webhook,
};
use tracing::{error, info, warn};
use ts_rs::TS;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::payments::PaymentStatus;
use crate::payments_repo::PaymentsRepository;
use crate::stripe_client::StripeConfig;
use crate::stripe_connected_accounts::NewStripeConnectedAccount;
use crate::stripe_connected_accounts_repo::StripeConnectedAccountsRepository;
use crate::stripe_webhooks::NewStripeWebhookEvent;
use crate::stripe_webhooks_repo::StripeWebhookEventsRepository;
use crate::web::AppState;

use super::{DataResponse, json_error};

/// Response for Stripe Connect onboarding
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct StripeOnboardingResponse {
    pub url: String,
}

/// Response for Stripe Connect status
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct StripeConnectStatusView {
    pub connected: bool,
    pub onboarding_complete: bool,
    pub charges_enabled: bool,
    pub payouts_enabled: bool,
    pub details_submitted: bool,
    pub stripe_account_id: Option<String>,
}

/// Response for Stripe dashboard link
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct StripeDashboardLinkResponse {
    pub url: String,
}

/// POST /clubs/{id}/stripe/onboard
/// Initiate Stripe Connect Express onboarding for a club
pub async fn start_onboarding(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(club_id): Path<Uuid>,
) -> impl IntoResponse {
    if !auth_user.0.is_admin && auth_user.0.club_id != Some(club_id) {
        return json_error(
            StatusCode::FORBIDDEN,
            "You must be a member of this club to manage Stripe Connect",
        )
        .into_response();
    }

    let stripe_config = match &state.stripe_config {
        Some(config) => config.clone(),
        None => {
            return json_error(StatusCode::SERVICE_UNAVAILABLE, "Stripe is not configured")
                .into_response();
        }
    };

    let repo = StripeConnectedAccountsRepository::new(state.pool.clone());

    // Check if club already has a connected account
    match repo.get_by_club_id(club_id).await {
        Ok(Some(existing)) if existing.onboarding_complete => {
            return json_error(
                StatusCode::CONFLICT,
                "Club already has a connected Stripe account",
            )
            .into_response();
        }
        Ok(Some(existing)) => {
            // Onboarding incomplete — generate a new account link
            let account_id: stripe::AccountId = match existing.stripe_account_id.parse() {
                Ok(id) => id,
                Err(e) => {
                    error!(error = %e, "Invalid Stripe account ID in database");
                    return json_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Invalid Stripe account configuration",
                    )
                    .into_response();
                }
            };
            match create_account_link(&stripe_config, &account_id).await {
                Ok(url) => {
                    return Json(DataResponse {
                        data: StripeOnboardingResponse { url },
                    })
                    .into_response();
                }
                Err(e) => {
                    error!(error = %e, "Failed to create Stripe account link");
                    return json_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to create Stripe onboarding link",
                    )
                    .into_response();
                }
            }
        }
        Ok(None) => {} // Continue to create new account
        Err(e) => {
            error!(error = %e, "Failed to check existing Stripe account");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to check Stripe account status",
            )
            .into_response();
        }
    }

    // Create a new Express account
    let mut create_params = CreateAccount::new();
    create_params.type_ = Some(AccountType::Express);

    let account = match Account::create(&stripe_config.client, create_params).await {
        Ok(account) => account,
        Err(e) => {
            error!(error = %e, "Failed to create Stripe Express account");
            metrics::counter!("stripe.api.errors").increment(1);
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create Stripe account",
            )
            .into_response();
        }
    };

    // Store the connected account
    let new_account = NewStripeConnectedAccount {
        club_id,
        stripe_account_id: account.id.to_string(),
    };

    if let Err(e) = repo.create(new_account).await {
        error!(error = %e, "Failed to store Stripe connected account");
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to store Stripe account",
        )
        .into_response();
    }

    metrics::counter!("stripe.connect.onboarding_started").increment(1);

    // Create an account link for onboarding
    match create_account_link(&stripe_config, &account.id).await {
        Ok(url) => Json(DataResponse {
            data: StripeOnboardingResponse { url },
        })
        .into_response(),
        Err(e) => {
            error!(error = %e, "Failed to create Stripe account link");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create Stripe onboarding link",
            )
            .into_response()
        }
    }
}

/// GET /clubs/{id}/stripe/status
/// Check Stripe Connect status for a club
pub async fn get_stripe_status(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(club_id): Path<Uuid>,
) -> impl IntoResponse {
    if !auth_user.0.is_admin && auth_user.0.club_id != Some(club_id) {
        return json_error(
            StatusCode::FORBIDDEN,
            "You must be a member of this club to view Stripe status",
        )
        .into_response();
    }

    let repo = StripeConnectedAccountsRepository::new(state.pool.clone());

    match repo.get_by_club_id(club_id).await {
        Ok(Some(account)) => Json(DataResponse {
            data: StripeConnectStatusView {
                connected: true,
                onboarding_complete: account.onboarding_complete,
                charges_enabled: account.charges_enabled,
                payouts_enabled: account.payouts_enabled,
                details_submitted: account.details_submitted,
                stripe_account_id: Some(account.stripe_account_id),
            },
        })
        .into_response(),
        Ok(None) => Json(DataResponse {
            data: StripeConnectStatusView {
                connected: false,
                onboarding_complete: false,
                charges_enabled: false,
                payouts_enabled: false,
                details_submitted: false,
                stripe_account_id: None,
            },
        })
        .into_response(),
        Err(e) => {
            error!(club_id = %club_id, error = %e, "Failed to get Stripe status");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get Stripe status",
            )
            .into_response()
        }
    }
}

/// POST /clubs/{id}/stripe/dashboard
/// Get a Stripe Express dashboard login link
pub async fn get_dashboard_link(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(club_id): Path<Uuid>,
) -> impl IntoResponse {
    if !auth_user.0.is_admin && auth_user.0.club_id != Some(club_id) {
        return json_error(
            StatusCode::FORBIDDEN,
            "You must be a member of this club to access the Stripe dashboard",
        )
        .into_response();
    }

    let stripe_config = match &state.stripe_config {
        Some(config) => config.clone(),
        None => {
            return json_error(StatusCode::SERVICE_UNAVAILABLE, "Stripe is not configured")
                .into_response();
        }
    };

    let repo = StripeConnectedAccountsRepository::new(state.pool.clone());

    let account = match repo.get_by_club_id(club_id).await {
        Ok(Some(account)) if account.onboarding_complete => account,
        Ok(Some(_)) => {
            return json_error(StatusCode::BAD_REQUEST, "Stripe onboarding is not complete")
                .into_response();
        }
        Ok(None) => {
            return json_error(StatusCode::NOT_FOUND, "Club has no Stripe account").into_response();
        }
        Err(e) => {
            error!(error = %e, "Failed to get Stripe account");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get Stripe account",
            )
            .into_response();
        }
    };

    let account_id: stripe::AccountId = match account.stripe_account_id.parse() {
        Ok(id) => id,
        Err(e) => {
            error!(error = %e, "Invalid Stripe account ID in database");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid Stripe account configuration",
            )
            .into_response();
        }
    };
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    match LoginLink::create(&stripe_config.client, &account_id, &base_url).await {
        Ok(link) => Json(DataResponse {
            data: StripeDashboardLinkResponse { url: link.url },
        })
        .into_response(),
        Err(e) => {
            error!(error = %e, "Failed to create Stripe dashboard link");
            metrics::counter!("stripe.api.errors").increment(1);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create dashboard link",
            )
            .into_response()
        }
    }
}

/// POST /stripe/webhooks
/// Handle incoming Stripe webhook events
pub async fn handle_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let stripe_config = match &state.stripe_config {
        Some(config) => config.clone(),
        None => {
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
    };

    metrics::counter!("stripe.webhook.received").increment(1);
    let start = std::time::Instant::now();

    // Get the Stripe-Signature header
    let signature = match headers.get("Stripe-Signature") {
        Some(sig) => match sig.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => {
                metrics::counter!("stripe.webhook.signature_invalid").increment(1);
                return StatusCode::BAD_REQUEST.into_response();
            }
        },
        None => {
            metrics::counter!("stripe.webhook.signature_invalid").increment(1);
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    let payload = match std::str::from_utf8(&body) {
        Ok(s) => s,
        Err(_) => {
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    // Verify the webhook signature
    let event = match Webhook::construct_event(payload, &signature, &stripe_config.webhook_secret) {
        Ok(event) => event,
        Err(e) => {
            warn!(error = %e, "Invalid webhook signature");
            metrics::counter!("stripe.webhook.signature_invalid").increment(1);
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    let webhook_repo = StripeWebhookEventsRepository::new(state.pool.clone());

    // Check idempotency
    let event_id = event.id.to_string();
    match webhook_repo.is_processed(&event_id).await {
        Ok(true) => {
            return StatusCode::OK.into_response();
        }
        Err(e) => {
            error!(error = %e, "Failed to check webhook idempotency");
        }
        _ => {}
    }

    // Record the event
    let event_type = event.type_.to_string();
    let new_event = NewStripeWebhookEvent {
        stripe_event_id: event_id.clone(),
        event_type: event_type.clone(),
        payload: serde_json::to_value(&event).unwrap_or_default(),
    };

    if let Err(e) = webhook_repo.create(new_event).await {
        // May fail if event already exists (race condition), that's OK
        warn!(error = %e, event_id = %event_id, "Failed to record webhook event");
    }

    // Process the event
    let process_result = process_webhook_event(&state, &stripe_config, &event_type, &event).await;

    match process_result {
        Ok(()) => {
            if let Err(e) = webhook_repo.mark_processed(&event_id).await {
                error!(error = %e, "Failed to mark webhook as processed");
            }
        }
        Err(e) => {
            error!(event_type = %event_type, error = %e, "Failed to process webhook event");
            if let Err(e2) = webhook_repo.mark_failed(&event_id, &e.to_string()).await {
                error!(error = %e2, "Failed to mark webhook as failed");
            }
        }
    }

    let duration_ms = start.elapsed().as_millis() as f64;
    metrics::histogram!("stripe.webhook.processing_ms").record(duration_ms);

    StatusCode::OK.into_response()
}

async fn process_webhook_event(
    state: &AppState,
    _stripe_config: &StripeConfig,
    event_type: &str,
    event: &Event,
) -> anyhow::Result<()> {
    match event_type {
        "account.updated" => {
            if let EventObject::Account(account) = &event.data.object {
                let repo = StripeConnectedAccountsRepository::new(state.pool.clone());
                let account_id = account.id.to_string();
                let charges_enabled = account.charges_enabled.unwrap_or(false);
                let payouts_enabled = account.payouts_enabled.unwrap_or(false);
                let details_submitted = account.details_submitted.unwrap_or(false);

                repo.update_status(
                    &account_id,
                    charges_enabled,
                    payouts_enabled,
                    details_submitted,
                )
                .await?;

                if charges_enabled && details_submitted {
                    metrics::counter!("stripe.connect.onboarding_completed").increment(1);
                }

                info!(
                    account_id = %account_id,
                    charges_enabled,
                    payouts_enabled,
                    details_submitted,
                    "Updated Stripe connected account status"
                );
            }
        }
        "checkout.session.completed" => {
            if let EventObject::CheckoutSession(session) = &event.data.object
                && let Some(ref pi_id) = session.payment_intent
            {
                let payments_repo = PaymentsRepository::new(state.pool.clone());
                let pi_id_str = pi_id.id().to_string();
                if let Ok(Some(payment)) = payments_repo.get_by_payment_intent_id(&pi_id_str).await
                    && (payment.status == PaymentStatus::Pending
                        || payment.status == PaymentStatus::Processing)
                {
                    payments_repo
                        .update_status(payment.id, PaymentStatus::Processing)
                        .await?;
                }
            }
        }
        "payment_intent.succeeded" => {
            if let EventObject::PaymentIntent(pi) = &event.data.object {
                let payments_repo = PaymentsRepository::new(state.pool.clone());
                let pi_id = pi.id.to_string();
                if let Ok(Some(payment)) = payments_repo.get_by_payment_intent_id(&pi_id).await {
                    payments_repo
                        .update_status(payment.id, PaymentStatus::Succeeded)
                        .await?;
                    metrics::counter!("stripe.payments.succeeded").increment(1);
                    info!(payment_id = %payment.id, "Payment succeeded");
                }
            }
        }
        "payment_intent.payment_failed" => {
            if let EventObject::PaymentIntent(pi) = &event.data.object {
                let payments_repo = PaymentsRepository::new(state.pool.clone());
                let pi_id = pi.id.to_string();
                if let Ok(Some(payment)) = payments_repo.get_by_payment_intent_id(&pi_id).await {
                    payments_repo
                        .update_status(payment.id, PaymentStatus::Failed)
                        .await?;
                    metrics::counter!("stripe.payments.failed").increment(1);
                    warn!(payment_id = %payment.id, "Payment failed");
                }
            }
        }
        _ => {
            info!(event_type = %event_type, "Unhandled webhook event type");
        }
    }

    Ok(())
}

async fn create_account_link(
    stripe_config: &StripeConfig,
    account_id: &stripe::AccountId,
) -> anyhow::Result<String> {
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let mut params = CreateAccountLink::new(account_id.clone(), AccountLinkType::AccountOnboarding);
    params.refresh_url = Some(&base_url);
    params.return_url = Some(&base_url);

    let link = AccountLink::create(&stripe_config.client, params).await?;
    Ok(link.url)
}
