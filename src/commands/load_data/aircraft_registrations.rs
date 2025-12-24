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

/// Copy owner data from aircraft_registrations to aircraft.owner_operator
/// Only updates aircraft records where owner_operator is NULL or empty
pub async fn copy_owners_to_aircraft(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> Result<usize> {
    info!("Copying owner data from aircraft_registrations to aircraft...");

    tokio::task::spawn_blocking(move || {
        let mut conn = diesel_pool.get()?;

        // Update aircraft.owner_operator from aircraft_registrations.registrant_name
        // Only update if owner_operator is NULL or empty string
        let query = r#"
            UPDATE aircraft
            SET owner_operator = ar.registrant_name,
                updated_at = CURRENT_TIMESTAMP
            FROM aircraft_registrations ar
            WHERE aircraft.id = ar.aircraft_id
              AND ar.registrant_name IS NOT NULL
              AND ar.registrant_name != ''
              AND (aircraft.owner_operator IS NULL OR aircraft.owner_operator = '')
        "#;

        let updated_count = diesel::sql_query(query).execute(&mut conn)?;

        info!(
            "Successfully copied owner data to {} aircraft records",
            updated_count
        );

        Ok::<usize, anyhow::Error>(updated_count)
    })
    .await?
}

pub async fn copy_owners_to_aircraft_with_metrics(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> EntityMetrics {
    let start = Instant::now();
    let mut metrics = EntityMetrics::new("Copy Owners to Aircraft");

    match copy_owners_to_aircraft(diesel_pool).await {
        Ok(updated) => {
            metrics.records_loaded = updated;
            metrics.success = true;
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
