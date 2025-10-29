mod aircraft_models;
mod aircraft_registrations;
mod airports_runways;
mod device_linking;
mod devices_receivers;
mod home_base_linking;

use anyhow::Result;
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use std::time::Instant;
use tracing::{info, warn};

use crate::email_reporter::{DataLoadReport, EmailConfig, send_email_report, send_failure_email};
use crate::geocoding::geocode_components;
use crate::locations::Point;
use crate::locations_repo::LocationsRepository;

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
        devices_receivers::load_devices_with_metrics(diesel_pool.clone(), devices_path).await
    {
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
) -> Option<crate::email_reporter::EntityMetrics> {
    use crate::email_reporter::EntityMetrics;
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
    use crate::schema::{clubs, locations};
    use diesel::dsl::count_star;
    use diesel::prelude::*;

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
