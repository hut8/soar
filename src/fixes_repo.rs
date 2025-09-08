use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::{debug, warn};

use crate::fixes::Fix;
use crate::ogn_aprs_aircraft::{AddressType, AdsbEmitterCategory, AircraftType};

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
        sqlx::query!(
            r#"
            INSERT INTO fixes (
                id,
                source,
                destination,
                via,
                raw_packet,
                timestamp,
                latitude,
                longitude,
                altitude_feet,
                aircraft_id,
                device_id,
                device_type,
                aircraft_type,
                flight_number,
                emitter_category,
                registration, model, squawk,
                ground_speed_knots, track_degrees, climb_fpm, turn_rate_rot,
                snr_db, bit_errors_corrected, freq_offset_khz,
                club_id, created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9,
                $10, $11, $12, $13,
                $14, $15, $16, $17, $18,
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
            fix.device_id.map(|a| a as i32),
            fix.device_type as _,
            fix.aircraft_type as _,
            fix.flight_number,
            fix.emitter_category as _,
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

        debug!(
            "Inserted fix for aircraft: {:?}",
            fix.get_aircraft_identifier()
        );
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
                    id,
                    source,
                    destination,
                    via,
                    raw_packet,
                    timestamp,
                    latitude, longitude, altitude_feet,
                    aircraft_id, device_id, device_type, aircraft_type,
                    flight_number, emitter_category, registration, model, squawk,
                    ground_speed_knots, track_degrees, climb_fpm, turn_rate_rot,
                    snr_db, bit_errors_corrected, freq_offset_khz,
                    club_id, created_at, updated_at
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6,
                    $7, $8, $9,
                    $10, $11, $12, $13,
                    $14, $15, $16, $17, $18,
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
                fix.device_id.map(|a| a as i32),
                fix.device_type as _,
                fix.aircraft_type as _,
                fix.flight_number,
                fix.emitter_category as _,
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
                    warn!(
                        "Failed to insert fix for {:?}: {}",
                        fix.get_aircraft_identifier(),
                        e
                    );
                    // Continue with other fixes rather than failing the entire batch
                }
            }
        }

        transaction.commit().await?;
        debug!("Inserted {} fixes in batch", inserted_count);
        Ok(inserted_count)
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

        let results = sqlx::query!(
            r#"
            SELECT
                id, source, destination, via, raw_packet, timestamp,
                latitude, longitude, altitude_feet,
                aircraft_id, device_id, device_type as "device_type: AddressType", aircraft_type as "aircraft_type: AircraftType",
                flight_number, emitter_category as "emitter_category: AdsbEmitterCategory", registration, model, squawk,
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
                device_id: row.device_id.map(|a| a as u32),
                device_type: row.device_type,
                aircraft_type: row.aircraft_type,
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
                bit_errors_corrected: row.bit_errors_corrected.map(|b| b as u32),
                freq_offset_khz: row.freq_offset_khz,
                club_id: row.club_id,
                created_at: row.created_at.unwrap_or_else(Utc::now),
                updated_at: row.updated_at.unwrap_or_else(Utc::now),
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
                aircraft_id, device_id, device_type as "device_type: AddressType", aircraft_type as "aircraft_type: AircraftType",
                flight_number, emitter_category as "emitter_category: AdsbEmitterCategory", registration, model, squawk,
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
                device_id: row.device_id.map(|a| a as u32),
                device_type: row.device_type,
                aircraft_type: row.aircraft_type,
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
                bit_errors_corrected: row.bit_errors_corrected.map(|b| b as u32),
                freq_offset_khz: row.freq_offset_khz,
                club_id: row.club_id,
                created_at: row.created_at.unwrap_or_else(Utc::now),
                updated_at: row.updated_at.unwrap_or_else(Utc::now),
            });
        }

        Ok(fixes)
    }

    /// Get recent fixes for an aircraft (without time range)
    pub async fn get_fixes_for_aircraft(
        &self,
        aircraft_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<Fix>> {
        let limit = limit.unwrap_or(100);

        let results = sqlx::query!(
            r#"
            SELECT
                id, source, destination, via, raw_packet, timestamp,
                latitude, longitude, altitude_feet,
                aircraft_id, device_id, device_type as "device_type: AddressType", aircraft_type as "aircraft_type: AircraftType",
                flight_number, emitter_category as "emitter_category: AdsbEmitterCategory", registration, model, squawk,
                ground_speed_knots, track_degrees, climb_fpm, turn_rate_rot,
                snr_db, bit_errors_corrected, freq_offset_khz,
                club_id, created_at, updated_at
            FROM fixes
            WHERE aircraft_id = $1
            ORDER BY timestamp DESC
            LIMIT $2
            "#,
            aircraft_id,
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
                device_id: row.device_id.map(|a| a as u32),
                device_type: row.device_type,
                aircraft_type: row.aircraft_type,
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
                bit_errors_corrected: row.bit_errors_corrected.map(|b| b as u32),
                freq_offset_khz: row.freq_offset_khz,
                club_id: row.club_id,
                created_at: row.created_at.unwrap_or_else(Utc::now),
                updated_at: row.updated_at.unwrap_or_else(Utc::now),
            });
        }

        Ok(fixes)
    }

    /// Get fixes within a time range
    pub async fn get_fixes_in_time_range(
        &self,
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
                aircraft_id, device_id, device_type as "device_type: AddressType", aircraft_type as "aircraft_type: AircraftType",
                flight_number, emitter_category as "emitter_category: AdsbEmitterCategory", registration, model, squawk,
                ground_speed_knots, track_degrees, climb_fpm, turn_rate_rot,
                snr_db, bit_errors_corrected, freq_offset_khz,
                club_id, created_at, updated_at
            FROM fixes
            WHERE timestamp BETWEEN $1 AND $2
            ORDER BY timestamp DESC
            LIMIT $3
            "#,
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
                device_id: row.device_id.map(|a| a as u32),
                device_type: row.device_type,
                aircraft_type: row.aircraft_type,
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
                bit_errors_corrected: row.bit_errors_corrected.map(|b| b as u32),
                freq_offset_khz: row.freq_offset_khz,
                club_id: row.club_id,
                created_at: row.created_at.unwrap_or_else(Utc::now),
                updated_at: row.updated_at.unwrap_or_else(Utc::now),
            });
        }

        Ok(fixes)
    }

    /// Get recent fixes (most recent first)
    pub async fn get_recent_fixes(&self, limit: i64) -> Result<Vec<Fix>> {
        let results = sqlx::query!(
            r#"
            SELECT
                id, source, destination, via, raw_packet, timestamp,
                latitude, longitude, altitude_feet,
                aircraft_id, device_id, device_type as "device_type: AddressType", aircraft_type as "aircraft_type: AircraftType",
                flight_number, emitter_category as "emitter_category: AdsbEmitterCategory", registration, model, squawk,
                ground_speed_knots, track_degrees, climb_fpm, turn_rate_rot,
                snr_db, bit_errors_corrected, freq_offset_khz,
                club_id, created_at, updated_at
            FROM fixes
            ORDER BY timestamp DESC
            LIMIT $1
            "#,
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
                device_id: row.device_id.map(|a| a as u32),
                device_type: row.device_type,
                aircraft_type: row.aircraft_type,
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
                bit_errors_corrected: row.bit_errors_corrected.map(|b| b as u32),
                freq_offset_khz: row.freq_offset_khz,
                club_id: row.club_id,
                created_at: row.created_at.unwrap_or_else(Utc::now),
                updated_at: row.updated_at.unwrap_or_else(Utc::now),
            });
        }

        Ok(fixes)
    }

    /// Get fixes for aircraft within time range (keeping the original method for compatibility)
    pub async fn get_fixes_for_aircraft_in_time_range(
        &self,
        aircraft_id: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<Fix>> {
        self.get_fixes_for_aircraft_in_time_range_impl(aircraft_id, start_time, end_time, limit)
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
        let limit = limit.unwrap_or(1000);

        let results = sqlx::query!(
            r#"
            SELECT
                id, source, destination, via, raw_packet, timestamp,
                latitude, longitude, altitude_feet,
                aircraft_id, device_id, device_type as "device_type: AddressType", aircraft_type as "aircraft_type: AircraftType",
                flight_number, emitter_category as "emitter_category: AdsbEmitterCategory", registration, model, squawk,
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
                device_id: row.device_id.map(|a| a as u32),
                device_type: row.device_type,
                aircraft_type: row.aircraft_type,
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
                bit_errors_corrected: row.bit_errors_corrected.map(|b| b as u32),
                freq_offset_khz: row.freq_offset_khz,
                club_id: row.club_id,
                created_at: row.created_at.unwrap_or_else(Utc::now),
                updated_at: row.updated_at.unwrap_or_else(Utc::now),
            });
        }

        Ok(fixes)
    }

    /// Delete old fixes beyond a retention period
    pub async fn delete_old_fixes(&self, retention_days: i32) -> Result<u64> {
        let cutoff_time = Utc::now() - chrono::Duration::days(retention_days as i64);

        let result = sqlx::query!("DELETE FROM fixes WHERE timestamp < $1", cutoff_time)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}
