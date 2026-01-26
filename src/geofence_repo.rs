//! Geofence repository for database operations

use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sql_query;
use diesel::sql_types;
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::geofence::{
    AircraftGeofence, CreateGeofenceRequest, Geofence, GeofenceExitEvent, GeofenceLayer,
    GeofenceSubscriber, UpdateGeofenceRequest,
};
use crate::schema::{aircraft_geofences, geofence_exit_events, geofence_subscribers, geofences};

type PgPool = Pool<ConnectionManager<PgConnection>>;

// Database record types

// Note: We don't use a simple GeofenceRecord struct because the `center` column
// is a Geography type that requires raw SQL to extract lat/lon coordinates.
// All queries use GeofenceWithCoords instead.

#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = geofence_subscribers)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GeofenceSubscriberRecord {
    pub geofence_id: Uuid,
    pub user_id: Uuid,
    pub send_email: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<GeofenceSubscriberRecord> for GeofenceSubscriber {
    fn from(record: GeofenceSubscriberRecord) -> Self {
        GeofenceSubscriber {
            geofence_id: record.geofence_id,
            user_id: record.user_id,
            send_email: record.send_email,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = aircraft_geofences)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AircraftGeofenceRecord {
    pub aircraft_id: Uuid,
    pub geofence_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl From<AircraftGeofenceRecord> for AircraftGeofence {
    fn from(record: AircraftGeofenceRecord) -> Self {
        AircraftGeofence {
            aircraft_id: record.aircraft_id,
            geofence_id: record.geofence_id,
            created_at: record.created_at,
        }
    }
}

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = geofence_exit_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GeofenceExitEventRecord {
    pub id: Uuid,
    pub geofence_id: Uuid,
    pub flight_id: Uuid,
    pub aircraft_id: Uuid,
    pub exit_time: DateTime<Utc>,
    pub exit_latitude: f64,
    pub exit_longitude: f64,
    pub exit_altitude_msl_ft: Option<i32>,
    pub exit_layer_floor_ft: i32,
    pub exit_layer_ceiling_ft: i32,
    pub exit_layer_radius_nm: f64,
    pub email_notifications_sent: i32,
    pub created_at: DateTime<Utc>,
}

// Query result type for geofence with coordinates
#[derive(QueryableByName, Debug)]
struct GeofenceWithCoords {
    #[diesel(sql_type = sql_types::Uuid)]
    id: Uuid,
    #[diesel(sql_type = sql_types::Text)]
    name: String,
    #[diesel(sql_type = sql_types::Nullable<sql_types::Text>)]
    description: Option<String>,
    #[diesel(sql_type = sql_types::Double)]
    center_latitude: f64,
    #[diesel(sql_type = sql_types::Double)]
    center_longitude: f64,
    #[diesel(sql_type = sql_types::Double)]
    max_radius_meters: f64,
    #[diesel(sql_type = sql_types::Jsonb)]
    layers: JsonValue,
    #[diesel(sql_type = sql_types::Uuid)]
    owner_user_id: Uuid,
    #[diesel(sql_type = sql_types::Nullable<sql_types::Uuid>)]
    club_id: Option<Uuid>,
    #[diesel(sql_type = sql_types::Timestamptz)]
    created_at: DateTime<Utc>,
    #[diesel(sql_type = sql_types::Timestamptz)]
    updated_at: DateTime<Utc>,
}

impl GeofenceWithCoords {
    fn into_geofence(self) -> Result<Geofence> {
        let layers: Vec<GeofenceLayer> = serde_json::from_value(self.layers)?;
        Ok(Geofence {
            id: self.id,
            name: self.name,
            description: self.description,
            center_latitude: self.center_latitude,
            center_longitude: self.center_longitude,
            max_radius_meters: self.max_radius_meters,
            layers,
            owner_user_id: self.owner_user_id,
            club_id: self.club_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

// Query result for geofence with counts
#[derive(QueryableByName, Debug)]
struct GeofenceWithCoordsAndCounts {
    #[diesel(sql_type = sql_types::Uuid)]
    id: Uuid,
    #[diesel(sql_type = sql_types::Text)]
    name: String,
    #[diesel(sql_type = sql_types::Nullable<sql_types::Text>)]
    description: Option<String>,
    #[diesel(sql_type = sql_types::Double)]
    center_latitude: f64,
    #[diesel(sql_type = sql_types::Double)]
    center_longitude: f64,
    #[diesel(sql_type = sql_types::Double)]
    max_radius_meters: f64,
    #[diesel(sql_type = sql_types::Jsonb)]
    layers: JsonValue,
    #[diesel(sql_type = sql_types::Uuid)]
    owner_user_id: Uuid,
    #[diesel(sql_type = sql_types::Nullable<sql_types::Uuid>)]
    club_id: Option<Uuid>,
    #[diesel(sql_type = sql_types::Timestamptz)]
    created_at: DateTime<Utc>,
    #[diesel(sql_type = sql_types::Timestamptz)]
    updated_at: DateTime<Utc>,
    #[diesel(sql_type = sql_types::BigInt)]
    aircraft_count: i64,
    #[diesel(sql_type = sql_types::BigInt)]
    subscriber_count: i64,
}

impl GeofenceWithCoordsAndCounts {
    fn into_geofence_with_counts(self) -> Result<(Geofence, i64, i64)> {
        let layers: Vec<GeofenceLayer> = serde_json::from_value(self.layers)?;
        Ok((
            Geofence {
                id: self.id,
                name: self.name,
                description: self.description,
                center_latitude: self.center_latitude,
                center_longitude: self.center_longitude,
                max_radius_meters: self.max_radius_meters,
                layers,
                owner_user_id: self.owner_user_id,
                club_id: self.club_id,
                created_at: self.created_at,
                updated_at: self.updated_at,
            },
            self.aircraft_count,
            self.subscriber_count,
        ))
    }
}

#[derive(Clone)]
pub struct GeofenceRepository {
    pool: PgPool,
}

impl GeofenceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new geofence
    pub async fn create(
        &self,
        owner_user_id: Uuid,
        request: CreateGeofenceRequest,
    ) -> Result<Geofence> {
        let pool = self.pool.clone();
        let layers_json = serde_json::to_value(&request.layers)?;
        let max_radius = request.max_radius_meters();

        tokio::task::spawn_blocking(move || -> Result<Geofence> {
            let mut conn = pool.get()?;

            // Use raw SQL to insert with geography point
            let result: GeofenceWithCoords = sql_query(
                r#"
                INSERT INTO geofences (name, description, center, max_radius_meters, layers, owner_user_id, club_id)
                VALUES ($1, $2, ST_SetSRID(ST_MakePoint($3, $4), 4326)::geography, $5, $6, $7, $8)
                RETURNING id, name, description,
                          ST_Y(center::geometry) as center_latitude,
                          ST_X(center::geometry) as center_longitude,
                          max_radius_meters, layers, owner_user_id, club_id,
                          created_at, updated_at
                "#,
            )
            .bind::<sql_types::Text, _>(&request.name)
            .bind::<sql_types::Nullable<sql_types::Text>, _>(request.description.as_deref())
            .bind::<sql_types::Double, _>(request.center_longitude)
            .bind::<sql_types::Double, _>(request.center_latitude)
            .bind::<sql_types::Double, _>(max_radius)
            .bind::<sql_types::Jsonb, _>(&layers_json)
            .bind::<sql_types::Uuid, _>(owner_user_id)
            .bind::<sql_types::Nullable<sql_types::Uuid>, _>(request.club_id)
            .get_result(&mut conn)?;

            result.into_geofence()
        })
        .await?
    }

    /// Get a geofence by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Geofence>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Option<Geofence>> {
            let mut conn = pool.get()?;

            let result: Option<GeofenceWithCoords> = sql_query(
                r#"
                SELECT id, name, description,
                       ST_Y(center::geometry) as center_latitude,
                       ST_X(center::geometry) as center_longitude,
                       max_radius_meters, layers, owner_user_id, club_id,
                       created_at, updated_at
                FROM geofences
                WHERE id = $1 AND deleted_at IS NULL
                "#,
            )
            .bind::<sql_types::Uuid, _>(id)
            .get_result(&mut conn)
            .optional()?;

            result.map(|r| r.into_geofence()).transpose()
        })
        .await?
    }

    /// Get geofences for a user (owned by user or user's club)
    pub async fn get_for_user(
        &self,
        user_id: Uuid,
        club_id: Option<Uuid>,
    ) -> Result<Vec<(Geofence, i64, i64)>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<(Geofence, i64, i64)>> {
            let mut conn = pool.get()?;

            let results: Vec<GeofenceWithCoordsAndCounts> = sql_query(
                r#"
                SELECT g.id, g.name, g.description,
                       ST_Y(g.center::geometry) as center_latitude,
                       ST_X(g.center::geometry) as center_longitude,
                       g.max_radius_meters, g.layers, g.owner_user_id, g.club_id,
                       g.created_at, g.updated_at,
                       COALESCE(ac.aircraft_count, 0) as aircraft_count,
                       COALESCE(sc.subscriber_count, 0) as subscriber_count
                FROM geofences g
                LEFT JOIN (
                    SELECT geofence_id, COUNT(*) as aircraft_count
                    FROM aircraft_geofences
                    GROUP BY geofence_id
                ) ac ON ac.geofence_id = g.id
                LEFT JOIN (
                    SELECT geofence_id, COUNT(*) as subscriber_count
                    FROM geofence_subscribers
                    GROUP BY geofence_id
                ) sc ON sc.geofence_id = g.id
                WHERE g.deleted_at IS NULL
                  AND (g.owner_user_id = $1 OR ($2 IS NOT NULL AND g.club_id = $2))
                ORDER BY g.name
                "#,
            )
            .bind::<sql_types::Uuid, _>(user_id)
            .bind::<sql_types::Nullable<sql_types::Uuid>, _>(club_id)
            .load(&mut conn)?;

            results
                .into_iter()
                .map(|r| r.into_geofence_with_counts())
                .collect()
        })
        .await?
    }

    /// Get geofences for a specific club
    pub async fn get_for_club(&self, club_id: Uuid) -> Result<Vec<(Geofence, i64, i64)>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<(Geofence, i64, i64)>> {
            let mut conn = pool.get()?;

            let results: Vec<GeofenceWithCoordsAndCounts> = sql_query(
                r#"
                SELECT g.id, g.name, g.description,
                       ST_Y(g.center::geometry) as center_latitude,
                       ST_X(g.center::geometry) as center_longitude,
                       g.max_radius_meters, g.layers, g.owner_user_id, g.club_id,
                       g.created_at, g.updated_at,
                       COALESCE(ac.aircraft_count, 0) as aircraft_count,
                       COALESCE(sc.subscriber_count, 0) as subscriber_count
                FROM geofences g
                LEFT JOIN (
                    SELECT geofence_id, COUNT(*) as aircraft_count
                    FROM aircraft_geofences
                    GROUP BY geofence_id
                ) ac ON ac.geofence_id = g.id
                LEFT JOIN (
                    SELECT geofence_id, COUNT(*) as subscriber_count
                    FROM geofence_subscribers
                    GROUP BY geofence_id
                ) sc ON sc.geofence_id = g.id
                WHERE g.deleted_at IS NULL AND g.club_id = $1
                ORDER BY g.name
                "#,
            )
            .bind::<sql_types::Uuid, _>(club_id)
            .load(&mut conn)?;

            results
                .into_iter()
                .map(|r| r.into_geofence_with_counts())
                .collect()
        })
        .await?
    }

    /// Update a geofence
    pub async fn update(
        &self,
        id: Uuid,
        request: UpdateGeofenceRequest,
    ) -> Result<Option<Geofence>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Option<Geofence>> {
            let mut conn = pool.get()?;

            // Use COALESCE for optional updates - simpler than dynamic query building
            let layers_json = request
                .layers
                .as_ref()
                .and_then(|l| serde_json::to_value(l).ok());
            let max_radius = request.max_radius_meters();

            let result: Option<GeofenceWithCoords> = sql_query(
                r#"
                UPDATE geofences SET
                    name = COALESCE($2, name),
                    description = COALESCE($3, description),
                    center = CASE
                        WHEN $4::float8 IS NOT NULL AND $5::float8 IS NOT NULL
                        THEN ST_SetSRID(ST_MakePoint($4, $5), 4326)::geography
                        ELSE center
                    END,
                    layers = COALESCE($6, layers),
                    max_radius_meters = COALESCE($7, max_radius_meters),
                    updated_at = NOW()
                WHERE id = $1 AND deleted_at IS NULL
                RETURNING id, name, description,
                          ST_Y(center::geometry) as center_latitude,
                          ST_X(center::geometry) as center_longitude,
                          max_radius_meters, layers, owner_user_id, club_id,
                          created_at, updated_at
                "#,
            )
            .bind::<sql_types::Uuid, _>(id)
            .bind::<sql_types::Nullable<sql_types::Text>, _>(request.name.as_deref())
            .bind::<sql_types::Nullable<sql_types::Text>, _>(request.description.as_deref())
            .bind::<sql_types::Nullable<sql_types::Double>, _>(request.center_longitude)
            .bind::<sql_types::Nullable<sql_types::Double>, _>(request.center_latitude)
            .bind::<sql_types::Nullable<sql_types::Jsonb>, _>(layers_json.as_ref())
            .bind::<sql_types::Nullable<sql_types::Double>, _>(max_radius)
            .get_result(&mut conn)
            .optional()?;

            result.map(|r| r.into_geofence()).transpose()
        })
        .await?
    }

    /// Soft delete a geofence
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;

            let rows = diesel::update(geofences::table)
                .filter(geofences::id.eq(id))
                .filter(geofences::deleted_at.is_null())
                .set(geofences::deleted_at.eq(Some(Utc::now())))
                .execute(&mut conn)?;

            Ok(rows > 0)
        })
        .await?
    }

    // ==================== Aircraft Links ====================

    /// Link an aircraft to a geofence
    pub async fn add_aircraft(
        &self,
        geofence_id: Uuid,
        aircraft_id: Uuid,
    ) -> Result<AircraftGeofence> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<AircraftGeofence> {
            let mut conn = pool.get()?;

            let record = diesel::insert_into(aircraft_geofences::table)
                .values((
                    aircraft_geofences::geofence_id.eq(geofence_id),
                    aircraft_geofences::aircraft_id.eq(aircraft_id),
                ))
                .on_conflict((
                    aircraft_geofences::aircraft_id,
                    aircraft_geofences::geofence_id,
                ))
                .do_nothing()
                .get_result::<AircraftGeofenceRecord>(&mut conn)
                .optional()?;

            // If we got nothing back (conflict), fetch the existing record
            let record = match record {
                Some(r) => r,
                None => aircraft_geofences::table
                    .filter(aircraft_geofences::geofence_id.eq(geofence_id))
                    .filter(aircraft_geofences::aircraft_id.eq(aircraft_id))
                    .first::<AircraftGeofenceRecord>(&mut conn)?,
            };

            Ok(record.into())
        })
        .await?
    }

    /// Remove an aircraft from a geofence
    pub async fn remove_aircraft(&self, geofence_id: Uuid, aircraft_id: Uuid) -> Result<bool> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;

            let rows = diesel::delete(aircraft_geofences::table)
                .filter(aircraft_geofences::geofence_id.eq(geofence_id))
                .filter(aircraft_geofences::aircraft_id.eq(aircraft_id))
                .execute(&mut conn)?;

            Ok(rows > 0)
        })
        .await?
    }

    /// Get aircraft linked to a geofence
    pub async fn get_aircraft(&self, geofence_id: Uuid) -> Result<Vec<Uuid>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<Uuid>> {
            let mut conn = pool.get()?;

            let aircraft_ids = aircraft_geofences::table
                .filter(aircraft_geofences::geofence_id.eq(geofence_id))
                .select(aircraft_geofences::aircraft_id)
                .load::<Uuid>(&mut conn)?;

            Ok(aircraft_ids)
        })
        .await?
    }

    /// Get geofences for an aircraft (used by detection logic)
    pub async fn get_geofences_for_aircraft(&self, aircraft_id: Uuid) -> Result<Vec<Geofence>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<Geofence>> {
            let mut conn = pool.get()?;

            let results: Vec<GeofenceWithCoords> = sql_query(
                r#"
                SELECT g.id, g.name, g.description,
                       ST_Y(g.center::geometry) as center_latitude,
                       ST_X(g.center::geometry) as center_longitude,
                       g.max_radius_meters, g.layers, g.owner_user_id, g.club_id,
                       g.created_at, g.updated_at
                FROM geofences g
                INNER JOIN aircraft_geofences ag ON ag.geofence_id = g.id
                WHERE ag.aircraft_id = $1 AND g.deleted_at IS NULL
                "#,
            )
            .bind::<sql_types::Uuid, _>(aircraft_id)
            .load(&mut conn)?;

            results.into_iter().map(|r| r.into_geofence()).collect()
        })
        .await?
    }

    // ==================== Subscribers ====================

    /// Subscribe a user to a geofence
    pub async fn add_subscriber(
        &self,
        geofence_id: Uuid,
        user_id: Uuid,
        send_email: bool,
    ) -> Result<GeofenceSubscriber> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<GeofenceSubscriber> {
            let mut conn = pool.get()?;

            let record = diesel::insert_into(geofence_subscribers::table)
                .values((
                    geofence_subscribers::geofence_id.eq(geofence_id),
                    geofence_subscribers::user_id.eq(user_id),
                    geofence_subscribers::send_email.eq(send_email),
                ))
                .on_conflict((
                    geofence_subscribers::geofence_id,
                    geofence_subscribers::user_id,
                ))
                .do_update()
                .set(geofence_subscribers::send_email.eq(send_email))
                .get_result::<GeofenceSubscriberRecord>(&mut conn)?;

            Ok(record.into())
        })
        .await?
    }

    /// Unsubscribe a user from a geofence
    pub async fn remove_subscriber(&self, geofence_id: Uuid, user_id: Uuid) -> Result<bool> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;

            let rows = diesel::delete(geofence_subscribers::table)
                .filter(geofence_subscribers::geofence_id.eq(geofence_id))
                .filter(geofence_subscribers::user_id.eq(user_id))
                .execute(&mut conn)?;

            Ok(rows > 0)
        })
        .await?
    }

    /// Get subscribers for a geofence
    pub async fn get_subscribers(&self, geofence_id: Uuid) -> Result<Vec<GeofenceSubscriber>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<GeofenceSubscriber>> {
            let mut conn = pool.get()?;

            let records = geofence_subscribers::table
                .filter(geofence_subscribers::geofence_id.eq(geofence_id))
                .order(geofence_subscribers::created_at.asc())
                .load::<GeofenceSubscriberRecord>(&mut conn)?;

            Ok(records.into_iter().map(|r| r.into()).collect())
        })
        .await?
    }

    /// Get users who want email notifications for a geofence
    pub async fn get_subscribers_for_email(&self, geofence_id: Uuid) -> Result<Vec<Uuid>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<Uuid>> {
            let mut conn = pool.get()?;

            let user_ids = geofence_subscribers::table
                .filter(geofence_subscribers::geofence_id.eq(geofence_id))
                .filter(geofence_subscribers::send_email.eq(true))
                .select(geofence_subscribers::user_id)
                .load::<Uuid>(&mut conn)?;

            Ok(user_ids)
        })
        .await?
    }

    // ==================== Exit Events ====================

    /// Create a geofence exit event
    #[allow(clippy::too_many_arguments)]
    pub async fn create_exit_event(
        &self,
        geofence_id: Uuid,
        flight_id: Uuid,
        aircraft_id: Uuid,
        exit_time: DateTime<Utc>,
        exit_latitude: f64,
        exit_longitude: f64,
        exit_altitude_msl_ft: Option<i32>,
        exit_layer: &GeofenceLayer,
    ) -> Result<GeofenceExitEvent> {
        let pool = self.pool.clone();
        let exit_layer = exit_layer.clone();

        tokio::task::spawn_blocking(move || -> Result<GeofenceExitEvent> {
            let mut conn = pool.get()?;

            let record: GeofenceExitEventRecord = diesel::insert_into(geofence_exit_events::table)
                .values((
                    geofence_exit_events::geofence_id.eq(geofence_id),
                    geofence_exit_events::flight_id.eq(flight_id),
                    geofence_exit_events::aircraft_id.eq(aircraft_id),
                    geofence_exit_events::exit_time.eq(exit_time),
                    geofence_exit_events::exit_latitude.eq(exit_latitude),
                    geofence_exit_events::exit_longitude.eq(exit_longitude),
                    geofence_exit_events::exit_altitude_msl_ft.eq(exit_altitude_msl_ft),
                    geofence_exit_events::exit_layer_floor_ft.eq(exit_layer.floor_ft),
                    geofence_exit_events::exit_layer_ceiling_ft.eq(exit_layer.ceiling_ft),
                    geofence_exit_events::exit_layer_radius_nm.eq(exit_layer.radius_nm),
                ))
                .get_result(&mut conn)?;

            Ok(GeofenceExitEvent {
                id: record.id,
                geofence_id: record.geofence_id,
                flight_id: record.flight_id,
                aircraft_id: record.aircraft_id,
                exit_time: record.exit_time,
                exit_latitude: record.exit_latitude,
                exit_longitude: record.exit_longitude,
                exit_altitude_msl_ft: record.exit_altitude_msl_ft,
                exit_layer: GeofenceLayer {
                    floor_ft: record.exit_layer_floor_ft,
                    ceiling_ft: record.exit_layer_ceiling_ft,
                    radius_nm: record.exit_layer_radius_nm,
                },
                email_notifications_sent: record.email_notifications_sent,
                created_at: record.created_at,
            })
        })
        .await?
    }

    /// Update the email notifications sent count
    pub async fn update_exit_event_email_count(&self, event_id: Uuid, count: i32) -> Result<bool> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;

            let rows = diesel::update(geofence_exit_events::table)
                .filter(geofence_exit_events::id.eq(event_id))
                .set(geofence_exit_events::email_notifications_sent.eq(count))
                .execute(&mut conn)?;

            Ok(rows > 0)
        })
        .await?
    }

    /// Get exit events for a geofence
    pub async fn get_exit_events_for_geofence(
        &self,
        geofence_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<GeofenceExitEvent>> {
        let pool = self.pool.clone();
        let limit = limit.unwrap_or(100).min(500);

        tokio::task::spawn_blocking(move || -> Result<Vec<GeofenceExitEvent>> {
            let mut conn = pool.get()?;

            let records = geofence_exit_events::table
                .filter(geofence_exit_events::geofence_id.eq(geofence_id))
                .order(geofence_exit_events::exit_time.desc())
                .limit(limit)
                .load::<GeofenceExitEventRecord>(&mut conn)?;

            Ok(records
                .into_iter()
                .map(|r| GeofenceExitEvent {
                    id: r.id,
                    geofence_id: r.geofence_id,
                    flight_id: r.flight_id,
                    aircraft_id: r.aircraft_id,
                    exit_time: r.exit_time,
                    exit_latitude: r.exit_latitude,
                    exit_longitude: r.exit_longitude,
                    exit_altitude_msl_ft: r.exit_altitude_msl_ft,
                    exit_layer: GeofenceLayer {
                        floor_ft: r.exit_layer_floor_ft,
                        ceiling_ft: r.exit_layer_ceiling_ft,
                        radius_nm: r.exit_layer_radius_nm,
                    },
                    email_notifications_sent: r.email_notifications_sent,
                    created_at: r.created_at,
                })
                .collect())
        })
        .await?
    }

    /// Get exit events for a flight
    pub async fn get_exit_events_for_flight(
        &self,
        flight_id: Uuid,
    ) -> Result<Vec<GeofenceExitEvent>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<GeofenceExitEvent>> {
            let mut conn = pool.get()?;

            let records = geofence_exit_events::table
                .filter(geofence_exit_events::flight_id.eq(flight_id))
                .order(geofence_exit_events::exit_time.asc())
                .load::<GeofenceExitEventRecord>(&mut conn)?;

            Ok(records
                .into_iter()
                .map(|r| GeofenceExitEvent {
                    id: r.id,
                    geofence_id: r.geofence_id,
                    flight_id: r.flight_id,
                    aircraft_id: r.aircraft_id,
                    exit_time: r.exit_time,
                    exit_latitude: r.exit_latitude,
                    exit_longitude: r.exit_longitude,
                    exit_altitude_msl_ft: r.exit_altitude_msl_ft,
                    exit_layer: GeofenceLayer {
                        floor_ft: r.exit_layer_floor_ft,
                        ceiling_ft: r.exit_layer_ceiling_ft,
                        radius_nm: r.exit_layer_radius_nm,
                    },
                    email_notifications_sent: r.email_notifications_sent,
                    created_at: r.created_at,
                })
                .collect())
        })
        .await?
    }
}
