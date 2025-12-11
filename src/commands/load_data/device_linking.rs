use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use std::time::Instant;
use tracing::info;

use soar::email_reporter::EntityMetrics;

type PgPool = Pool<ConnectionManager<PgConnection>>;

/// Link aircraft_registrations.aircraft_id by matching registration numbers
/// This should be called after both aircraft and aircraft_registrations are loaded
pub async fn link_aircraft_to_devices_with_metrics(pool: PgPool) -> EntityMetrics {
    let start = Instant::now();
    let mut metrics = EntityMetrics::new("Link Aircraft to Devices");

    info!("Linking aircraft registrations to aircraft by registration number...");

    match link_aircraft_to_devices(&pool).await {
        Ok(count) => {
            info!("Successfully linked {} aircraft to aircraft", count);
            metrics.records_loaded = count;

            // Get total count of aircraft_registrations with aircraft_id set
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

/// Link aircraft.club_id from linked aircraft_registrations
/// This should be called after aircraft have been linked to aircraft
pub async fn link_devices_to_clubs_with_metrics(pool: PgPool) -> EntityMetrics {
    let start = Instant::now();
    let mut metrics = EntityMetrics::new("Link Devices to Clubs");

    info!("Linking aircraft to clubs from aircraft registrations...");

    match link_devices_to_clubs(&pool).await {
        Ok(count) => {
            info!("Successfully linked {} aircraft to clubs", count);
            metrics.records_loaded = count;

            // Get total count of aircraft with club_id set
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

/// Link aircraft_registrations.aircraft_id by matching with aircraft.registration
async fn link_aircraft_to_devices(pool: &PgPool) -> Result<usize> {
    let pool = pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        // Update aircraft_registrations.aircraft_id by matching registration numbers
        let count = diesel::sql_query(
            r#"
            UPDATE aircraft_registrations
            SET aircraft_id = aircraft.id
            FROM aircraft
            WHERE aircraft_registrations.registration_number = aircraft.registration
              AND aircraft_registrations.aircraft_id IS NULL
              AND aircraft.registration != ''
            "#,
        )
        .execute(&mut conn)?;

        Ok(count)
    })
    .await?
}

/// Link aircraft.club_id from linked aircraft_registrations
async fn link_devices_to_clubs(pool: &PgPool) -> Result<usize> {
    let pool = pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        // Update aircraft.club_id from linked aircraft_registrations
        let count = diesel::sql_query(
            r#"
            UPDATE aircraft
            SET club_id = aircraft_registrations.club_id
            FROM aircraft_registrations
            WHERE aircraft.id = aircraft_registrations.aircraft_id
              AND aircraft_registrations.club_id IS NOT NULL
              AND aircraft.club_id IS NULL
            "#,
        )
        .execute(&mut conn)?;

        Ok(count)
    })
    .await?
}

/// Get count of aircraft with club_id set
async fn get_devices_with_club_count(pool: &PgPool) -> Result<i64> {
    use diesel::dsl::count_star;
    use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
    use soar::schema::aircraft::dsl::*;

    let pool = pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        let count = aircraft
            .filter(club_id.is_not_null())
            .select(count_star())
            .first::<i64>(&mut conn)?;

        Ok(count)
    })
    .await?
}

/// Get count of aircraft_registrations with aircraft_id set
async fn get_aircraft_with_device_count(pool: &PgPool) -> Result<i64> {
    use diesel::dsl::count_star;
    use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
    use soar::schema::aircraft_registrations::dsl::*;

    let pool = pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        let count = aircraft_registrations
            .filter(aircraft_id.is_not_null())
            .select(count_star())
            .first::<i64>(&mut conn)?;

        Ok(count)
    })
    .await?
}
