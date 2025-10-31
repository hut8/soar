use anyhow::Result;
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use std::time::Instant;
use tracing::{error, info};

use soar::email_reporter::EntityMetrics;
use soar::faa::aircraft_model_repo::AircraftModelRepository;
use soar::faa::aircraft_models::read_aircraft_models_file;

pub async fn load_aircraft_models(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
    aircraft_models_path: &str,
) -> Result<(usize, i64)> {
    info!("Loading aircraft models from: {}", aircraft_models_path);

    let aircraft_models = read_aircraft_models_file(aircraft_models_path)?;
    info!(
        "Successfully loaded {} aircraft models",
        aircraft_models.len()
    );

    let model_repo = AircraftModelRepository::new(diesel_pool);
    info!(
        "Upserting {} aircraft models into database...",
        aircraft_models.len()
    );

    let upserted_count = model_repo.upsert_aircraft_models(aircraft_models).await?;
    info!("Successfully upserted {} aircraft models", upserted_count);

    let total_count = model_repo.get_aircraft_model_count().await?;
    info!("Total aircraft models in database: {}", total_count);

    Ok((upserted_count, total_count))
}

pub async fn load_aircraft_models_with_metrics(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
    aircraft_models_path: Option<String>,
) -> Option<EntityMetrics> {
    if let Some(path) = aircraft_models_path {
        let start = Instant::now();
        let mut metrics = EntityMetrics::new("Aircraft Models");

        match load_aircraft_models(diesel_pool, &path).await {
            Ok((loaded, total)) => {
                metrics.records_loaded = loaded;
                metrics.records_in_db = Some(total);
                metrics.success = true;
            }
            Err(e) => {
                error!("Failed to load aircraft models: {}", e);
                metrics.success = false;
                metrics.error_message = Some(e.to_string());
            }
        }

        metrics.duration_secs = start.elapsed().as_secs_f64();
        Some(metrics)
    } else {
        info!("Skipping aircraft models - no path provided");
        None
    }
}
