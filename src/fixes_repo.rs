use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use tracing::{debug, info, instrument, trace};
use uuid::Uuid;

use crate::fixes::Fix;
use crate::ogn_aprs_aircraft::{
    AddressType as ForeignAddressType, AircraftType as ForeignAircraftType,
};
use crate::web::PgPool;

// Import the main AddressType from aircraft module
use crate::aircraft::AddressType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, DbEnum)]
#[db_enum(existing_type_path = "crate::schema::sql_types::AircraftTypeOgn")]
pub enum AircraftTypeOgn {
    Reserved,
    Glider,
    TowTug,
    HelicopterGyro,
    SkydiverParachute,
    DropPlane,
    HangGlider,
    Paraglider,
    RecipEngine,
    JetTurboprop,
    Unknown,
    Balloon,
    Airship,
    Uav,
    StaticObstacle,
}

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

impl From<ForeignAircraftType> for AircraftTypeOgn {
    fn from(foreign_type: ForeignAircraftType) -> Self {
        match foreign_type {
            ForeignAircraftType::Reserved => AircraftTypeOgn::Reserved,
            ForeignAircraftType::Glider => AircraftTypeOgn::Glider,
            ForeignAircraftType::TowTug => AircraftTypeOgn::TowTug,
            ForeignAircraftType::HelicopterGyro => AircraftTypeOgn::HelicopterGyro,
            ForeignAircraftType::SkydiverParachute => AircraftTypeOgn::SkydiverParachute,
            ForeignAircraftType::DropPlane => AircraftTypeOgn::DropPlane,
            ForeignAircraftType::HangGlider => AircraftTypeOgn::HangGlider,
            ForeignAircraftType::Paraglider => AircraftTypeOgn::Paraglider,
            ForeignAircraftType::RecipEngine => AircraftTypeOgn::RecipEngine,
            ForeignAircraftType::JetTurboprop => AircraftTypeOgn::JetTurboprop,
            ForeignAircraftType::Unknown => AircraftTypeOgn::Unknown,
            ForeignAircraftType::Balloon => AircraftTypeOgn::Balloon,
            ForeignAircraftType::Airship => AircraftTypeOgn::Airship,
            ForeignAircraftType::Uav => AircraftTypeOgn::Uav,
            ForeignAircraftType::StaticObstacle => AircraftTypeOgn::StaticObstacle,
        }
    }
}

impl From<AircraftTypeOgn> for ForeignAircraftType {
    fn from(wrapper_type: AircraftTypeOgn) -> Self {
        match wrapper_type {
            AircraftTypeOgn::Reserved => ForeignAircraftType::Reserved,
            AircraftTypeOgn::Glider => ForeignAircraftType::Glider,
            AircraftTypeOgn::TowTug => ForeignAircraftType::TowTug,
            AircraftTypeOgn::HelicopterGyro => ForeignAircraftType::HelicopterGyro,
            AircraftTypeOgn::SkydiverParachute => ForeignAircraftType::SkydiverParachute,
            AircraftTypeOgn::DropPlane => ForeignAircraftType::DropPlane,
            AircraftTypeOgn::HangGlider => ForeignAircraftType::HangGlider,
            AircraftTypeOgn::Paraglider => ForeignAircraftType::Paraglider,
            AircraftTypeOgn::RecipEngine => ForeignAircraftType::RecipEngine,
            AircraftTypeOgn::JetTurboprop => ForeignAircraftType::JetTurboprop,
            AircraftTypeOgn::Unknown => ForeignAircraftType::Unknown,
            AircraftTypeOgn::Balloon => ForeignAircraftType::Balloon,
            AircraftTypeOgn::Airship => ForeignAircraftType::Airship,
            AircraftTypeOgn::Uav => ForeignAircraftType::Uav,
            AircraftTypeOgn::StaticObstacle => ForeignAircraftType::StaticObstacle,
        }
    }
}

// Queryable struct for Diesel DSL queries (excluding geography and geom columns)
#[derive(Queryable, Debug)]
struct FixDslRow {
    id: Uuid,
    source: String,
    aprs_type: String,
    via: Vec<Option<String>>, // NOT NULL array that can contain NULL elements
    timestamp: DateTime<Utc>,
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
    receiver_id: Uuid,
    raw_message_id: Uuid,
    altitude_agl_valid: bool,
    time_gap_seconds: Option<i32>,
}

impl From<FixDslRow> for Fix {
    fn from(row: FixDslRow) -> Self {
        Self {
            id: row.id,
            source: row.source,
            aprs_type: row.aprs_type,
            via: row.via, // Now directly a Vec<Option<String>>
            timestamp: row.timestamp,
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
            aircraft_id: row.aircraft_id, // Now directly a Uuid
            is_active: row.is_active,
            receiver_id: row.receiver_id,
            raw_message_id: row.raw_message_id,
            altitude_agl_valid: row.altitude_agl_valid,
            time_gap_seconds: row.time_gap_seconds,
        }
    }
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
    pub async fn insert(&self, fix: &Fix) -> Result<()> {
        use crate::schema::fixes::dsl::*;

        let new_fix = fix.clone();
        let pool = self.pool.clone();

        // Note: aircraft_id, receiver_id, and raw_message_id are already populated in the Fix
        // by the generic processor context before Fix creation

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            match diesel::insert_into(fixes)
                .values(&new_fix)
                .execute(&mut conn)
            {
                Ok(_) => {
                    metrics::counter!("aprs.fixes.inserted").increment(1);
                    trace!(
                        "Inserted fix | Aircraft: {:?} | {:.6},{:.6} @ {}ft | https://maps.google.com/maps?q={:.6},{:.6}",
                        new_fix.aircraft_id,
                        new_fix.latitude,
                        new_fix.longitude,
                        new_fix.altitude_msl_feet.map_or("Unknown".to_string(), |a| a.to_string()),
                        new_fix.latitude,
                        new_fix.longitude
                    );
                    Ok(())
                }
                Err(diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                )) => {
                    // Duplicate fix on redelivery - this is expected after crashes
                    debug!(
                        "Duplicate fix detected for device {} at {}",
                        new_fix.aircraft_id, new_fix.timestamp
                    );
                    metrics::counter!("aprs.fixes.duplicate_on_redelivery").increment(1);
                    // Not an error - just skip the duplicate
                    Ok(())
                }
                Err(e) => Err(e.into()),
            }
        })
        .await?
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

    /// Batch update AGL values for multiple fixes in a single query
    /// This is much more efficient than individual updates when processing many fixes
    /// Returns the number of fixes updated
    pub async fn batch_update_altitude_agl(
        &self,
        tasks: &[crate::elevation::AglDatabaseTask],
    ) -> Result<usize> {
        if tasks.is_empty() {
            return Ok(0);
        }

        let pool = self.pool.clone();

        // Clone the task data for the blocking task
        let tasks_data: Vec<(Uuid, Option<i32>)> = tasks
            .iter()
            .map(|task| (task.fix_id, task.altitude_agl_feet))
            .collect();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Build a true batch UPDATE using a VALUES clause for efficiency
            // This is 10-100x faster than individual UPDATEs in a loop
            // UPDATE fixes SET altitude_agl_feet = data.agl, altitude_agl_valid = true
            // FROM (VALUES (uuid1, agl1), (uuid2, agl2), ...) AS data(id, agl)
            // WHERE fixes.id = data.id

            // Build the VALUES clause
            // Safe to use format! here since UUIDs are validated by Uuid type
            // and agl values are Option<i32> from our own code
            let mut value_clauses = Vec::new();

            for (fix_id, agl_value) in &tasks_data {
                let agl_str = match agl_value {
                    Some(v) => v.to_string(),
                    None => "NULL".to_string(),
                };
                value_clauses.push(format!("('{}'::uuid, {}::int4)", fix_id, agl_str));
            }

            let values_clause = value_clauses.join(", ");
            let sql = format!(
                "UPDATE fixes SET altitude_agl_feet = data.agl, altitude_agl_valid = true \
                 FROM (VALUES {}) AS data(id, agl) \
                 WHERE fixes.id = data.id",
                values_clause
            );

            // Execute the batch UPDATE
            let updated_count = diesel::sql_query(sql).execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(updated_count)
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
    ) -> Result<Vec<crate::fixes::FixWithRawPacket>> {
        let device_id_param = *aircraft_id;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::{fixes, raw_messages};
            let mut conn = pool.get()?;

            let mut query = fixes::table
                .inner_join(
                    raw_messages::table.on(fixes::raw_message_id
                        .eq(raw_messages::id)
                        .and(fixes::received_at.eq(raw_messages::received_at))),
                )
                .filter(fixes::aircraft_id.eq(device_id_param))
                .filter(fixes::received_at.between(start_time, end_time))
                .order(fixes::received_at.desc())
                .into_boxed();

            // Only apply limit if specified
            if let Some(limit_value) = limit {
                query = query.limit(limit_value);
            }

            // Select all Fix fields plus raw_message from raw_messages as raw_packet
            let results = query
                .select((Fix::as_select(), raw_messages::raw_message))
                .load::<(Fix, Vec<u8>)>(&mut conn)?
                .into_iter()
                .map(|(fix, raw_packet_bytes)| {
                    crate::fixes::FixWithRawPacket::new(
                        fix,
                        Some(String::from_utf8_lossy(&raw_packet_bytes).to_string()),
                    )
                })
                .collect();

            Ok::<Vec<crate::fixes::FixWithRawPacket>, anyhow::Error>(results)
        })
        .await??;

        Ok(result)
    }

    /// Get recent fixes for a device with optional lookback period
    ///
    /// # Arguments
    /// * `device_uuid` - The device UUID to query
    /// * `limit` - Maximum number of fixes to return (default: 100)
    /// * `lookback_hours` - How many hours back to search (default: 24)
    pub async fn get_fixes_for_device(
        &self,
        device_uuid: uuid::Uuid,
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
                .filter(aircraft_id.eq(device_uuid))
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

    /// Get fixes for a specific device with optional after timestamp and limit
    pub async fn get_fixes_by_device(
        &self,
        device_uuid: Uuid,
        after: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<Fix>> {
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;
            let mut query = fixes.filter(aircraft_id.eq(device_uuid)).into_boxed();
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

    /// Get the most recent fix for a device after a given timestamp
    pub async fn get_latest_fix_for_device(
        &self,
        device_uuid: Uuid,
        after: DateTime<Utc>,
    ) -> Result<Option<Fix>> {
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;
            let fix_result = fixes
                .filter(aircraft_id.eq(device_uuid))
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

    pub async fn get_fixes_by_device_paginated(
        &self,
        device_uuid: Uuid,
        after: Option<DateTime<Utc>>,
        page: i64,
        per_page: i64,
        active_only: Option<bool>,
    ) -> Result<(Vec<crate::fixes::FixWithRawPacket>, i64)> {
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::{fixes, raw_messages};
            let mut conn = pool.get()?;

            // Build base query for count
            let mut count_query = fixes::table
                .filter(fixes::aircraft_id.eq(device_uuid))
                .into_boxed();
            if let Some(after_timestamp) = after {
                count_query = count_query.filter(fixes::received_at.gt(after_timestamp));
            }
            if active_only == Some(true) {
                count_query = count_query.filter(fixes::is_active.eq(true));
            }
            let total_count = count_query.count().get_result::<i64>(&mut conn)?;

            // Build query for paginated results with raw packet data
            let mut query = fixes::table
                .inner_join(
                    raw_messages::table.on(fixes::raw_message_id
                        .eq(raw_messages::id)
                        .and(fixes::received_at.eq(raw_messages::received_at))),
                )
                .filter(fixes::aircraft_id.eq(device_uuid))
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
                .select((Fix::as_select(), raw_messages::raw_message))
                .load::<(Fix, Vec<u8>)>(&mut conn)?
                .into_iter()
                .map(|(fix, raw_packet_bytes)| {
                    crate::fixes::FixWithRawPacket::new(
                        fix,
                        Some(String::from_utf8_lossy(&raw_packet_bytes).to_string()),
                    )
                })
                .collect();

            Ok::<(Vec<crate::fixes::FixWithRawPacket>, i64), anyhow::Error>((results, total_count))
        })
        .await??;

        Ok(result)
    }

    /// Get fixes by receiver ID with pagination (last 24 hours only)
    pub async fn get_fixes_by_receiver_id_paginated(
        &self,
        receiver_uuid: Uuid,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<Fix>, i64)> {
        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;

            // Only get fixes from the last 24 hours
            let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(24);

            // Get total count
            let total_count = fixes
                .filter(receiver_id.eq(receiver_uuid))
                .filter(received_at.gt(cutoff_time))
                .count()
                .get_result::<i64>(&mut conn)?;

            // Get paginated results (most recent first)
            let offset = (page - 1) * per_page;
            let results = fixes
                .filter(receiver_id.eq(receiver_uuid))
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

    /// Get aircraft with their recent fixes in a bounding box for efficient area subscriptions
    /// This replaces the inefficient global fetch + filter approach
    #[instrument(skip(self), fields(fixes_per_device = fixes_per_device.unwrap_or(5)))]
    pub async fn get_devices_with_fixes_in_bounding_box(
        &self,
        nw_lat: f64,
        nw_lng: f64,
        se_lat: f64,
        se_lng: f64,
        cutoff_time: DateTime<Utc>,
        fixes_per_device: Option<i64>,
    ) -> Result<Vec<(crate::aircraft::AircraftModel, Vec<Fix>)>> {
        info!("Starting bounding box query");
        let pool = self.pool.clone();
        let fixes_per_device = fixes_per_device.unwrap_or(5);

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            info!("Got database connection, executing first query for aircraft");

            // First query: Get aircraft with fixes in the bounding box
            // Optimized with EXISTS + LIMIT 1 pattern and geometry casting for performance
            // Handles antimeridian (international date line) crossing by splitting into two boxes
            let devices_sql = r#"
                WITH params AS (
                    SELECT
                        $1::double precision AS left_lng,
                        $2::double precision AS bottom_lat,
                        $3::double precision AS right_lng,
                        $4::double precision AS top_lat,
                        $5::timestamptz AS since_ts
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
                        END AS boxes
                    FROM params
                )
                SELECT d.*
                FROM aircraft d
                WHERE EXISTS (
                    SELECT 1
                    FROM fixes f, params p, parts
                    WHERE f.aircraft_id = d.id
                      AND f.received_at >= p.since_ts
                      AND (
                          f.location_geom && parts.boxes[1]
                          OR (array_length(parts.boxes, 1) = 2 AND f.location_geom && parts.boxes[2])
                      )
                    LIMIT 1
                )
            "#;

            #[derive(QueryableByName)]
            struct AircraftRow {
                #[diesel(sql_type = diesel::sql_types::Int4)]
                address: i32,
                #[diesel(sql_type = crate::schema::sql_types::AddressType)]
                address_type: crate::aircraft::AddressType,
                #[diesel(sql_type = diesel::sql_types::Text)]
                aircraft_model: String,
                #[diesel(sql_type = diesel::sql_types::Text)]
                registration: String,
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
                from_ddb: bool,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
                frequency_mhz: Option<bigdecimal::BigDecimal>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                pilot_name: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                home_base_airport_ident: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<crate::schema::sql_types::AircraftTypeOgn>)]
                aircraft_type_ogn: Option<AircraftTypeOgn>,
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
            }

            let device_rows: Vec<AircraftRow> = diesel::sql_query(devices_sql)
                .bind::<diesel::sql_types::Double, _>(nw_lng)  // min_lon
                .bind::<diesel::sql_types::Double, _>(se_lat)  // min_lat
                .bind::<diesel::sql_types::Double, _>(se_lng)  // max_lon
                .bind::<diesel::sql_types::Double, _>(nw_lat)  // max_lat
                .bind::<diesel::sql_types::Timestamptz, _>(cutoff_time)
                .load(&mut conn)?;

            info!("First query returned {} device rows", device_rows.len());

            if device_rows.is_empty() {
                return Ok(Vec::new());
            }

            // Extract device IDs for the second query
            let device_ids: Vec<uuid::Uuid> = device_rows.iter().map(|row| row.id).collect();

            // Convert rows to AircraftModel
            let device_models: Vec<crate::aircraft::AircraftModel> = device_rows
                .into_iter()
                .map(|row| crate::aircraft::AircraftModel {
                    address: row.address,
                    address_type: row.address_type,
                    aircraft_model: row.aircraft_model,
                    registration: row.registration,
                    competition_number: row.competition_number,
                    tracked: row.tracked,
                    identified: row.identified,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    id: row.id,
                    from_ddb: row.from_ddb,
                    frequency_mhz: row.frequency_mhz,
                    pilot_name: row.pilot_name,
                    home_base_airport_ident: row.home_base_airport_ident,
                    aircraft_type_ogn: row.aircraft_type_ogn.map(|t| t.into()),
                    last_fix_at: row.last_fix_at,
                    club_id: row.club_id,
                    icao_model_code: row.icao_model_code,
                    adsb_emitter_category: row.adsb_emitter_category,
                    tracker_device_type: row.tracker_device_type,
                    country_code: row.country_code,
                })
                .collect();

            info!("Executing second query for fixes with {} aircraft IDs", device_ids.len());

            // Second query: Get recent fixes for the aircraft using the aircraft_id index
            // This is much faster than repeating the spatial query
            // Time-based pruning filter reduces rows before windowing for better performance
            let fixes_sql = r#"
                WITH ranked AS (
                    SELECT f.*,
                           ROW_NUMBER() OVER (PARTITION BY f.aircraft_id ORDER BY f.received_at DESC) AS rn
                    FROM fixes f
                    WHERE f.aircraft_id = ANY($1)
                      AND f.received_at >= $3
                )
                SELECT *
                FROM ranked
                WHERE rn <= $2
                ORDER BY aircraft_id, received_at DESC
            "#;

            // QueryableByName version of FixDslRow for raw SQL query
            // Note: Excludes location and geom geography/geometry columns
            #[derive(QueryableByName)]
            struct FixRow {
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                id: uuid::Uuid,
                #[diesel(sql_type = diesel::sql_types::Text)]
                source: String,
                #[diesel(sql_type = diesel::sql_types::Text)]
                aprs_type: String,
                #[diesel(sql_type = diesel::sql_types::Array<diesel::sql_types::Nullable<diesel::sql_types::Text>>)]
                via: Vec<Option<String>>,
                #[diesel(sql_type = diesel::sql_types::Timestamptz)]
                timestamp: DateTime<Utc>,
                #[diesel(sql_type = diesel::sql_types::Double)]
                latitude: f64,
                #[diesel(sql_type = diesel::sql_types::Double)]
                longitude: f64,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Int4>)]
                altitude_msl_feet: Option<i32>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                flight_number: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                squawk: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float4>)]
                ground_speed_knots: Option<f32>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float4>)]
                track_degrees: Option<f32>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Int4>)]
                climb_fpm: Option<i32>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float4>)]
                turn_rate_rot: Option<f32>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Jsonb>)]
                source_metadata: Option<serde_json::Value>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
                flight_id: Option<uuid::Uuid>,
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                aircraft_id: uuid::Uuid,
                #[diesel(sql_type = diesel::sql_types::Timestamptz)]
                received_at: DateTime<Utc>,
                #[diesel(sql_type = diesel::sql_types::Bool)]
                is_active: bool,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Int4>)]
                altitude_agl_feet: Option<i32>,
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                receiver_id: uuid::Uuid,
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                raw_message_id: uuid::Uuid,
                #[diesel(sql_type = diesel::sql_types::Bool)]
                altitude_agl_valid: bool,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Int4>)]
                time_gap_seconds: Option<i32>,
            }

            let fix_rows: Vec<FixRow> = diesel::sql_query(fixes_sql)
                .bind::<diesel::sql_types::Array<diesel::sql_types::Uuid>, _>(&device_ids)
                .bind::<diesel::sql_types::BigInt, _>(fixes_per_device)
                .bind::<diesel::sql_types::Timestamptz, _>(cutoff_time)
                .load(&mut conn)?;

            info!("Second query returned {} fix rows", fix_rows.len());

            // Group fixes by aircraft_id
            let mut fixes_by_device: std::collections::HashMap<uuid::Uuid, Vec<Fix>> =
                std::collections::HashMap::new();
            for fix_row in fix_rows {
                let aircraft_id = fix_row.aircraft_id;
                // Convert FixRow to Fix
                let fix = Fix {
                    id: fix_row.id,
                    source: fix_row.source,
                    aprs_type: fix_row.aprs_type,
                    via: fix_row.via,
                    timestamp: fix_row.timestamp,
                    received_at: fix_row.received_at,
                    latitude: fix_row.latitude,
                    longitude: fix_row.longitude,
                    altitude_msl_feet: fix_row.altitude_msl_feet,
                    altitude_agl_feet: fix_row.altitude_agl_feet,
                    flight_id: fix_row.flight_id,
                    flight_number: fix_row.flight_number,
                    squawk: fix_row.squawk,
                    ground_speed_knots: fix_row.ground_speed_knots,
                    track_degrees: fix_row.track_degrees,
                    climb_fpm: fix_row.climb_fpm,
                    turn_rate_rot: fix_row.turn_rate_rot,
                    source_metadata: fix_row.source_metadata,
                    aircraft_id: fix_row.aircraft_id,
                    is_active: fix_row.is_active,
                    receiver_id: fix_row.receiver_id,
                    raw_message_id: fix_row.raw_message_id,
                    altitude_agl_valid: fix_row.altitude_agl_valid,
                    time_gap_seconds: fix_row.time_gap_seconds,
                };
                fixes_by_device
                    .entry(aircraft_id)
                    .or_default()
                    .push(fix);
            }

            // Combine aircraft with their fixes
            let results: Vec<(crate::aircraft::AircraftModel, Vec<Fix>)> = device_models
                .into_iter()
                .map(|device| {
                    let fixes = fixes_by_device.remove(&device.id).unwrap_or_default();
                    (device, fixes)
                })
                .collect();

            Ok::<Vec<(crate::aircraft::AircraftModel, Vec<Fix>)>, anyhow::Error>(results)
        })
        .await??;

        Ok(result)
    }

    /// Update flight_id for fixes by device_address within a time range
    /// This is used by flight detection processor to link fixes to flights after they're created
    pub async fn update_flight_id_by_device_and_time(
        &self,
        aircraft_id: Uuid,
        flight_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<usize, anyhow::Error> {
        let pool = self.pool.clone();
        let device_id_param = aircraft_id;
        let flight_id_param = flight_id;

        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;

            let updated_count = if let Some(end_time) = end_time {
                diesel::update(fixes)
                    .filter(aircraft_id.eq(device_id_param))
                    .filter(timestamp.ge(start_time))
                    .filter(timestamp.le(end_time))
                    .filter(flight_id.is_null())
                    .set(flight_id.eq(flight_id_param))
                    .execute(&mut conn)?
            } else {
                diesel::update(fixes)
                    .filter(aircraft_id.eq(device_id_param))
                    .filter(timestamp.ge(start_time))
                    .filter(flight_id.is_null())
                    .set(flight_id.eq(flight_id_param))
                    .execute(&mut conn)?
            };

            Ok::<usize, anyhow::Error>(updated_count)
        })
        .await??;

        debug!(
            "Updated {} fixes with flight_id {} for device {}",
            result, flight_id_param, device_id_param
        );

        Ok(result)
    }

    /// Get fixes for a specific flight ID
    pub async fn get_fixes_for_flight(
        &self,
        flight_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<Fix>> {
        let limit = limit.unwrap_or(1000);
        let flight_id_param = flight_id;
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;

            let results = fixes
                .filter(flight_id.eq(flight_id_param))
                .order(timestamp.asc())
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
    pub async fn clear_flight_id(&self, flight_id_param: Uuid) -> Result<usize> {
        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;

            let updated_count = diesel::update(fixes)
                .filter(flight_id.eq(flight_id_param))
                .set(flight_id.eq(None::<Uuid>))
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(updated_count)
        })
        .await??;

        debug!(
            "Cleared flight_id from {} fixes for flight {}",
            result, flight_id_param
        );

        Ok(result)
    }

    /// Get fix counts grouped by APRS type for a specific receiver ID (last 24 hours only)
    pub async fn get_fix_counts_by_aprs_type_for_receiver(
        &self,
        receiver_uuid: Uuid,
    ) -> Result<Vec<crate::actions::receivers::AprsTypeCount>> {
        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            use diesel::dsl::count_star;
            let mut conn = pool.get()?;

            // Only get fixes from the last 24 hours
            let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(24);

            // Group by aprs_type and count
            let counts = fixes
                .filter(receiver_id.eq(receiver_uuid))
                .filter(received_at.gt(cutoff_time))
                .group_by(aprs_type)
                .select((aprs_type, count_star()))
                .order_by(count_star().desc())
                .load::<(String, i64)>(&mut conn)?;

            // Convert to AprsTypeCount structs
            let result: Vec<crate::actions::receivers::AprsTypeCount> = counts
                .into_iter()
                .map(
                    |(type_name, count)| crate::actions::receivers::AprsTypeCount {
                        aprs_type: type_name,
                        count,
                    },
                )
                .collect();

            Ok::<Vec<crate::actions::receivers::AprsTypeCount>, anyhow::Error>(result)
        })
        .await??;

        Ok(result)
    }

    /// Get fix counts grouped by device for a specific receiver ID (last 24 hours only)
    pub async fn get_fix_counts_by_device_for_receiver(
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
                .filter(timestamp.ge(cutoff_time))
                .order(timestamp.desc())
                .select(Fix::as_select())
                .first(&mut conn)
                .optional()?;

            Ok::<Option<Fix>, anyhow::Error>(fix_row)
        })
        .await??;

        Ok(result)
    }
}
