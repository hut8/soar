use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::schema::user_fixes;
use crate::user_fixes::{AirportUserPresence, UserFix};

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

    /// Find distinct users who were present within a radius of an airport on a given date.
    /// Uses the airport's `location` geography column and user_fixes `location_geog` column
    /// for an efficient ST_DWithin query.
    pub async fn get_users_present_at_airport(
        &self,
        airport_id: i32,
        date: NaiveDate,
        radius_meters: f64,
    ) -> Result<Vec<UserPresenceRecord>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<UserPresenceRecord>> {
            let mut conn = pool.get()?;

            let sql = r#"
                SELECT DISTINCT ON (u.id)
                       u.id as user_id,
                       u.first_name,
                       u.last_name,
                       u.is_licensed,
                       u.is_instructor,
                       u.is_tow_pilot,
                       u.is_examiner,
                       uf.timestamp as last_seen_at
                FROM user_fixes uf
                JOIN users u ON u.id = uf.user_id
                JOIN airports a ON a.id = $1
                WHERE a.location IS NOT NULL
                  AND uf.timestamp >= ($2::date)::timestamptz
                  AND uf.timestamp < ($2::date + interval '1 day')::timestamptz
                  AND ST_DWithin(
                        uf.location_geog,
                        a.location,
                        $3
                      )
                  AND u.deleted_at IS NULL
                ORDER BY u.id, uf.timestamp DESC
            "#;

            let results: Vec<UserPresenceRecord> = diesel::sql_query(sql)
                .bind::<diesel::sql_types::Integer, _>(airport_id)
                .bind::<diesel::sql_types::Date, _>(date)
                .bind::<diesel::sql_types::Double, _>(radius_meters)
                .load::<UserPresenceRecord>(&mut conn)?;

            Ok(results)
        })
        .await?
    }
}

#[derive(QueryableByName, Debug, Clone)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserPresenceRecord {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub user_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub first_name: String,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub last_name: String,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub is_licensed: bool,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub is_instructor: bool,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub is_tow_pilot: bool,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub is_examiner: bool,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub last_seen_at: DateTime<Utc>,
}

impl From<UserPresenceRecord> for AirportUserPresence {
    fn from(record: UserPresenceRecord) -> Self {
        Self {
            user_id: record.user_id,
            first_name: record.first_name,
            last_name: record.last_name,
            is_licensed: record.is_licensed,
            is_instructor: record.is_instructor,
            is_tow_pilot: record.is_tow_pilot,
            is_examiner: record.is_examiner,
            last_seen_at: record.last_seen_at,
        }
    }
}
