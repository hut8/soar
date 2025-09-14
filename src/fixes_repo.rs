use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::sql_query;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::fixes::Fix;
use crate::ogn_aprs_aircraft::{AddressType, AdsbEmitterCategory, AircraftType};
use crate::web::PgPool;

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
    device_id: Option<i32>,
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
            device_id: fix.device_id.map(|d| d as i32),
            device_type: fix.device_type,
            aircraft_type: fix.aircraft_type,
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
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
    device_id: Option<i32>,
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
}

impl From<FixRow> for Fix {
    fn from(row: FixRow) -> Self {
        // Helper function to parse enum from string
        fn parse_device_type(s: Option<String>) -> Option<AddressType> {
            s.and_then(|s| match s.as_str() {
                "Unknown" => Some(AddressType::Unknown),
                "Icao" => Some(AddressType::Icao),
                "Flarm" => Some(AddressType::Flarm),
                "OgnTracker" => Some(AddressType::OgnTracker),
                _ => None,
            })
        }

        fn parse_aircraft_type(s: Option<String>) -> Option<AircraftType> {
            s.and_then(|s| match s.as_str() {
                "Reserved0" => Some(AircraftType::Reserved0),
                "GliderMotorGlider" => Some(AircraftType::GliderMotorGlider),
                "TowTug" => Some(AircraftType::TowTug),
                "HelicopterGyro" => Some(AircraftType::HelicopterGyro),
                "SkydiverParachute" => Some(AircraftType::SkydiverParachute),
                "DropPlane" => Some(AircraftType::DropPlane),
                "HangGlider" => Some(AircraftType::HangGlider),
                "Paraglider" => Some(AircraftType::Paraglider),
                "RecipEngine" => Some(AircraftType::RecipEngine),
                "JetTurboprop" => Some(AircraftType::JetTurboprop),
                "Unknown" => Some(AircraftType::Unknown),
                "Balloon" => Some(AircraftType::Balloon),
                "Airship" => Some(AircraftType::Airship),
                "Uav" => Some(AircraftType::Uav),
                "ReservedE" => Some(AircraftType::ReservedE),
                "StaticObstacle" => Some(AircraftType::StaticObstacle),
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
            device_id: row.device_id.map(|d| d as u32),
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

        let pool = self.pool.clone();
        let new_fix = NewFix::from(fix);
        let aircraft_identifier = fix.get_aircraft_identifier();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            diesel::insert_into(fixes)
                .values(&new_fix)
                .execute(&mut conn)?;

            Ok::<(), anyhow::Error>(())
        }).await??;

        debug!(
            "Inserted fix for aircraft: {:?}",
            aircraft_identifier
        );
        Ok(())
    }

    /// Insert multiple fixes in a batch transaction
    pub async fn insert_batch(&self, fix_list: &[Fix]) -> Result<usize> {
        if fix_list.is_empty() {
            return Ok(0);
        }

        use crate::schema::fixes::dsl::*;

        let pool = self.pool.clone();
        let fixes_data: Vec<NewFix> = fix_list.iter().map(NewFix::from).collect();
        let fixes_count = fixes_data.len();

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let mut inserted_count = 0;

            // Use a transaction for batch processing
            conn.transaction(|conn| {
                for fix_data in fixes_data {
                    match diesel::insert_into(fixes)
                        .values(&fix_data)
                        .execute(conn)
                    {
                        Ok(_) => inserted_count += 1,
                        Err(e) => {
                            warn!(
                                "Failed to insert fix with ID {:?}: {}",
                                fix_data.id,
                                e
                            );
                            // Continue with other fixes rather than failing the entire batch
                        }
                    }
                }

                Ok::<(), diesel::result::Error>(())
            })?;

            Ok::<usize, anyhow::Error>(inserted_count)
        }).await??;

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
        }).await??;

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
        }).await??;

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
        }).await??;

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
        }).await??;

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
        }).await??;

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

    /// Private implementation for the original aircraft + time range method
    async fn get_fixes_for_aircraft_in_time_range_impl(
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

            let deleted_count = diesel::delete(fixes.filter(timestamp.lt(cutoff_time)))
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(deleted_count)
        }).await??;

        Ok(result as u64)
    }
}
