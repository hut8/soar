use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use std::time::Instant;
use tracing::info;

use crate::email_reporter::EntityMetrics;

type PgPool = Pool<ConnectionManager<PgConnection>>;

/// Link aircraft_registrations.device_id by matching registration numbers
/// This should be called after both devices and aircraft_registrations are loaded
pub async fn link_aircraft_to_devices_with_metrics(pool: PgPool) -> EntityMetrics {
    let start = Instant::now();
    let mut metrics = EntityMetrics::new("Link Aircraft to Devices");

    info!("Linking aircraft registrations to devices by registration number...");

    match link_aircraft_to_devices(&pool).await {
        Ok(count) => {
            info!("Successfully linked {} aircraft to devices", count);
            metrics.records_loaded = count;

            // Get total count of aircraft_registrations with device_id set
            match get_aircraft_with_device_count(&pool).await {
                Ok(total) => {
                    metrics.records_in_db = Some(total);
                }
                Err(e) => {
                    info!("Failed to get aircraft device count: {}", e);
                    metrics.records_in_db = None;
                }
            }
            metrics.success = true;
        }
        Err(e) => {
            metrics.success = false;
            metrics.error_message = Some(e.to_string());
        }
    }

    metrics.duration_secs = start.elapsed().as_secs_f64();
    metrics
}

/// Link devices.club_id from linked aircraft_registrations
/// This should be called after aircraft have been linked to devices
pub async fn link_devices_to_clubs_with_metrics(pool: PgPool) -> EntityMetrics {
    let start = Instant::now();
    let mut metrics = EntityMetrics::new("Link Devices to Clubs");

    info!("Linking devices to clubs from aircraft registrations...");

    match link_devices_to_clubs(&pool).await {
        Ok(count) => {
            info!("Successfully linked {} devices to clubs", count);
            metrics.records_loaded = count;

            // Get total count of devices with club_id set
            match get_devices_with_club_count(&pool).await {
                Ok(total) => {
                    metrics.records_in_db = Some(total);
                }
                Err(e) => {
                    info!("Failed to get device club count: {}", e);
                    metrics.records_in_db = None;
                }
            }
            metrics.success = true;
        }
        Err(e) => {
            metrics.success = false;
            metrics.error_message = Some(e.to_string());
        }
    }

    metrics.duration_secs = start.elapsed().as_secs_f64();
    metrics
}

/// Link aircraft_registrations.device_id by matching with devices.registration
async fn link_aircraft_to_devices(pool: &PgPool) -> Result<usize> {
    let pool = pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        // Update aircraft_registrations.device_id by matching registration numbers
        let count = diesel::sql_query(
            r#"
            UPDATE aircraft_registrations
            SET device_id = devices.id
            FROM devices
            WHERE aircraft_registrations.registration_number = devices.registration
              AND aircraft_registrations.device_id IS NULL
              AND devices.registration != ''
            "#,
        )
        .execute(&mut conn)?;

        Ok(count)
    })
    .await?
}

/// Link devices.club_id from linked aircraft_registrations
async fn link_devices_to_clubs(pool: &PgPool) -> Result<usize> {
    let pool = pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        // Update devices.club_id from linked aircraft_registrations
        let count = diesel::sql_query(
            r#"
            UPDATE devices
            SET club_id = aircraft_registrations.club_id
            FROM aircraft_registrations
            WHERE devices.id = aircraft_registrations.device_id
              AND aircraft_registrations.club_id IS NOT NULL
              AND devices.club_id IS NULL
            "#,
        )
        .execute(&mut conn)?;

        Ok(count)
    })
    .await?
}

/// Get count of devices with club_id set
async fn get_devices_with_club_count(pool: &PgPool) -> Result<i64> {
    let pool = pool.clone();

    tokio::task::spawn_blocking(move || {
        use diesel::RunQueryDsl;
        use diesel::dsl::sql;
        use diesel::sql_types::BigInt;

        let mut conn = pool.get()?;

        let count: i64 = diesel::select(sql::<BigInt>(
            "COUNT(*) FROM devices WHERE club_id IS NOT NULL",
        ))
        .get_result(&mut conn)?;

        Ok(count)
    })
    .await?
}

/// Get count of aircraft_registrations with device_id set
async fn get_aircraft_with_device_count(pool: &PgPool) -> Result<i64> {
    let pool = pool.clone();

    tokio::task::spawn_blocking(move || {
        use diesel::RunQueryDsl;
        use diesel::dsl::sql;
        use diesel::sql_types::BigInt;

        let mut conn = pool.get()?;

        let count: i64 = diesel::select(sql::<BigInt>(
            "COUNT(*) FROM aircraft_registrations WHERE device_id IS NOT NULL",
        ))
        .get_result(&mut conn)?;

        Ok(count)
    })
    .await?
}
