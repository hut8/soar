use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use serde::Deserialize;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;
use tracing::{error, info, warn};

use soar::aircraft_types::{IcaoAircraftCategory, WingType};
use soar::email_reporter::EntityMetrics;
use soar::schema::aircraft_types;

// Statically embed the aircraft types CSV file into the binary
// Data source: https://www.kaggle.com/datasets/colmog/aircraft-and-aircraft-manufacturers
const AIRCRAFT_TYPES_CSV: &str = include_str!("../../../data/aircraft-and-manufacturers.csv");

/// CSV record from aircraft-and-manufacturers.csv
/// Columns: IATACode,ICAOCode,Model,Aircraft_Manufacturer,WingType,Type,Manufacturer
#[derive(Debug, Deserialize)]
struct CsvRecord {
    #[serde(rename = "IATACode")]
    iata_code: String,
    #[serde(rename = "ICAOCode")]
    icao_code: String,
    #[serde(rename = "Model")]
    model: String,
    #[serde(rename = "Aircraft_Manufacturer")]
    aircraft_manufacturer: String,
    #[serde(rename = "WingType")]
    wing_type: String,
    #[serde(rename = "Type")]
    aircraft_type: String,
    // Note: "Manufacturer" column is ignored (duplicate of Aircraft_Manufacturer)
}

/// Processed record ready for database insertion
#[derive(Debug)]
struct AircraftTypeRecord {
    icao_code: String,
    iata_code: String, // Empty string for NULL/N/A/(undefined)
    description: String,
    manufacturer: Option<String>,
    wing_type: Option<WingType>,
    aircraft_category: Option<IcaoAircraftCategory>,
}

#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = aircraft_types)]
struct NewAircraftType {
    icao_code: String,
    iata_code: String,
    description: String,
    manufacturer: Option<String>,
    wing_type: Option<WingType>,
    aircraft_category: Option<IcaoAircraftCategory>,
}

/// Normalize IATA code: convert "N/A" and "(undefined)" to empty string
fn normalize_iata_code(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed == "N/A" || trimmed == "(undefined)" {
        String::new()
    } else {
        trimmed.to_string()
    }
}

/// Normalize manufacturer: convert "(undefined)" to None
fn normalize_manufacturer(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed == "(undefined)" {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn read_aircraft_types_embedded() -> Result<Vec<AircraftTypeRecord>> {
    info!("Reading aircraft types from embedded CSV data");

    let mut reader = csv::Reader::from_reader(AIRCRAFT_TYPES_CSV.as_bytes());
    let mut records = Vec::new();

    for result in reader.deserialize() {
        let csv_record: CsvRecord = match result {
            Ok(r) => r,
            Err(e) => {
                warn!("Skipping malformed CSV row: {}", e);
                continue;
            }
        };

        // Parse wing type
        let wing_type = match WingType::from_str(&csv_record.wing_type) {
            Ok(wt) => Some(wt),
            Err(e) => {
                warn!(
                    "Invalid wing type '{}' for ICAO {}: {}",
                    csv_record.wing_type, csv_record.icao_code, e
                );
                None
            }
        };

        // Parse aircraft category
        let aircraft_category = match IcaoAircraftCategory::from_str(&csv_record.aircraft_type) {
            Ok(ac) => Some(ac),
            Err(e) => {
                warn!(
                    "Invalid aircraft category '{}' for ICAO {}: {}",
                    csv_record.aircraft_type, csv_record.icao_code, e
                );
                None
            }
        };

        records.push(AircraftTypeRecord {
            icao_code: csv_record.icao_code.trim().to_string(),
            iata_code: normalize_iata_code(&csv_record.iata_code),
            description: csv_record.model.trim().to_string(),
            manufacturer: normalize_manufacturer(&csv_record.aircraft_manufacturer),
            wing_type,
            aircraft_category,
        });
    }

    info!(
        "Parsed {} aircraft types from embedded CSV data",
        records.len()
    );
    Ok(records)
}

async fn upsert_aircraft_types(
    pool: Pool<ConnectionManager<PgConnection>>,
    types: Vec<AircraftTypeRecord>,
) -> Result<usize> {
    // Deduplicate by (icao_code, iata_code), merging descriptions with " / " separator
    let mut dedup_map: HashMap<(String, String), NewAircraftType> = HashMap::new();

    for record in types {
        let key = (record.icao_code.clone(), record.iata_code.clone());
        dedup_map
            .entry(key)
            .and_modify(|existing| {
                // Merge descriptions if different
                if !existing.description.contains(&record.description) {
                    existing.description =
                        format!("{} / {}", existing.description, record.description);
                }
            })
            .or_insert_with(|| NewAircraftType {
                icao_code: record.icao_code,
                iata_code: record.iata_code,
                description: record.description,
                manufacturer: record.manufacturer,
                wing_type: record.wing_type,
                aircraft_category: record.aircraft_category,
            });
    }

    let new_types: Vec<NewAircraftType> = dedup_map.into_values().collect();
    let count = new_types.len();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        diesel::insert_into(aircraft_types::table)
            .values(&new_types)
            .on_conflict((aircraft_types::icao_code, aircraft_types::iata_code))
            .do_update()
            .set((
                aircraft_types::description.eq(diesel::dsl::sql("excluded.description")),
                aircraft_types::manufacturer.eq(diesel::dsl::sql("excluded.manufacturer")),
                aircraft_types::wing_type.eq(diesel::dsl::sql("excluded.wing_type")),
                aircraft_types::aircraft_category
                    .eq(diesel::dsl::sql("excluded.aircraft_category")),
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
    info!("Loading aircraft types from embedded CSV data");

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
