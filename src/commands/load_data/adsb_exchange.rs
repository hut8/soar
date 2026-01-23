use anyhow::{Context, Result};
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use serde::Deserialize;
use std::fs;
use std::time::Instant;
use tracing::{debug, error, info};

use soar::aircraft::AddressType;
use soar::aircraft_types::{AircraftCategory, EngineType};
use soar::email_reporter::EntityMetrics;
use soar::schema::aircraft;

#[derive(Debug, Deserialize)]
struct AdsbExchangeRecord {
    icao: String,
    #[serde(rename = "reg")]
    registration: Option<String>,
    #[serde(rename = "icaotype")]
    icao_type_code: Option<String>,
    #[serde(rename = "ownop")]
    owner_operator: Option<String>,
    /// Year is a string in the JSON (e.g., "1970" or ""), not a number
    year: Option<String>,
    manufacturer: Option<String>,
    model: Option<String>,
    faa_pia: Option<bool>,
    faa_ladd: Option<bool>,
    short_type: Option<String>,
    #[serde(rename = "mil")]
    is_military: Option<bool>,
}

/// Parse short_type format: [Category][NumEngines][EngineType]
/// Example: "L2J" = Landplane, 2 engines, Jet
fn parse_short_type(
    short_type: &str,
) -> (Option<AircraftCategory>, Option<i16>, Option<EngineType>) {
    let chars: Vec<char> = short_type.chars().collect();

    if chars.len() < 3 {
        return (None, None, None);
    }

    let category = AircraftCategory::from_short_type_char(chars[0]);

    let engine_count = chars[1].to_digit(10).map(|n| n as i16);

    let engine_type = EngineType::from_short_type_char(chars[2]);

    (category, engine_count, engine_type)
}

/// Parse ICAO hex address and determine address type
/// If ICAO starts with '~', it's a non-ICAO address (e.g., from TIS-B)
fn parse_icao_address(icao_hex: &str) -> Result<(i32, AddressType)> {
    let (cleaned_hex, address_type) = if let Some(stripped) = icao_hex.strip_prefix('~') {
        (stripped, AddressType::Unknown)
    } else {
        (icao_hex, AddressType::Icao)
    };

    let address = i32::from_str_radix(cleaned_hex, 16)
        .with_context(|| format!("Failed to parse ICAO hex address: {}", icao_hex))?;

    Ok((address, address_type))
}

/// Canonicalize registration using flydent
fn canonicalize_registration(registration: &str) -> String {
    let parser = flydent::Parser::new();
    match parser.parse(registration, false, false) {
        Some(r) => r.canonical_callsign().to_string(),
        None => registration.to_string(),
    }
}

/// Build aircraft model string from manufacturer and model
/// Returns concatenation of manufacturer + " " + model if both present
fn build_aircraft_model(manufacturer: Option<&String>, model: Option<&String>) -> String {
    match (manufacturer, model) {
        (Some(mfr), Some(mdl)) if !mfr.is_empty() && !mdl.is_empty() => {
            format!("{} {}", mfr, mdl)
        }
        (Some(mfr), None) if !mfr.is_empty() => mfr.clone(),
        (None, Some(mdl)) if !mdl.is_empty() => mdl.clone(),
        _ => String::new(),
    }
}

#[derive(Debug, Insertable)]
#[diesel(table_name = aircraft)]
struct NewAircraftAdsb {
    address: i32,
    address_type: AddressType,
    registration: String,
    aircraft_model: String,
    competition_number: String,
    tracked: bool,
    identified: bool,
    from_ogn_ddb: bool,
    from_adsbx_ddb: bool,
    icao_model_code: Option<String>,
    owner_operator: Option<String>,
    aircraft_category: Option<AircraftCategory>,
    engine_count: Option<i16>,
    engine_type: Option<EngineType>,
    faa_pia: Option<bool>,
    faa_ladd: Option<bool>,
    year: Option<i16>,
    is_military: Option<bool>,
}

pub async fn load_adsb_exchange_data(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
    adsb_path: &str,
) -> Result<(usize, usize)> {
    info!("Loading ADS-B Exchange data from: {}", adsb_path);

    // Read and parse the NDJSON file (newline-delimited JSON)
    // Each line is a separate JSON object, not a single array
    let file_content = fs::read_to_string(adsb_path)?;
    let mut records = Vec::new();
    let mut parse_errors = 0;

    for (line_num, line) in file_content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match serde_json::from_str::<AdsbExchangeRecord>(line) {
            Ok(record) => records.push(record),
            Err(e) => {
                debug!(
                    "Failed to parse line {} from ADS-B Exchange: {}",
                    line_num + 1,
                    e
                );
                parse_errors += 1;
            }
        }
    }

    info!(
        "Successfully parsed {} records from ADS-B Exchange ({} parse errors)",
        records.len(),
        parse_errors
    );

    let mut inserted_count = 0;
    let mut skipped_invalid_icao = 0;
    let mut skipped_no_data = 0;

    // Process records in batches for better performance
    const BATCH_SIZE: usize = 1000;

    for batch in records.chunks(BATCH_SIZE) {
        let mut batch_inserts = Vec::new();

        for record in batch {
            // Parse ICAO address
            let (address, address_type) = match parse_icao_address(&record.icao) {
                Ok(parsed) => parsed,
                Err(e) => {
                    debug!("Skipping record with invalid ICAO {}: {}", record.icao, e);
                    skipped_invalid_icao += 1;
                    continue;
                }
            };

            // Validate icao_model_code is exactly 3 or 4 characters (per ICAO Doc 8643) if present
            let icao_model_code = record
                .icao_type_code
                .as_ref()
                .filter(|code| {
                    let len = code.len();
                    len == 3 || len == 4
                })
                .cloned();

            // Skip records with no useful data
            // NOTE: registration is intentionally NOT checked here - we want to preserve registrations
            // from other sources (FAA, FlarmNet) which are more authoritative
            if icao_model_code.is_none()
                && record.owner_operator.is_none()
                && record.manufacturer.is_none()
                && record.model.is_none()
                && record.faa_pia.is_none()
                && record.faa_ladd.is_none()
                && record.short_type.is_none()
                && record.year.is_none()
                && record.is_military.is_none()
            {
                debug!(
                    "Skipping ICAO {} - no useful data (reg: {:?})",
                    record.icao, record.registration
                );
                skipped_no_data += 1;
                continue;
            }

            // Parse short_type into components
            let (category, engine_count, engine_type) = record
                .short_type
                .as_ref()
                .map(|st| parse_short_type(st))
                .unwrap_or((None, None, None));

            // Canonicalize registration if present
            let registration = record
                .registration
                .as_ref()
                .filter(|r| !r.is_empty())
                .map(|r| {
                    let canonical = canonicalize_registration(r);
                    debug!(
                        "ICAO {}: registration '{}' -> canonical '{}'",
                        record.icao, r, canonical
                    );
                    canonical
                })
                .unwrap_or_default();

            // Build aircraft model from manufacturer and model
            let aircraft_model =
                build_aircraft_model(record.manufacturer.as_ref(), record.model.as_ref());

            // Parse year from string (e.g., "1970") to i16, skipping empty strings
            let year = record
                .year
                .as_ref()
                .filter(|y| !y.is_empty())
                .and_then(|y| y.parse::<i16>().ok());

            batch_inserts.push(NewAircraftAdsb {
                address,
                address_type,
                registration,
                aircraft_model,
                competition_number: String::new(),
                tracked: true,
                identified: true,
                from_ogn_ddb: false,
                from_adsbx_ddb: true,
                icao_model_code,
                owner_operator: record.owner_operator.clone(),
                aircraft_category: category,
                engine_count,
                engine_type,
                faa_pia: record.faa_pia,
                faa_ladd: record.faa_ladd,
                year,
                is_military: record.is_military,
            });
        }

        // Insert/update batch
        if !batch_inserts.is_empty() {
            let pool = diesel_pool.clone();
            match tokio::task::spawn_blocking(move || {
                let mut conn = pool.get()?;

                // Use ON CONFLICT to handle existing records
                // Only update fields if they're currently NULL or empty (preserve more authoritative sources)
                let result = diesel::insert_into(aircraft::table)
                    .values(&batch_inserts)
                    .on_conflict((aircraft::address_type, aircraft::address))
                    .do_update()
                    .set((
                        // Only update registration if current value is NULL or empty string
                        aircraft::registration.eq(diesel::dsl::sql(
                            "CASE WHEN (aircraft.registration IS NULL OR aircraft.registration = '')
                             THEN NULLIF(excluded.registration, '')
                             ELSE aircraft.registration END"
                        )),
                        // Only update icao_model_code if current value is NULL
                        aircraft::icao_model_code.eq(diesel::dsl::sql("COALESCE(aircraft.icao_model_code, excluded.icao_model_code)")),
                        // Only update aircraft_model if current value is NULL or empty string
                        aircraft::aircraft_model.eq(diesel::dsl::sql(
                            "CASE WHEN (aircraft.aircraft_model IS NULL OR aircraft.aircraft_model = '')
                             THEN excluded.aircraft_model
                             ELSE aircraft.aircraft_model END"
                        )),
                        // Only update owner_operator if current value is NULL or empty string
                        aircraft::owner_operator.eq(diesel::dsl::sql(
                            "CASE WHEN (aircraft.owner_operator IS NULL OR aircraft.owner_operator = '')
                             THEN excluded.owner_operator
                             ELSE aircraft.owner_operator END"
                        )),
                        // Only update aircraft_category if current value is NULL (preserve FAA/FlarmNet data)
                        aircraft::aircraft_category.eq(diesel::dsl::sql("COALESCE(aircraft.aircraft_category, excluded.aircraft_category)")),
                        // Only update engine_count if current value is NULL (preserve FAA/FlarmNet data)
                        aircraft::engine_count.eq(diesel::dsl::sql("COALESCE(aircraft.engine_count, excluded.engine_count)")),
                        // Only update engine_type if current value is NULL (preserve FAA/FlarmNet data)
                        aircraft::engine_type.eq(diesel::dsl::sql("COALESCE(aircraft.engine_type, excluded.engine_type)")),
                        // Only update faa_pia if current value is NULL (preserve FAA data)
                        aircraft::faa_pia.eq(diesel::dsl::sql("COALESCE(aircraft.faa_pia, excluded.faa_pia)")),
                        // Only update faa_ladd if current value is NULL (preserve FAA data)
                        aircraft::faa_ladd.eq(diesel::dsl::sql("COALESCE(aircraft.faa_ladd, excluded.faa_ladd)")),
                        // Only update year and is_military if current value is NULL (FAA data is canonical)
                        aircraft::year.eq(diesel::dsl::sql("COALESCE(aircraft.year, excluded.year)")),
                        aircraft::is_military.eq(diesel::dsl::sql("COALESCE(aircraft.is_military, excluded.is_military)")),
                        aircraft::from_adsbx_ddb.eq(diesel::dsl::sql("true")),
                        aircraft::address_type.eq(diesel::dsl::sql("excluded.address_type")),
                        aircraft::updated_at.eq(diesel::dsl::now),
                    ))
                    .execute(&mut conn)?;

                Ok::<usize, anyhow::Error>(result)
            })
            .await
            {
                Ok(Ok(rows_affected)) => {
                    // Note: rows_affected includes both inserts and updates
                    // We can't easily distinguish between them with this approach
                    inserted_count += rows_affected;
                }
                Ok(Err(e)) => {
                    error!("Failed to insert/update batch: {}", e);
                }
                Err(e) => {
                    error!("Task failed for batch: {}", e);
                }
            }
        }
    }

    info!(
        "ADS-B Exchange data processing complete: {} aircraft inserted/updated",
        inserted_count
    );
    info!(
        "Skipped: {} parse errors, {} invalid ICAO addresses, {} records with no data",
        parse_errors, skipped_invalid_icao, skipped_no_data
    );

    Ok((inserted_count, 0))
}

pub async fn load_adsb_exchange_with_metrics(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
    adsb_path: Option<String>,
) -> Option<EntityMetrics> {
    if let Some(path) = adsb_path {
        let start = Instant::now();
        let mut metrics = EntityMetrics::new("ADS-B Exchange Aircraft");

        match load_adsb_exchange_data(diesel_pool, &path).await {
            Ok((loaded, _updated)) => {
                metrics.records_loaded = loaded;
                metrics.success = true;
            }
            Err(e) => {
                error!("Failed to load ADS-B Exchange data: {}", e);
                metrics.success = false;
                metrics.error_message = Some(e.to_string());
            }
        }

        metrics.duration_secs = start.elapsed().as_secs_f64();
        Some(metrics)
    } else {
        info!("Skipping ADS-B Exchange data - no path provided");
        None
    }
}
