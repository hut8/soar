use anyhow::Result;
use diesel::prelude::*;
use uuid::Uuid;

use crate::payments::{NewPayment, Payment, PaymentModel, PaymentStatus};
use crate::web::PgPool;

#[derive(Clone)]
pub struct PaymentsRepository {
    pool: PgPool,
}

impl PaymentsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a payment by ID
    pub async fn get_by_id(&self, payment_id: Uuid) -> Result<Option<Payment>> {
        use crate::schema::payments::dsl;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let payment: Option<PaymentModel> = dsl::payments
                .filter(dsl::id.eq(payment_id))
                .first::<PaymentModel>(&mut conn)
                .optional()?;

            Ok::<Option<PaymentModel>, anyhow::Error>(payment)
        })
        .await??;

        Ok(result.map(|model| model.into()))
    }

    /// Get payments for a specific user
    pub async fn get_by_user_id(&self, user_id: Uuid) -> Result<Vec<Payment>> {
        use crate::schema::payments::dsl;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let payments: Vec<PaymentModel> = dsl::payments
                .filter(dsl::user_id.eq(user_id))
                .order_by(dsl::created_at.desc())
                .load::<PaymentModel>(&mut conn)?;

            Ok::<Vec<PaymentModel>, anyhow::Error>(payments)
        })
        .await??;

        Ok(result.into_iter().map(|model| model.into()).collect())
    }

    /// Get payments for a specific club
    pub async fn get_by_club_id(&self, club_id: Uuid) -> Result<Vec<Payment>> {
        use crate::schema::payments::dsl;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let payments: Vec<PaymentModel> = dsl::payments
                .filter(dsl::club_id.eq(club_id))
                .order_by(dsl::created_at.desc())
                .load::<PaymentModel>(&mut conn)?;

            Ok::<Vec<PaymentModel>, anyhow::Error>(payments)
        })
        .await??;

        Ok(result.into_iter().map(|model| model.into()).collect())
    }

    /// Get a payment by Stripe payment intent ID
    pub async fn get_by_payment_intent_id(
        &self,
        payment_intent_id: &str,
    ) -> Result<Option<Payment>> {
        use crate::schema::payments::dsl;

        let pool = self.pool.clone();
        let payment_intent_id = payment_intent_id.to_string();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let payment: Option<PaymentModel> = dsl::payments
                .filter(dsl::stripe_payment_intent_id.eq(&payment_intent_id))
                .first::<PaymentModel>(&mut conn)
                .optional()?;

            Ok::<Option<PaymentModel>, anyhow::Error>(payment)
        })
        .await??;

        Ok(result.map(|model| model.into()))
    }

    /// Create a new payment
    pub async fn create(&self, new_payment: NewPayment) -> Result<Payment> {
        use crate::schema::payments::dsl;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let inserted: PaymentModel = diesel::insert_into(dsl::payments)
                .values(&new_payment)
                .get_result(&mut conn)?;

            Ok::<PaymentModel, anyhow::Error>(inserted)
        })
        .await??;

        Ok(result.into())
    }

    /// Update payment status
    pub async fn update_status(
        &self,
        payment_id: Uuid,
        status: PaymentStatus,
    ) -> Result<Option<Payment>> {
        use crate::schema::payments;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let updated: Option<PaymentModel> = diesel::update(payments::table)
                .filter(payments::id.eq(payment_id))
                .set((
                    payments::status.eq(status),
                    payments::updated_at.eq(diesel::dsl::now),
                ))
                .get_result(&mut conn)
                .optional()?;

            Ok::<Option<PaymentModel>, anyhow::Error>(updated)
        })
        .await??;

        Ok(result.map(|model| model.into()))
    }

    /// Update Stripe IDs on a payment (after checkout session creation)
    pub async fn update_stripe_ids(
        &self,
        payment_id: Uuid,
        payment_intent_id: Option<String>,
        charge_id: Option<String>,
    ) -> Result<Option<Payment>> {
        use crate::schema::payments;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let updated: Option<PaymentModel> = diesel::update(payments::table)
                .filter(payments::id.eq(payment_id))
                .set((
                    payments::stripe_payment_intent_id.eq(payment_intent_id),
                    payments::stripe_charge_id.eq(charge_id),
                    payments::updated_at.eq(diesel::dsl::now),
                ))
                .get_result(&mut conn)
                .optional()?;

            Ok::<Option<PaymentModel>, anyhow::Error>(updated)
        })
        .await??;

        Ok(result.map(|model| model.into()))
    }
}
