use anyhow::{Context, Result};
use chrono::{Local, Utc};
use diesel::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use std::env;
use std::fs;
use std::path::Path;
use tracing::{error, info};

use soar::airspace::NewAirspace;
use soar::airspaces_repo::AirspacesRepository;
use soar::openaip_client::{
    OpenAipClient, map_airspace_type, map_altitude_reference, map_altitude_unit, map_icao_class,
};
use soar::schema::airspace_sync_log;

/// Get the airspaces data directory based on environment
fn get_airspaces_directory(date: &str) -> Result<String> {
    let soar_env = env::var("SOAR_ENV").unwrap_or_default();
    let is_production = soar_env == "production";
    let is_staging = soar_env == "staging";

    if is_production {
        Ok(format!("/tmp/soar/airspaces-{}", date))
    } else if is_staging {
        Ok(format!("/tmp/soar/staging/airspaces-{}", date))
    } else {
        // Use ~/.cache/soar for development
        let home_dir =
            env::var("HOME").map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
        Ok(format!("{}/.cache/soar/airspaces-{}", home_dir, date))
    }
}

pub async fn handle_pull_airspaces(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
    incremental: bool,
    countries: Option<Vec<String>>,
) -> Result<()> {
    info!("Starting pull-airspaces operation");

    // Get OpenAIP API key from environment
    let api_key = env::var("OPENAIP_API_KEY").context(
        "OPENAIP_API_KEY environment variable not set. \
         Get your API key from https://www.openaip.net/users/clients",
    )?;

    // Create temporary directory with date
    let date = Local::now().format("%Y%m%d").to_string();
    let temp_dir = get_airspaces_directory(&date)?;
    info!("Creating airspaces directory: {}", temp_dir);
    fs::create_dir_all(&temp_dir)?;

    // Create sync log entry
    let sync_id = create_sync_log(&diesel_pool, countries.as_ref())?;

    // Initialize client and repository
    let client = OpenAipClient::new(api_key);
    let repo = AirspacesRepository::new(diesel_pool.clone());

    // Determine updated_after for incremental sync
    let updated_after = if incremental {
        get_last_sync_time(&diesel_pool)?
    } else {
        None
    };

    if let Some(since) = updated_after {
        info!("Performing incremental sync (updated after {})", since);
    } else {
        info!("Performing full sync");
    }

    // Fetch airspaces and save to temp directory
    let mut total_fetched = 0;
    let mut total_inserted = 0;
    let mut errors = Vec::new();

    match countries {
        Some(ref country_list) => {
            // Fetch specific countries
            for country in country_list {
                info!("Fetching airspaces for country: {}", country);

                let airspaces_file = format!("{}/airspaces_{}.json", temp_dir, country);

                // Check if file already exists
                if Path::new(&airspaces_file).exists() {
                    info!(
                        "Airspaces file already exists for {}, skipping fetch: {}",
                        country, airspaces_file
                    );
                } else {
                    match client
                        .fetch_all_airspaces(Some(country), updated_after)
                        .await
                    {
                        Ok(airspaces) => {
                            total_fetched += airspaces.len();

                            // Save to JSON file
                            let json = serde_json::to_string_pretty(&airspaces)?;
                            fs::write(&airspaces_file, json)?;
                            info!(
                                "Saved {} airspaces for {} to {}",
                                airspaces.len(),
                                country,
                                airspaces_file
                            );
                        }
                        Err(e) => {
                            let msg = format!("Failed to fetch airspaces for {}: {}", country, e);
                            error!("{}", msg);
                            errors.push(msg);
                            continue;
                        }
                    }
                }

                // Now process the saved file
                info!("Processing airspaces from {}", airspaces_file);
                match fs::read_to_string(&airspaces_file) {
                    Ok(json_content) => {
                        match serde_json::from_str::<Vec<soar::openaip_client::OpenAipAirspace>>(
                            &json_content,
                        ) {
                            Ok(airspaces) => match convert_airspaces(airspaces) {
                                Ok(converted) => match repo.upsert_airspaces(converted).await {
                                    Ok(count) => {
                                        total_inserted += count;
                                        info!("Upserted {} airspaces for {}", count, country);
                                    }
                                    Err(e) => {
                                        let msg = format!(
                                            "Failed to upsert airspaces for {}: {}",
                                            country, e
                                        );
                                        error!("{}", msg);
                                        errors.push(msg);
                                    }
                                },
                                Err(e) => {
                                    let msg = format!(
                                        "Failed to convert airspaces for {}: {}",
                                        country, e
                                    );
                                    error!("{}", msg);
                                    errors.push(msg);
                                }
                            },
                            Err(e) => {
                                let msg = format!(
                                    "Failed to parse airspaces JSON for {}: {}",
                                    country, e
                                );
                                error!("{}", msg);
                                errors.push(msg);
                            }
                        }
                    }
                    Err(e) => {
                        let msg = format!("Failed to read airspaces file for {}: {}", country, e);
                        error!("{}", msg);
                        errors.push(msg);
                    }
                }
            }
        }
        None => {
            // Global sync - no country filter
            info!("Fetching all airspaces globally (this may take 10-20 minutes)");

            let airspaces_file = format!("{}/airspaces_global.json", temp_dir);

            // Check if file already exists
            if Path::new(&airspaces_file).exists() {
                info!(
                    "Airspaces file already exists, skipping fetch: {}",
                    airspaces_file
                );
            } else {
                match client.fetch_all_airspaces(None, updated_after).await {
                    Ok(airspaces) => {
                        total_fetched = airspaces.len();

                        // Save to JSON file
                        let json = serde_json::to_string_pretty(&airspaces)?;
                        fs::write(&airspaces_file, json)?;
                        info!("Saved {} airspaces to {}", airspaces.len(), airspaces_file);
                    }
                    Err(e) => {
                        let msg = format!("Failed to fetch airspaces: {}", e);
                        error!("{}", msg);
                        errors.push(msg);
                    }
                }
            }

            // Now process the saved file
            if !errors.is_empty() {
                // Skip processing if fetch failed
            } else {
                info!("Processing airspaces from {}", airspaces_file);
                match fs::read_to_string(&airspaces_file) {
                    Ok(json_content) => {
                        match serde_json::from_str::<Vec<soar::openaip_client::OpenAipAirspace>>(
                            &json_content,
                        ) {
                            Ok(airspaces) => match convert_airspaces(airspaces) {
                                Ok(converted) => match repo.upsert_airspaces(converted).await {
                                    Ok(count) => {
                                        total_inserted = count;
                                        info!("Upserted {} airspaces", total_inserted);
                                    }
                                    Err(e) => {
                                        let msg = format!("Failed to upsert airspaces: {}", e);
                                        error!("{}", msg);
                                        errors.push(msg);
                                    }
                                },
                                Err(e) => {
                                    let msg = format!("Failed to convert airspaces: {}", e);
                                    error!("{}", msg);
                                    errors.push(msg);
                                }
                            },
                            Err(e) => {
                                let msg = format!("Failed to parse airspaces JSON: {}", e);
                                error!("{}", msg);
                                errors.push(msg);
                            }
                        }
                    }
                    Err(e) => {
                        let msg = format!("Failed to read airspaces file: {}", e);
                        error!("{}", msg);
                        errors.push(msg);
                    }
                }
            }
        }
    }

    info!("Airspaces data saved to: {}", temp_dir);

    // Update sync log
    let success = errors.is_empty();
    let error_message = if !errors.is_empty() {
        Some(errors.join("; "))
    } else {
        None
    };

    update_sync_log(
        &diesel_pool,
        sync_id,
        success,
        total_fetched as i32,
        total_inserted as i32,
        0, // airspaces_updated - we don't track this separately
        error_message.as_deref(),
    )?;

    // Record metrics
    metrics::counter!("airspace_sync.total_fetched_total").increment(total_fetched as u64);
    metrics::counter!("airspace_sync.total_inserted_total").increment(total_inserted as u64);
    metrics::gauge!("airspace_sync.last_run_timestamp").set(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as f64,
    );
    metrics::gauge!("airspace_sync.success").set(if success { 1.0 } else { 0.0 });

    if success {
        info!(
            "Airspace sync completed successfully: {} fetched, {} upserted",
            total_fetched, total_inserted
        );
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Airspace sync completed with errors: {}",
            error_message.unwrap()
        ))
    }
}

/// Convert OpenAIP airspaces to our database format
fn convert_airspaces(
    openaip_airspaces: Vec<soar::openaip_client::OpenAipAirspace>,
) -> Result<Vec<(NewAirspace, serde_json::Value)>> {
    openaip_airspaces
        .into_iter()
        .map(|oa| {
            // Convert altitude limits
            let (lower_value, lower_unit, lower_ref) = convert_altitude_limit(&oa.lower_limit);
            let (upper_value, upper_unit, upper_ref) = convert_altitude_limit(&oa.upper_limit);

            let airspace = NewAirspace {
                openaip_id: oa.id,
                name: oa.name,
                airspace_class: map_icao_class(oa.icao_class),
                airspace_type: map_airspace_type(oa.airspace_type),
                country_code: Some(oa.country),
                lower_value,
                lower_unit,
                lower_reference: lower_ref,
                upper_value,
                upper_unit,
                upper_reference: upper_ref,
                remarks: oa.remarks,
                activity_type: oa.activity.map(|a| a.to_string()),
                openaip_updated_at: oa.updated_at,
            };

            Ok((airspace, oa.geometry))
        })
        .collect()
}

/// Convert altitude limit to database format
fn convert_altitude_limit(
    limit: &Option<soar::openaip_client::AltitudeLimit>,
) -> (
    Option<i32>,
    Option<String>,
    Option<soar::airspace::AltitudeReference>,
) {
    match limit {
        Some(l) => {
            let value = l.value.map(|v| v as i32);
            let unit = map_altitude_unit(l.unit);
            let reference = map_altitude_reference(l.reference_datum);
            (value, unit, reference)
        }
        None => (None, None, None),
    }
}

/// Create sync log entry
fn create_sync_log(
    pool: &Pool<ConnectionManager<PgConnection>>,
    countries: Option<&Vec<String>>,
) -> Result<uuid::Uuid> {
    use uuid::Uuid;

    let mut conn = pool.get()?;
    let sync_id = Uuid::new_v4();

    diesel::insert_into(airspace_sync_log::table)
        .values((
            airspace_sync_log::id.eq(sync_id),
            airspace_sync_log::countries_filter.eq(countries.map(|c| {
                c.iter()
                    .map(|s| Some(s.clone()))
                    .collect::<Vec<Option<String>>>()
            })),
        ))
        .execute(&mut conn)?;

    Ok(sync_id)
}

/// Update sync log entry with results
fn update_sync_log(
    pool: &Pool<ConnectionManager<PgConnection>>,
    sync_id: uuid::Uuid,
    success: bool,
    fetched: i32,
    inserted: i32,
    updated: i32,
    error_msg: Option<&str>,
) -> Result<()> {
    let mut conn = pool.get()?;

    diesel::update(airspace_sync_log::table.find(sync_id))
        .set((
            airspace_sync_log::completed_at.eq(Utc::now()),
            airspace_sync_log::success.eq(success),
            airspace_sync_log::airspaces_fetched.eq(fetched),
            airspace_sync_log::airspaces_inserted.eq(inserted),
            airspace_sync_log::airspaces_updated.eq(updated),
            airspace_sync_log::error_message.eq(error_msg),
        ))
        .execute(&mut conn)?;

    Ok(())
}

/// Get last successful sync time for incremental updates
fn get_last_sync_time(
    pool: &Pool<ConnectionManager<PgConnection>>,
) -> Result<Option<chrono::DateTime<Utc>>> {
    let mut conn = pool.get()?;

    let last_sync: Option<Option<chrono::DateTime<Utc>>> = airspace_sync_log::table
        .filter(airspace_sync_log::success.eq(true))
        .filter(airspace_sync_log::completed_at.is_not_null())
        .select(airspace_sync_log::completed_at)
        .order(airspace_sync_log::completed_at.desc())
        .first(&mut conn)
        .optional()?;

    // Flatten Option<Option<DateTime>> to Option<DateTime>
    // The outer Option is from .optional() (no rows), inner is from nullable column
    Ok(last_sync.and_then(|x| x))
}
