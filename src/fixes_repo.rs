use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::{debug, warn};

use crate::fixes::Fix;
use crate::ogn_aprs_aircraft::{AddressType, AdsbEmitterCategory, AircraftType};

pub struct FixesRepository {
    pool: PgPool,
}

impl FixesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new fix into the database
    pub async fn insert(&self, fix: &Fix) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO fixes (
                id, source, destination, via, raw_packet, timestamp,
                latitude, longitude, altitude_feet,
                aircraft_id, address, address_type, aircraft_type,
                flight_number, emitter_category, registration, model, squawk,
                ground_speed_knots, track_degrees, climb_fpm, turn_rate_rot,
                snr_db, bit_errors_corrected, freq_offset_khz,
                club_id, created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9,
                $10, $11, $12::address_type, $13::aircraft_type,
                $14, $15::adsb_emitter_category, $16, $17, $18,
                $19, $20, $21, $22,
                $23, $24, $25,
                $26, $27, $28
            )
            "#,
            fix.id,
            fix.source,
            fix.destination,
            &fix.via,
            fix.raw_packet,
            fix.timestamp,
            fix.latitude,
            fix.longitude,
            fix.altitude_feet,
            fix.aircraft_id,
            fix.address.map(|a| a as i32),
            fix.address_type.as_ref().map(|t| match t {
                AddressType::Unknown => "Unknown",
                AddressType::Icao => "Icao",
                AddressType::Flarm => "Flarm",
                AddressType::OgnTracker => "OgnTracker",
            }),
            fix.aircraft_type.as_ref().map(|t| match t {
                AircraftType::Reserved0 => "Reserved0",
                AircraftType::GliderMotorGlider => "GliderMotorGlider",
                AircraftType::TowTug => "TowTug",
                AircraftType::HelicopterGyro => "HelicopterGyro",
                AircraftType::SkydiverParachute => "SkydiverParachute",
                AircraftType::DropPlane => "DropPlane",
                AircraftType::HangGlider => "HangGlider",
                AircraftType::Paraglider => "Paraglider",
                AircraftType::RecipEngine => "RecipEngine",
                AircraftType::JetTurboprop => "JetTurboprop",
                AircraftType::Unknown => "Unknown",
                AircraftType::Balloon => "Balloon",
                AircraftType::Airship => "Airship",
                AircraftType::Uav => "Uav",
                AircraftType::ReservedE => "ReservedE",
                AircraftType::StaticObstacle => "StaticObstacle",
            }),
            fix.flight_number,
            fix.emitter_category.as_ref().map(|c| match c {
                AdsbEmitterCategory::A0 => "A0",
                AdsbEmitterCategory::A1 => "A1",
                AdsbEmitterCategory::A2 => "A2",
                AdsbEmitterCategory::A3 => "A3",
                AdsbEmitterCategory::A4 => "A4",
                AdsbEmitterCategory::A5 => "A5",
                AdsbEmitterCategory::A6 => "A6",
                AdsbEmitterCategory::A7 => "A7",
                AdsbEmitterCategory::B0 => "B0",
                AdsbEmitterCategory::B1 => "B1",
                AdsbEmitterCategory::B2 => "B2",
                AdsbEmitterCategory::B3 => "B3",
                AdsbEmitterCategory::B4 => "B4",
                AdsbEmitterCategory::B6 => "B6",
                AdsbEmitterCategory::B7 => "B7",
                AdsbEmitterCategory::C0 => "C0",
                AdsbEmitterCategory::C1 => "C1",
                AdsbEmitterCategory::C2 => "C2",
                AdsbEmitterCategory::C3 => "C3",
                AdsbEmitterCategory::C4 => "C4",
                AdsbEmitterCategory::C5 => "C5",
            }),
            fix.registration,
            fix.model,
            fix.squawk,
            fix.ground_speed_knots,
            fix.track_degrees,
            fix.climb_fpm,
            fix.turn_rate_rot,
            fix.snr_db,
            fix.bit_errors_corrected.map(|b| b as i32),
            fix.freq_offset_khz,
            fix.club_id,
            fix.created_at,
            fix.updated_at
        )
        .execute(&self.pool)
        .await?;

        debug!("Inserted fix for aircraft: {:?}", fix.get_aircraft_identifier());
        Ok(())
    }

    /// Insert multiple fixes in a batch transaction
    pub async fn insert_batch(&self, fixes: &[Fix]) -> Result<usize> {
        if fixes.is_empty() {
            return Ok(0);
        }

        let mut transaction = self.pool.begin().await?;
        let mut inserted_count = 0;

        for fix in fixes {
            let result = sqlx::query!(
                r#"
                INSERT INTO fixes (
                    id, source, destination, via, raw_packet, timestamp,
                    latitude, longitude, altitude_feet,
                    aircraft_id, address, address_type, aircraft_type,
                    flight_number, emitter_category, registration, model, squawk,
                    ground_speed_knots, track_degrees, climb_fpm, turn_rate_rot,
                    snr_db, bit_errors_corrected, freq_offset_khz,
                    club_id, created_at, updated_at
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6,
                    $7, $8, $9,
                    $10, $11, $12::address_type, $13::aircraft_type,
                    $14, $15::adsb_emitter_category, $16, $17, $18,
                    $19, $20, $21, $22,
                    $23, $24, $25,
                    $26, $27, $28
                )
                "#,
                fix.id,
                fix.source,
                fix.destination,
                &fix.via,
                fix.raw_packet,
                fix.timestamp,
                fix.latitude,
                fix.longitude,
                fix.altitude_feet,
                fix.aircraft_id,
                fix.address.map(|a| a as i32),
                fix.address_type.as_ref().map(|t| match t {
                    AddressType::Unknown => "Unknown",
                    AddressType::Icao => "Icao",
                    AddressType::Flarm => "Flarm",
                    AddressType::OgnTracker => "OgnTracker",
                }),
                fix.aircraft_type.as_ref().map(|t| match t {
                    AircraftType::Reserved0 => "Reserved0",
                    AircraftType::GliderMotorGlider => "GliderMotorGlider",
                    AircraftType::TowTug => "TowTug",
                    AircraftType::HelicopterGyro => "HelicopterGyro",
                    AircraftType::SkydiverParachute => "SkydiverParachute",
                    AircraftType::DropPlane => "DropPlane",
                    AircraftType::HangGlider => "HangGlider",
                    AircraftType::Paraglider => "Paraglider",
                    AircraftType::RecipEngine => "RecipEngine",
                    AircraftType::JetTurboprop => "JetTurboprop",
                    AircraftType::Unknown => "Unknown",
                    AircraftType::Balloon => "Balloon",
                    AircraftType::Airship => "Airship",
                    AircraftType::Uav => "Uav",
                    AircraftType::ReservedE => "ReservedE",
                    AircraftType::StaticObstacle => "StaticObstacle",
                }),
                fix.flight_number,
                fix.emitter_category.as_ref().map(|c| match c {
                    AdsbEmitterCategory::A0 => "A0",
                    AdsbEmitterCategory::A1 => "A1",
                    AdsbEmitterCategory::A2 => "A2",
                    AdsbEmitterCategory::A3 => "A3",
                    AdsbEmitterCategory::A4 => "A4",
                    AdsbEmitterCategory::A5 => "A5",
                    AdsbEmitterCategory::A6 => "A6",
                    AdsbEmitterCategory::A7 => "A7",
                    AdsbEmitterCategory::B0 => "B0",
                    AdsbEmitterCategory::B1 => "B1",
                    AdsbEmitterCategory::B2 => "B2",
                    AdsbEmitterCategory::B3 => "B3",
                    AdsbEmitterCategory::B4 => "B4",
                    AdsbEmitterCategory::B6 => "B6",
                    AdsbEmitterCategory::B7 => "B7",
                    AdsbEmitterCategory::C0 => "C0",
                    AdsbEmitterCategory::C1 => "C1",
                    AdsbEmitterCategory::C2 => "C2",
                    AdsbEmitterCategory::C3 => "C3",
                    AdsbEmitterCategory::C4 => "C4",
                    AdsbEmitterCategory::C5 => "C5",
                }),
                fix.registration,
                fix.model,
                fix.squawk,
                fix.ground_speed_knots,
                fix.track_degrees,
                fix.climb_fpm,
                fix.turn_rate_rot,
                fix.snr_db,
                fix.bit_errors_corrected.map(|b| b as i32),
                fix.freq_offset_khz,
                fix.club_id,
                fix.created_at,
                fix.updated_at
            )
            .execute(&mut *transaction)
            .await;

            match result {
                Ok(_) => inserted_count += 1,
                Err(e) => {
                    warn!("Failed to insert fix for {:?}: {}", fix.get_aircraft_identifier(), e);
                    // Continue with other fixes rather than failing the entire batch
                }
            }
        }

        transaction.commit().await?;
        debug!("Inserted {} fixes in batch", inserted_count);
        Ok(inserted_count)
    }

    /// Get fixes for a specific aircraft ID within a time range
    pub async fn get_fixes_for_aircraft(
        &self,
        aircraft_id: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<Fix>> {
        let limit = limit.unwrap_or(1000);

        let results = sqlx::query!(
            r#"
            SELECT 
                id, source, destination, via, raw_packet, timestamp,
                latitude, longitude, altitude_feet,
                aircraft_id, address, address_type::text, aircraft_type::text,
                flight_number, emitter_category::text, registration, model, squawk,
                ground_speed_knots, track_degrees, climb_fpm, turn_rate_rot,
                snr_db, bit_errors_corrected, freq_offset_khz,
                club_id, created_at, updated_at
            FROM fixes
            WHERE aircraft_id = $1
            AND timestamp BETWEEN $2 AND $3
            ORDER BY timestamp DESC
            LIMIT $4
            "#,
            aircraft_id,
            start_time,
            end_time,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        let mut fixes = Vec::new();
        for row in results {
            fixes.push(Fix {
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
                address: row.address.map(|a| a as u32),
                address_type: row.address_type.as_deref().and_then(|t| match t {
                    "Unknown" => Some(AddressType::Unknown),
                    "Icao" => Some(AddressType::Icao),
                    "Flarm" => Some(AddressType::Flarm),
                    "OgnTracker" => Some(AddressType::OgnTracker),
                    _ => None,
                }),
                aircraft_type: row.aircraft_type.as_deref().and_then(|t| match t {
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
                }),
                flight_number: row.flight_number,
                emitter_category: row.emitter_category.as_deref().and_then(|c| match c {
                    "A0" => Some(AdsbEmitterCategory::A0),
                    "A1" => Some(AdsbEmitterCategory::A1),
                    "A2" => Some(AdsbEmitterCategory::A2),
                    "A3" => Some(AdsbEmitterCategory::A3),
                    "A4" => Some(AdsbEmitterCategory::A4),
                    "A5" => Some(AdsbEmitterCategory::A5),
                    "A6" => Some(AdsbEmitterCategory::A6),
                    "A7" => Some(AdsbEmitterCategory::A7),
                    "B0" => Some(AdsbEmitterCategory::B0),
                    "B1" => Some(AdsbEmitterCategory::B1),
                    "B2" => Some(AdsbEmitterCategory::B2),
                    "B3" => Some(AdsbEmitterCategory::B3),
                    "B4" => Some(AdsbEmitterCategory::B4),
                    "B6" => Some(AdsbEmitterCategory::B6),
                    "B7" => Some(AdsbEmitterCategory::B7),
                    "C0" => Some(AdsbEmitterCategory::C0),
                    "C1" => Some(AdsbEmitterCategory::C1),
                    "C2" => Some(AdsbEmitterCategory::C2),
                    "C3" => Some(AdsbEmitterCategory::C3),
                    "C4" => Some(AdsbEmitterCategory::C4),
                    "C5" => Some(AdsbEmitterCategory::C5),
                    _ => None,
                }),
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
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(fixes)
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

        let results = sqlx::query!(
            r#"
            SELECT 
                id, source, destination, via, raw_packet, timestamp,
                latitude, longitude, altitude_feet,
                aircraft_id, address, address_type::text, aircraft_type::text,
                flight_number, emitter_category::text, registration, model, squawk,
                ground_speed_knots, track_degrees, climb_fpm, turn_rate_rot,
                snr_db, bit_errors_corrected, freq_offset_khz,
                club_id, created_at, updated_at,
                ST_Distance(location, ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography) as distance_meters
            FROM fixes
            WHERE timestamp > $3
            AND ST_DWithin(location, ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography, $4)
            ORDER BY timestamp DESC
            LIMIT $5
            "#,
            center_lat,
            center_lon,
            cutoff_time,
            radius_m,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        let mut fixes = Vec::new();
        for row in results {
            fixes.push(Fix {
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
                address: row.address.map(|a| a as u32),
                address_type: row.address_type.as_deref().and_then(|t| match t {
                    "Unknown" => Some(AddressType::Unknown),
                    "Icao" => Some(AddressType::Icao),
                    "Flarm" => Some(AddressType::Flarm),
                    "OgnTracker" => Some(AddressType::OgnTracker),
                    _ => None,
                }),
                aircraft_type: row.aircraft_type.as_deref().and_then(|t| match t {
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
                }),
                flight_number: row.flight_number,
                emitter_category: row.emitter_category.as_deref().and_then(|c| match c {
                    "A0" => Some(AdsbEmitterCategory::A0),
                    "A1" => Some(AdsbEmitterCategory::A1),
                    "A2" => Some(AdsbEmitterCategory::A2),
                    "A3" => Some(AdsbEmitterCategory::A3),
                    "A4" => Some(AdsbEmitterCategory::A4),
                    "A5" => Some(AdsbEmitterCategory::A5),
                    "A6" => Some(AdsbEmitterCategory::A6),
                    "A7" => Some(AdsbEmitterCategory::A7),
                    "B0" => Some(AdsbEmitterCategory::B0),
                    "B1" => Some(AdsbEmitterCategory::B1),
                    "B2" => Some(AdsbEmitterCategory::B2),
                    "B3" => Some(AdsbEmitterCategory::B3),
                    "B4" => Some(AdsbEmitterCategory::B4),
                    "B6" => Some(AdsbEmitterCategory::B6),
                    "B7" => Some(AdsbEmitterCategory::B7),
                    "C0" => Some(AdsbEmitterCategory::C0),
                    "C1" => Some(AdsbEmitterCategory::C1),
                    "C2" => Some(AdsbEmitterCategory::C2),
                    "C3" => Some(AdsbEmitterCategory::C3),
                    "C4" => Some(AdsbEmitterCategory::C4),
                    "C5" => Some(AdsbEmitterCategory::C5),
                    _ => None,
                }),
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
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(fixes)
    }

    /// Delete old fixes beyond a retention period
    pub async fn delete_old_fixes(&self, retention_days: i32) -> Result<u64> {
        let cutoff_time = Utc::now() - chrono::Duration::days(retention_days as i64);

        let result = sqlx::query!(
            "DELETE FROM fixes WHERE timestamp < $1",
            cutoff_time
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}