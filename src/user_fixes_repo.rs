use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::schema::user_fixes;
use crate::user_fixes::UserFix;

type PgPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = user_fixes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserFixRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub latitude: f64,
    pub longitude: f64,
    pub heading: Option<f64>,
    // Note: location_geom and location_geog are generated columns, not queried directly
    pub raw: Option<JsonValue>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = user_fixes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewUserFix {
    pub id: Uuid,
    pub user_id: Uuid,
    pub latitude: f64,
    pub longitude: f64,
    pub heading: Option<f64>,
    pub raw: Option<JsonValue>,
}

impl From<UserFixRecord> for UserFix {
    fn from(record: UserFixRecord) -> Self {
        UserFix {
            id: record.id,
            user_id: record.user_id,
            latitude: record.latitude,
            longitude: record.longitude,
            heading: record.heading,
            raw: record.raw,
            timestamp: record.timestamp,
        }
    }
}

pub struct UserFixesRepository {
    pool: PgPool,
}

impl UserFixesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new user fix
    pub async fn create(
        &self,
        user_id: Uuid,
        latitude: f64,
        longitude: f64,
        heading: Option<f64>,
        raw: Option<JsonValue>,
    ) -> Result<UserFix> {
        let pool = self.pool.clone();
        let new_fix = NewUserFix {
            id: Uuid::now_v7(),
            user_id,
            latitude,
            longitude,
            heading,
            raw,
        };

        tokio::task::spawn_blocking(move || -> Result<UserFix> {
            let mut conn = pool.get()?;
            let fix = diesel::insert_into(user_fixes::table)
                .values(&new_fix)
                .returning(UserFixRecord::as_returning())
                .get_result(&mut conn)?;
            Ok(fix.into())
        })
        .await?
    }

    /// Get the most recent fix for a user
    pub async fn get_latest_for_user(&self, user_id: Uuid) -> Result<Option<UserFix>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Option<UserFix>> {
            let mut conn = pool.get()?;
            let fix = user_fixes::table
                .filter(user_fixes::user_id.eq(user_id))
                .order(user_fixes::timestamp.desc())
                .select(UserFixRecord::as_select())
                .first(&mut conn)
                .optional()?;
            Ok(fix.map(|r| r.into()))
        })
        .await?
    }

    /// Get recent fixes for a user
    pub async fn get_recent_for_user(&self, user_id: Uuid, limit: i64) -> Result<Vec<UserFix>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<UserFix>> {
            let mut conn = pool.get()?;
            let fixes = user_fixes::table
                .filter(user_fixes::user_id.eq(user_id))
                .order(user_fixes::timestamp.desc())
                .limit(limit)
                .select(UserFixRecord::as_select())
                .load(&mut conn)?;
            Ok(fixes.into_iter().map(|r| r.into()).collect())
        })
        .await?
    }
}
