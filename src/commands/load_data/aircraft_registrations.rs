use anyhow::Result;
use diesel::PgConnection;
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
