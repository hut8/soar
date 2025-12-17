use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use uuid::Uuid;

use crate::schema::watchlist;
use crate::watchlist::WatchlistEntry;

type PgPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = watchlist)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct WatchlistRecord {
    pub user_id: Uuid,
    pub aircraft_id: Uuid,
    pub send_email: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<WatchlistRecord> for WatchlistEntry {
    fn from(record: WatchlistRecord) -> Self {
        WatchlistEntry {
            user_id: record.user_id,
            aircraft_id: record.aircraft_id,
            send_email: record.send_email,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

pub struct WatchlistRepository {
    pool: PgPool,
}

impl WatchlistRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get all watchlist entries for a user
    pub async fn get_by_user(&self, user_id: Uuid) -> Result<Vec<WatchlistEntry>> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<WatchlistEntry>> {
            let mut conn = pool.get()?;
            let records = watchlist::table
                .filter(watchlist::user_id.eq(user_id))
                .order(watchlist::created_at.desc())
                .load::<WatchlistRecord>(&mut conn)?;
            Ok(records.into_iter().map(|r| r.into()).collect())
        })
        .await?
    }

    /// Get users who want email notifications for an aircraft
    pub async fn get_users_for_aircraft_email(&self, aircraft_id: Uuid) -> Result<Vec<Uuid>> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<Uuid>> {
            let mut conn = pool.get()?;
            let user_ids = watchlist::table
                .filter(watchlist::aircraft_id.eq(aircraft_id))
                .filter(watchlist::send_email.eq(true))
                .select(watchlist::user_id)
                .load::<Uuid>(&mut conn)?;
            Ok(user_ids)
        })
        .await?
    }

    /// Add aircraft to user's watchlist
    pub async fn add(
        &self,
        user_id: Uuid,
        aircraft_id: Uuid,
        send_email: bool,
    ) -> Result<WatchlistEntry> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<WatchlistEntry> {
            let mut conn = pool.get()?;
            let record = diesel::insert_into(watchlist::table)
                .values((
                    watchlist::user_id.eq(user_id),
                    watchlist::aircraft_id.eq(aircraft_id),
                    watchlist::send_email.eq(send_email),
                ))
                .on_conflict((watchlist::user_id, watchlist::aircraft_id))
                .do_update()
                .set(watchlist::send_email.eq(send_email))
                .get_result::<WatchlistRecord>(&mut conn)?;
            Ok(record.into())
        })
        .await?
    }

    /// Update email preference for a watchlist entry
    pub async fn update_email_preference(
        &self,
        user_id: Uuid,
        aircraft_id: Uuid,
        send_email: bool,
    ) -> Result<bool> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;
            let rows = diesel::update(watchlist::table)
                .filter(watchlist::user_id.eq(user_id))
                .filter(watchlist::aircraft_id.eq(aircraft_id))
                .set(watchlist::send_email.eq(send_email))
                .execute(&mut conn)?;
            Ok(rows > 0)
        })
        .await?
    }

    /// Remove aircraft from user's watchlist
    pub async fn remove(&self, user_id: Uuid, aircraft_id: Uuid) -> Result<bool> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;
            let rows = diesel::delete(watchlist::table)
                .filter(watchlist::user_id.eq(user_id))
                .filter(watchlist::aircraft_id.eq(aircraft_id))
                .execute(&mut conn)?;
            Ok(rows > 0)
        })
        .await?
    }

    /// Clear all watchlist entries for a user
    pub async fn clear_user(&self, user_id: Uuid) -> Result<usize> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<usize> {
            let mut conn = pool.get()?;
            let rows = diesel::delete(watchlist::table)
                .filter(watchlist::user_id.eq(user_id))
                .execute(&mut conn)?;
            Ok(rows)
        })
        .await?
    }
}
