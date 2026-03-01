use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, DbEnum, TS)]
#[db_enum(existing_type_path = "crate::schema::sql_types::PaymentType")]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "snake_case")]
pub enum PaymentType {
    #[db_enum(rename = "tow_charge")]
    TowCharge,
    #[db_enum(rename = "membership_dues")]
    MembershipDues,
    #[db_enum(rename = "platform_subscription")]
    PlatformSubscription,
    #[db_enum(rename = "other")]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, DbEnum, TS)]
#[db_enum(existing_type_path = "crate::schema::sql_types::PaymentStatus")]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    #[db_enum(rename = "pending")]
    Pending,
    #[db_enum(rename = "processing")]
    Processing,
    #[db_enum(rename = "succeeded")]
    Succeeded,
    #[db_enum(rename = "failed")]
    Failed,
    #[db_enum(rename = "refunded")]
    Refunded,
    #[db_enum(rename = "canceled")]
    Canceled,
}

/// API model for payments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: Uuid,
    pub user_id: Uuid,
    pub club_id: Option<Uuid>,
    pub stripe_payment_intent_id: Option<String>,
    pub stripe_invoice_id: Option<String>,
    pub stripe_charge_id: Option<String>,
    pub payment_type: PaymentType,
    pub status: PaymentStatus,
    pub amount_cents: i32,
    pub currency: String,
    pub platform_fee_cents: i32,
    pub description: Option<String>,
    pub metadata: serde_json::Value,
    pub idempotency_key: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Diesel model for the payments table
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::payments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PaymentModel {
    pub id: Uuid,
    pub user_id: Uuid,
    pub club_id: Option<Uuid>,
    pub stripe_payment_intent_id: Option<String>,
    pub stripe_invoice_id: Option<String>,
    pub stripe_charge_id: Option<String>,
    pub payment_type: PaymentType,
    pub status: PaymentStatus,
    pub amount_cents: i32,
    pub currency: String,
    pub platform_fee_cents: i32,
    pub description: Option<String>,
    pub metadata: serde_json::Value,
    pub idempotency_key: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insert model for new payments
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::payments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewPayment {
    pub user_id: Uuid,
    pub club_id: Option<Uuid>,
    pub payment_type: PaymentType,
    pub amount_cents: i32,
    pub currency: String,
    pub platform_fee_cents: i32,
    pub description: Option<String>,
    pub metadata: serde_json::Value,
    pub idempotency_key: Option<String>,
    pub created_by: Uuid,
}

impl From<PaymentModel> for Payment {
    fn from(model: PaymentModel) -> Self {
        Self {
            id: model.id,
            user_id: model.user_id,
            club_id: model.club_id,
            stripe_payment_intent_id: model.stripe_payment_intent_id,
            stripe_invoice_id: model.stripe_invoice_id,
            stripe_charge_id: model.stripe_charge_id,
            payment_type: model.payment_type,
            status: model.status,
            amount_cents: model.amount_cents,
            currency: model.currency,
            platform_fee_cents: model.platform_fee_cents,
            description: model.description,
            metadata: model.metadata,
            idempotency_key: model.idempotency_key,
            created_by: model.created_by,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
