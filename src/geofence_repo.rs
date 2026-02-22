//! Geofence repository for database operations

use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sql_types;
use moka::sync::Cache;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use crate::geofence::{
    AircraftGeofence, CreateGeofenceRequest, Geofence, GeofenceExitEvent, GeofenceLayer,
    GeofenceSubscriber, UpdateGeofenceRequest,
};
use crate::postgis_functions::{st_make_point, st_set_srid, st_x, st_y};
use crate::schema::{aircraft_geofences, geofence_exit_events, geofence_subscribers, geofences};

type PgPool = Pool<ConnectionManager<PgConnection>>;

// Database record types

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

/// Type alias for the select expression that extracts geofence data with coordinates
type GeofenceSelectExpr = (
    geofences::id,
    geofences::name,
    geofences::description,
    st_y<geofences::center>,
    st_x<geofences::center>,
    geofences::max_radius_meters,
    geofences::layers,
    geofences::owner_user_id,
    geofences::club_id,
    geofences::created_at,
    geofences::updated_at,
);

/// Row type returned by GeofenceSelectExpr
type GeofenceRow = (
    Uuid,
    String,
    Option<String>,
    f64,
    f64,
    f64,
    JsonValue,
    Uuid,
    Option<Uuid>,
    DateTime<Utc>,
    DateTime<Utc>,
);

/// Build the select expression for geofences with coordinate extraction
fn geofence_select() -> GeofenceSelectExpr {
    use crate::schema::geofences::dsl;
    (
        dsl::id,
        dsl::name,
        dsl::description,
        st_y(dsl::center),
        st_x(dsl::center),
        dsl::max_radius_meters,
        dsl::layers,
        dsl::owner_user_id,
        dsl::club_id,
        dsl::created_at,
        dsl::updated_at,
    )
}

/// Convert a row to a Geofence
fn row_to_geofence(row: GeofenceRow) -> Result<Geofence> {
    let layers: Vec<GeofenceLayer> = serde_json::from_value(row.6)?;
    Ok(Geofence {
        id: row.0,
        name: row.1,
        description: row.2,
        center_latitude: row.3,
        center_longitude: row.4,
        max_radius_meters: row.5,
        layers,
        owner_user_id: row.7,
        club_id: row.8,
        created_at: row.9,
        updated_at: row.10,
    })
}

#[derive(Clone)]
pub struct GeofenceRepository {
    pool: PgPool,
    /// Cache of geofences per aircraft_id with 60-second TTL.
    /// Most aircraft have zero geofences, so caching the empty result
    /// eliminates a DB round-trip per fix for every in-flight aircraft.
    geofence_cache: Arc<Cache<Uuid, Vec<Geofence>>>,
}

impl GeofenceRepository {
    pub fn new(pool: PgPool) -> Self {
        let geofence_cache = Arc::new(
            Cache::builder()
                .max_capacity(10_000)
                .time_to_live(Duration::from_secs(60))
                .build(),
        );
        Self {
            pool,
            geofence_cache,
        }
    }

    /// Create a new geofence
    pub async fn create(
        &self,
        owner_user_id: Uuid,
        request: CreateGeofenceRequest,
    ) -> Result<Geofence> {
        use geofences::dsl;

        let pool = self.pool.clone();
        let layers_json = serde_json::to_value(&request.layers)?;
        let max_radius = request.max_radius_meters();
        let lat = request.center_latitude;
        let lon = request.center_longitude;

        tokio::task::spawn_blocking(move || -> Result<Geofence> {
            let mut conn = pool.get()?;

            // Insert using query builder with PostGIS functions
            let id: Uuid = diesel::insert_into(dsl::geofences)
                .values((
                    dsl::name.eq(&request.name),
                    dsl::description.eq(&request.description),
                    dsl::center.eq(st_set_srid(st_make_point(lon, lat), 4326)),
                    dsl::max_radius_meters.eq(max_radius),
                    dsl::layers.eq(&layers_json),
                    dsl::owner_user_id.eq(owner_user_id),
                    dsl::club_id.eq(request.club_id),
                ))
                .returning(dsl::id)
                .get_result(&mut conn)?;

            // Fetch the complete record with coordinates
            let row: GeofenceRow = dsl::geofences
                .filter(dsl::id.eq(id))
                .select(geofence_select())
                .first(&mut conn)?;

            row_to_geofence(row)
        })
        .await?
    }

    /// Get a geofence by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Geofence>> {
        use geofences::dsl;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Option<Geofence>> {
            let mut conn = pool.get()?;

            let row: Option<GeofenceRow> = dsl::geofences
                .filter(dsl::id.eq(id))
                .filter(dsl::deleted_at.is_null())
                .select(geofence_select())
                .first(&mut conn)
                .optional()?;

            row.map(row_to_geofence).transpose()
        })
        .await?
    }

    /// Get geofences for a user (owned by user or user's club)
    pub async fn get_for_user(
        &self,
        user_id: Uuid,
        user_club_id: Option<Uuid>,
    ) -> Result<Vec<(Geofence, i64, i64)>> {
        use diesel::sql_query;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<(Geofence, i64, i64)>> {
            let mut conn = pool.get()?;

            // For counts, we still need a more complex query
            // Using raw SQL for the aggregation but type-safe binding
            #[derive(QueryableByName, Debug)]
            struct GeofenceWithCounts {
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

            let results: Vec<GeofenceWithCounts> = sql_query(
                r#"
                SELECT g.id, g.name, g.description,
                       ST_Y(g.center) as center_latitude,
                       ST_X(g.center) as center_longitude,
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
                  AND (g.owner_user_id = $1 OR g.club_id = $2)
                ORDER BY g.name
                "#,
            )
            .bind::<sql_types::Uuid, _>(user_id)
            .bind::<sql_types::Nullable<sql_types::Uuid>, _>(user_club_id)
            .load(&mut conn)?;

            results
                .into_iter()
                .map(|r| {
                    let layers: Vec<GeofenceLayer> = serde_json::from_value(r.layers)?;
                    Ok((
                        Geofence {
                            id: r.id,
                            name: r.name,
                            description: r.description,
                            center_latitude: r.center_latitude,
                            center_longitude: r.center_longitude,
                            max_radius_meters: r.max_radius_meters,
                            layers,
                            owner_user_id: r.owner_user_id,
                            club_id: r.club_id,
                            created_at: r.created_at,
                            updated_at: r.updated_at,
                        },
                        r.aircraft_count,
                        r.subscriber_count,
                    ))
                })
                .collect()
        })
        .await?
    }

    /// Get geofences for a club
    pub async fn get_for_club(&self, club_id: Uuid) -> Result<Vec<(Geofence, i64, i64)>> {
        use diesel::sql_query;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<(Geofence, i64, i64)>> {
            let mut conn = pool.get()?;

            #[derive(QueryableByName, Debug)]
            struct GeofenceWithCounts {
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

            let results: Vec<GeofenceWithCounts> = sql_query(
                r#"
                SELECT g.id, g.name, g.description,
                       ST_Y(g.center) as center_latitude,
                       ST_X(g.center) as center_longitude,
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
                .map(|r| {
                    let layers: Vec<GeofenceLayer> = serde_json::from_value(r.layers)?;
                    Ok((
                        Geofence {
                            id: r.id,
                            name: r.name,
                            description: r.description,
                            center_latitude: r.center_latitude,
                            center_longitude: r.center_longitude,
                            max_radius_meters: r.max_radius_meters,
                            layers,
                            owner_user_id: r.owner_user_id,
                            club_id: r.club_id,
                            created_at: r.created_at,
                            updated_at: r.updated_at,
                        },
                        r.aircraft_count,
                        r.subscriber_count,
                    ))
                })
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
        use geofences::dsl;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Option<Geofence>> {
            let mut conn = pool.get()?;

            // Check if geofence exists
            let exists: bool = diesel::select(diesel::dsl::exists(
                dsl::geofences
                    .filter(dsl::id.eq(id))
                    .filter(dsl::deleted_at.is_null()),
            ))
            .get_result(&mut conn)?;

            if !exists {
                return Ok(None);
            }

            // Build dynamic update
            // Note: Diesel's AsChangeset doesn't work well with optional PostGIS expressions,
            // so we update fields individually when present
            if let Some(ref name) = request.name {
                diesel::update(dsl::geofences.filter(dsl::id.eq(id)))
                    .set(dsl::name.eq(name))
                    .execute(&mut conn)?;
            }

            if request.description.is_some() {
                diesel::update(dsl::geofences.filter(dsl::id.eq(id)))
                    .set(dsl::description.eq(&request.description))
                    .execute(&mut conn)?;
            }

            if let (Some(lon), Some(lat)) = (request.center_longitude, request.center_latitude) {
                diesel::update(dsl::geofences.filter(dsl::id.eq(id)))
                    .set(dsl::center.eq(st_set_srid(st_make_point(lon, lat), 4326)))
                    .execute(&mut conn)?;
            }

            if let Some(ref layers) = request.layers {
                let layers_json = serde_json::to_value(layers)?;
                let max_radius = request.max_radius_meters();
                diesel::update(dsl::geofences.filter(dsl::id.eq(id)))
                    .set((
                        dsl::layers.eq(&layers_json),
                        dsl::max_radius_meters.eq(max_radius.unwrap_or(0.0)),
                    ))
                    .execute(&mut conn)?;
            }

            // Update timestamp
            diesel::update(dsl::geofences.filter(dsl::id.eq(id)))
                .set(dsl::updated_at.eq(Utc::now()))
                .execute(&mut conn)?;

            // Fetch updated record
            let row: GeofenceRow = dsl::geofences
                .filter(dsl::id.eq(id))
                .select(geofence_select())
                .first(&mut conn)?;

            Ok(Some(row_to_geofence(row)?))
        })
        .await?
    }

    /// Soft delete a geofence
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        use geofences::dsl;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;

            let rows_affected = diesel::update(dsl::geofences.filter(dsl::id.eq(id)))
                .set(dsl::deleted_at.eq(Some(Utc::now())))
                .execute(&mut conn)?;

            Ok(rows_affected > 0)
        })
        .await?
    }

    // ==================== Aircraft Links ====================

    /// Add an aircraft to a geofence
    pub async fn add_aircraft(&self, geofence_id: Uuid, aircraft_id: Uuid) -> Result<()> {
        use aircraft_geofences::dsl;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut conn = pool.get()?;

            diesel::insert_into(dsl::aircraft_geofences)
                .values((
                    dsl::geofence_id.eq(geofence_id),
                    dsl::aircraft_id.eq(aircraft_id),
                ))
                .on_conflict_do_nothing()
                .execute(&mut conn)?;

            Ok(())
        })
        .await?
    }

    /// Remove an aircraft from a geofence
    pub async fn remove_aircraft(&self, geofence_id: Uuid, aircraft_id: Uuid) -> Result<bool> {
        use aircraft_geofences::dsl;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;

            let rows_affected = diesel::delete(
                dsl::aircraft_geofences
                    .filter(dsl::geofence_id.eq(geofence_id))
                    .filter(dsl::aircraft_id.eq(aircraft_id)),
            )
            .execute(&mut conn)?;

            Ok(rows_affected > 0)
        })
        .await?
    }

    /// Get aircraft IDs linked to a geofence
    pub async fn get_aircraft(&self, geofence_id: Uuid) -> Result<Vec<Uuid>> {
        use aircraft_geofences::dsl;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<Uuid>> {
            let mut conn = pool.get()?;

            let ids: Vec<Uuid> = dsl::aircraft_geofences
                .filter(dsl::geofence_id.eq(geofence_id))
                .select(dsl::aircraft_id)
                .load(&mut conn)?;

            Ok(ids)
        })
        .await?
    }

    /// Get all geofences that an aircraft is linked to.
    /// Results are cached for 60 seconds to avoid a DB round-trip per fix.
    pub async fn get_geofences_for_aircraft(&self, aircraft_id: Uuid) -> Result<Vec<Geofence>> {
        // Check cache first
        if let Some(cached) = self.geofence_cache.get(&aircraft_id) {
            metrics::counter!("geofence_repo.geofences_for_aircraft.cache_hit").increment(1);
            return Ok(cached);
        }
        metrics::counter!("geofence_repo.geofences_for_aircraft.cache_miss").increment(1);

        use aircraft_geofences::dsl as ag_dsl;
        use geofences::dsl;

        let pool = self.pool.clone();

        let result: Vec<Geofence> =
            tokio::task::spawn_blocking(move || -> Result<Vec<Geofence>> {
                let mut conn = pool.get()?;

                let rows: Vec<GeofenceRow> = dsl::geofences
                    .inner_join(ag_dsl::aircraft_geofences.on(ag_dsl::geofence_id.eq(dsl::id)))
                    .filter(ag_dsl::aircraft_id.eq(aircraft_id))
                    .filter(dsl::deleted_at.is_null())
                    .select(geofence_select())
                    .load(&mut conn)?;

                rows.into_iter().map(row_to_geofence).collect()
            })
            .await??;

        // Cache the result (including empty results — most aircraft have no geofences)
        self.geofence_cache.insert(aircraft_id, result.clone());

        Ok(result)
    }

    // ==================== Subscribers ====================

    /// Subscribe a user to a geofence
    pub async fn add_subscriber(
        &self,
        geofence_id: Uuid,
        user_id: Uuid,
        send_email: bool,
    ) -> Result<GeofenceSubscriber> {
        use geofence_subscribers::dsl;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<GeofenceSubscriber> {
            let mut conn = pool.get()?;

            let record: GeofenceSubscriberRecord = diesel::insert_into(dsl::geofence_subscribers)
                .values((
                    dsl::geofence_id.eq(geofence_id),
                    dsl::user_id.eq(user_id),
                    dsl::send_email.eq(send_email),
                ))
                .on_conflict((dsl::geofence_id, dsl::user_id))
                .do_update()
                .set(dsl::send_email.eq(send_email))
                .returning(GeofenceSubscriberRecord::as_returning())
                .get_result(&mut conn)?;

            Ok(record.into())
        })
        .await?
    }

    /// Unsubscribe a user from a geofence
    pub async fn remove_subscriber(&self, geofence_id: Uuid, user_id: Uuid) -> Result<bool> {
        use geofence_subscribers::dsl;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;

            let rows_affected = diesel::delete(
                dsl::geofence_subscribers
                    .filter(dsl::geofence_id.eq(geofence_id))
                    .filter(dsl::user_id.eq(user_id)),
            )
            .execute(&mut conn)?;

            Ok(rows_affected > 0)
        })
        .await?
    }

    /// Get subscribers for a geofence
    pub async fn get_subscribers(&self, geofence_id: Uuid) -> Result<Vec<GeofenceSubscriber>> {
        use geofence_subscribers::dsl;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<GeofenceSubscriber>> {
            let mut conn = pool.get()?;

            let records: Vec<GeofenceSubscriberRecord> = dsl::geofence_subscribers
                .filter(dsl::geofence_id.eq(geofence_id))
                .select(GeofenceSubscriberRecord::as_select())
                .load(&mut conn)?;

            Ok(records.into_iter().map(|r| r.into()).collect())
        })
        .await?
    }

    /// Get subscribers who want email notifications
    pub async fn get_subscribers_for_email(&self, geofence_id: Uuid) -> Result<Vec<Uuid>> {
        use geofence_subscribers::dsl;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<Uuid>> {
            let mut conn = pool.get()?;

            let user_ids: Vec<Uuid> = dsl::geofence_subscribers
                .filter(dsl::geofence_id.eq(geofence_id))
                .filter(dsl::send_email.eq(true))
                .select(dsl::user_id)
                .load(&mut conn)?;

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
        use geofence_exit_events::dsl;

        let pool = self.pool.clone();
        let exit_layer = exit_layer.clone();

        tokio::task::spawn_blocking(move || -> Result<GeofenceExitEvent> {
            let mut conn = pool.get()?;

            let record: GeofenceExitEventRecord = diesel::insert_into(dsl::geofence_exit_events)
                .values((
                    dsl::geofence_id.eq(geofence_id),
                    dsl::flight_id.eq(flight_id),
                    dsl::aircraft_id.eq(aircraft_id),
                    dsl::exit_time.eq(exit_time),
                    dsl::exit_latitude.eq(exit_latitude),
                    dsl::exit_longitude.eq(exit_longitude),
                    dsl::exit_altitude_msl_ft.eq(exit_altitude_msl_ft),
                    dsl::exit_layer_floor_ft.eq(exit_layer.floor_ft),
                    dsl::exit_layer_ceiling_ft.eq(exit_layer.ceiling_ft),
                    dsl::exit_layer_radius_nm.eq(exit_layer.radius_nm),
                ))
                .returning(GeofenceExitEventRecord::as_returning())
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

    /// Update the email count for an exit event
    pub async fn update_exit_event_email_count(&self, id: Uuid, count: i32) -> Result<()> {
        use geofence_exit_events::dsl;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut conn = pool.get()?;

            diesel::update(dsl::geofence_exit_events.filter(dsl::id.eq(id)))
                .set(dsl::email_notifications_sent.eq(count))
                .execute(&mut conn)?;

            Ok(())
        })
        .await?
    }

    /// Get exit events for a geofence with optional limit
    pub async fn get_exit_events_for_geofence(
        &self,
        geofence_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<GeofenceExitEvent>> {
        use geofence_exit_events::dsl;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<GeofenceExitEvent>> {
            let mut conn = pool.get()?;

            let mut query = dsl::geofence_exit_events
                .filter(dsl::geofence_id.eq(geofence_id))
                .order(dsl::exit_time.desc())
                .into_boxed();

            if let Some(lim) = limit {
                query = query.limit(lim);
            }

            let records: Vec<GeofenceExitEventRecord> = query
                .select(GeofenceExitEventRecord::as_select())
                .load(&mut conn)?;

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

    /// Get exit events for a geofence (no limit)
    pub async fn get_exit_events(&self, geofence_id: Uuid) -> Result<Vec<GeofenceExitEvent>> {
        use geofence_exit_events::dsl;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<GeofenceExitEvent>> {
            let mut conn = pool.get()?;

            let records: Vec<GeofenceExitEventRecord> = dsl::geofence_exit_events
                .filter(dsl::geofence_id.eq(geofence_id))
                .order(dsl::exit_time.desc())
                .select(GeofenceExitEventRecord::as_select())
                .load(&mut conn)?;

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
        use geofence_exit_events::dsl;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<GeofenceExitEvent>> {
            let mut conn = pool.get()?;

            let records: Vec<GeofenceExitEventRecord> = dsl::geofence_exit_events
                .filter(dsl::flight_id.eq(flight_id))
                .order(dsl::exit_time.asc())
                .select(GeofenceExitEventRecord::as_select())
                .load(&mut conn)?;

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

    /// Check if a user owns a geofence
    pub async fn is_owner(&self, geofence_id: Uuid, user_id: Uuid) -> Result<bool> {
        use geofences::dsl;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;

            let is_owner: bool = diesel::select(diesel::dsl::exists(
                dsl::geofences
                    .filter(dsl::id.eq(geofence_id))
                    .filter(dsl::owner_user_id.eq(user_id))
                    .filter(dsl::deleted_at.is_null()),
            ))
            .get_result(&mut conn)?;

            Ok(is_owner)
        })
        .await?
    }

    /// Get the club ID for a geofence (if any)
    pub async fn get_club_id(&self, geofence_id: Uuid) -> Result<Option<Uuid>> {
        use geofences::dsl;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Option<Uuid>> {
            let mut conn = pool.get()?;

            let club_id: Option<Option<Uuid>> = dsl::geofences
                .filter(dsl::id.eq(geofence_id))
                .filter(dsl::deleted_at.is_null())
                .select(dsl::club_id)
                .first(&mut conn)
                .optional()?;

            Ok(club_id.flatten())
        })
        .await?
    }
}
