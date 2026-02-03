use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use tracing::{debug, info, instrument, trace};
use uuid::Uuid;

use crate::fixes::Fix;
use crate::ogn_aprs_aircraft::AddressType as ForeignAddressType;
use crate::web::PgPool;

// Import the main AddressType from aircraft module
use crate::aircraft::AddressType;

// Conversions between foreign types and wrapper types
impl From<ForeignAddressType> for AddressType {
    fn from(foreign_type: ForeignAddressType) -> Self {
        match foreign_type {
            ForeignAddressType::Unknown => AddressType::Unknown,
            ForeignAddressType::Icao => AddressType::Icao,
            ForeignAddressType::Flarm => AddressType::Flarm,
            ForeignAddressType::OgnTracker => AddressType::Ogn,
        }
    }
}

impl From<AddressType> for ForeignAddressType {
    fn from(wrapper_type: AddressType) -> Self {
        match wrapper_type {
            AddressType::Unknown => ForeignAddressType::Unknown,
            AddressType::Icao => ForeignAddressType::Icao,
            AddressType::Flarm => ForeignAddressType::Flarm,
            AddressType::Ogn => ForeignAddressType::OgnTracker,
        }
    }
}

// Queryable struct for Diesel DSL queries (excluding geography and geom columns)
#[derive(Queryable, Debug)]
struct FixDslRow {
    id: Uuid,
    source: String,
    latitude: f64,
    longitude: f64,
    altitude_msl_feet: Option<i32>,
    flight_number: Option<String>,
    squawk: Option<String>,
    ground_speed_knots: Option<f32>,
    track_degrees: Option<f32>,
    climb_fpm: Option<i32>,
    turn_rate_rot: Option<f32>,
    source_metadata: Option<serde_json::Value>,
    flight_id: Option<Uuid>,
    aircraft_id: Uuid,
    received_at: DateTime<Utc>,
    is_active: bool,
    altitude_agl_feet: Option<i32>,
    receiver_id: Option<Uuid>,
    raw_message_id: Uuid,
    altitude_agl_valid: bool,
    time_gap_seconds: Option<i32>,
}

impl From<FixDslRow> for Fix {
    fn from(row: FixDslRow) -> Self {
        Self {
            id: row.id,
            source: row.source,
            received_at: row.received_at,
            latitude: row.latitude,
            longitude: row.longitude,
            altitude_msl_feet: row.altitude_msl_feet,
            altitude_agl_feet: row.altitude_agl_feet,
            flight_id: row.flight_id,
            flight_number: row.flight_number,
            squawk: row.squawk,
            ground_speed_knots: row.ground_speed_knots,
            track_degrees: row.track_degrees,
            climb_fpm: row.climb_fpm,
            turn_rate_rot: row.turn_rate_rot,
            source_metadata: row.source_metadata,
            aircraft_id: row.aircraft_id,
            is_active: row.is_active,
            receiver_id: row.receiver_id,
            raw_message_id: row.raw_message_id,
            altitude_agl_valid: row.altitude_agl_valid,
            time_gap_seconds: row.time_gap_seconds,
        }
    }
}

/// Cluster result from grid-based spatial clustering
#[derive(Debug, Clone)]
pub struct ClusterResult {
    pub grid_lat: f64,
    pub grid_lng: f64,
    pub aircraft_count: i64,
    pub centroid_lat: f64,
    pub centroid_lng: f64,
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lng: f64,
    pub max_lng: f64,
}

#[derive(Clone)]
pub struct FixesRepository {
    pool: PgPool,
}

impl FixesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new fix into the database
    #[tracing::instrument(skip(self, fix), fields(aircraft_id = %fix.aircraft_id, flight_id = ?fix.flight_id))]
    pub async fn insert(&self, fix: &Fix) -> Result<()> {
        use crate::schema::fixes::dsl::*;

        let new_fix = fix.clone();
        let pool = self.pool.clone();

        // Note: aircraft_id, receiver_id, and raw_message_id are already populated in the Fix
        // by the generic processor context before Fix creation

        let insert_pool = pool.clone();
        let inserted = tokio::task::spawn_blocking(move || {
            let mut conn = insert_pool.get()?;

            match diesel::insert_into(fixes)
                .values(&new_fix)
                .execute(&mut conn)
            {
                Ok(_) => {
                    metrics::counter!("aprs.fixes.inserted_total").increment(1);
                    trace!(
                        "Inserted fix | Aircraft: {:?} | {:.6},{:.6} @ {}ft | https://maps.google.com/maps?q={:.6},{:.6}",
                        new_fix.aircraft_id,
                        new_fix.latitude,
                        new_fix.longitude,
                        new_fix.altitude_msl_feet.map_or("Unknown".to_string(), |a| a.to_string()),
                        new_fix.latitude,
                        new_fix.longitude
                    );
                    Ok::<bool, anyhow::Error>(true)
                }
                Err(diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                )) => {
                    // Duplicate fix on redelivery - this is expected after crashes
                    debug!(
                        "Duplicate fix detected for aircraft {} at {}",
                        new_fix.aircraft_id, new_fix.received_at
                    );
                    metrics::counter!("aprs.fixes.duplicate_on_redelivery_total").increment(1);
                    // Not an error - just skip the duplicate
                    Ok::<bool, anyhow::Error>(false)
                }
                Err(e) => Err(e.into()),
            }
        })
        .await??;

        // Schedule async update of aircraft position fields.
        // This data is only read by web API queries, not the fix processing pipeline,
        // so a few milliseconds of staleness is acceptable.
        if inserted {
            let fix_for_update = fix.clone();
            tokio::spawn(async move {
                let _ = tokio::task::spawn_blocking(move || {
                    if let Ok(fix_json) = serde_json::to_value(&fix_for_update)
                        && let Ok(mut conn) = pool.get()
                    {
                        use crate::schema::aircraft;
                        let _ = diesel::update(aircraft::table)
                            .filter(aircraft::id.eq(fix_for_update.aircraft_id))
                            .set((
                                aircraft::current_fix.eq(fix_json),
                                aircraft::latitude.eq(fix_for_update.latitude),
                                aircraft::longitude.eq(fix_for_update.longitude),
                                aircraft::last_fix_at.eq(fix_for_update.received_at),
                            ))
                            .execute(&mut conn);
                    }
                })
                .await;
            });
        }

        Ok(())
    }

    /// Update the altitude_agl field for a specific fix
    pub async fn update_altitude_agl(
        &self,
        fix_id: Uuid,
        altitude_agl_value: Option<i32>,
    ) -> Result<()> {
        use crate::schema::fixes::dsl::*;
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(fixes.filter(id.eq(fix_id)))
                .set((
                    altitude_agl_feet.eq(altitude_agl_value),
                    altitude_agl_valid.eq(true), // Mark as valid even if NULL (no elevation data)
                ))
                .execute(&mut conn)?;
            Ok::<(), anyhow::Error>(())
        })
        .await?
    }

    /// Get fixes for a specific aircraft ID within a time range (original method)
    pub async fn get_fixes_for_aircraft_with_time_range(
        &self,
        aircraft_id: &uuid::Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<Fix>> {
        let aircraft_id_param = *aircraft_id;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes;
            let mut conn = pool.get()?;

            let mut query = fixes::table
                .filter(fixes::aircraft_id.eq(aircraft_id_param))
                .filter(fixes::received_at.between(start_time, end_time))
                .order(fixes::received_at.desc())
                .into_boxed();

            // Only apply limit if specified
            if let Some(limit_value) = limit {
                query = query.limit(limit_value);
            }

            let results = query.select(Fix::as_select()).load::<Fix>(&mut conn)?;

            Ok::<Vec<Fix>, anyhow::Error>(results)
        })
        .await??;

        Ok(result)
    }

    /// Get recent fixes for an aircraft with optional lookback period
    ///
    /// # Arguments
    /// * `aircraft_uuid` - The aircraft UUID to query
    /// * `limit` - Maximum number of fixes to return (default: 100)
    /// * `lookback_hours` - How many hours back to search (default: 24)
    pub async fn get_fixes_for_aircraft(
        &self,
        aircraft_uuid: uuid::Uuid,
        limit: Option<i64>,
        lookback_hours: Option<i64>,
    ) -> Result<Vec<Fix>> {
        use crate::schema::fixes::dsl::*;

        let limit = limit.unwrap_or(100);
        let lookback = lookback_hours.unwrap_or(24);
        let cutoff_time = Utc::now() - Duration::hours(lookback);
        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let results = fixes
                .filter(aircraft_id.eq(aircraft_uuid))
                .filter(received_at.ge(cutoff_time))
                .order(received_at.desc())
                .limit(limit)
                .select(Fix::as_select())
                .load::<Fix>(&mut conn)?;

            Ok::<Vec<Fix>, anyhow::Error>(results)
        })
        .await??;

        Ok(result)
    }

    /// Get recent fixes (most recent first)
    /// Only returns fixes from the last 24 hours for partition pruning
    pub async fn get_recent_fixes(&self, limit: i64) -> Result<Vec<Fix>> {
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;

            let cutoff_time = Utc::now() - Duration::hours(24);
            let results = fixes
                .filter(received_at.ge(cutoff_time))
                .order(received_at.desc())
                .limit(limit)
                .select(Fix::as_select())
                .load::<Fix>(&mut conn)?;

            Ok::<Vec<Fix>, anyhow::Error>(results)
        })
        .await??;

        Ok(result)
    }

    /// Get fixes for a specific aircraft with optional after timestamp and limit
    pub async fn get_fixes_by_aircraft(
        &self,
        aircraft_uuid: Uuid,
        after: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<Fix>> {
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;
            let mut query = fixes.filter(aircraft_id.eq(aircraft_uuid)).into_boxed();
            if let Some(after_timestamp) = after {
                query = query.filter(received_at.gt(after_timestamp));
            }
            let results = query
                .order(received_at.asc())
                .limit(limit)
                .select(Fix::as_select())
                .load::<Fix>(&mut conn)?;
            Ok::<Vec<Fix>, anyhow::Error>(results)
        })
        .await??;

        Ok(result)
    }

    /// Get the most recent fix for an aircraft after a given timestamp
    pub async fn get_latest_fix_for_aircraft(
        &self,
        aircraft_uuid: Uuid,
        after: DateTime<Utc>,
    ) -> Result<Option<Fix>> {
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;
            let fix_result = fixes
                .filter(aircraft_id.eq(aircraft_uuid))
                .filter(received_at.gt(after))
                .order(received_at.desc())
                .limit(1)
                .select(Fix::as_select())
                .first::<Fix>(&mut conn)
                .optional()?;
            Ok::<Option<Fix>, anyhow::Error>(fix_result)
        })
        .await??;

        Ok(result)
    }

    pub async fn get_fixes_by_aircraft_paginated(
        &self,
        aircraft_uuid: Uuid,
        after: Option<DateTime<Utc>>,
        page: i64,
        per_page: i64,
        active_only: Option<bool>,
    ) -> Result<(Vec<Fix>, i64)> {
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes;
            let mut conn = pool.get()?;

            // Build base query for count
            let mut count_query = fixes::table
                .filter(fixes::aircraft_id.eq(aircraft_uuid))
                .into_boxed();
            if let Some(after_timestamp) = after {
                count_query = count_query.filter(fixes::received_at.gt(after_timestamp));
            }
            if active_only == Some(true) {
                count_query = count_query.filter(fixes::is_active.eq(true));
            }
            let total_count = count_query.count().get_result::<i64>(&mut conn)?;

            // Build query for paginated results
            let mut query = fixes::table
                .filter(fixes::aircraft_id.eq(aircraft_uuid))
                .into_boxed();
            if let Some(after_timestamp) = after {
                query = query.filter(fixes::received_at.gt(after_timestamp));
            }
            if active_only == Some(true) {
                query = query.filter(fixes::is_active.eq(true));
            }
            let offset = (page - 1) * per_page;
            let results = query
                .order(fixes::received_at.desc())
                .limit(per_page)
                .offset(offset)
                .select(Fix::as_select())
                .load::<Fix>(&mut conn)?;

            Ok::<(Vec<Fix>, i64), anyhow::Error>((results, total_count))
        })
        .await??;

        Ok(result)
    }

    /// Get fixes by receiver ID with pagination (last 24 hours only)
    /// Returns fixes with full aircraft information
    pub async fn get_fixes_by_receiver_id_paginated(
        &self,
        receiver_uuid: Uuid,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<crate::fixes::FixWithAircraftInfo>, i64)> {
        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || {
            use crate::actions::views::AircraftView;
            use crate::aircraft::AircraftModel;
            use crate::schema::{aircraft, fixes};
            let mut conn = pool.get()?;

            // Only get fixes from the last 24 hours
            let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(24);

            // Get total count
            let total_count = fixes::table
                .filter(fixes::receiver_id.eq(receiver_uuid))
                .filter(fixes::received_at.gt(cutoff_time))
                .count()
                .get_result::<i64>(&mut conn)?;

            // Get paginated results (most recent first)
            let offset = (page - 1) * per_page;
            let fix_results = fixes::table
                .filter(fixes::receiver_id.eq(receiver_uuid))
                .filter(fixes::received_at.gt(cutoff_time))
                .order(fixes::received_at.desc())
                .limit(per_page)
                .offset(offset)
                .select(Fix::as_select())
                .load::<Fix>(&mut conn)?;

            // Get aircraft for these fixes
            let aircraft_ids: Vec<Uuid> = fix_results.iter().map(|fix| fix.aircraft_id).collect();
            let aircraft_models: Vec<AircraftModel> = aircraft::table
                .filter(aircraft::id.eq_any(&aircraft_ids))
                .select(AircraftModel::as_select())
                .load::<AircraftModel>(&mut conn)?;

            // Create a map of aircraft ID to AircraftView
            let aircraft_map: std::collections::HashMap<Uuid, AircraftView> = aircraft_models
                .into_iter()
                .map(|model| (model.id, AircraftView::from_device_model(model)))
                .collect();

            // Combine fixes with their aircraft
            let results = fix_results
                .into_iter()
                .map(|fix| {
                    let aircraft = aircraft_map.get(&fix.aircraft_id).cloned();
                    crate::fixes::FixWithAircraftInfo::new(fix, aircraft)
                })
                .collect();

            Ok::<(Vec<crate::fixes::FixWithAircraftInfo>, i64), anyhow::Error>((
                results,
                total_count,
            ))
        })
        .await??;

        Ok(result)
    }

    /// Get fixes by source (receiver callsign) with pagination (last 24 hours only)
    pub async fn get_fixes_by_source_paginated(
        &self,
        source_callsign: &str,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<Fix>, i64)> {
        let pool = self.pool.clone();
        let source_callsign = source_callsign.to_string();

        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;

            // Only get fixes from the last 24 hours for partition pruning
            let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(24);

            // Get total count
            let total_count = fixes
                .filter(source.eq(&source_callsign))
                .filter(received_at.gt(cutoff_time))
                .count()
                .get_result::<i64>(&mut conn)?;

            // Get paginated results (most recent first)
            let offset = (page - 1) * per_page;
            let results = fixes
                .filter(source.eq(&source_callsign))
                .filter(received_at.gt(cutoff_time))
                .order(received_at.desc())
                .limit(per_page)
                .offset(offset)
                .select(Fix::as_select())
                .load::<Fix>(&mut conn)?;

            Ok::<(Vec<Fix>, i64), anyhow::Error>((results, total_count))
        })
        .await??;

        Ok(result)
    }

    /// Count aircraft in bounding box with cached location
    pub async fn count_aircraft_in_bounding_box(
        &self,
        nw_lat: f64,
        nw_lng: f64,
        se_lat: f64,
        se_lng: f64,
        cutoff_time: DateTime<Utc>,
    ) -> Result<i64> {
        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Count aircraft with cached location in the bounding box
            let count_sql = r#"
                WITH params AS (
                    SELECT
                        $1::double precision AS left_lng,
                        $2::double precision AS bottom_lat,
                        $3::double precision AS right_lng,
                        $4::double precision AS top_lat,
                        $5::timestamptz AS cutoff_time
                ),
                parts AS (
                    SELECT
                        CASE WHEN left_lng <= right_lng THEN
                            ARRAY[
                                ST_MakeEnvelope(left_lng, bottom_lat, right_lng, top_lat, 4326)::geometry
                            ]
                        ELSE
                            ARRAY[
                                ST_MakeEnvelope(left_lng, bottom_lat, 180, top_lat, 4326)::geometry,
                                ST_MakeEnvelope(-180, bottom_lat, right_lng, top_lat, 4326)::geometry
                            ]
                        END AS boxes,
                        cutoff_time
                    FROM params
                )
                SELECT COUNT(*)
                FROM aircraft d, parts
                WHERE d.last_fix_at >= parts.cutoff_time
                  AND d.location_geom IS NOT NULL
                  AND (
                      d.location_geom && parts.boxes[1]
                      OR (array_length(parts.boxes, 1) = 2 AND d.location_geom && parts.boxes[2])
                  )
            "#;

            #[derive(QueryableByName)]
            struct CountRow {
                #[diesel(sql_type = diesel::sql_types::BigInt)]
                count: i64,
            }

            let count_result: CountRow = diesel::sql_query(count_sql)
                .bind::<diesel::sql_types::Double, _>(nw_lng)  // min_lon
                .bind::<diesel::sql_types::Double, _>(se_lat)  // min_lat
                .bind::<diesel::sql_types::Double, _>(se_lng)  // max_lon
                .bind::<diesel::sql_types::Double, _>(nw_lat)  // max_lat
                .bind::<diesel::sql_types::Timestamptz, _>(cutoff_time)
                .get_result(&mut conn)?;

            Ok::<i64, anyhow::Error>(count_result.count)
        })
        .await??;

        Ok(result)
    }

    /// Get clustered aircraft in bounding box using PostGIS ST_SnapToGrid
    #[instrument(skip(self))]
    pub async fn get_clustered_aircraft_in_bounding_box(
        &self,
        nw_lat: f64,
        nw_lng: f64,
        se_lat: f64,
        se_lng: f64,
        cutoff_time: DateTime<Utc>,
        grid_size: f64,
    ) -> Result<Vec<ClusterResult>> {
        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let cluster_sql = r#"
                WITH params AS (
                    SELECT
                        $1::double precision AS left_lng,
                        $2::double precision AS bottom_lat,
                        $3::double precision AS right_lng,
                        $4::double precision AS top_lat,
                        $5::timestamptz AS cutoff_time,
                        $6::double precision AS grid_size
                ),
                parts AS (
                    SELECT
                        CASE WHEN left_lng <= right_lng THEN
                            ARRAY[
                                ST_MakeEnvelope(left_lng, bottom_lat, right_lng, top_lat, 4326)::geometry
                            ]
                        ELSE
                            ARRAY[
                                ST_MakeEnvelope(left_lng, bottom_lat, 180, top_lat, 4326)::geometry,
                                ST_MakeEnvelope(-180, bottom_lat, right_lng, top_lat, 4326)::geometry
                            ]
                        END AS boxes,
                        cutoff_time,
                        grid_size
                    FROM params
                ),
                aircraft_in_bbox AS (
                    SELECT
                        d.id,
                        d.latitude,
                        d.longitude,
                        d.location_geom,
                        parts.grid_size
                    FROM aircraft d, parts
                    WHERE d.last_fix_at >= parts.cutoff_time
                      AND d.location_geom IS NOT NULL
                      AND (
                          d.location_geom && parts.boxes[1]
                          OR (array_length(parts.boxes, 1) = 2 AND d.location_geom && parts.boxes[2])
                      )
                ),
                grid_clusters AS (
                    SELECT
                        -- Use floor-based cell assignment to get the southwest corner of each grid cell
                        -- ST_SnapToGrid rounds to nearest, but we need floor semantics for proper cell alignment
                        ST_SetSRID(ST_MakePoint(
                            FLOOR(longitude / grid_size) * grid_size,
                            FLOOR(latitude / grid_size) * grid_size
                        ), 4326) AS grid_point,
                        COUNT(*) AS aircraft_count,
                        AVG(latitude) AS centroid_lat,
                        AVG(longitude) AS centroid_lng,
                        MIN(latitude) AS min_lat,
                        MAX(latitude) AS max_lat,
                        MIN(longitude) AS min_lng,
                        MAX(longitude) AS max_lng
                    FROM aircraft_in_bbox
                    GROUP BY grid_point
                )
                SELECT
                    ST_X(grid_point) AS grid_lng,
                    ST_Y(grid_point) AS grid_lat,
                    aircraft_count,
                    centroid_lat,
                    centroid_lng,
                    min_lat,
                    max_lat,
                    min_lng,
                    max_lng
                FROM grid_clusters
                ORDER BY aircraft_count DESC
            "#;

            #[derive(QueryableByName)]
            struct ClusterRow {
                #[diesel(sql_type = diesel::sql_types::Double)]
                grid_lng: f64,
                #[diesel(sql_type = diesel::sql_types::Double)]
                grid_lat: f64,
                #[diesel(sql_type = diesel::sql_types::BigInt)]
                aircraft_count: i64,
                #[diesel(sql_type = diesel::sql_types::Double)]
                centroid_lat: f64,
                #[diesel(sql_type = diesel::sql_types::Double)]
                centroid_lng: f64,
                #[diesel(sql_type = diesel::sql_types::Double)]
                min_lat: f64,
                #[diesel(sql_type = diesel::sql_types::Double)]
                max_lat: f64,
                #[diesel(sql_type = diesel::sql_types::Double)]
                min_lng: f64,
                #[diesel(sql_type = diesel::sql_types::Double)]
                max_lng: f64,
            }

            let clusters: Vec<ClusterRow> = diesel::sql_query(cluster_sql)
                .bind::<diesel::sql_types::Double, _>(nw_lng)
                .bind::<diesel::sql_types::Double, _>(se_lat)
                .bind::<diesel::sql_types::Double, _>(se_lng)
                .bind::<diesel::sql_types::Double, _>(nw_lat)
                .bind::<diesel::sql_types::Timestamptz, _>(cutoff_time)
                .bind::<diesel::sql_types::Double, _>(grid_size)
                .load(&mut conn)?;

            let results: Vec<ClusterResult> = clusters
                .into_iter()
                .map(|row| ClusterResult {
                    grid_lat: row.grid_lat,
                    grid_lng: row.grid_lng,
                    aircraft_count: row.aircraft_count,
                    centroid_lat: row.centroid_lat,
                    centroid_lng: row.centroid_lng,
                    min_lat: row.min_lat,
                    max_lat: row.max_lat,
                    min_lng: row.min_lng,
                    max_lng: row.max_lng,
                })
                .collect();

            Ok::<Vec<ClusterResult>, anyhow::Error>(results)
        })
        .await??;

        Ok(result)
    }

    /// Get aircraft with their recent fixes in a bounding box for efficient area subscriptions
    /// This replaces the inefficient global fetch + filter approach
    #[instrument(skip(self), fields(fixes_per_aircraft = fixes_per_aircraft.unwrap_or(5)))]
    pub async fn get_aircraft_with_fixes_in_bounding_box(
        &self,
        nw_lat: f64,
        nw_lng: f64,
        se_lat: f64,
        se_lng: f64,
        cutoff_time: DateTime<Utc>,
        fixes_per_aircraft: Option<i64>,
    ) -> Result<Vec<(crate::aircraft::AircraftModel, Vec<Fix>)>> {
        info!("Starting bounding box query (using current_fix from aircraft table)");
        let pool = self.pool.clone();
        // fixes_per_aircraft parameter is ignored - we use current_fix from aircraft table instead

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            info!("Got database connection, executing first query for aircraft");

            // First query: Get aircraft with cached location in the bounding box
            // Uses cached location_geom column for fast spatial queries with GIST index
            // Handles antimeridian (international date line) crossing by splitting into two boxes
            let aircraft_sql = r#"
                WITH params AS (
                    SELECT
                        $1::double precision AS left_lng,
                        $2::double precision AS bottom_lat,
                        $3::double precision AS right_lng,
                        $4::double precision AS top_lat,
                        $5::timestamptz AS cutoff_time
                ),
                parts AS (
                    SELECT
                        CASE WHEN left_lng <= right_lng THEN
                            ARRAY[
                                ST_MakeEnvelope(left_lng, bottom_lat, right_lng, top_lat, 4326)::geometry
                            ]
                        ELSE
                            ARRAY[
                                ST_MakeEnvelope(left_lng, bottom_lat, 180, top_lat, 4326)::geometry,
                                ST_MakeEnvelope(-180, bottom_lat, right_lng, top_lat, 4326)::geometry
                            ]
                        END AS boxes,
                        cutoff_time
                    FROM params
                )
                SELECT d.*
                FROM aircraft d, parts
                WHERE d.last_fix_at >= parts.cutoff_time
                  AND d.location_geom IS NOT NULL
                  AND (
                      d.location_geom && parts.boxes[1]
                      OR (array_length(parts.boxes, 1) = 2 AND d.location_geom && parts.boxes[2])
                  )
            "#;

            #[derive(QueryableByName)]
            struct AircraftRow {
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Int4>)]
                icao_address: Option<i32>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Int4>)]
                flarm_address: Option<i32>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Int4>)]
                ogn_address: Option<i32>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Int4>)]
                other_address: Option<i32>,
                #[diesel(sql_type = diesel::sql_types::Text)]
                aircraft_model: String,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                registration: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Text)]
                competition_number: String,
                #[diesel(sql_type = diesel::sql_types::Bool)]
                tracked: bool,
                #[diesel(sql_type = diesel::sql_types::Bool)]
                identified: bool,
                #[diesel(sql_type = diesel::sql_types::Timestamptz)]
                created_at: DateTime<Utc>,
                #[diesel(sql_type = diesel::sql_types::Timestamptz)]
                updated_at: DateTime<Utc>,
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                id: uuid::Uuid,
                #[diesel(sql_type = diesel::sql_types::Bool)]
                from_ogn_ddb: bool,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
                frequency_mhz: Option<bigdecimal::BigDecimal>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                pilot_name: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                home_base_airport_ident: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<crate::schema::sql_types::AircraftCategory>)]
                aircraft_category: Option<crate::aircraft_types::AircraftCategory>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
                last_fix_at: Option<DateTime<Utc>>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
                club_id: Option<uuid::Uuid>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                icao_model_code: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<crate::schema::sql_types::AdsbEmitterCategory>)]
                adsb_emitter_category: Option<crate::ogn_aprs_aircraft::AdsbEmitterCategory>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                tracker_device_type: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Bpchar>)]
                country_code: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Double>)]
                latitude: Option<f64>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Double>)]
                longitude: Option<f64>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Jsonb>)]
                current_fix: Option<serde_json::Value>,
            }

            let aircraft_rows: Vec<AircraftRow> = diesel::sql_query(aircraft_sql)
                .bind::<diesel::sql_types::Double, _>(nw_lng)  // min_lon
                .bind::<diesel::sql_types::Double, _>(se_lat)  // min_lat
                .bind::<diesel::sql_types::Double, _>(se_lng)  // max_lon
                .bind::<diesel::sql_types::Double, _>(nw_lat)  // max_lat
                .bind::<diesel::sql_types::Timestamptz, _>(cutoff_time)  // cutoff_time
                .load(&mut conn)?;

            info!("First query returned {} aircraft rows", aircraft_rows.len());

            if aircraft_rows.is_empty() {
                return Ok(Vec::new());
            }

            // Convert rows to AircraftModel
            let aircraft_models: Vec<crate::aircraft::AircraftModel> = aircraft_rows
                .into_iter()
                .map(|row| crate::aircraft::AircraftModel {
                    icao_address: row.icao_address,
                    flarm_address: row.flarm_address,
                    ogn_address: row.ogn_address,
                    other_address: row.other_address,
                    aircraft_model: row.aircraft_model,
                    registration: row.registration,
                    competition_number: row.competition_number,
                    tracked: row.tracked,
                    identified: row.identified,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    id: row.id,
                    from_ogn_ddb: row.from_ogn_ddb,
            from_adsbx_ddb: false,
                    frequency_mhz: row.frequency_mhz,
                    pilot_name: row.pilot_name,
                    home_base_airport_ident: row.home_base_airport_ident,
                    aircraft_category: row.aircraft_category,
                    last_fix_at: row.last_fix_at,
                    club_id: row.club_id,
                    icao_model_code: row.icao_model_code,
                    adsb_emitter_category: row.adsb_emitter_category,
                    tracker_device_type: row.tracker_device_type,
                    country_code: row.country_code,
                    latitude: row.latitude,
                    longitude: row.longitude,
                    owner_operator: None,           // Not selected in this query
                    engine_count: None,              // Not selected in this query
                    engine_type: None,         // Not selected in this query
                    faa_pia: None,                  // Not selected in this query
                    faa_ladd: None,                 // Not selected in this query
                    year: None,                     // Not selected in this query
                    is_military: None,              // Not selected in this query
                    current_fix: row.current_fix,
                    images: None,                   // Not selected in this query
                    pending_registration: None,     // Not selected in this query
                })
                .collect();

            info!("Returning {} aircraft with current_fix from aircraft table (no fixes query needed)", aircraft_models.len());

            // Return aircraft with empty fix arrays - current_fix field contains the latest position
            let results: Vec<(crate::aircraft::AircraftModel, Vec<Fix>)> = aircraft_models
                .into_iter()
                .map(|aircraft| (aircraft, Vec::new()))
                .collect();

            Ok::<Vec<(crate::aircraft::AircraftModel, Vec<Fix>)>, anyhow::Error>(results)
        })
        .await??;

        Ok(result)
    }

    /// Update flight_id for fixes by aircraft_address within a time range
    /// This is used by flight detection processor to link fixes to flights after they're created
    pub async fn update_flight_id_by_aircraft_and_time(
        &self,
        aircraft_id: Uuid,
        flight_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<usize, anyhow::Error> {
        let pool = self.pool.clone();
        let aircraft_id_param = aircraft_id;
        let flight_id_param = flight_id;

        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;

            let updated_count = if let Some(end_time) = end_time {
                diesel::update(fixes)
                    .filter(aircraft_id.eq(aircraft_id_param))
                    .filter(received_at.ge(start_time))
                    .filter(received_at.le(end_time))
                    .filter(flight_id.is_null())
                    .set(flight_id.eq(flight_id_param))
                    .execute(&mut conn)?
            } else {
                diesel::update(fixes)
                    .filter(aircraft_id.eq(aircraft_id_param))
                    .filter(received_at.ge(start_time))
                    .filter(flight_id.is_null())
                    .set(flight_id.eq(flight_id_param))
                    .execute(&mut conn)?
            };

            Ok::<usize, anyhow::Error>(updated_count)
        })
        .await??;

        debug!(
            "Updated {} fixes with flight_id {} for aircraft {}",
            result, flight_id_param, aircraft_id_param
        );

        Ok(result)
    }

    /// Get fixes for a specific flight ID
    ///
    /// # Parameters
    /// - `flight_id`: The flight to query fixes for
    /// - `limit`: Maximum number of fixes to return (default: 1000)
    /// - `start_time`: Start time filter (received_at >= start_time) - REQUIRED for partition pruning
    /// - `end_time`: Optional end time filter (received_at <= end_time)
    ///
    /// NOTE: start_time is required to ensure proper partition pruning. For active flight tracking,
    /// use (now - 18 hours) to match the coalescing hard timeout. For API queries, use the flight's
    /// start_at timestamp.
    pub async fn get_fixes_for_flight(
        &self,
        flight_id: Uuid,
        limit: Option<i64>,
        start_time: DateTime<Utc>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<Vec<Fix>> {
        let limit = limit.unwrap_or(1000);
        let flight_id_param = flight_id;
        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;

            let mut query = fixes
                .filter(flight_id.eq(flight_id_param))
                .filter(received_at.ge(start_time))
                .into_boxed();

            // Apply end_time filter if provided
            if let Some(end) = end_time {
                query = query.filter(received_at.le(end));
            }

            let results = query
                .order(received_at.asc())
                .limit(limit)
                .select(Fix::as_select())
                .load::<Fix>(&mut conn)?;

            Ok::<Vec<Fix>, anyhow::Error>(results)
        })
        .await??;

        Ok(result)
    }

    /// Clear flight_id from all fixes associated with a flight
    /// Used when deleting spurious flights
    /// Uses a time range based on flight start/end times for partition pruning
    /// (rather than "now - 24h" which fails if processing old queued messages)
    pub async fn clear_flight_id(
        &self,
        flight_id_param: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<usize> {
        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;

            let updated_count = diesel::update(fixes)
                .filter(flight_id.eq(flight_id_param))
                .filter(received_at.ge(start_time))
                .filter(received_at.le(end_time))
                .set(flight_id.eq(None::<Uuid>))
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(updated_count)
        })
        .await??;

        debug!(
            "Cleared flight_id from {} fixes for flight {} (time range: {} to {})",
            result, flight_id_param, start_time, end_time
        );

        Ok(result)
    }

    /// Get fix counts grouped by APRS type for a specific receiver ID (last 24 hours only)
    /// Note: aprs_type is now stored in source_metadata->>'aprs_type'
    pub async fn get_fix_counts_by_aprs_type_for_receiver(
        &self,
        receiver_uuid: Uuid,
    ) -> Result<Vec<crate::actions::receivers::AprsTypeCount>> {
        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Only get fixes from the last 24 hours
            let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(24);

            // Group by aprs_type (extracted from source_metadata) and count
            // Use raw SQL to extract aprs_type from JSONB
            let counts = diesel::sql_query(
                "SELECT source_metadata->>'aprs_type' as aprs_type, COUNT(*) as count
                 FROM fixes
                 WHERE receiver_id = $1 AND received_at > $2
                 GROUP BY source_metadata->>'aprs_type'
                 ORDER BY count DESC",
            )
            .bind::<diesel::sql_types::Uuid, _>(receiver_uuid)
            .bind::<diesel::sql_types::Timestamptz, _>(cutoff_time)
            .load::<crate::actions::receivers::AprsTypeCount>(&mut conn)?;

            Ok::<Vec<crate::actions::receivers::AprsTypeCount>, anyhow::Error>(counts)
        })
        .await??;

        Ok(result)
    }

    /// Get fix counts grouped by aircraft for a specific receiver ID (last 24 hours only)
    pub async fn get_fix_counts_by_aircraft_for_receiver(
        &self,
        receiver_uuid: Uuid,
    ) -> Result<Vec<crate::actions::receivers::AircraftFixCount>> {
        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            use diesel::dsl::count_star;
            let mut conn = pool.get()?;

            // Only get fixes from the last 24 hours
            let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(24);

            // Group by aircraft_id and count
            let counts = fixes
                .filter(receiver_id.eq(receiver_uuid))
                .filter(received_at.gt(cutoff_time))
                .group_by(aircraft_id)
                .select((aircraft_id, count_star()))
                .order_by(count_star().desc())
                .load::<(uuid::Uuid, i64)>(&mut conn)?;

            // Convert to AircraftFixCount structs
            let result: Vec<crate::actions::receivers::AircraftFixCount> = counts
                .into_iter()
                .map(
                    |(dev_id, count)| crate::actions::receivers::AircraftFixCount {
                        aircraft_id: dev_id,
                        count,
                    },
                )
                .collect();

            Ok::<Vec<crate::actions::receivers::AircraftFixCount>, anyhow::Error>(result)
        })
        .await??;

        Ok(result)
    }

    /// Get the last fix for a specific flight (most recent by timestamp)
    /// Used for coalescing validation to compare positions between timeout and reappearance
    pub async fn get_last_fix_for_flight(&self, flight_id_val: Uuid) -> Result<Option<Fix>> {
        use crate::schema::fixes::dsl::*;
        use chrono::{Duration, Utc};

        let pool = self.pool.clone();

        // Only look at fixes from the last 48 hours for partition pruning
        let cutoff_time = Utc::now() - Duration::hours(48);

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let fix_row: Option<Fix> = fixes
                .filter(flight_id.eq(flight_id_val))
                .filter(received_at.ge(cutoff_time))
                .order(received_at.desc())
                .select(Fix::as_select())
                .first(&mut conn)
                .optional()?;

            Ok::<Option<Fix>, anyhow::Error>(fix_row)
        })
        .await??;

        Ok(result)
    }

    /// Get position fixes within an H3 hexagon cell
    /// Returns fixes that fall within the spatial boundary of the H3 cell,
    /// filtered by timestamp range and optional receiver/altitude filters
    ///
    /// Uses `first_seen` and `last_seen` timestamps (from hex aggregates) for
    /// efficient PostgreSQL partition pruning.
    #[allow(clippy::too_many_arguments)]
    pub async fn get_fixes_in_h3_cell(
        &self,
        h3_index: i64,
        receiver_id_filter: Option<Uuid>,
        min_altitude: Option<i32>,
        max_altitude: Option<i32>,
        limit: i64,
        offset: i64,
        first_seen: chrono::DateTime<chrono::Utc>,
        last_seen: chrono::DateTime<chrono::Utc>,
    ) -> Result<(Vec<Fix>, i64)> {
        use diesel::sql_types;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Convert H3 index to h3o::CellIndex
            let cell = h3o::CellIndex::try_from(h3_index as u64)
                .map_err(|e| anyhow::anyhow!("Invalid H3 index: {}", e))?;

            // Get boundary coordinates
            let boundary = cell.boundary();

            // Build WKT polygon from boundary points
            // h3o returns LatLng, but PostGIS expects lng lat order
            let mut coords: Vec<String> = boundary
                .iter()
                .map(|latlng| format!("{} {}", latlng.lng(), latlng.lat()))
                .collect();

            // Close the polygon by adding first point again
            let first_coord = boundary.iter().next().unwrap();
            coords.push(format!("{} {}", first_coord.lng(), first_coord.lat()));

            let polygon_wkt = format!("POLYGON(({}))", coords.join(", "));

            // Use exact timestamps from hex aggregates for efficient partition pruning
            let time_filter = format!(
                "received_at >= '{}' AND received_at <= '{}'",
                first_seen.format("%Y-%m-%d %H:%M:%S%:z"),
                last_seen.format("%Y-%m-%d %H:%M:%S%:z")
            );

            // Build base query for counting
            // Only include OGN/APRS fixes (those with receiver_id) - exclude ADS-B/Beast/SBS
            let mut count_sql = format!(
                r#"
                SELECT COUNT(*)
                FROM fixes
                WHERE ST_Within(location_geom, ST_GeomFromText('{}', 4326))
                  AND {}
                  AND receiver_id IS NOT NULL
                "#,
                polygon_wkt, time_filter
            );

            // Build base query for selecting fixes
            // Note: aprs_type and via are now in source_metadata, not separate columns
            // Only include OGN/APRS fixes (those with receiver_id) - exclude ADS-B/Beast/SBS
            let mut select_sql = format!(
                r#"
                SELECT id, source, timestamp, latitude, longitude,
                       altitude_msl_feet, altitude_agl_feet, flight_number, squawk,
                       ground_speed_knots, track_degrees, climb_fpm, turn_rate_rot,
                       source_metadata, flight_id, aircraft_id, received_at, is_active,
                       receiver_id, raw_message_id, altitude_agl_valid, time_gap_seconds
                FROM fixes
                WHERE ST_Within(location_geom, ST_GeomFromText('{}', 4326))
                  AND {}
                  AND receiver_id IS NOT NULL
                "#,
                polygon_wkt, time_filter
            );

            // Add optional filters
            if let Some(rid) = receiver_id_filter {
                let filter = format!(" AND receiver_id = '{}'", rid);
                count_sql.push_str(&filter);
                select_sql.push_str(&filter);
            }

            if let Some(min_alt) = min_altitude {
                let filter = format!(" AND altitude_msl_feet >= {}", min_alt);
                count_sql.push_str(&filter);
                select_sql.push_str(&filter);
            }

            if let Some(max_alt) = max_altitude {
                let filter = format!(" AND altitude_msl_feet <= {}", max_alt);
                count_sql.push_str(&filter);
                select_sql.push_str(&filter);
            }

            // Get total count
            #[derive(QueryableByName)]
            struct CountResult {
                #[diesel(sql_type = sql_types::BigInt)]
                count: i64,
            }

            let total: i64 = diesel::sql_query(count_sql)
                .get_result::<CountResult>(&mut conn)?
                .count;

            // Add ordering and pagination to select query
            select_sql.push_str(&format!(
                " ORDER BY timestamp DESC LIMIT {} OFFSET {}",
                limit, offset
            ));

            // Execute select query
            let fixes_result: Vec<Fix> = diesel::sql_query(select_sql).load(&mut conn)?;

            Ok::<(Vec<Fix>, i64), anyhow::Error>((fixes_result, total))
        })
        .await?
    }
}
