use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use stripe::{
    CheckoutSession, CreateCheckoutSession, CreateCheckoutSessionLineItems,
    CreateCheckoutSessionLineItemsPriceData, CreateCheckoutSessionLineItemsPriceDataProductData,
    CreateCheckoutSessionPaymentIntentData, CreateCheckoutSessionPaymentIntentDataTransferData,
    Currency,
};
use tracing::error;
use ts_rs::TS;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::payments::{NewPayment, Payment, PaymentStatus, PaymentType};
use crate::payments_repo::PaymentsRepository;
use crate::stripe_connected_accounts_repo::StripeConnectedAccountsRepository;
use crate::web::AppState;

use super::{DataListResponse, DataResponse, json_error};

/// View model for payments (API response)
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct PaymentView {
    pub id: String,
    pub user_id: String,
    pub club_id: Option<String>,
    pub payment_type: String,
    pub status: String,
    pub amount_cents: i32,
    pub currency: String,
    pub platform_fee_cents: i32,
    pub description: Option<String>,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Payment> for PaymentView {
    fn from(p: Payment) -> Self {
        Self {
            id: p.id.to_string(),
            user_id: p.user_id.to_string(),
            club_id: p.club_id.map(|id| id.to_string()),
            payment_type: serde_json::to_value(p.payment_type)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default(),
            status: serde_json::to_value(p.status)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default(),
            amount_cents: p.amount_cents,
            currency: p.currency,
            platform_fee_cents: p.platform_fee_cents,
            description: p.description,
            created_by: p.created_by.to_string(),
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
        }
    }
}

/// Request body for creating a charge
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct CreateChargeRequest {
    pub user_id: String,
    pub amount_cents: i32,
    pub payment_type: String,
    pub description: Option<String>,
}

/// Response for checkout session creation
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct CheckoutResponse {
    pub checkout_url: String,
}

/// POST /clubs/{id}/charges
/// Club admin creates a charge for a member
pub async fn create_charge(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(club_id): Path<Uuid>,
    Json(request): Json<CreateChargeRequest>,
) -> impl IntoResponse {
    if !auth_user.0.is_admin && auth_user.0.club_id != Some(club_id) {
        return json_error(
            StatusCode::FORBIDDEN,
            "You must be a member of this club to create charges",
        )
        .into_response();
    }

    if request.amount_cents <= 0 {
        return json_error(StatusCode::BAD_REQUEST, "Amount must be greater than 0")
            .into_response();
    }

    let target_user_id: Uuid = match request.user_id.parse() {
        Ok(id) => id,
        Err(_) => {
            return json_error(StatusCode::BAD_REQUEST, "Invalid user ID").into_response();
        }
    };

    let payment_type = match request.payment_type.as_str() {
        "tow_charge" => PaymentType::TowCharge,
        "membership_dues" => PaymentType::MembershipDues,
        "other" => PaymentType::Other,
        _ => {
            return json_error(
                StatusCode::BAD_REQUEST,
                "Invalid payment type. Must be: tow_charge, membership_dues, or other",
            )
            .into_response();
        }
    };

    let stripe_config = match &state.stripe_config {
        Some(config) => config.clone(),
        None => {
            return json_error(StatusCode::SERVICE_UNAVAILABLE, "Stripe is not configured")
                .into_response();
        }
    };

    let platform_fee_cents = stripe_config.calculate_platform_fee(request.amount_cents);

    let repo = PaymentsRepository::new(state.pool.clone());

    let new_payment = NewPayment {
        user_id: target_user_id,
        club_id: Some(club_id),
        payment_type,
        amount_cents: request.amount_cents,
        currency: "usd".to_string(),
        platform_fee_cents,
        description: request.description,
        metadata: serde_json::json!({}),
        idempotency_key: None,
        created_by: auth_user.0.id,
    };

    match repo.create(new_payment).await {
        Ok(payment) => {
            metrics::counter!("stripe.payments.created").increment(1);
            (
                StatusCode::CREATED,
                Json(DataResponse {
                    data: PaymentView::from(payment),
                }),
            )
                .into_response()
        }
        Err(e) => {
            error!(club_id = %club_id, error = %e, "Failed to create charge");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create charge").into_response()
        }
    }
}

/// GET /clubs/{id}/charges
/// List charges for a club
pub async fn list_club_charges(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(club_id): Path<Uuid>,
) -> impl IntoResponse {
    if !auth_user.0.is_admin && auth_user.0.club_id != Some(club_id) {
        return json_error(
            StatusCode::FORBIDDEN,
            "You must be a member of this club to view charges",
        )
        .into_response();
    }

    let repo = PaymentsRepository::new(state.pool.clone());

    match repo.get_by_club_id(club_id).await {
        Ok(payments) => {
            let views: Vec<PaymentView> = payments.into_iter().map(PaymentView::from).collect();
            Json(DataListResponse { data: views }).into_response()
        }
        Err(e) => {
            error!(club_id = %club_id, error = %e, "Failed to list club charges");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to list charges").into_response()
        }
    }
}

/// GET /payments
/// List current user's payments
pub async fn list_my_payments(
    auth_user: AuthUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let repo = PaymentsRepository::new(state.pool.clone());

    match repo.get_by_user_id(auth_user.0.id).await {
        Ok(payments) => {
            let views: Vec<PaymentView> = payments.into_iter().map(PaymentView::from).collect();
            Json(DataListResponse { data: views }).into_response()
        }
        Err(e) => {
            error!(user_id = %auth_user.0.id, error = %e, "Failed to list user payments");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to list payments").into_response()
        }
    }
}

/// GET /payments/{id}
/// Get payment details
pub async fn get_payment(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(payment_id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = PaymentsRepository::new(state.pool.clone());

    match repo.get_by_id(payment_id).await {
        Ok(Some(payment)) => {
            // Only allow access to own payments or admin
            if !auth_user.0.is_admin && payment.user_id != auth_user.0.id {
                return json_error(StatusCode::FORBIDDEN, "Access denied").into_response();
            }
            Json(DataResponse {
                data: PaymentView::from(payment),
            })
            .into_response()
        }
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Payment not found").into_response(),
        Err(e) => {
            error!(payment_id = %payment_id, error = %e, "Failed to get payment");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get payment").into_response()
        }
    }
}

/// POST /payments/{id}/checkout
/// Create a Stripe Checkout Session for a pending payment
pub async fn create_checkout(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Path(payment_id): Path<Uuid>,
) -> impl IntoResponse {
    let stripe_config = match &state.stripe_config {
        Some(config) => config.clone(),
        None => {
            return json_error(StatusCode::SERVICE_UNAVAILABLE, "Stripe is not configured")
                .into_response();
        }
    };

    let payments_repo = PaymentsRepository::new(state.pool.clone());

    let payment = match payments_repo.get_by_id(payment_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return json_error(StatusCode::NOT_FOUND, "Payment not found").into_response();
        }
        Err(e) => {
            error!(error = %e, "Failed to get payment");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get payment")
                .into_response();
        }
    };

    // Only the payment's user can pay for it
    if payment.user_id != auth_user.0.id {
        return json_error(
            StatusCode::FORBIDDEN,
            "You can only pay for your own charges",
        )
        .into_response();
    }

    if payment.status != PaymentStatus::Pending {
        return json_error(StatusCode::BAD_REQUEST, "Payment is not in pending status")
            .into_response();
    }

    // Get the connected account for the club
    let club_id = match payment.club_id {
        Some(id) => id,
        None => {
            return json_error(StatusCode::BAD_REQUEST, "Payment has no associated club")
                .into_response();
        }
    };

    let stripe_accounts_repo = StripeConnectedAccountsRepository::new(state.pool.clone());
    let connected_account = match stripe_accounts_repo.get_by_club_id(club_id).await {
        Ok(Some(account)) if account.charges_enabled => account,
        Ok(Some(_)) => {
            return json_error(
                StatusCode::BAD_REQUEST,
                "Club's Stripe account cannot accept charges yet",
            )
            .into_response();
        }
        Ok(None) => {
            return json_error(StatusCode::BAD_REQUEST, "Club has no Stripe account")
                .into_response();
        }
        Err(e) => {
            error!(error = %e, "Failed to get connected account");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get club's Stripe account",
            )
            .into_response();
        }
    };

    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let description = payment
        .description
        .clone()
        .unwrap_or_else(|| "Payment".to_string());

    let success_url = format!("{}/payments/success?payment_id={}", base_url, payment_id);
    let cancel_url = format!("{}/payments/cancel?payment_id={}", base_url, payment_id);

    let mut checkout_params = CreateCheckoutSession::new();
    checkout_params.success_url = Some(&success_url);
    checkout_params.cancel_url = Some(&cancel_url);
    checkout_params.mode = Some(stripe::CheckoutSessionMode::Payment);
    checkout_params.line_items = Some(vec![CreateCheckoutSessionLineItems {
        price_data: Some(CreateCheckoutSessionLineItemsPriceData {
            currency: Currency::USD,
            product_data: Some(CreateCheckoutSessionLineItemsPriceDataProductData {
                name: description,
                ..Default::default()
            }),
            unit_amount: Some(payment.amount_cents as i64),
            ..Default::default()
        }),
        quantity: Some(1),
        ..Default::default()
    }]);

    // Set up Connect: application fee + transfer to connected account
    checkout_params.payment_intent_data = Some(CreateCheckoutSessionPaymentIntentData {
        application_fee_amount: Some(payment.platform_fee_cents as i64),
        transfer_data: Some(CreateCheckoutSessionPaymentIntentDataTransferData {
            destination: connected_account.stripe_account_id.clone(),
            ..Default::default()
        }),
        ..Default::default()
    });

    // Allow cards and ACH bank transfers
    checkout_params.payment_method_types = Some(vec![
        stripe::CreateCheckoutSessionPaymentMethodTypes::Card,
        stripe::CreateCheckoutSessionPaymentMethodTypes::UsBankAccount,
    ]);

    let session = match CheckoutSession::create(&stripe_config.client, checkout_params).await {
        Ok(session) => session,
        Err(e) => {
            error!(error = %e, "Failed to create checkout session");
            metrics::counter!("stripe.api.errors").increment(1);
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create checkout session",
            )
            .into_response();
        }
    };

    // Update payment with the payment intent ID from the session
    if let Some(ref pi) = session.payment_intent {
        let pi_id = pi.id().to_string();
        if let Err(e) = payments_repo
            .update_stripe_ids(payment_id, Some(pi_id), None)
            .await
        {
            error!(error = %e, "Failed to update payment with Stripe IDs");
        }
    }

    // Update status to processing
    if let Err(e) = payments_repo
        .update_status(payment_id, PaymentStatus::Processing)
        .await
    {
        error!(error = %e, "Failed to update payment status");
    }

    let checkout_url = session.url.unwrap_or_default();

    Json(DataResponse {
        data: CheckoutResponse { checkout_url },
    })
    .into_response()
}
