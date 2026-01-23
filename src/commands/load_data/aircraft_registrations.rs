use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use std::time::Instant;
use tracing::{error, info};

use soar::aircraft_registrations::read_aircraft_file;
use soar::aircraft_registrations_repo::AircraftRegistrationsRepository;
use soar::email_reporter::EntityMetrics;

pub async fn load_aircraft_registrations(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
    aircraft_registrations_path: &str,
) -> Result<(usize, i64)> {
    info!(
        "Loading aircraft registrations from: {}",
        aircraft_registrations_path
    );

    let aircraft_registrations = read_aircraft_file(aircraft_registrations_path)?;
    info!(
        "Successfully loaded {} aircraft registrations",
        aircraft_registrations.len()
    );

    let aircraft_repo = AircraftRegistrationsRepository::new(diesel_pool);
    info!(
        "Upserting {} aircraft registrations into database...",
        aircraft_registrations.len()
    );

    let upserted_count = aircraft_repo
        .upsert_aircraft_registrations(aircraft_registrations)
        .await?;
    info!(
        "Successfully upserted {} aircraft registrations",
        upserted_count
    );

    let total_count = aircraft_repo.get_aircraft_registration_count().await?;
    info!("Total aircraft registrations in database: {}", total_count);

    Ok((upserted_count, total_count))
}

pub async fn load_aircraft_registrations_with_metrics(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
    aircraft_registrations_path: Option<String>,
) -> Option<EntityMetrics> {
    if let Some(path) = aircraft_registrations_path {
        let start = Instant::now();
        let mut metrics = EntityMetrics::new("Aircraft Registrations");

        match load_aircraft_registrations(diesel_pool, &path).await {
            Ok((loaded, total)) => {
                metrics.records_loaded = loaded;
                metrics.records_in_db = Some(total);
                metrics.success = true;
            }
            Err(e) => {
                error!("Failed to load aircraft registrations: {}", e);
                metrics.success = false;
                metrics.error_message = Some(e.to_string());
            }
        }

        metrics.duration_secs = start.elapsed().as_secs_f64();
        Some(metrics)
    } else {
        info!("Skipping aircraft registrations - no path provided");
        None
    }
}

/// Copy owner data and year from aircraft_registrations to aircraft
/// Owner data: Only updates aircraft records where owner_operator is NULL or empty
/// Year data: Always updates (FAA data is canonical)
pub async fn copy_owners_to_aircraft(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> Result<usize> {
    info!("Copying owner and year data from aircraft_registrations to aircraft...");

    tokio::task::spawn_blocking(move || {
        let mut conn = diesel_pool.get()?;

        // Update aircraft.owner_operator and aircraft.year from aircraft_registrations
        // owner_operator: Only update if current value is NULL or empty string
        // year: Always update from FAA data (canonical source)
        let query = r#"
            UPDATE aircraft
            SET owner_operator = CASE
                    WHEN (aircraft.owner_operator IS NULL OR aircraft.owner_operator = '')
                         AND ar.registrant_name IS NOT NULL
                         AND ar.registrant_name != ''
                    THEN ar.registrant_name
                    ELSE aircraft.owner_operator
                END,
                year = ar.year_mfr,
                updated_at = CURRENT_TIMESTAMP
            FROM aircraft_registrations ar
            WHERE aircraft.id = ar.aircraft_id
              AND (
                  (ar.registrant_name IS NOT NULL AND ar.registrant_name != ''
                   AND (aircraft.owner_operator IS NULL OR aircraft.owner_operator = ''))
                  OR ar.year_mfr IS NOT NULL
              )
        "#;

        let updated_count = diesel::sql_query(query).execute(&mut conn)?;

        info!(
            "Successfully copied owner and year data to {} aircraft records",
            updated_count
        );

        Ok::<usize, anyhow::Error>(updated_count)
    })
    .await?
}

/// Get the count of aircraft that have an owner_operator set (not null and not empty)
async fn get_aircraft_with_owner_count(
    diesel_pool: &Pool<ConnectionManager<PgConnection>>,
) -> Result<i64> {
    use diesel::dsl::count_star;
    use soar::schema::aircraft;

    let pool = diesel_pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        let count: i64 = aircraft::table
            .filter(aircraft::owner_operator.is_not_null())
            .filter(aircraft::owner_operator.ne(""))
            .select(count_star())
            .first(&mut conn)?;

        Ok(count)
    })
    .await?
}

pub async fn copy_owners_to_aircraft_with_metrics(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> EntityMetrics {
    let start = Instant::now();
    let mut metrics = EntityMetrics::new("Copy Owners to Aircraft");

    match copy_owners_to_aircraft(diesel_pool.clone()).await {
        Ok(updated) => {
            metrics.records_loaded = updated;
            metrics.success = true;

            // Get the total count of aircraft with owner_operator set
            match get_aircraft_with_owner_count(&diesel_pool).await {
                Ok(count) => metrics.records_in_db = Some(count),
                Err(e) => {
                    tracing::warn!("Failed to get aircraft with owner count: {}", e);
                    metrics.records_in_db = None;
                }
            }
        }
        Err(e) => {
            error!("Failed to copy owners to aircraft: {}", e);
            metrics.success = false;
            metrics.error_message = Some(e.to_string());
        }
    }

    metrics.duration_secs = start.elapsed().as_secs_f64();
    metrics
}
