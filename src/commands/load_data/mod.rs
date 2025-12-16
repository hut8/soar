mod aircraft_models;
mod aircraft_registrations;
mod airports_runways;
mod device_backfill;
mod device_linking;
mod devices_receivers;
mod home_base_linking;
mod receiver_geocoding;

use anyhow::Result;
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use std::time::Instant;
use tracing::{info, warn};

use soar::email_reporter::{
    DataLoadReport, EmailConfig, EntityMetrics, send_email_report, send_failure_email,
};
use soar::geocoding::geocode_components;
use soar::locations::Point;
use soar::locations_repo::LocationsRepository;

/// Helper function to record Prometheus metrics for a data load stage
fn record_stage_metrics(metrics: &EntityMetrics, stage_name: &str) {
    let stage_name = stage_name.to_string();
    metrics::histogram!("data_load.stage.duration_seconds", "stage" => stage_name.clone())
        .record(metrics.duration_secs);
    metrics::counter!("data_load.stage.records_loaded", "stage" => stage_name.clone())
        .increment(metrics.records_loaded as u64);
    metrics::gauge!("data_load.stage.success", "stage" => stage_name.clone())
        .set(if metrics.success { 1.0 } else { 0.0 });
    if let Some(records_in_db) = metrics.records_in_db {
        metrics::gauge!("data_load.stage.records_in_db", "stage" => stage_name)
            .set(records_in_db as f64);
    }
}

/// Main entry point for loading all data types with email reporting
#[allow(clippy::too_many_arguments)]
pub async fn handle_load_data(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
    aircraft_models_path: Option<String>,
    aircraft_registrations_path: Option<String>,
    airports_path: Option<String>,
    runways_path: Option<String>,
    receivers_path: Option<String>,
    devices_path: Option<String>,
    geocode: bool,
    link_home_bases: bool,
) -> Result<()> {
    sentry::configure_scope(|scope| {
        scope.set_tag("operation", "load-data");
    });

    let overall_start = Instant::now();
    let mut report = DataLoadReport::new();

    // Try to load email config - if it fails, we'll skip email reporting
    let email_config = EmailConfig::from_env().ok();
    if email_config.is_none() {
        warn!("Email configuration not found in environment - skipping email reports");
        warn!("Set SMTP_SERVER, SMTP_USERNAME, SMTP_PASSWORD, EMAIL_FROM, EMAIL_TO");
    }

    // Load each entity type with metrics tracking
    if let Some(metrics) = aircraft_models::load_aircraft_models_with_metrics(
        diesel_pool.clone(),
        aircraft_models_path,
    )
    .await
    {
        record_stage_metrics(&metrics, "aircraft_models");

        // Send immediate failure email if configured
        if !metrics.success
            && let Some(ref config) = email_config
        {
            let _ = send_failure_email(
                config,
                &metrics.name,
                metrics.error_message.as_deref().unwrap_or("Unknown error"),
            );
        }
        report.add_entity(metrics);
    }

    if let Some(metrics) = aircraft_registrations::load_aircraft_registrations_with_metrics(
        diesel_pool.clone(),
        aircraft_registrations_path,
    )
    .await
    {
        record_stage_metrics(&metrics, "aircraft_registrations");
        if !metrics.success
            && let Some(ref config) = email_config
        {
            let _ = send_failure_email(
                config,
                &metrics.name,
                metrics.error_message.as_deref().unwrap_or("Unknown error"),
            );
        }
        report.add_entity(metrics);
    }

    if let Some(metrics) =
        airports_runways::load_airports_with_metrics(diesel_pool.clone(), airports_path).await
    {
        record_stage_metrics(&metrics, "airports");
        if !metrics.success
            && let Some(ref config) = email_config
        {
            let _ = send_failure_email(
                config,
                &metrics.name,
                metrics.error_message.as_deref().unwrap_or("Unknown error"),
            );
        }
        report.add_entity(metrics);
    }

    if let Some(metrics) =
        airports_runways::load_runways_with_metrics(diesel_pool.clone(), runways_path).await
    {
        record_stage_metrics(&metrics, "runways");
        if !metrics.success
            && let Some(ref config) = email_config
        {
            let _ = send_failure_email(
                config,
                &metrics.name,
                metrics.error_message.as_deref().unwrap_or("Unknown error"),
            );
        }
        report.add_entity(metrics);
    }

    if let Some(metrics) =
        devices_receivers::load_receivers_with_metrics(diesel_pool.clone(), receivers_path).await
    {
        record_stage_metrics(&metrics, "receivers");
        if !metrics.success
            && let Some(ref config) = email_config
        {
            let _ = send_failure_email(
                config,
                &metrics.name,
                metrics.error_message.as_deref().unwrap_or("Unknown error"),
            );
        }
        report.add_entity(metrics);
    }

    // Geocode receivers after they're loaded (1 second between requests to respect Nominatim rate limit)
    // Note: Individual receiver geocoding failures are normal and will be reported in the summary email
    {
        let metrics = receiver_geocoding::geocode_receivers_with_metrics(diesel_pool.clone()).await;
        record_stage_metrics(&metrics, "receiver_geocoding");

        // Don't send immediate failure email for receiver geocoding - failures are normal
        // Failed receivers will be listed in the summary email instead
        report.add_entity(metrics);
    }

    if let Some(metrics) =
        devices_receivers::load_devices_with_metrics(diesel_pool.clone(), devices_path).await
    {
        record_stage_metrics(&metrics, "devices");
        if !metrics.success
            && let Some(ref config) = email_config
        {
            let _ = send_failure_email(
                config,
                &metrics.name,
                metrics.error_message.as_deref().unwrap_or("Unknown error"),
            );
        }
        report.add_entity(metrics);
    }

    // Link aircraft to devices by registration number (after both are loaded)
    {
        let metrics =
            device_linking::link_aircraft_to_devices_with_metrics(diesel_pool.clone()).await;
        record_stage_metrics(&metrics, "link_aircraft_to_devices");

        if !metrics.success
            && let Some(ref config) = email_config
        {
            let _ = send_failure_email(
                config,
                &metrics.name,
                metrics.error_message.as_deref().unwrap_or("Unknown error"),
            );
        }
        report.add_entity(metrics);
    }

    // Link devices to clubs from aircraft (after aircraft are linked to devices)
    {
        let metrics = device_linking::link_devices_to_clubs_with_metrics(diesel_pool.clone()).await;
        record_stage_metrics(&metrics, "link_devices_to_clubs");

        if !metrics.success
            && let Some(ref config) = email_config
        {
            let _ = send_failure_email(
                config,
                &metrics.name,
                metrics.error_message.as_deref().unwrap_or("Unknown error"),
            );
        }
        report.add_entity(metrics);
    }

    // Geocoding (if requested)
    if geocode && let Some(metrics) = geocode_soaring_clubs(diesel_pool.clone()).await {
        record_stage_metrics(&metrics, "geocoding");
        if !metrics.success
            && let Some(ref config) = email_config
        {
            let _ = send_failure_email(
                config,
                &metrics.name,
                metrics.error_message.as_deref().unwrap_or("Unknown error"),
            );
        }
        report.add_entity(metrics);
    }

    // Link home bases (if requested)
    if link_home_bases {
        let metrics = home_base_linking::link_home_bases_with_metrics(diesel_pool.clone()).await;
        record_stage_metrics(&metrics, "link_home_bases");

        if !metrics.success
            && let Some(ref config) = email_config
        {
            let _ = send_failure_email(
                config,
                &metrics.name,
                metrics.error_message.as_deref().unwrap_or("Unknown error"),
            );
        }
        report.add_entity(metrics);
    }

    // Backfill country codes for ICAO devices
    {
        let metrics =
            device_backfill::backfill_country_codes_with_metrics(diesel_pool.clone()).await;
        record_stage_metrics(&metrics, "backfill_country_codes");

        if !metrics.success
            && let Some(ref config) = email_config
        {
            let _ = send_failure_email(
                config,
                &metrics.name,
                metrics.error_message.as_deref().unwrap_or("Unknown error"),
            );
        }
        report.add_entity(metrics);
    }

    // Backfill tail numbers for US ICAO devices
    {
        let metrics =
            device_backfill::backfill_tail_numbers_with_metrics(diesel_pool.clone()).await;
        record_stage_metrics(&metrics, "backfill_tail_numbers");

        if !metrics.success
            && let Some(ref config) = email_config
        {
            let _ = send_failure_email(
                config,
                &metrics.name,
                metrics.error_message.as_deref().unwrap_or("Unknown error"),
            );
        }
        report.add_entity(metrics);
    }

    report.total_duration_secs = overall_start.elapsed().as_secs_f64();

    // Query for duplicate device addresses before sending email
    if let Ok(duplicates) = query_duplicate_devices(diesel_pool.clone()).await {
        report.duplicate_devices = duplicates;
        if !report.duplicate_devices.is_empty() {
            warn!(
                "Found {} devices with duplicate addresses",
                report.duplicate_devices.len()
            );
        }
    }

    // Record overall metrics
    metrics::histogram!("data_load.total_duration_seconds").record(report.total_duration_secs);
    metrics::gauge!("data_load.overall_success").set(if report.overall_success {
        1.0
    } else {
        0.0
    });
    metrics::gauge!("data_load.last_run_timestamp").set(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as f64,
    );

    // Send final summary email report if config is available
    if let Some(config) = email_config
        && let Err(e) = send_email_report(&config, &report)
    {
        warn!("Failed to send email report: {}", e);
    }

    // Return error if any entity failed
    if !report.overall_success {
        return Err(anyhow::anyhow!(
            "Data load completed with failures - check email report"
        ));
    }

    info!(
        "Data load completed successfully in {:.1}s",
        report.total_duration_secs
    );
    Ok(())
}

async fn geocode_soaring_clubs(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> Option<soar::email_reporter::EntityMetrics> {
    use soar::email_reporter::EntityMetrics;
    use tracing::error;

    let start = Instant::now();
    let mut metrics = EntityMetrics::new("Geocoding Soaring Clubs");

    info!("Geocoding soaring club locations...");

    let locations_repo = LocationsRepository::new(diesel_pool.clone());

    match locations_repo.get_locations_for_geocoding(Some(1000)).await {
        Ok(locations) => {
            let total_locations = locations.len();
            info!(
                "Found {} soaring club locations to geocode",
                total_locations
            );
            metrics.records_loaded = total_locations;

            let mut geocoded_count = 0;
            for location in locations {
                match geocode_components(
                    location.street1.as_deref(),
                    None, // street2
                    location.city.as_deref(),
                    location.state.as_deref(),
                    location.zip_code.as_deref(),
                    None, // country
                )
                .await
                {
                    Ok(point) => {
                        let geolocation_point = Point::new(point.latitude, point.longitude);
                        match locations_repo
                            .update_geolocation(location.id, geolocation_point)
                            .await
                        {
                            Ok(true) => {
                                geocoded_count += 1;
                                info!(
                                    "Geocoded location {} to ({}, {})",
                                    location.id, point.latitude, point.longitude
                                );
                            }
                            Ok(false) => {
                                warn!("Location {} not found for geolocation update", location.id)
                            }
                            Err(e) => {
                                warn!("Failed to update geolocation for {}: {}", location.id, e)
                            }
                        }
                    }
                    Err(e) => warn!("Geocoding failed for location {}: {}", location.id, e),
                }
            }

            info!(
                "Successfully geocoded {} out of {} locations",
                geocoded_count, total_locations
            );

            // Get total count of clubs with geocoded locations
            match get_geocoded_clubs_count(&diesel_pool).await {
                Ok(total) => {
                    metrics.records_in_db = Some(total);
                }
                Err(e) => {
                    warn!("Failed to get geocoded clubs count: {}", e);
                    metrics.records_in_db = None;
                }
            }
            metrics.success = true;
        }
        Err(e) => {
            error!("Failed to get locations for geocoding: {}", e);
            metrics.success = false;
            metrics.error_message = Some(e.to_string());
        }
    }

    metrics.duration_secs = start.elapsed().as_secs_f64();
    Some(metrics)
}

/// Get count of clubs with geocoded locations (via join with locations table)
async fn get_geocoded_clubs_count(
    diesel_pool: &Pool<ConnectionManager<PgConnection>>,
) -> Result<i64> {
    use diesel::dsl::count_star;
    use diesel::prelude::*;
    use soar::schema::{clubs, locations};

    let pool = diesel_pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        let result: i64 = clubs::table
            .inner_join(locations::table.on(clubs::location_id.eq(locations::id.nullable())))
            .filter(locations::geolocation.is_not_null())
            .select(count_star())
            .get_result(&mut conn)?;

        Ok(result)
    })
    .await?
}

/// Query for devices that have duplicate addresses (same address, different address_type)
async fn query_duplicate_devices(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> Result<Vec<soar::aircraft::AircraftModel>> {
    use diesel::prelude::*;
    use diesel::sql_types::Integer;
    use soar::schema::aircraft;

    tokio::task::spawn_blocking(move || {
        let mut conn = diesel_pool.get()?;

        // First, find all addresses that appear more than once
        #[derive(QueryableByName)]
        struct DuplicateAddressRow {
            #[diesel(sql_type = Integer)]
            address: i32,
        }

        let duplicate_addresses: Vec<i32> =
            diesel::sql_query("SELECT address FROM aircraft GROUP BY address HAVING COUNT(*) > 1")
                .load::<DuplicateAddressRow>(&mut conn)?
                .into_iter()
                .map(|row| row.address)
                .collect();

        if duplicate_addresses.is_empty() {
            return Ok(Vec::new());
        }

        // Now fetch all device rows for those duplicate addresses
        let duplicate_devices = aircraft::table
            .filter(aircraft::address.eq_any(duplicate_addresses))
            .order((aircraft::address.asc(), aircraft::address_type.asc()))
            .select(soar::aircraft::AircraftModel::as_select())
            .load(&mut conn)?;

        Ok(duplicate_devices)
    })
    .await?
}
