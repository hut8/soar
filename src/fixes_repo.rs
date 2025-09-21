use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::sql_query;
use diesel_derive_enum::DbEnum;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::fixes::Fix;
use crate::ogn_aprs_aircraft::{
    AddressType as ForeignAddressType, AdsbEmitterCategory, AircraftType as ForeignAircraftType,
};
use crate::web::PgPool;

// Import the main AddressType from devices module
use crate::devices::AddressType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, DbEnum)]
#[db_enum(existing_type_path = "crate::schema::sql_types::AircraftType")]
pub enum AircraftType {
    Reserved0,
    GliderMotorGlider,
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
    ReservedE,
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

impl From<ForeignAircraftType> for AircraftType {
    fn from(foreign_type: ForeignAircraftType) -> Self {
        match foreign_type {
            ForeignAircraftType::Reserved0 => AircraftType::Reserved0,
            ForeignAircraftType::GliderMotorGlider => AircraftType::GliderMotorGlider,
            ForeignAircraftType::TowTug => AircraftType::TowTug,
            ForeignAircraftType::HelicopterGyro => AircraftType::HelicopterGyro,
            ForeignAircraftType::SkydiverParachute => AircraftType::SkydiverParachute,
            ForeignAircraftType::DropPlane => AircraftType::DropPlane,
            ForeignAircraftType::HangGlider => AircraftType::HangGlider,
            ForeignAircraftType::Paraglider => AircraftType::Paraglider,
            ForeignAircraftType::RecipEngine => AircraftType::RecipEngine,
            ForeignAircraftType::JetTurboprop => AircraftType::JetTurboprop,
            ForeignAircraftType::Unknown => AircraftType::Unknown,
            ForeignAircraftType::Balloon => AircraftType::Balloon,
            ForeignAircraftType::Airship => AircraftType::Airship,
            ForeignAircraftType::Uav => AircraftType::Uav,
            ForeignAircraftType::ReservedE => AircraftType::ReservedE,
            ForeignAircraftType::StaticObstacle => AircraftType::StaticObstacle,
        }
    }
}

impl From<AircraftType> for ForeignAircraftType {
    fn from(wrapper_type: AircraftType) -> Self {
        match wrapper_type {
            AircraftType::Reserved0 => ForeignAircraftType::Reserved0,
            AircraftType::GliderMotorGlider => ForeignAircraftType::GliderMotorGlider,
            AircraftType::TowTug => ForeignAircraftType::TowTug,
            AircraftType::HelicopterGyro => ForeignAircraftType::HelicopterGyro,
            AircraftType::SkydiverParachute => ForeignAircraftType::SkydiverParachute,
            AircraftType::DropPlane => ForeignAircraftType::DropPlane,
            AircraftType::HangGlider => ForeignAircraftType::HangGlider,
            AircraftType::Paraglider => ForeignAircraftType::Paraglider,
            AircraftType::RecipEngine => ForeignAircraftType::RecipEngine,
            AircraftType::JetTurboprop => ForeignAircraftType::JetTurboprop,
            AircraftType::Unknown => ForeignAircraftType::Unknown,
            AircraftType::Balloon => ForeignAircraftType::Balloon,
            AircraftType::Airship => ForeignAircraftType::Airship,
            AircraftType::Uav => ForeignAircraftType::Uav,
            AircraftType::ReservedE => ForeignAircraftType::ReservedE,
            AircraftType::StaticObstacle => ForeignAircraftType::StaticObstacle,
        }
    }
}

// Diesel model for inserting new fixes
#[derive(Insertable)]
#[diesel(table_name = crate::schema::fixes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewFix {
    id: Uuid,
    source: String,
    destination: String,
    via: Vec<String>,
    raw_packet: String,
    timestamp: DateTime<Utc>,
    latitude: f64,
    longitude: f64,
    altitude_feet: Option<i32>,
    aircraft_id: Option<String>,
    device_id: Option<Uuid>,
    device_type: Option<AddressType>,
    aircraft_type: Option<AircraftType>,
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
    unparsed_data: Option<String>,
}

impl From<&Fix> for NewFix {
    fn from(fix: &Fix) -> Self {
        Self {
            id: fix.id,
            source: fix.source.clone(),
            destination: fix.destination.clone(),
            via: fix.via.clone(),
            raw_packet: fix.raw_packet.clone(),
            timestamp: fix.timestamp,
            latitude: fix.latitude,
            longitude: fix.longitude,
            altitude_feet: fix.altitude_feet,
            aircraft_id: fix.aircraft_id.clone(),
            device_id: None, // Will be resolved during insertion based on raw device_id and device_type
            device_type: fix.device_type.map(AddressType::from),
            aircraft_type: fix.aircraft_type.map(AircraftType::from),
            flight_number: fix.flight_number.clone(),
            emitter_category: fix.emitter_category,
            registration: fix.registration.clone(),
            model: fix.model.clone(),
            squawk: fix.squawk.clone(),
            ground_speed_knots: fix.ground_speed_knots,
            track_degrees: fix.track_degrees,
            climb_fpm: fix.climb_fpm,
            turn_rate_rot: fix.turn_rate_rot,
            snr_db: fix.snr_db,
            bit_errors_corrected: fix.bit_errors_corrected.map(|b| b as i32),
            freq_offset_khz: fix.freq_offset_khz,
            club_id: fix.club_id,
            unparsed_data: fix.unparsed_data.clone(),
        }
    }
}

// QueryableByName for complex queries with raw SQL
#[derive(QueryableByName, Debug)]
struct FixRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    source: String,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    destination: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Array<diesel::sql_types::Text>>)]
    via: Option<Vec<String>>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    raw_packet: String,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    timestamp: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Float8)]
    latitude: f64,
    #[diesel(sql_type = diesel::sql_types::Float8)]
    longitude: f64,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
    altitude_feet: Option<i32>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    aircraft_id: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
    device_id: Option<Uuid>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    device_type: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    aircraft_type: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    flight_number: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    emitter_category: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    registration: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    model: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    squawk: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float4>)]
    ground_speed_knots: Option<f32>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float4>)]
    track_degrees: Option<f32>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
    climb_fpm: Option<i32>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float4>)]
    turn_rate_rot: Option<f32>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float4>)]
    snr_db: Option<f32>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
    bit_errors_corrected: Option<i32>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float4>)]
    freq_offset_khz: Option<f32>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
    club_id: Option<Uuid>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    unparsed_data: Option<String>,
}

impl From<FixRow> for Fix {
    fn from(row: FixRow) -> Self {
        // Helper function to parse enum from string
        fn parse_device_type(s: Option<String>) -> Option<ForeignAddressType> {
            s.and_then(|s| match s.as_str() {
                "unknown" => Some(ForeignAddressType::Unknown),
                "icao" => Some(ForeignAddressType::Icao),
                "flarm" => Some(ForeignAddressType::Flarm),
                "ogn" => Some(ForeignAddressType::OgnTracker),
                // Support legacy snake_case values for backward compatibility
                "unknown_type" => Some(ForeignAddressType::Unknown),
                "icao_address" => Some(ForeignAddressType::Icao),
                "flarm_id" => Some(ForeignAddressType::Flarm),
                "ogn_tracker" => Some(ForeignAddressType::OgnTracker),
                // Support legacy CamelCase values for backward compatibility during migration
                "Unknown" => Some(ForeignAddressType::Unknown),
                "Icao" => Some(ForeignAddressType::Icao),
                "Flarm" => Some(ForeignAddressType::Flarm),
                "OgnTracker" => Some(ForeignAddressType::OgnTracker),
                _ => None,
            })
        }

        fn parse_aircraft_type(s: Option<String>) -> Option<ForeignAircraftType> {
            s.and_then(|s| match s.as_str() {
                "Reserved0" => Some(ForeignAircraftType::Reserved0),
                "GliderMotorGlider" => Some(ForeignAircraftType::GliderMotorGlider),
                "TowTug" => Some(ForeignAircraftType::TowTug),
                "HelicopterGyro" => Some(ForeignAircraftType::HelicopterGyro),
                "SkydiverParachute" => Some(ForeignAircraftType::SkydiverParachute),
                "DropPlane" => Some(ForeignAircraftType::DropPlane),
                "HangGlider" => Some(ForeignAircraftType::HangGlider),
                "Paraglider" => Some(ForeignAircraftType::Paraglider),
                "RecipEngine" => Some(ForeignAircraftType::RecipEngine),
                "JetTurboprop" => Some(ForeignAircraftType::JetTurboprop),
                "Unknown" => Some(ForeignAircraftType::Unknown),
                "Balloon" => Some(ForeignAircraftType::Balloon),
                "Airship" => Some(ForeignAircraftType::Airship),
                "Uav" => Some(ForeignAircraftType::Uav),
                "ReservedE" => Some(ForeignAircraftType::ReservedE),
                "StaticObstacle" => Some(ForeignAircraftType::StaticObstacle),
                _ => None,
            })
        }

        fn parse_emitter_category(s: Option<String>) -> Option<AdsbEmitterCategory> {
            s.and_then(|s| s.parse::<AdsbEmitterCategory>().ok())
        }

        Self {
            id: row.id,
            source: row.source,
            destination: row.destination,
            via: row.via.unwrap_or_default(),
            raw_packet: row.raw_packet,
            timestamp: row.timestamp,
            latitude: row.latitude,
            longitude: row.longitude,
            altitude_feet: row.altitude_feet,
            aircraft_id: row.aircraft_id,
            device_id: None, // TODO: Look up raw device_id from devices table if needed
            device_type: parse_device_type(row.device_type),
            aircraft_type: parse_aircraft_type(row.aircraft_type),
            flight_number: row.flight_number,
            emitter_category: parse_emitter_category(row.emitter_category),
            registration: row.registration,
            model: row.model,
            squawk: row.squawk,
            ground_speed_knots: row.ground_speed_knots,
            track_degrees: row.track_degrees,
            climb_fpm: row.climb_fpm,
            turn_rate_rot: row.turn_rate_rot,
            snr_db: row.snr_db,
            bit_errors_corrected: row.bit_errors_corrected.map(|b| b as u32),
            freq_offset_khz: row.freq_offset_khz,
            club_id: row.club_id,
            unparsed_data: row.unparsed_data,
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

    /// Look up device UUID by raw device_id and device_type
    fn lookup_device_uuid(
        conn: &mut diesel::PgConnection,
        raw_device_id: u32,
        device_type: AddressType,
    ) -> Result<Option<Uuid>> {
        use crate::schema::devices::dsl::*;

        let device_uuid = devices
            .filter(address.eq(raw_device_id as i32))
            .filter(address_type.eq(device_type))
            .select(id)
            .first::<Uuid>(conn)
            .optional()?;

        Ok(device_uuid)
    }

    /// Insert a new fix into the database
    pub async fn insert(&self, fix: &Fix) -> Result<()> {
        use crate::schema::fixes::dsl::*;

        let pool = self.pool.clone();
        let mut new_fix = NewFix::from(fix);
        let aircraft_identifier = fix.get_aircraft_identifier();

        // Look up device UUID if we have raw device info
        if let (Some(raw_device_id), Some(device_type_ref)) =
            (fix.device_id, fix.device_type.as_ref())
        {
            let device_type_enum = AddressType::from(*device_type_ref);

            tokio::task::spawn_blocking(move || {
                let mut conn = pool.get()?;

                // Look up the device UUID
                new_fix.device_id =
                    Self::lookup_device_uuid(&mut conn, raw_device_id, device_type_enum)?;

                diesel::insert_into(fixes)
                    .values(&new_fix)
                    .execute(&mut conn)?;

                Ok::<(), anyhow::Error>(())
            })
            .await??;
        } else {
            tokio::task::spawn_blocking(move || {
                let mut conn = pool.get()?;

                diesel::insert_into(fixes)
                    .values(&new_fix)
                    .execute(&mut conn)?;

                Ok::<(), anyhow::Error>(())
            })
            .await??;
        }

        debug!("Inserted fix for aircraft: {:?}", aircraft_identifier);
        Ok(())
    }

    /// Insert multiple fixes in a batch transaction
    pub async fn insert_batch(&self, fix_list: &[Fix]) -> Result<usize> {
        if fix_list.is_empty() {
            return Ok(0);
        }

        use crate::schema::fixes::dsl::*;

        let pool = self.pool.clone();
        let fixes_data: Vec<(Fix, NewFix)> = fix_list
            .iter()
            .map(|fix| (fix.clone(), NewFix::from(fix)))
            .collect();
        let fixes_count = fixes_data.len();

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let mut inserted_count = 0;

            // Use a transaction for batch processing
            conn.transaction(|conn| {
                for (original_fix, mut fix_data) in fixes_data {
                    // Look up device UUID if we have raw device info
                    if let (Some(raw_device_id), Some(device_type_ref)) =
                        (original_fix.device_id, original_fix.device_type.as_ref())
                    {
                        let device_type_enum = AddressType::from(*device_type_ref);
                        if let Ok(Some(device_uuid)) =
                            Self::lookup_device_uuid(conn, raw_device_id, device_type_enum)
                        {
                            fix_data.device_id = Some(device_uuid);
                        }
                    }

                    match diesel::insert_into(fixes).values(&fix_data).execute(conn) {
                        Ok(_) => inserted_count += 1,
                        Err(e) => {
                            warn!("Failed to insert fix with ID {:?}: {}", fix_data.id, e);
                            // Continue with other fixes rather than failing the entire batch
                        }
                    }
                }

                Ok::<(), diesel::result::Error>(())
            })?;

            Ok::<usize, anyhow::Error>(inserted_count)
        })
        .await??;

        debug!("Inserted {} out of {} fixes in batch", result, fixes_count);
        Ok(result)
    }

    /// Get fixes for a specific aircraft ID within a time range (original method)
    pub async fn get_fixes_for_aircraft_with_time_range(
        &self,
        aircraft_id: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<Fix>> {
        let limit = limit.unwrap_or(1000);
        let aircraft_id = aircraft_id.to_string();

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let sql = r#"
                SELECT
                    id, source, destination, via, raw_packet, timestamp,
                    latitude, longitude, altitude_feet,
                    aircraft_id, device_id, device_type, aircraft_type,
                    flight_number, emitter_category, registration, model, squawk,
                    ground_speed_knots, track_degrees, climb_fpm, turn_rate_rot,
                    snr_db, bit_errors_corrected, freq_offset_khz,
                    club_id
                FROM fixes
                WHERE aircraft_id = $1
                AND timestamp BETWEEN $2 AND $3
                ORDER BY timestamp DESC
                LIMIT $4
            "#;

            let results: Vec<FixRow> = sql_query(sql)
                .bind::<diesel::sql_types::Varchar, _>(&aircraft_id)
                .bind::<diesel::sql_types::Timestamptz, _>(start_time)
                .bind::<diesel::sql_types::Timestamptz, _>(end_time)
                .bind::<diesel::sql_types::BigInt, _>(limit)
                .load::<FixRow>(&mut conn)?;

            Ok::<Vec<FixRow>, anyhow::Error>(results)
        })
        .await??;

        Ok(result.into_iter().map(Fix::from).collect())
    }

    /// Get recent fixes within a geographic area
    pub async fn get_recent_fixes_in_area(
        &self,
        center_lat: f64,
        center_lon: f64,
        radius_km: f64,
        max_age_minutes: i32,
        limit: Option<i64>,
    ) -> Result<Vec<Fix>> {
        let limit = limit.unwrap_or(100);
        let radius_m = radius_km * 1000.0;
        let cutoff_time = Utc::now() - chrono::Duration::minutes(max_age_minutes as i64);

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Note: The spatial query uses a virtual 'location' column created from lat/lon
            // We need to construct the point from latitude and longitude in the query
            let sql = r#"
                SELECT
                    id, source, destination, via, raw_packet, timestamp,
                    latitude, longitude, altitude_feet,
                    aircraft_id, device_id, device_type, aircraft_type,
                    flight_number, emitter_category, registration, model, squawk,
                    ground_speed_knots, track_degrees, climb_fpm, turn_rate_rot,
                    snr_db, bit_errors_corrected, freq_offset_khz,
                    club_id
                FROM fixes
                WHERE timestamp > $3
                AND ST_DWithin(
                    ST_SetSRID(ST_MakePoint(longitude, latitude), 4326)::geography,
                    ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography,
                    $4
                )
                ORDER BY timestamp DESC
                LIMIT $5
            "#;

            let results: Vec<FixRow> = sql_query(sql)
                .bind::<diesel::sql_types::Float8, _>(center_lat)
                .bind::<diesel::sql_types::Float8, _>(center_lon)
                .bind::<diesel::sql_types::Timestamptz, _>(cutoff_time)
                .bind::<diesel::sql_types::Float8, _>(radius_m)
                .bind::<diesel::sql_types::BigInt, _>(limit)
                .load::<FixRow>(&mut conn)?;

            Ok::<Vec<FixRow>, anyhow::Error>(results)
        })
        .await??;

        Ok(result.into_iter().map(Fix::from).collect())
    }

    /// Get recent fixes for an aircraft (without time range)
    pub async fn get_fixes_for_aircraft(
        &self,
        aircraft_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<Fix>> {
        let limit = limit.unwrap_or(100);
        let aircraft_id = aircraft_id.to_string();

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let sql = r#"
                SELECT
                    id, source, destination, via, raw_packet, timestamp,
                    latitude, longitude, altitude_feet,
                    aircraft_id, device_id, device_type, aircraft_type,
                    flight_number, emitter_category, registration, model, squawk,
                    ground_speed_knots, track_degrees, climb_fpm, turn_rate_rot,
                    snr_db, bit_errors_corrected, freq_offset_khz,
                    club_id
                FROM fixes
                WHERE aircraft_id = $1
                ORDER BY timestamp DESC
                LIMIT $2
            "#;

            let results: Vec<FixRow> = sql_query(sql)
                .bind::<diesel::sql_types::Varchar, _>(&aircraft_id)
                .bind::<diesel::sql_types::BigInt, _>(limit)
                .load::<FixRow>(&mut conn)?;

            Ok::<Vec<FixRow>, anyhow::Error>(results)
        })
        .await??;

        Ok(result.into_iter().map(Fix::from).collect())
    }

    /// Get fixes within a time range
    pub async fn get_fixes_in_time_range(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<Fix>> {
        let limit = limit.unwrap_or(1000);

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let sql = r#"
                SELECT
                    id, source, destination, via, raw_packet, timestamp,
                    latitude, longitude, altitude_feet,
                    aircraft_id, device_id, device_type, aircraft_type,
                    flight_number, emitter_category, registration, model, squawk,
                    ground_speed_knots, track_degrees, climb_fpm, turn_rate_rot,
                    snr_db, bit_errors_corrected, freq_offset_khz,
                    club_id
                FROM fixes
                WHERE timestamp BETWEEN $1 AND $2
                ORDER BY timestamp DESC
                LIMIT $3
            "#;

            let results: Vec<FixRow> = sql_query(sql)
                .bind::<diesel::sql_types::Timestamptz, _>(start_time)
                .bind::<diesel::sql_types::Timestamptz, _>(end_time)
                .bind::<diesel::sql_types::BigInt, _>(limit)
                .load::<FixRow>(&mut conn)?;

            Ok::<Vec<FixRow>, anyhow::Error>(results)
        })
        .await??;

        Ok(result.into_iter().map(Fix::from).collect())
    }

    /// Get recent fixes (most recent first)
    pub async fn get_recent_fixes(&self, limit: i64) -> Result<Vec<Fix>> {
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let sql = r#"
                SELECT
                    id, source, destination, via, raw_packet, timestamp,
                    latitude, longitude, altitude_feet,
                    aircraft_id, device_id, device_type, aircraft_type,
                    flight_number, emitter_category, registration, model, squawk,
                    ground_speed_knots, track_degrees, climb_fpm, turn_rate_rot,
                    snr_db, bit_errors_corrected, freq_offset_khz,
                    club_id
                FROM fixes
                ORDER BY timestamp DESC
                LIMIT $1
            "#;

            let results: Vec<FixRow> = sql_query(sql)
                .bind::<diesel::sql_types::BigInt, _>(limit)
                .load::<FixRow>(&mut conn)?;

            Ok::<Vec<FixRow>, anyhow::Error>(results)
        })
        .await??;

        Ok(result.into_iter().map(Fix::from).collect())
    }

    /// Get fixes for aircraft within time range (keeping the original method for compatibility)
    pub async fn get_fixes_for_aircraft_in_time_range(
        &self,
        aircraft_id: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<Fix>> {
        // This is now the same as get_fixes_for_aircraft_with_time_range
        self.get_fixes_for_aircraft_with_time_range(aircraft_id, start_time, end_time, limit)
            .await
    }

    /// Delete old fixes beyond a retention period
    pub async fn delete_old_fixes(&self, retention_days: i32) -> Result<u64> {
        use crate::schema::fixes::dsl::*;

        let cutoff_time = Utc::now() - chrono::Duration::days(retention_days as i64);
        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let deleted_count =
                diesel::delete(fixes.filter(timestamp.lt(cutoff_time))).execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(deleted_count)
        })
        .await??;

        Ok(result as u64)
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
            let mut conn = pool.get()?;

            let sql = if after.is_some() {
                r#"
                    SELECT id, source, destination, via, raw_packet, timestamp,
                           latitude, longitude, altitude_feet, aircraft_id, device_type,
                           aircraft_type, flight_number, emitter_category, registration,
                           model, squawk, ground_speed_knots, track_degrees, climb_fpm,
                           turn_rate_rot, snr_db, bit_errors_corrected, freq_offset_khz,
                           club_id, flight_id, unparsed_data, device_id
                    FROM fixes
                    WHERE device_id = $1 AND timestamp > $2
                    ORDER BY timestamp DESC
                    LIMIT $3
                "#
            } else {
                r#"
                    SELECT id, source, destination, via, raw_packet, timestamp,
                           latitude, longitude, altitude_feet, aircraft_id, device_type,
                           aircraft_type, flight_number, emitter_category, registration,
                           model, squawk, ground_speed_knots, track_degrees, climb_fpm,
                           turn_rate_rot, snr_db, bit_errors_corrected, freq_offset_khz,
                           club_id, flight_id, unparsed_data, device_id
                    FROM fixes
                    WHERE device_id = $1
                    ORDER BY timestamp DESC
                    LIMIT $2
                "#
            };

            let results: Vec<FixRow> = if let Some(after_time) = after {
                diesel::sql_query(sql)
                    .bind::<diesel::sql_types::Uuid, _>(device_uuid)
                    .bind::<diesel::sql_types::Timestamptz, _>(after_time)
                    .bind::<diesel::sql_types::BigInt, _>(limit)
                    .load::<FixRow>(&mut conn)?
            } else {
                diesel::sql_query(sql)
                    .bind::<diesel::sql_types::Uuid, _>(device_uuid)
                    .bind::<diesel::sql_types::BigInt, _>(limit)
                    .load::<FixRow>(&mut conn)?
            };

            Ok::<Vec<FixRow>, anyhow::Error>(results)
        })
        .await??;

        Ok(result.into_iter().map(Fix::from).collect())
    }
}
