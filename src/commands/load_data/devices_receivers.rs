use anyhow::Result;
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use std::time::Instant;
use tracing::{error, info};

use soar::aircraft::read_flarmnet_file;
use soar::aircraft_repo::AircraftRepository;
use soar::email_reporter::EntityMetrics;
use soar::receiver_repo::ReceiverRepository;
use soar::receivers::read_receivers_file;

pub async fn load_receivers(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
    receivers_path: &str,
) -> Result<(usize, i64)> {
    info!("Loading receivers from: {}", receivers_path);

    let receivers_data = read_receivers_file(receivers_path)?;
    let receivers = receivers_data.receivers.unwrap_or_default();
    info!("Successfully loaded {} receivers", receivers.len());

    let receiver_repo = ReceiverRepository::new(diesel_pool);
    info!("Upserting {} receivers into database...", receivers.len());

    let upserted_count = receiver_repo.upsert_receivers(receivers).await?;
    info!("Successfully upserted {} receivers", upserted_count);

    let total_count = receiver_repo.get_receiver_count().await?;
    info!("Total receivers in database: {}", total_count);

    Ok((upserted_count, total_count))
}

pub async fn load_receivers_with_metrics(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
    receivers_path: Option<String>,
) -> Option<EntityMetrics> {
    if let Some(path) = receivers_path {
        let start = Instant::now();
        let mut metrics = EntityMetrics::new("Receivers");

        match load_receivers(diesel_pool, &path).await {
            Ok((loaded, total)) => {
                metrics.records_loaded = loaded;
                metrics.records_in_db = Some(total);
                metrics.success = true;
            }
            Err(e) => {
                error!("Failed to load receivers: {}", e);
                metrics.success = false;
                metrics.error_message = Some(e.to_string());
            }
        }

        metrics.duration_secs = start.elapsed().as_secs_f64();
        Some(metrics)
    } else {
        info!("Skipping receivers - no path provided");
        None
    }
}

pub async fn load_devices(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
    devices_path: &str,
) -> Result<(usize, i64)> {
    info!("Loading devices from: {}", devices_path);

    let devices = read_flarmnet_file(devices_path)?;
    info!("Successfully loaded {} aircraft", devices.len());

    let aircraft_repo = AircraftRepository::new(diesel_pool);
    info!("Upserting {} aircraft into database...", devices.len());

    let upserted_count = aircraft_repo.upsert_aircraft(devices).await?;
    info!("Successfully upserted {} aircraft", upserted_count);

    let total_count = aircraft_repo.get_aircraft_count().await?;
    info!("Total aircraft in database: {}", total_count);

    Ok((upserted_count, total_count))
}

pub async fn load_devices_with_metrics(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
    devices_path: Option<String>,
) -> Option<EntityMetrics> {
    if let Some(path) = devices_path {
        let start = Instant::now();
        let mut metrics = EntityMetrics::new("Aircraft (from Unified DDB)");

        match load_devices(diesel_pool, &path).await {
            Ok((loaded, total)) => {
                metrics.records_loaded = loaded;
                metrics.records_in_db = Some(total);
                metrics.success = true;
            }
            Err(e) => {
                error!("Failed to load devices: {}", e);
                metrics.success = false;
                metrics.error_message = Some(e.to_string());
            }
        }

        metrics.duration_secs = start.elapsed().as_secs_f64();
        Some(metrics)
    } else {
        info!("Skipping devices - no path provided");
        None
    }
}
