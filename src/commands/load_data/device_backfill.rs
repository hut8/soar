use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use std::time::Instant;
use tracing::{info, warn};

use soar::aircraft::{AddressType, Aircraft, AircraftModel};
use soar::email_reporter::EntityMetrics;

type PgPool = Pool<ConnectionManager<PgConnection>>;

/// Backfill country codes for ICAO aircraft that don't have one
/// This should be called after aircraft are loaded
pub async fn backfill_country_codes_with_metrics(pool: PgPool) -> EntityMetrics {
    let start = Instant::now();
    let mut metrics = EntityMetrics::new("Backfill Aircraft Country Codes");

    info!("Backfilling country codes for ICAO aircraft...");

    match backfill_country_codes(&pool).await {
        Ok(count) => {
            info!(
                "Successfully backfilled country codes for {} aircraft",
                count
            );
            metrics.records_loaded = count;

            // Get total count of aircraft with country_code set
            match get_devices_with_country_code_count(&pool).await {
                Ok(total) => {
                    metrics.records_in_db = Some(total);
                }
                Err(e) => {
                    warn!("Failed to get device country code count: {}", e);
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

/// Backfill tail numbers for US ICAO aircraft that don't have a registration
/// This should be called after country codes are backfilled
pub async fn backfill_tail_numbers_with_metrics(pool: PgPool) -> EntityMetrics {
    let start = Instant::now();
    let mut metrics = EntityMetrics::new("Backfill US Aircraft Tail Numbers");

    info!("Backfilling tail numbers for US ICAO aircraft...");

    match backfill_tail_numbers(&pool).await {
        Ok(count) => {
            info!(
                "Successfully backfilled tail numbers for {} aircraft",
                count
            );
            metrics.records_loaded = count;

            // Get total count of US ICAO aircraft with registration set
            match get_us_icao_devices_with_registration_count(&pool).await {
                Ok(total) => {
                    metrics.records_in_db = Some(total);
                }
                Err(e) => {
                    warn!("Failed to get US ICAO device registration count: {}", e);
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

/// Backfill country codes for ICAO aircraft that don't have one
async fn backfill_country_codes(pool: &PgPool) -> Result<usize> {
    use soar::schema::aircraft::dsl::*;

    let pool = pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        // Load all ICAO aircraft without country_code
        let devices_to_update: Vec<AircraftModel> = aircraft
            .filter(address_type.eq(AddressType::Icao))
            .filter(country_code.is_null())
            .load::<AircraftModel>(&mut conn)?;

        info!(
            "Found {} ICAO aircraft without country codes",
            devices_to_update.len()
        );

        let mut updated_count = 0;

        // Iterate over each device and extract country code
        for device_model in devices_to_update {
            if let Some(extracted_country_code) = Aircraft::extract_country_code_from_icao(
                device_model.address as u32,
                AddressType::Icao,
            ) {
                // Update the device with the extracted country code
                match diesel::update(aircraft.filter(id.eq(device_model.id)))
                    .set(country_code.eq(&extracted_country_code))
                    .execute(&mut conn)
                {
                    Ok(_) => {
                        updated_count += 1;
                        info!(
                            "Updated device {} with country code: {}",
                            device_model.id, extracted_country_code
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to update device {} with country code: {}",
                            device_model.id, e
                        );
                    }
                }
            } else {
                warn!(
                    "Could not extract country code for device {} (address: {:06X})",
                    device_model.id, device_model.address
                );
            }
        }

        Ok(updated_count)
    })
    .await?
}

/// Backfill tail numbers for US ICAO aircraft that don't have a registration
async fn backfill_tail_numbers(pool: &PgPool) -> Result<usize> {
    use soar::schema::aircraft::dsl::*;

    let pool = pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        // Load all US ICAO aircraft without registration
        let devices_to_update: Vec<AircraftModel> = aircraft
            .filter(address_type.eq(AddressType::Icao))
            .filter(country_code.eq("US"))
            .filter(registration.eq(""))
            .load::<AircraftModel>(&mut conn)?;

        info!(
            "Found {} US ICAO aircraft without registrations",
            devices_to_update.len()
        );

        let mut updated_count = 0;

        // Iterate over each device and extract tail number
        for device_model in devices_to_update {
            if let Some(tail_number) = Aircraft::extract_tail_number_from_icao(
                device_model.address as u32,
                AddressType::Icao,
            ) {
                // Update the device with the extracted tail number
                match diesel::update(aircraft.filter(id.eq(device_model.id)))
                    .set(registration.eq(&tail_number))
                    .execute(&mut conn)
                {
                    Ok(_) => {
                        updated_count += 1;
                        info!(
                            "Updated device {} with tail number: {}",
                            device_model.id, tail_number
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to update device {} with tail number: {}",
                            device_model.id, e
                        );
                    }
                }
            } else {
                warn!(
                    "Could not extract tail number for device {} (address: {:06X})",
                    device_model.id, device_model.address
                );
            }
        }

        Ok(updated_count)
    })
    .await?
}

/// Get count of aircraft with country_code set
async fn get_devices_with_country_code_count(pool: &PgPool) -> Result<i64> {
    use diesel::dsl::count_star;
    use soar::schema::aircraft::dsl::*;

    let pool = pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        let count = aircraft
            .filter(country_code.is_not_null())
            .select(count_star())
            .first::<i64>(&mut conn)?;

        Ok(count)
    })
    .await?
}

/// Get count of US ICAO aircraft with registration set
async fn get_us_icao_devices_with_registration_count(pool: &PgPool) -> Result<i64> {
    use diesel::dsl::count_star;
    use soar::schema::aircraft::dsl::*;

    let pool = pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        let count = aircraft
            .filter(address_type.eq(AddressType::Icao))
            .filter(country_code.eq("US"))
            .filter(registration.ne(""))
            .select(count_star())
            .first::<i64>(&mut conn)?;

        Ok(count)
    })
    .await?
}
