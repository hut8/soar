use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use serde::Deserialize;
use std::time::Instant;
use tracing::{error, info};

use soar::email_reporter::EntityMetrics;
use soar::schema::aircraft_types;

// Statically embed the aircraft types JSON file into the binary
const AIRCRAFT_TYPES_JSON: &str = include_str!("../../../data/aircraft-types-icao-iata.json");

#[derive(Debug, Deserialize)]
struct AircraftTypeRecord {
    #[serde(rename = "icaoCode")]
    icao_code: String,
    #[serde(rename = "iataCode")]
    iata_code: Option<String>,
    description: String,
}

#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = aircraft_types)]
struct NewAircraftType {
    icao_code: String,
    iata_code: Option<String>,
    description: String,
}

fn read_aircraft_types_embedded() -> Result<Vec<AircraftTypeRecord>> {
    info!("Reading aircraft types from embedded data");
    let records: Vec<AircraftTypeRecord> = serde_json::from_str(AIRCRAFT_TYPES_JSON)?;
    info!("Parsed {} aircraft types from embedded data", records.len());
    Ok(records)
}

async fn upsert_aircraft_types(
    pool: Pool<ConnectionManager<PgConnection>>,
    types: Vec<AircraftTypeRecord>,
) -> Result<usize> {
    let new_types: Vec<NewAircraftType> = types
        .into_iter()
        .map(|record| NewAircraftType {
            icao_code: record.icao_code,
            iata_code: record.iata_code,
            description: record.description,
        })
        .collect();

    let count = new_types.len();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        diesel::insert_into(aircraft_types::table)
            .values(&new_types)
            .on_conflict(aircraft_types::icao_code)
            .do_update()
            .set((
                aircraft_types::iata_code.eq(diesel::dsl::sql("excluded.iata_code")),
                aircraft_types::description.eq(diesel::dsl::sql("excluded.description")),
                aircraft_types::updated_at.eq(diesel::dsl::now),
            ))
            .execute(&mut conn)?;
        Ok::<usize, anyhow::Error>(count)
    })
    .await?
}

async fn get_aircraft_types_count(pool: Pool<ConnectionManager<PgConnection>>) -> Result<i64> {
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        let count = aircraft_types::table.count().get_result(&mut conn)?;
        Ok::<i64, anyhow::Error>(count)
    })
    .await?
}

pub async fn load_aircraft_types(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> Result<(usize, i64)> {
    info!("Loading aircraft types from embedded data");

    let types = read_aircraft_types_embedded()?;

    info!("Upserting {} aircraft types into database...", types.len());
    let upserted_count = upsert_aircraft_types(diesel_pool.clone(), types).await?;
    info!("Successfully upserted {} aircraft types", upserted_count);

    let total_count = get_aircraft_types_count(diesel_pool).await?;
    info!("Total aircraft types in database: {}", total_count);

    Ok((upserted_count, total_count))
}

pub async fn load_aircraft_types_with_metrics(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> EntityMetrics {
    let start = Instant::now();
    let mut metrics = EntityMetrics::new("Aircraft Types");

    match load_aircraft_types(diesel_pool).await {
        Ok((loaded, total)) => {
            metrics.records_loaded = loaded;
            metrics.records_in_db = Some(total);
            metrics.success = true;
        }
        Err(e) => {
            error!("Failed to load aircraft types: {}", e);
            metrics.success = false;
            metrics.error_message = Some(e.to_string());
        }
    }

    metrics.duration_secs = start.elapsed().as_secs_f64();
    metrics
}
