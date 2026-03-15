use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use uuid::Uuid;

use crate::push_subscriptions::{PushSubscriptionRequest, PushSubscriptionView};
use crate::schema::{push_subscriptions, users};

type PgPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = push_subscriptions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PushSubscriptionRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub endpoint: String,
    pub p256dh_key: String,
    pub auth_key: String,
    pub notify_takeoff: bool,
    pub notify_landing: bool,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<PushSubscriptionRecord> for PushSubscriptionView {
    fn from(r: PushSubscriptionRecord) -> Self {
        Self {
            id: r.id,
            notify_takeoff: r.notify_takeoff,
            notify_landing: r.notify_landing,
            created_at: r.created_at,
        }
    }
}

#[derive(Clone)]
pub struct PushSubscriptionsRepository {
    pool: PgPool,
}

impl PushSubscriptionsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get all subscriptions for a user
    pub async fn get_by_user(&self, user_id: Uuid) -> Result<Vec<PushSubscriptionView>> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<PushSubscriptionView>> {
            let mut conn = pool.get()?;
            let results = push_subscriptions::table
                .filter(push_subscriptions::user_id.eq(user_id))
                .order(push_subscriptions::created_at.desc())
                .load::<PushSubscriptionRecord>(&mut conn)?;
            Ok(results.into_iter().map(|r| r.into()).collect())
        })
        .await?
    }

    /// Create or update a push subscription (upsert on endpoint)
    pub async fn upsert(
        &self,
        user_id: Uuid,
        req: &PushSubscriptionRequest,
        user_agent_str: Option<String>,
    ) -> Result<PushSubscriptionView> {
        let pool = self.pool.clone();
        let endpoint = req.endpoint.clone();
        let p256dh_key = req.p256dh_key.clone();
        let auth_key = req.auth_key.clone();
        let notify_takeoff = req.notify_takeoff;
        let notify_landing = req.notify_landing;

        tokio::task::spawn_blocking(move || -> Result<PushSubscriptionView> {
            let mut conn = pool.get()?;
            let record = diesel::insert_into(push_subscriptions::table)
                .values((
                    push_subscriptions::user_id.eq(user_id),
                    push_subscriptions::endpoint.eq(&endpoint),
                    push_subscriptions::p256dh_key.eq(&p256dh_key),
                    push_subscriptions::auth_key.eq(&auth_key),
                    push_subscriptions::notify_takeoff.eq(notify_takeoff),
                    push_subscriptions::notify_landing.eq(notify_landing),
                    push_subscriptions::user_agent.eq(&user_agent_str),
                ))
                .on_conflict(push_subscriptions::endpoint)
                .do_update()
                .set((
                    push_subscriptions::user_id.eq(user_id),
                    push_subscriptions::p256dh_key.eq(&p256dh_key),
                    push_subscriptions::auth_key.eq(&auth_key),
                    push_subscriptions::notify_takeoff.eq(notify_takeoff),
                    push_subscriptions::notify_landing.eq(notify_landing),
                    push_subscriptions::user_agent.eq(&user_agent_str),
                    push_subscriptions::updated_at.eq(diesel::dsl::now),
                ))
                .get_result::<PushSubscriptionRecord>(&mut conn)?;
            Ok(record.into())
        })
        .await?
    }

    /// Delete a subscription by id (must belong to user)
    pub async fn delete(&self, user_id: Uuid, subscription_id: Uuid) -> Result<bool> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;
            let rows = diesel::delete(push_subscriptions::table)
                .filter(push_subscriptions::id.eq(subscription_id))
                .filter(push_subscriptions::user_id.eq(user_id))
                .execute(&mut conn)?;
            Ok(rows > 0)
        })
        .await?
    }

    /// Delete a subscription by endpoint (used when push returns 410 Gone)
    pub async fn delete_by_endpoint(&self, endpoint: &str) -> Result<bool> {
        let pool = self.pool.clone();
        let endpoint = endpoint.to_string();
        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;
            let rows = diesel::delete(push_subscriptions::table)
                .filter(push_subscriptions::endpoint.eq(&endpoint))
                .execute(&mut conn)?;
            Ok(rows > 0)
        })
        .await?
    }

    /// Get all subscriptions for users in a given club that want takeoff notifications
    pub async fn get_takeoff_subscribers(
        &self,
        club_id: Uuid,
    ) -> Result<Vec<PushSubscriptionRecord>> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<PushSubscriptionRecord>> {
            let mut conn = pool.get()?;
            let results = push_subscriptions::table
                .inner_join(users::table)
                .filter(users::club_id.eq(club_id))
                .filter(users::deleted_at.is_null())
                .filter(push_subscriptions::notify_takeoff.eq(true))
                .select(PushSubscriptionRecord::as_select())
                .load(&mut conn)?;
            Ok(results)
        })
        .await?
    }

    /// Get all subscriptions for users in a given club that want landing notifications
    pub async fn get_landing_subscribers(
        &self,
        club_id: Uuid,
    ) -> Result<Vec<PushSubscriptionRecord>> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<PushSubscriptionRecord>> {
            let mut conn = pool.get()?;
            let results = push_subscriptions::table
                .inner_join(users::table)
                .filter(users::club_id.eq(club_id))
                .filter(users::deleted_at.is_null())
                .filter(push_subscriptions::notify_landing.eq(true))
                .select(PushSubscriptionRecord::as_select())
                .load(&mut conn)?;
            Ok(results)
        })
        .await?
    }
}
