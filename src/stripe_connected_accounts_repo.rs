use anyhow::Result;
use diesel::prelude::*;
use uuid::Uuid;

use crate::stripe_connected_accounts::{
    NewStripeConnectedAccount, StripeConnectedAccount, StripeConnectedAccountModel,
};
use crate::web::PgPool;

#[derive(Clone)]
pub struct StripeConnectedAccountsRepository {
    pool: PgPool,
}

impl StripeConnectedAccountsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get the connected account for a club
    pub async fn get_by_club_id(&self, club_id: Uuid) -> Result<Option<StripeConnectedAccount>> {
        use crate::schema::stripe_connected_accounts::dsl;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let account: Option<StripeConnectedAccountModel> = dsl::stripe_connected_accounts
                .filter(dsl::club_id.eq(club_id))
                .first::<StripeConnectedAccountModel>(&mut conn)
                .optional()?;

            Ok::<Option<StripeConnectedAccountModel>, anyhow::Error>(account)
        })
        .await??;

        Ok(result.map(|model| model.into()))
    }

    /// Get by Stripe account ID (for webhook processing)
    pub async fn get_by_stripe_account_id(
        &self,
        stripe_account_id: &str,
    ) -> Result<Option<StripeConnectedAccount>> {
        use crate::schema::stripe_connected_accounts::dsl;

        let pool = self.pool.clone();
        let stripe_account_id = stripe_account_id.to_string();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let account: Option<StripeConnectedAccountModel> = dsl::stripe_connected_accounts
                .filter(dsl::stripe_account_id.eq(&stripe_account_id))
                .first::<StripeConnectedAccountModel>(&mut conn)
                .optional()?;

            Ok::<Option<StripeConnectedAccountModel>, anyhow::Error>(account)
        })
        .await??;

        Ok(result.map(|model| model.into()))
    }

    /// Create a new connected account record
    pub async fn create(
        &self,
        new_account: NewStripeConnectedAccount,
    ) -> Result<StripeConnectedAccount> {
        use crate::schema::stripe_connected_accounts::dsl;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let inserted: StripeConnectedAccountModel =
                diesel::insert_into(dsl::stripe_connected_accounts)
                    .values(&new_account)
                    .get_result(&mut conn)?;

            Ok::<StripeConnectedAccountModel, anyhow::Error>(inserted)
        })
        .await??;

        Ok(result.into())
    }

    /// Update account status (from webhook data)
    pub async fn update_status(
        &self,
        stripe_account_id: &str,
        charges_enabled: bool,
        payouts_enabled: bool,
        details_submitted: bool,
    ) -> Result<Option<StripeConnectedAccount>> {
        use crate::schema::stripe_connected_accounts;

        let pool = self.pool.clone();
        let stripe_account_id = stripe_account_id.to_string();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let onboarding_complete = charges_enabled && details_submitted;

            let updated: Option<StripeConnectedAccountModel> =
                diesel::update(stripe_connected_accounts::table)
                    .filter(stripe_connected_accounts::stripe_account_id.eq(&stripe_account_id))
                    .set((
                        stripe_connected_accounts::charges_enabled.eq(charges_enabled),
                        stripe_connected_accounts::payouts_enabled.eq(payouts_enabled),
                        stripe_connected_accounts::details_submitted.eq(details_submitted),
                        stripe_connected_accounts::onboarding_complete.eq(onboarding_complete),
                        stripe_connected_accounts::updated_at.eq(diesel::dsl::now),
                    ))
                    .get_result(&mut conn)
                    .optional()?;

            Ok::<Option<StripeConnectedAccountModel>, anyhow::Error>(updated)
        })
        .await??;

        Ok(result.map(|model| model.into()))
    }
}
