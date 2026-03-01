use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// API model for Stripe connected accounts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeConnectedAccount {
    pub id: Uuid,
    pub club_id: Uuid,
    pub stripe_account_id: String,
    pub onboarding_complete: bool,
    pub charges_enabled: bool,
    pub payouts_enabled: bool,
    pub details_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Diesel model for the stripe_connected_accounts table
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::stripe_connected_accounts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct StripeConnectedAccountModel {
    pub id: Uuid,
    pub club_id: Uuid,
    pub stripe_account_id: String,
    pub onboarding_complete: bool,
    pub charges_enabled: bool,
    pub payouts_enabled: bool,
    pub details_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insert model for new Stripe connected accounts
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::stripe_connected_accounts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewStripeConnectedAccount {
    pub club_id: Uuid,
    pub stripe_account_id: String,
}

impl From<StripeConnectedAccountModel> for StripeConnectedAccount {
    fn from(model: StripeConnectedAccountModel) -> Self {
        Self {
            id: model.id,
            club_id: model.club_id,
            stripe_account_id: model.stripe_account_id,
            onboarding_complete: model.onboarding_complete,
            charges_enabled: model.charges_enabled,
            payouts_enabled: model.payouts_enabled,
            details_submitted: model.details_submitted,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
