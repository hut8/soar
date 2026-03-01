use anyhow::Result;
use diesel::prelude::*;

use crate::stripe_webhooks::{NewStripeWebhookEvent, StripeWebhookEventModel};
use crate::web::PgPool;

#[derive(Clone)]
pub struct StripeWebhookEventsRepository {
    pool: PgPool,
}

impl StripeWebhookEventsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Check if an event has already been processed (idempotency)
    pub async fn is_processed(&self, stripe_event_id: &str) -> Result<bool> {
        use crate::schema::stripe_webhook_events::dsl;

        let pool = self.pool.clone();
        let stripe_event_id = stripe_event_id.to_string();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let exists: bool = diesel::select(diesel::dsl::exists(
                dsl::stripe_webhook_events
                    .filter(dsl::stripe_event_id.eq(&stripe_event_id))
                    .filter(dsl::processed.eq(true)),
            ))
            .get_result(&mut conn)?;

            Ok::<bool, anyhow::Error>(exists)
        })
        .await??;

        Ok(result)
    }

    /// Record a new webhook event
    pub async fn create(
        &self,
        new_event: NewStripeWebhookEvent,
    ) -> Result<StripeWebhookEventModel> {
        use crate::schema::stripe_webhook_events::dsl;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let inserted: StripeWebhookEventModel = diesel::insert_into(dsl::stripe_webhook_events)
                .values(&new_event)
                .on_conflict(dsl::stripe_event_id)
                .do_nothing()
                .get_result(&mut conn)
                .optional()?
                .ok_or_else(|| anyhow::anyhow!("Event already exists"))?;

            Ok::<StripeWebhookEventModel, anyhow::Error>(inserted)
        })
        .await??;

        Ok(result)
    }

    /// Mark an event as processed
    pub async fn mark_processed(&self, stripe_event_id: &str) -> Result<()> {
        use crate::schema::stripe_webhook_events;

        let pool = self.pool.clone();
        let stripe_event_id = stripe_event_id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            diesel::update(stripe_webhook_events::table)
                .filter(stripe_webhook_events::stripe_event_id.eq(&stripe_event_id))
                .set(stripe_webhook_events::processed.eq(true))
                .execute(&mut conn)?;

            Ok::<(), anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    /// Mark an event as failed with an error message
    pub async fn mark_failed(&self, stripe_event_id: &str, error: &str) -> Result<()> {
        use crate::schema::stripe_webhook_events;

        let pool = self.pool.clone();
        let stripe_event_id = stripe_event_id.to_string();
        let error = error.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            diesel::update(stripe_webhook_events::table)
                .filter(stripe_webhook_events::stripe_event_id.eq(&stripe_event_id))
                .set((
                    stripe_webhook_events::processed.eq(true),
                    stripe_webhook_events::processing_error.eq(Some(&error)),
                ))
                .execute(&mut conn)?;

            Ok::<(), anyhow::Error>(())
        })
        .await??;

        Ok(())
    }
}
