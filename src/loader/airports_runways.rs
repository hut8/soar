use anyhow::Result;
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use std::time::Instant;
use tracing::{error, info};

use crate::airports::read_airports_csv_file;
use crate::airports_repo::AirportsRepository;
use crate::email_reporter::EntityMetrics;
use crate::runways::read_runways_csv_file;
use crate::runways_repo::RunwaysRepository;

pub async fn load_airports(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
    airports_path: &str,
) -> Result<(usize, i64)> {
    info!("Loading airports from: {}", airports_path);

    let airports = read_airports_csv_file(airports_path)?;
    info!("Successfully loaded {} airports", airports.len());

    let airports_repo = AirportsRepository::new(diesel_pool);
    info!("Upserting {} airports into database...", airports.len());

    let upserted_count = airports_repo.upsert_airports(airports).await?;
    info!("Successfully upserted {} airports", upserted_count);

    let total_count = airports_repo.get_airport_count().await?;
    info!("Total airports in database: {}", total_count);

    Ok((upserted_count, total_count))
}

pub async fn load_airports_with_metrics(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
    airports_path: Option<String>,
) -> Option<EntityMetrics> {
    if let Some(path) = airports_path {
        let start = Instant::now();
        let mut metrics = EntityMetrics::new("Airports");

        match load_airports(diesel_pool, &path).await {
            Ok((loaded, total)) => {
                metrics.records_loaded = loaded;
                metrics.records_in_db = Some(total);
                metrics.success = true;
            }
            Err(e) => {
                error!("Failed to load airports: {}", e);
                metrics.success = false;
                metrics.error_message = Some(e.to_string());
            }
        }

        metrics.duration_secs = start.elapsed().as_secs_f64();
        Some(metrics)
    } else {
        info!("Skipping airports - no path provided");
        None
    }
}

pub async fn load_runways(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
    runways_path: &str,
) -> Result<(usize, i64)> {
    info!("Loading runways from: {}", runways_path);

    let runways = read_runways_csv_file(runways_path)?;
    info!("Successfully loaded {} runways", runways.len());

    let runways_repo = RunwaysRepository::new(diesel_pool);
    info!("Upserting {} runways into database...", runways.len());

    let upserted_count = runways_repo.upsert_runways(runways).await?;
    info!("Successfully upserted {} runways", upserted_count);

    let total_count = runways_repo.get_runway_count().await?;
    info!("Total runways in database: {}", total_count);

    Ok((upserted_count, total_count))
}

pub async fn load_runways_with_metrics(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
    runways_path: Option<String>,
) -> Option<EntityMetrics> {
    if let Some(path) = runways_path {
        let start = Instant::now();
        let mut metrics = EntityMetrics::new("Runways");

        match load_runways(diesel_pool, &path).await {
            Ok((loaded, total)) => {
                metrics.records_loaded = loaded;
                metrics.records_in_db = Some(total);
                metrics.success = true;
            }
            Err(e) => {
                error!("Failed to load runways: {}", e);
                metrics.success = false;
                metrics.error_message = Some(e.to_string());
            }
        }

        metrics.duration_secs = start.elapsed().as_secs_f64();
        Some(metrics)
    } else {
        info!("Skipping runways - no path provided");
        None
    }
}
