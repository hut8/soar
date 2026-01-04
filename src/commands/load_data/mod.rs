mod adsb_exchange;
mod aircraft_backfill;
mod aircraft_home_base;
mod aircraft_models;
mod aircraft_registrations;
mod aircraft_types;
mod airports_runways;
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

use soar::airports_repo::AirportsRepository;
use soar::email_reporter::{
    DataLoadReport, EmailConfig, EntityMetrics, send_email_report, send_failure_email,
};
use soar::geocoding::{GeocodingService, geocode_components};
use soar::locations::Point;
use soar::locations_repo::LocationsRepository;

/// Helper function to record Prometheus metrics for a data load stage
fn record_stage_metrics(metrics: &EntityMetrics, stage_name: &str) {
    let stage_name = stage_name.to_string();
    metrics::histogram!("data_load.stage.duration_seconds", "stage" => stage_name.clone())
        .record(metrics.duration_secs);
    metrics::counter!("data_load.stage.records_loaded_total", "stage" => stage_name.clone())
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
    adsb_exchange_path: Option<String>,
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

    // Load aircraft types reference data (ICAO/IATA type codes) from embedded data
    {
        let metrics = aircraft_types::load_aircraft_types_with_metrics(diesel_pool.clone()).await;
        record_stage_metrics(&metrics, "aircraft_types");
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

    // Load ADS-B Exchange data to enhance aircraft records with ICAO type codes and owner/operator info
    if let Some(metrics) =
        adsb_exchange::load_adsb_exchange_with_metrics(diesel_pool.clone(), adsb_exchange_path)
            .await
    {
        record_stage_metrics(&metrics, "adsb_exchange");
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

    // Copy owner data from aircraft_registrations to aircraft (after ADS-B Exchange data loading)
    {
        let metrics =
            aircraft_registrations::copy_owners_to_aircraft_with_metrics(diesel_pool.clone()).await;
        record_stage_metrics(&metrics, "copy_owners_to_aircraft");

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
    if geocode {
        // Geocode aircraft registration addresses
        if let Some(metrics) = geocode_aircraft_registration_locations(diesel_pool.clone()).await {
            record_stage_metrics(&metrics, "geocoding_aircraft_registrations");
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

        // Geocode soaring clubs
        if let Some(metrics) = geocode_soaring_clubs(diesel_pool.clone()).await {
            record_stage_metrics(&metrics, "geocoding_clubs");
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

        // Geocode airports
        if let Some(metrics) = geocode_airports(diesel_pool.clone()).await {
            record_stage_metrics(&metrics, "geocoding_airports");
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

        // Calculate aircraft home bases (copy from clubs + calculate from flight stats)
        let metrics =
            aircraft_home_base::calculate_aircraft_home_bases_with_metrics(diesel_pool.clone())
                .await;
        record_stage_metrics(&metrics, "calculate_aircraft_home_bases");

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
            aircraft_backfill::backfill_country_codes_with_metrics(diesel_pool.clone()).await;
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
            aircraft_backfill::backfill_tail_numbers_with_metrics(diesel_pool.clone()).await;
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

async fn geocode_aircraft_registration_locations(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> Option<soar::email_reporter::EntityMetrics> {
    use diesel::prelude::*;
    use soar::email_reporter::EntityMetrics;
    use tracing::error;

    const BATCH_SIZE: usize = 1000;
    const MAX_TOTAL_GEOCODE: usize = 10_000;
    const MAX_GOOGLE_MAPS: usize = 100;

    let start = Instant::now();
    let mut metrics = EntityMetrics::new("Geocoding Aircraft Registration Addresses");

    info!(
        "Geocoding aircraft registration location addresses (max {}, Google Maps limit: {})...",
        MAX_TOTAL_GEOCODE, MAX_GOOGLE_MAPS
    );

    // Query locations that don't have geolocation and are linked to aircraft registrations
    let mut all_locations = match tokio::task::spawn_blocking({
        let pool = diesel_pool.clone();
        move || {
            use soar::schema::{aircraft_registrations, locations};
            let mut conn = pool.get()?;

            let results = locations::table
                .inner_join(
                    aircraft_registrations::table
                        .on(aircraft_registrations::location_id.eq(locations::id.nullable())),
                )
                .filter(locations::geolocation.is_null())
                .filter(
                    locations::street1
                        .is_not_null()
                        .or(locations::city.is_not_null())
                        .or(locations::state.is_not_null()),
                )
                .select(soar::locations::LocationModel::as_select())
                .distinct_on(locations::id)
                .limit(MAX_TOTAL_GEOCODE as i64)
                .load(&mut conn)?;

            Ok::<Vec<soar::locations::LocationModel>, anyhow::Error>(results)
        }
    })
    .await
    {
        Ok(Ok(locations)) => locations,
        Ok(Err(e)) => {
            error!("Failed to query aircraft registration locations: {}", e);
            metrics.success = false;
            metrics.error_message = Some(e.to_string());
            metrics.duration_secs = start.elapsed().as_secs_f64();
            return Some(metrics);
        }
        Err(e) => {
            error!(
                "Task join error querying aircraft registration locations: {}",
                e
            );
            metrics.success = false;
            metrics.error_message = Some(e.to_string());
            metrics.duration_secs = start.elapsed().as_secs_f64();
            return Some(metrics);
        }
    };

    let total_locations = all_locations.len();
    info!(
        "Found {} aircraft registration locations to geocode (limited to {})",
        total_locations, MAX_TOTAL_GEOCODE
    );
    metrics.records_loaded = total_locations;

    if total_locations == 0 {
        metrics.success = true;
        metrics.duration_secs = start.elapsed().as_secs_f64();
        return Some(metrics);
    }

    let locations_repo = LocationsRepository::new(diesel_pool.clone());
    let mut geocoded_count = 0;
    let mut google_maps_count = 0;
    let mut batch_number = 0;

    // Process in batches
    while !all_locations.is_empty() {
        batch_number += 1;
        let batch_size = BATCH_SIZE.min(all_locations.len());
        let batch: Vec<_> = all_locations.drain(..batch_size).collect();

        info!(
            "Processing batch {} ({} locations, Google Maps used: {}/{})",
            batch_number,
            batch.len(),
            google_maps_count,
            MAX_GOOGLE_MAPS
        );

        for location_model in batch {
            let location: soar::locations::Location = location_model.into();

            // Check if we've hit Google Maps limit - stop processing entirely if so
            if google_maps_count >= MAX_GOOGLE_MAPS {
                warn!(
                    "Reached Google Maps limit ({}/{}), stopping geocoding for this run. {} locations remaining.",
                    google_maps_count,
                    MAX_GOOGLE_MAPS,
                    all_locations.len() + 1
                );
                break;
            }

            // geocode_components will try Nominatim → Google Maps (Photon is disabled)
            let geocode_result = geocode_components(
                location.street1.as_deref(),
                None, // street2
                location.city.as_deref(),
                location.state.as_deref(),
                location.zip_code.as_deref(),
                location.country_code.as_deref(),
            )
            .await;

            match geocode_result {
                Ok(result) => {
                    // Track which service was used
                    if result.service == GeocodingService::GoogleMaps {
                        google_maps_count += 1;
                        info!(
                            "Used Google Maps for location {} ({}/{})",
                            location.id, google_maps_count, MAX_GOOGLE_MAPS
                        );
                    }

                    let geolocation_point =
                        Point::new(result.point.latitude, result.point.longitude);
                    match locations_repo
                        .update_geolocation(location.id, geolocation_point)
                        .await
                    {
                        Ok(true) => {
                            geocoded_count += 1;
                            info!(
                                "Geocoded aircraft registration location {} to ({}, {}) via {:?}",
                                location.id,
                                result.point.latitude,
                                result.point.longitude,
                                result.service
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

        // Break out of batch processing if we hit Google Maps limit
        if google_maps_count >= MAX_GOOGLE_MAPS {
            break;
        }

        info!(
            "Completed batch {} - geocoded {} so far (Google Maps: {}/{})",
            batch_number, geocoded_count, google_maps_count, MAX_GOOGLE_MAPS
        );
    }

    info!(
        "Successfully geocoded {} out of {} aircraft registration locations",
        geocoded_count, total_locations
    );

    metrics.success = true;
    metrics.duration_secs = start.elapsed().as_secs_f64();
    Some(metrics)
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

    match locations_repo.get_locations_for_geocoding(None).await {
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
                    Ok(result) => {
                        let geolocation_point =
                            Point::new(result.point.latitude, result.point.longitude);
                        match locations_repo
                            .update_geolocation(location.id, geolocation_point)
                            .await
                        {
                            Ok(true) => {
                                geocoded_count += 1;
                                info!(
                                    "Geocoded soaring club location {} to ({}, {}) via {:?}",
                                    location.id,
                                    result.point.latitude,
                                    result.point.longitude,
                                    result.service
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

/// Geocode airports using reverse geocoding (coordinates -> address)
/// Creates location records and links them to airports
async fn geocode_airports(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> Option<soar::email_reporter::EntityMetrics> {
    use bigdecimal::ToPrimitive;
    use soar::email_reporter::EntityMetrics;
    use soar::geocoding::Geocoder;
    use soar::locations::Location;
    use tracing::error;

    let start = Instant::now();
    let mut metrics = EntityMetrics::new("Geocoding Airports");

    info!("Reverse geocoding airport locations...");

    let airports_repo = AirportsRepository::new(diesel_pool.clone());
    let locations_repo = LocationsRepository::new(diesel_pool.clone());

    // Get airports that don't have a location_id yet and have coordinates
    match get_airports_for_geocoding(&diesel_pool).await {
        Ok(airports) => {
            let total_airports = airports.len();
            info!("Found {} airports to reverse geocode", total_airports);
            metrics.records_loaded = total_airports;

            let geocoder = Geocoder::new_batch_geocoding();
            let mut geocoded_count = 0;

            for airport in airports {
                // Convert BigDecimal to f64
                let latitude = match airport.latitude_deg.as_ref().and_then(|lat| lat.to_f64()) {
                    Some(lat) => lat,
                    None => {
                        warn!("Airport {} has invalid latitude, skipping", airport.id);
                        continue;
                    }
                };

                let longitude = match airport.longitude_deg.as_ref().and_then(|lon| lon.to_f64()) {
                    Some(lon) => lon,
                    None => {
                        warn!("Airport {} has invalid longitude, skipping", airport.id);
                        continue;
                    }
                };

                // Reverse geocode the airport coordinates using Nominatim with Google Maps fallback
                match geocoder.reverse_geocode(latitude, longitude).await {
                    Ok(result) => {
                        // Create a location with the reverse geocoded address
                        let location = Location::new(
                            result.street1,
                            None, // street2
                            result.city,
                            result.state,
                            result.zip_code,
                            result.country.map(|c| c.chars().take(2).collect()), // country code
                            Some(soar::locations::Point::new(latitude, longitude)),
                        );

                        // Use find_or_create to avoid duplicate locations
                        let params = soar::locations_repo::LocationParams {
                            street1: location.street1.clone(),
                            street2: location.street2.clone(),
                            city: location.city.clone(),
                            state: location.state.clone(),
                            zip_code: location.zip_code.clone(),
                            country_code: location.country_code.clone(),
                            geolocation: location.geolocation,
                        };
                        match locations_repo.find_or_create(params).await {
                            Ok(created_location) => {
                                // Link the airport to the location
                                match airports_repo
                                    .update_location_id(airport.id, created_location.id)
                                    .await
                                {
                                    Ok(true) => {
                                        geocoded_count += 1;
                                        info!(
                                            "Reverse geocoded airport {} ({}) to location {}: {}",
                                            airport.id,
                                            airport.ident,
                                            created_location.id,
                                            result.display_name
                                        );
                                    }
                                    Ok(false) => {
                                        warn!(
                                            "Airport {} not found for location_id update",
                                            airport.id
                                        )
                                    }
                                    Err(e) => {
                                        warn!(
                                            "Failed to update location_id for airport {}: {}",
                                            airport.id, e
                                        )
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to create location for airport {}: {}",
                                    airport.id, e
                                )
                            }
                        }
                    }
                    Err(e) => warn!(
                        "Reverse geocoding failed for airport {} ({}, {}): {}",
                        airport.id, latitude, longitude, e
                    ),
                }
            }

            info!(
                "Successfully reverse geocoded {} out of {} airports",
                geocoded_count, total_airports
            );

            // Get total count of airports with geocoded locations
            match get_geocoded_airports_count(&diesel_pool).await {
                Ok(total) => {
                    metrics.records_in_db = Some(total);
                }
                Err(e) => {
                    warn!("Failed to get geocoded airports count: {}", e);
                    metrics.records_in_db = None;
                }
            }
            metrics.success = true;
        }
        Err(e) => {
            error!("Failed to get airports for geocoding: {}", e);
            metrics.success = false;
            metrics.error_message = Some(e.to_string());
        }
    }

    metrics.duration_secs = start.elapsed().as_secs_f64();
    Some(metrics)
}

/// Get airports that need geocoding (have coordinates but no location_id)
async fn get_airports_for_geocoding(
    diesel_pool: &Pool<ConnectionManager<PgConnection>>,
) -> Result<Vec<soar::airports::Airport>> {
    use diesel::prelude::*;
    use soar::schema::airports;

    let pool = diesel_pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        let results: Vec<soar::airports::AirportModel> = airports::table
            .filter(airports::location_id.is_null())
            .filter(airports::latitude_deg.is_not_null())
            .filter(airports::longitude_deg.is_not_null())
            .limit(2500) // Process in batches to avoid overwhelming the geocoding service
            .select(soar::airports::AirportModel::as_select())
            .load::<soar::airports::AirportModel>(&mut conn)?;

        Ok::<Vec<soar::airports::Airport>, anyhow::Error>(
            results.into_iter().map(|model| model.into()).collect(),
        )
    })
    .await?
}

/// Get count of airports with geocoded locations
async fn get_geocoded_airports_count(
    diesel_pool: &Pool<ConnectionManager<PgConnection>>,
) -> Result<i64> {
    use diesel::dsl::count_star;
    use diesel::prelude::*;
    use soar::schema::airports;

    let pool = diesel_pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        let result: i64 = airports::table
            .filter(airports::location_id.is_not_null())
            .select(count_star())
            .get_result(&mut conn)?;

        Ok(result)
    })
    .await?
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
