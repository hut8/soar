use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use tracing::{debug, info, instrument};
use uuid::Uuid;

use crate::fixes::Fix;
use crate::ogn_aprs_aircraft::{
    AddressType as ForeignAddressType, AdsbEmitterCategory, AircraftType as ForeignAircraftType,
};
use crate::web::PgPool;

// Import the main AddressType from devices module
use crate::devices::AddressType;

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

// Queryable struct for Diesel DSL queries (excluding geography column)
#[derive(Queryable, Debug)]
struct FixDslRow {
    id: Uuid,
    source: String,
    destination: String,
    via: Vec<Option<String>>, // NOT NULL array that can contain NULL elements
    raw_packet: String,
    timestamp: DateTime<Utc>,
    latitude: f64,
    longitude: f64,
    altitude_feet: Option<i32>,
    altitude_agl: Option<i32>,
    device_address: i32,
    address_type: AddressType,
    aircraft_type_ogn: Option<AircraftTypeOgn>,
    flight_number: Option<String>,
    emitter_category: Option<AdsbEmitterCategory>,
    registration: Option<String>,
    model: Option<String>,
    squawk: Option<String>,
    ground_speed_knots: Option<f32>,
    track_degrees: Option<f32>,
    climb_fpm: Option<i32>,
    turn_rate_rot: Option<f32>,
    snr_db: Option<f32>,
    bit_errors_corrected: Option<i32>,
    freq_offset_khz: Option<f32>,
    club_id: Option<Uuid>,
    flight_id: Option<Uuid>,
    unparsed_data: Option<String>,
    device_id: Uuid,
    received_at: DateTime<Utc>,
    lag: Option<i32>,
    is_active: bool,
}

impl From<FixDslRow> for Fix {
    fn from(row: FixDslRow) -> Self {
        Self {
            id: row.id,
            source: row.source,
            destination: row.destination,
            via: row.via, // Now directly a Vec<Option<String>>
            raw_packet: row.raw_packet,
            timestamp: row.timestamp,
            received_at: row.received_at,
            lag: row.lag,
            latitude: row.latitude,
            longitude: row.longitude,
            altitude_feet: row.altitude_feet,
            altitude_agl: row.altitude_agl,
            device_address: row.device_address,
            address_type: row.address_type,
            aircraft_type_ogn: row.aircraft_type_ogn.map(|t| t.into()),
            flight_id: row.flight_id,
            flight_number: row.flight_number,
            emitter_category: row.emitter_category,
            registration: row.registration,
            model: row.model,
            squawk: row.squawk,
            ground_speed_knots: row.ground_speed_knots,
            track_degrees: row.track_degrees,
            climb_fpm: row.climb_fpm,
            turn_rate_rot: row.turn_rate_rot,
            snr_db: row.snr_db,
            bit_errors_corrected: row.bit_errors_corrected,
            freq_offset_khz: row.freq_offset_khz,
            club_id: row.club_id,
            unparsed_data: row.unparsed_data,
            device_id: row.device_id, // Now directly a Uuid
            is_active: row.is_active,
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

    /// Look up device UUID by device_address (hex string) and address_type
    fn lookup_device_uuid_by_address(
        conn: &mut diesel::PgConnection,
        device_address: &str,
        address_type_: AddressType,
    ) -> Result<Uuid> {
        // Convert hex string to integer for database lookup
        let device_address_int = u32::from_str_radix(device_address, 16)?;
        let opt_uuid = Self::lookup_device_uuid(conn, device_address_int, address_type_)?;
        opt_uuid.ok_or_else(|| anyhow::anyhow!("Device not found"))
    }

    /// Look up device UUID by raw device_id and address_type (legacy method)
    fn lookup_device_uuid(
        conn: &mut diesel::PgConnection,
        raw_device_id: u32,
        address_type_: AddressType,
    ) -> Result<Option<Uuid>> {
        use crate::schema::devices::dsl::*;

        let device_uuid = devices
            .filter(address.eq(raw_device_id as i32))
            .filter(address_type.eq(address_type_))
            .select(id)
            .first::<Uuid>(conn)
            .optional()?;

        Ok(device_uuid)
    }

    /// Insert a new fix into the database
    pub async fn insert(&self, fix: &Fix) -> Result<()> {
        use crate::schema::fixes::dsl::*;

        let mut new_fix = fix.clone();
        let pool = self.pool.clone();

        // Look up device UUID if we have device address and address type
        let dev_address = fix.device_address_hex();
        let address_type_enum = fix.address_type;
        let dev_address_owned = dev_address.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Look up the device UUID using device address and address type
            new_fix.device_id = Self::lookup_device_uuid_by_address(
                &mut conn,
                    &dev_address_owned,
                    address_type_enum,
                )?;

            diesel::insert_into(fixes)
                .values(&new_fix)
                .execute(&mut conn)?;

            debug!(
                "Inserted fix | Device: {:?} ({:?}-{:?}) | {:.6},{:.6} @ {}ft | https://maps.google.com/maps?q={:.6},{:.6}",
                new_fix.device_id,
                new_fix.address_type,
                new_fix.device_address,
                new_fix.latitude,
                new_fix.longitude,
                new_fix.altitude_feet.map_or("Unknown".to_string(), |a| a.to_string()),
                new_fix.latitude,
                new_fix.longitude
            );

                Ok::<(), anyhow::Error>(())
            })
            .await?
    }

    /// Update the altitude_agl field for a specific fix
    pub async fn update_altitude_agl(&self, fix_id: Uuid, altitude_agl_value: i32) -> Result<()> {
        use crate::schema::fixes::dsl::*;
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(fixes.filter(id.eq(fix_id)))
                .set(altitude_agl.eq(Some(altitude_agl_value)))
                .execute(&mut conn)?;
            Ok::<(), anyhow::Error>(())
        })
        .await?
    }

    /// Get fixes for a specific aircraft ID within a time range (original method)
    pub async fn get_fixes_for_aircraft_with_time_range(
        &self,
        device_id: &uuid::Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<Fix>> {
        let limit = limit.unwrap_or(1000);
        let device_id_param = *device_id;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;

            let results = fixes
                .filter(device_id.eq(device_id_param))
                .filter(timestamp.between(start_time, end_time))
                .order(timestamp.desc())
                .limit(limit)
                .select(Fix::as_select())
                .load::<Fix>(&mut conn)?;

            Ok::<Vec<Fix>, anyhow::Error>(results)
        })
        .await??;

        Ok(result)
    }

    /// Get recent fixes for an aircraft (without time range)
    pub async fn get_fixes_for_device(
        &self,
        device_uuid: uuid::Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<Fix>> {
        use crate::schema::fixes::dsl::*;

        let limit = limit.unwrap_or(100);
        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let results = fixes
                .filter(device_id.eq(device_uuid))
                .order(timestamp.desc())
                .limit(limit)
                .select(Fix::as_select())
                .load::<Fix>(&mut conn)?;

            Ok::<Vec<Fix>, anyhow::Error>(results)
        })
        .await??;

        Ok(result)
    }

    /// Get recent fixes (most recent first)
    pub async fn get_recent_fixes(&self, limit: i64) -> Result<Vec<Fix>> {
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;

            let results = fixes
                .order(timestamp.desc())
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
            let mut query = fixes.filter(device_id.eq(device_uuid)).into_boxed();
            if let Some(after_timestamp) = after {
                query = query.filter(timestamp.gt(after_timestamp));
            }
            let results = query
                .order(timestamp.asc())
                .limit(limit)
                .select(Fix::as_select())
                .load::<Fix>(&mut conn)?;
            Ok::<Vec<Fix>, anyhow::Error>(results)
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
    ) -> Result<(Vec<Fix>, i64)> {
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;

            // Build base query for count
            let mut count_query = fixes.filter(device_id.eq(device_uuid)).into_boxed();
            if let Some(after_timestamp) = after {
                count_query = count_query.filter(timestamp.gt(after_timestamp));
            }
            let total_count = count_query.count().get_result::<i64>(&mut conn)?;

            // Build query for paginated results
            let mut query = fixes.filter(device_id.eq(device_uuid)).into_boxed();
            if let Some(after_timestamp) = after {
                query = query.filter(timestamp.gt(after_timestamp));
            }
            let offset = (page - 1) * per_page;
            let results = query
                .order(timestamp.desc())
                .limit(per_page)
                .offset(offset)
                .select(Fix::as_select())
                .load::<Fix>(&mut conn)?;

            Ok::<(Vec<Fix>, i64), anyhow::Error>((results, total_count))
        })
        .await??;

        Ok(result)
    }

    /// Get devices with their recent fixes in a bounding box for efficient area subscriptions
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
    ) -> Result<Vec<(crate::devices::DeviceModel, Vec<Fix>)>> {
        info!("Starting bounding box query");
        let pool = self.pool.clone();
        let fixes_per_device = fixes_per_device.unwrap_or(5);

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            info!("Got database connection, executing first query for devices");

            // First query: Get devices with fixes in the bounding box
            let devices_sql = r#"
                WITH bbox AS (
                    SELECT ST_MakeEnvelope($1, $2, $3, $4, 4326)::geography AS g
                )
                SELECT DISTINCT d.*
                FROM fixes f
                JOIN devices d ON d.id = f.device_id
                CROSS JOIN bbox
                WHERE f.received_at >= $5
                  AND ST_Intersects(f.location, bbox.g)
            "#;

            #[derive(QueryableByName)]
            struct DeviceRow {
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                id: uuid::Uuid,
                #[diesel(sql_type = diesel::sql_types::Text)]
                registration: String,
                #[diesel(sql_type = diesel::sql_types::Int4)]
                address: i32,
                #[diesel(sql_type = crate::schema::sql_types::AddressType)]
                address_type: crate::devices::AddressType,
                #[diesel(sql_type = diesel::sql_types::Text)]
                aircraft_model: String,
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
                #[diesel(sql_type = diesel::sql_types::Bool)]
                from_ddb: bool,
            }

            let device_rows: Vec<DeviceRow> = diesel::sql_query(devices_sql)
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

            // Convert rows to DeviceModel
            let device_models: Vec<crate::devices::DeviceModel> = device_rows
                .into_iter()
                .map(|row| crate::devices::DeviceModel {
                    id: row.id,
                    registration: row.registration,
                    address: row.address,
                    address_type: row.address_type,
                    aircraft_model: row.aircraft_model,
                    competition_number: row.competition_number,
                    tracked: row.tracked,
                    identified: row.identified,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    from_ddb: row.from_ddb,
                })
                .collect();

            info!("Executing second query for fixes with {} device IDs", device_ids.len());

            // Second query: Get recent fixes for the devices using the device_id index
            // This is much faster than repeating the spatial query
            let fixes_sql = r#"
                WITH ranked AS (
                    SELECT f.*,
                           ROW_NUMBER() OVER (PARTITION BY f.device_id ORDER BY f.received_at DESC) AS rn
                    FROM fixes f
                    WHERE f.device_id = ANY($1)
                )
                SELECT *
                FROM ranked
                WHERE rn <= $2
                ORDER BY device_id, received_at DESC
            "#;

            // QueryableByName version of FixDslRow for raw SQL query
            #[derive(QueryableByName)]
            struct FixRow {
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                id: uuid::Uuid,
                #[diesel(sql_type = diesel::sql_types::Text)]
                source: String,
                #[diesel(sql_type = diesel::sql_types::Text)]
                destination: String,
                #[diesel(sql_type = diesel::sql_types::Array<diesel::sql_types::Nullable<diesel::sql_types::Text>>)]
                via: Vec<Option<String>>,
                #[diesel(sql_type = diesel::sql_types::Text)]
                raw_packet: String,
                #[diesel(sql_type = diesel::sql_types::Timestamptz)]
                timestamp: DateTime<Utc>,
                #[diesel(sql_type = diesel::sql_types::Double)]
                latitude: f64,
                #[diesel(sql_type = diesel::sql_types::Double)]
                longitude: f64,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Int4>)]
                altitude_feet: Option<i32>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Int4>)]
                altitude_agl: Option<i32>,
                #[diesel(sql_type = diesel::sql_types::Int4)]
                device_address: i32,
                #[diesel(sql_type = crate::schema::sql_types::AddressType)]
                address_type: AddressType,
                #[diesel(sql_type = diesel::sql_types::Nullable<crate::schema::sql_types::AircraftTypeOgn>)]
                aircraft_type_ogn: Option<AircraftTypeOgn>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                flight_number: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<crate::schema::sql_types::AdsbEmitterCategory>)]
                emitter_category: Option<AdsbEmitterCategory>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                registration: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                model: Option<String>,
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
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float4>)]
                snr_db: Option<f32>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Int4>)]
                bit_errors_corrected: Option<i32>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float4>)]
                freq_offset_khz: Option<f32>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
                club_id: Option<uuid::Uuid>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
                flight_id: Option<uuid::Uuid>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                unparsed_data: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                device_id: uuid::Uuid,
                #[diesel(sql_type = diesel::sql_types::Timestamptz)]
                received_at: DateTime<Utc>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Int4>)]
                lag: Option<i32>,
                #[diesel(sql_type = diesel::sql_types::Bool)]
                is_active: bool,
            }

            let fix_rows: Vec<FixRow> = diesel::sql_query(fixes_sql)
                .bind::<diesel::sql_types::Array<diesel::sql_types::Uuid>, _>(&device_ids)
                .bind::<diesel::sql_types::BigInt, _>(fixes_per_device)
                .load(&mut conn)?;

            info!("Second query returned {} fix rows", fix_rows.len());

            // Group fixes by device_id
            let mut fixes_by_device: std::collections::HashMap<uuid::Uuid, Vec<Fix>> =
                std::collections::HashMap::new();
            for fix_row in fix_rows {
                let device_id = fix_row.device_id;
                // Convert FixRow to Fix
                let fix = Fix {
                    id: fix_row.id,
                    source: fix_row.source,
                    destination: fix_row.destination,
                    via: fix_row.via,
                    raw_packet: fix_row.raw_packet,
                    timestamp: fix_row.timestamp,
                    received_at: fix_row.received_at,
                    lag: fix_row.lag,
                    latitude: fix_row.latitude,
                    longitude: fix_row.longitude,
                    altitude_feet: fix_row.altitude_feet,
                    altitude_agl: fix_row.altitude_agl,
                    device_address: fix_row.device_address,
                    address_type: fix_row.address_type,
                    aircraft_type_ogn: fix_row.aircraft_type_ogn.map(|t| t.into()),
                    flight_id: fix_row.flight_id,
                    flight_number: fix_row.flight_number,
                    emitter_category: fix_row.emitter_category,
                    registration: fix_row.registration,
                    model: fix_row.model,
                    squawk: fix_row.squawk,
                    ground_speed_knots: fix_row.ground_speed_knots,
                    track_degrees: fix_row.track_degrees,
                    climb_fpm: fix_row.climb_fpm,
                    turn_rate_rot: fix_row.turn_rate_rot,
                    snr_db: fix_row.snr_db,
                    bit_errors_corrected: fix_row.bit_errors_corrected,
                    freq_offset_khz: fix_row.freq_offset_khz,
                    club_id: fix_row.club_id,
                    unparsed_data: fix_row.unparsed_data,
                    device_id: fix_row.device_id,
                    is_active: fix_row.is_active,
                };
                fixes_by_device
                    .entry(device_id)
                    .or_default()
                    .push(fix);
            }

            // Combine devices with their fixes
            let results: Vec<(crate::devices::DeviceModel, Vec<Fix>)> = device_models
                .into_iter()
                .map(|device| {
                    let fixes = fixes_by_device.remove(&device.id).unwrap_or_default();
                    (device, fixes)
                })
                .collect();

            Ok::<Vec<(crate::devices::DeviceModel, Vec<Fix>)>, anyhow::Error>(results)
        })
        .await??;

        Ok(result)
    }

    /// Update flight_id for fixes by device_address within a time range
    /// This is used by flight detection processor to link fixes to flights after they're created
    pub async fn update_flight_id_by_device_and_time(
        &self,
        device_id: Uuid,
        flight_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<usize, anyhow::Error> {
        let pool = self.pool.clone();
        let device_id_param = device_id;
        let flight_id_param = flight_id;

        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;

            let updated_count = if let Some(end_time) = end_time {
                diesel::update(fixes)
                    .filter(device_id.eq(device_id_param))
                    .filter(timestamp.ge(start_time))
                    .filter(timestamp.le(end_time))
                    .filter(flight_id.is_null())
                    .set(flight_id.eq(flight_id_param))
                    .execute(&mut conn)?
            } else {
                diesel::update(fixes)
                    .filter(device_address.eq(&device_address))
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
}
