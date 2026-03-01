use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// API model for Stripe customers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeCustomer {
    pub id: Uuid,
    pub user_id: Uuid,
    pub stripe_customer_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Diesel model for the stripe_customers table
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::stripe_customers)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct StripeCustomerModel {
    pub id: Uuid,
    pub user_id: Uuid,
    pub stripe_customer_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insert model for new Stripe customers
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::stripe_customers)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewStripeCustomer {
    pub user_id: Uuid,
    pub stripe_customer_id: String,
}

impl From<StripeCustomerModel> for StripeCustomer {
    fn from(model: StripeCustomerModel) -> Self {
        Self {
            id: model.id,
            user_id: model.user_id,
            stripe_customer_id: model.stripe_customer_id,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
