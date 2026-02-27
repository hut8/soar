use anyhow::Result;
use diesel::prelude::*;
use uuid::Uuid;

use crate::stripe_customers::{NewStripeCustomer, StripeCustomer, StripeCustomerModel};
use crate::web::PgPool;

#[derive(Clone)]
pub struct StripeCustomersRepository {
    pool: PgPool,
}

impl StripeCustomersRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get-or-create pattern: returns existing Stripe customer or creates a new one
    pub async fn get_or_create(
        &self,
        user_id: Uuid,
        create_stripe_customer: impl FnOnce() -> Result<String> + Send + 'static,
    ) -> Result<StripeCustomer> {
        // Check if already exists
        if let Some(existing) = self.get_by_user_id(user_id).await? {
            return Ok(existing);
        }

        // Create in Stripe and store locally
        let stripe_customer_id = create_stripe_customer()?;
        self.create(NewStripeCustomer {
            user_id,
            stripe_customer_id,
        })
        .await
    }

    /// Get by user ID
    pub async fn get_by_user_id(&self, user_id: Uuid) -> Result<Option<StripeCustomer>> {
        use crate::schema::stripe_customers::dsl;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let customer: Option<StripeCustomerModel> = dsl::stripe_customers
                .filter(dsl::user_id.eq(user_id))
                .first::<StripeCustomerModel>(&mut conn)
                .optional()?;

            Ok::<Option<StripeCustomerModel>, anyhow::Error>(customer)
        })
        .await??;

        Ok(result.map(|model| model.into()))
    }

    /// Get by Stripe customer ID
    pub async fn get_by_stripe_customer_id(
        &self,
        stripe_customer_id: &str,
    ) -> Result<Option<StripeCustomer>> {
        use crate::schema::stripe_customers::dsl;

        let pool = self.pool.clone();
        let stripe_customer_id = stripe_customer_id.to_string();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let customer: Option<StripeCustomerModel> = dsl::stripe_customers
                .filter(dsl::stripe_customer_id.eq(&stripe_customer_id))
                .first::<StripeCustomerModel>(&mut conn)
                .optional()?;

            Ok::<Option<StripeCustomerModel>, anyhow::Error>(customer)
        })
        .await??;

        Ok(result.map(|model| model.into()))
    }

    /// Create a new customer record
    pub async fn create(&self, new_customer: NewStripeCustomer) -> Result<StripeCustomer> {
        use crate::schema::stripe_customers::dsl;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let inserted: StripeCustomerModel = diesel::insert_into(dsl::stripe_customers)
                .values(&new_customer)
                .get_result(&mut conn)?;

            Ok::<StripeCustomerModel, anyhow::Error>(inserted)
        })
        .await??;

        Ok(result.into())
    }
}
