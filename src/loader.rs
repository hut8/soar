use anyhow::Result;
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use sqlx::PgPool;
use tracing::{error, info, warn};

use crate::aircraft_registrations::read_aircraft_file;
use crate::aircraft_registrations_repo::AircraftRegistrationsRepository;
use crate::airports::read_airports_csv_file;
use crate::airports_repo::AirportsRepository;
use crate::clubs_repo::ClubsRepository;
use crate::device_repo::DeviceRepository;
use crate::devices::DeviceFetcher;
use crate::faa::aircraft_model_repo::AircraftModelRepository;
use crate::faa::aircraft_models::read_aircraft_models_file;
use crate::geocoding::geocode_components;
use crate::locations::Point;
use crate::locations_repo::LocationsRepository;
use crate::receiver_repo::ReceiverRepository;
use crate::receivers::read_receivers_file;
use crate::runways::read_runways_csv_file;
use crate::runways_repo::RunwaysRepository;

#[allow(clippy::too_many_arguments)]
pub async fn handle_load_data(
    sqlx_pool: PgPool,
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
    aircraft_models_path: Option<String>,
    aircraft_registrations_path: Option<String>,
    airports_path: Option<String>,
    runways_path: Option<String>,
    receivers_path: Option<String>,
    pull_devices: bool,
    geocode: bool,
    link_home_bases: bool,
) -> Result<()> {
    info!(
        "Loading data - Models: {:?}, Registrations: {:?}, Airports: {:?}, Runways: {:?}, Receivers: {:?}, Pull Devices: {}, Geocode: {}",
        aircraft_models_path,
        aircraft_registrations_path,
        airports_path,
        runways_path,
        receivers_path,
        pull_devices,
        geocode
    );

    // Load aircraft models first (if provided)
    if let Some(aircraft_models_path) = aircraft_models_path {
        info!("Loading aircraft models from: {}", aircraft_models_path);
        match read_aircraft_models_file(&aircraft_models_path) {
            Ok(aircraft_models) => {
                info!(
                    "Successfully loaded {} aircraft models",
                    aircraft_models.len()
                );

                // Create aircraft model repository and upsert models
                let model_repo = AircraftModelRepository::new(sqlx_pool.clone());
                info!(
                    "Upserting {} aircraft models into database...",
                    aircraft_models.len()
                );

                match model_repo.upsert_aircraft_models(aircraft_models).await {
                    Ok(upserted_count) => {
                        info!("Successfully upserted {} aircraft models", upserted_count);

                        // Get final count from database
                        match model_repo.get_aircraft_model_count().await {
                            Ok(total_count) => {
                                info!("Total aircraft models in database: {}", total_count);
                            }
                            Err(e) => {
                                error!("Failed to get aircraft model count: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to upsert aircraft models: {}", e);
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to read aircraft models file: {}", e);
                return Err(e);
            }
        }
    } else {
        info!("Skipping aircraft models - no path provided");
    }

    // Load aircraft registrations second (if provided)
    if let Some(ref aircraft_registrations_path) = aircraft_registrations_path {
        info!(
            "Loading aircraft registrations from: {}",
            aircraft_registrations_path
        );
        match read_aircraft_file(aircraft_registrations_path) {
            Ok(aircraft_list) => {
                info!(
                    "Successfully loaded {} aircraft registrations",
                    aircraft_list.len()
                );

                // Create aircraft registrations repository and upsert registrations
                let registrations_repo = AircraftRegistrationsRepository::new(diesel_pool.clone());
                info!(
                    "Upserting {} aircraft registrations into database...",
                    aircraft_list.len()
                );

                match registrations_repo
                    .upsert_aircraft_registrations(aircraft_list)
                    .await
                {
                    Ok(upserted_count) => {
                        info!(
                            "Successfully upserted {} aircraft registrations",
                            upserted_count
                        );

                        // Get final count from database
                        match registrations_repo.get_aircraft_registration_count().await {
                            Ok(total_count) => {
                                info!("Total aircraft registrations in database: {}", total_count);
                            }
                            Err(e) => {
                                error!("Failed to get aircraft registration count: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to upsert aircraft registrations: {}", e);
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to read aircraft registrations file: {}", e);
                return Err(e);
            }
        }
    } else {
        info!("Skipping aircraft registrations - no path provided");
    }

    // Load airports third (if provided)
    if let Some(airports_path) = airports_path {
        info!("Loading airports from: {}", airports_path);
        match read_airports_csv_file(&airports_path) {
            Ok(airports_list) => {
                info!("Successfully loaded {} airports", airports_list.len());

                // Create airports repository and upsert airports
                let airports_repo = AirportsRepository::new(diesel_pool.clone());
                info!(
                    "Upserting {} airports into database...",
                    airports_list.len()
                );

                match airports_repo.upsert_airports(airports_list).await {
                    Ok(upserted_count) => {
                        info!("Successfully upserted {} airports", upserted_count);

                        // Get final count from database
                        match airports_repo.get_airport_count().await {
                            Ok(total_count) => {
                                info!("Total airports in database: {}", total_count);
                            }
                            Err(e) => {
                                error!("Failed to get airport count: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to upsert airports: {}", e);
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to read airports file: {}", e);
                return Err(e);
            }
        }
    } else {
        info!("Skipping airports - no path provided");
    }

    // Load runways fourth (if provided)
    if let Some(runways_path) = runways_path {
        info!("Loading runways from: {}", runways_path);
        match read_runways_csv_file(&runways_path) {
            Ok(runways_list) => {
                info!("Successfully loaded {} runways", runways_list.len());

                // Create runways repository and upsert runways
                let runways_repo = RunwaysRepository::new(sqlx_pool.clone());
                info!("Upserting {} runways into database...", runways_list.len());

                match runways_repo.upsert_runways(runways_list).await {
                    Ok(upserted_count) => {
                        info!("Successfully upserted {} runways", upserted_count);

                        // Get final count from database
                        match runways_repo.get_runway_count().await {
                            Ok(total_count) => {
                                info!("Total runways in database: {}", total_count);
                            }
                            Err(e) => {
                                error!("Failed to get runway count: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to upsert runways: {}", e);
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to read runways file: {}", e);
                return Err(e);
            }
        }
    } else {
        info!("Skipping runways - no path provided");
    }

    // Load receivers fifth (if provided)
    if let Some(receivers_path) = receivers_path {
        info!("Loading receivers from: {}", receivers_path);
        match read_receivers_file(&receivers_path) {
            Ok(receivers_data) => {
                let receiver_count = receivers_data
                    .receivers
                    .as_ref()
                    .map(|r| r.len())
                    .unwrap_or(0);
                info!("Successfully loaded {} receivers", receiver_count);

                // Create receivers repository and upsert receivers
                let receivers_repo = ReceiverRepository::new(sqlx_pool.clone());
                info!("Upserting {} receivers into database...", receiver_count);

                match receivers_repo
                    .upsert_receivers_from_data(receivers_data)
                    .await
                {
                    Ok(upserted_count) => {
                        info!("Successfully upserted {} receivers", upserted_count);

                        // Get final count from database
                        match receivers_repo.get_receiver_count().await {
                            Ok(total_count) => {
                                info!("Total receivers in database: {}", total_count);
                            }
                            Err(e) => {
                                error!("Failed to get receiver count: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to upsert receivers: {}", e);
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to read receivers file: {}", e);
                return Err(e);
            }
        }
    } else {
        info!("Skipping receivers - no path provided");
    }

    // Pull devices if requested
    if pull_devices {
        info!("Pulling devices from DDB...");

        // Create device fetcher and fetch devices
        let device_fetcher = DeviceFetcher::new();

        match device_fetcher.fetch_all().await {
            Ok(devices) => {
                let device_count = devices.len();
                info!("Successfully fetched {} devices from DDB", device_count);

                if device_count == 0 {
                    info!("No devices found in DDB response");
                } else {
                    // Create device repository and upsert devices
                    let device_repo = DeviceRepository::new(diesel_pool.clone());

                    info!("Upserting {} devices into database...", devices.len());
                    match device_repo.upsert_devices(devices).await {
                        Ok(upserted_count) => {
                            info!("Successfully upserted {} devices", upserted_count);

                            // Get final count from database
                            match device_repo.get_device_count().await {
                                Ok(total_count) => {
                                    info!("Total devices in database: {}", total_count);
                                }
                                Err(e) => {
                                    error!("Failed to get device count: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to upsert devices: {}", e);
                            return Err(e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to fetch devices from DDB: {}", e);
                return Err(anyhow::anyhow!("Failed to fetch devices from DDB: {}", e));
            }
        }
    } else {
        info!("Skipping device pull - not requested");
    }

    // Geocode locations if requested
    if geocode {
        info!("Starting geocoding of locations...");

        // Create locations repository
        let locations_repo = LocationsRepository::new(sqlx_pool.clone());

        // Get locations that need geocoding
        match locations_repo.get_locations_for_geocoding(Some(1000)).await {
            Ok(locations) => {
                let location_count = locations.len();

                if location_count == 0 {
                    info!("No locations need geocoding");
                } else {
                    info!("Found {} locations that need geocoding", location_count);

                    let mut successful_geocodes = 0;
                    let mut failed_geocodes = 0;

                    for location in locations {
                        // Check if street1 is a PO Box and skip it for initial geocoding attempt
                        let (use_street1, use_street2) =
                            if let Some(street1_val) = &location.street1 {
                                let street1_upper = street1_val.to_uppercase();
                                if street1_upper.contains("PO BOX")
                                    || street1_upper.contains("P.O. BOX")
                                    || street1_upper.starts_with("BOX ")
                                {
                                    // Skip PO Box addresses, use only city/state/zip/country
                                    (None, location.street2.clone())
                                } else {
                                    (location.street1.clone(), location.street2.clone())
                                }
                            } else {
                                (location.street1.clone(), location.street2.clone())
                            };

                        info!(
                            "Geocoding location {} - {}, {}, {}, {}",
                            location.id,
                            use_street1.as_deref().unwrap_or(""),
                            location.city.as_deref().unwrap_or(""),
                            location.state.as_deref().unwrap_or(""),
                            location.country_mail_code.as_deref().unwrap_or("US")
                        );

                        // First attempt: Try full address (excluding PO boxes)
                        let country_name = match location.country_mail_code.as_deref() {
                            Some("US") | None => Some("United States".to_string()),
                            Some("CA") => Some("Canada".to_string()),
                            Some("MX") => Some("Mexico".to_string()),
                            Some("GB") => Some("United Kingdom".to_string()),
                            Some(code) => Some(code.to_string()),
                        };

                        // Normalize zip code to 5 digits if available
                        let zip_5_digits = location
                            .zip_code
                            .as_ref()
                            .map(|zip| if zip.len() >= 5 { &zip[..5] } else { zip });

                        let result = geocode_components(
                            use_street1.as_deref(),
                            use_street2.as_deref(),
                            location.city.as_deref(),
                            location.state.as_deref(),
                            zip_5_digits,
                            country_name.as_deref(),
                        )
                        .await;

                        let final_result = match result {
                            Ok(point) => Ok(point),
                            Err(_) => {
                                // Fallback: Try with just city, state, zip, country
                                info!(
                                    "Full address failed for location {}, trying city/state/zip fallback",
                                    location.id
                                );
                                geocode_components(
                                    None, // No street
                                    None, // No street2
                                    location.city.as_deref(),
                                    location.state.as_deref(),
                                    zip_5_digits,
                                    country_name.as_deref(),
                                )
                                .await
                            }
                        };

                        match final_result {
                            Ok(point) => {
                                // Update the location in the database with the geocoded coordinates
                                let geolocation_point = Point::new(point.latitude, point.longitude);
                                match locations_repo
                                    .update_geolocation(location.id, geolocation_point)
                                    .await
                                {
                                    Ok(updated) => {
                                        if updated {
                                            info!(
                                                "Successfully geocoded location {} to ({}, {})",
                                                location.id, point.latitude, point.longitude
                                            );
                                            successful_geocodes += 1;
                                        } else {
                                            warn!(
                                                "Location {} was not found for update",
                                                location.id
                                            );
                                            failed_geocodes += 1;
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to update location {}: {}", location.id, e);
                                        failed_geocodes += 1;
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to geocode location {} (city: {}, state: {}): {}",
                                    location.id,
                                    location.city.as_deref().unwrap_or("?"),
                                    location.state.as_deref().unwrap_or("?"),
                                    e
                                );
                                failed_geocodes += 1;
                            }
                        }

                        // Rate limiting: wait 1.1 seconds between requests to be respectful to Nominatim
                        tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;
                    }

                    info!(
                        "Geocoding completed: {} successful, {} failed",
                        successful_geocodes, failed_geocodes
                    );
                }
            }
            Err(e) => {
                error!("Failed to get locations for geocoding: {}", e);
                return Err(e);
            }
        }
    } else {
        info!("Skipping geocoding - not requested");
    }

    // Link home bases if requested
    if link_home_bases {
        info!("Starting home base linking for soaring clubs...");

        // Create repositories
        let clubs_repo = ClubsRepository::new(sqlx_pool.clone());
        let airports_repo = AirportsRepository::new(diesel_pool.clone());

        // Get soaring clubs without home base airport IDs
        match clubs_repo.get_soaring_clubs_without_home_base().await {
            Ok(clubs) => {
                let club_count = clubs.len();

                if club_count == 0 {
                    info!("No soaring clubs need home base linking");
                } else {
                    info!(
                        "Found {} soaring clubs that need home base linking",
                        club_count
                    );

                    let mut linked_count = 0;
                    let mut failed_count = 0;
                    let max_distance_miles = 10.0;
                    let max_distance_meters = max_distance_miles * 1609.34; // Convert miles to meters
                    let allowed_types = ["large_airport", "medium_airport", "small_airport"];

                    for club in clubs {
                        if let Some(location) = club.base_location {
                            info!(
                                "Processing club: {} at ({}, {})",
                                club.name, location.latitude, location.longitude
                            );

                            // Find nearest airports within 10 miles
                            match airports_repo
                                .find_nearest_airports(
                                    location.latitude,
                                    location.longitude,
                                    max_distance_meters,
                                    50, // limit to 50 results to check
                                )
                                .await
                            {
                                Ok(nearby_airports) => {
                                    // Filter by allowed airport types
                                    let suitable_airports: Vec<_> = nearby_airports
                                        .into_iter()
                                        .filter(|(airport, _distance)| {
                                            allowed_types.contains(&airport.airport_type.as_str())
                                        })
                                        .collect();

                                    if let Some((nearest_airport, distance)) =
                                        suitable_airports.first()
                                    {
                                        info!(
                                            "Found suitable airport: {} ({}) at {:.2} miles from {}",
                                            nearest_airport.name,
                                            nearest_airport.ident,
                                            distance / 1609.34, // Convert meters to miles
                                            club.name
                                        );

                                        // Update the club's home base airport ID
                                        match clubs_repo
                                            .update_home_base_airport(club.id, nearest_airport.id)
                                            .await
                                        {
                                            Ok(updated) => {
                                                if updated {
                                                    info!(
                                                        "Successfully linked {} to airport {} ({})",
                                                        club.name,
                                                        nearest_airport.name,
                                                        nearest_airport.ident
                                                    );
                                                    linked_count += 1;
                                                } else {
                                                    warn!(
                                                        "Failed to update club {} - not found",
                                                        club.name
                                                    );
                                                    failed_count += 1;
                                                }
                                            }
                                            Err(e) => {
                                                error!(
                                                    "Failed to update club {} home base: {}",
                                                    club.name, e
                                                );
                                                failed_count += 1;
                                            }
                                        }
                                    } else {
                                        info!(
                                            "No suitable airports found within {} miles of {}",
                                            max_distance_miles, club.name
                                        );
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to find nearest airports for club {}: {}",
                                        club.name, e
                                    );
                                    failed_count += 1;
                                }
                            }
                        } else {
                            warn!("Club {} has no geolocation data, skipping", club.name);
                            failed_count += 1;
                        }
                    }

                    info!(
                        "Home base linking completed: {} successfully linked, {} failed",
                        linked_count, failed_count
                    );
                }
            }
            Err(e) => {
                error!("Failed to get clubs for home base linking: {}", e);
                return Err(e);
            }
        }
    } else {
        info!("Skipping home base linking - not requested");
    }

    // Link aircraft to devices if either devices were pulled or aircraft were loaded
    if pull_devices || aircraft_registrations_path.is_some() {
        info!("Linking aircraft to devices based on registration numbers...");

        match link_aircraft_to_devices(&sqlx_pool).await {
            Ok(linked_count) => {
                info!("Successfully linked {} aircraft to devices", linked_count);
            }
            Err(e) => {
                error!("Failed to link aircraft to devices: {}", e);
                // Don't return error - this is not critical for data loading
            }
        }
    } else {
        info!("Skipping aircraft-device linking - no devices pulled and no aircraft loaded");
    }

    Ok(())
}

/// Link aircraft registrations to devices based on matching registration numbers
/// Only updates aircraft that don't already have a device_id set
async fn link_aircraft_to_devices(pool: &PgPool) -> Result<u32> {
    info!("Starting aircraft-device linking process...");

    // Query to find aircraft without device_id that have matching devices
    let result = sqlx::query!(
        r#"
        UPDATE aircraft_registrations
        SET device_id = devices.device_id
        FROM devices
        WHERE aircraft_registrations.registration_number = devices.registration
        AND aircraft_registrations.device_id IS NULL
        "#
    )
    .execute(pool)
    .await?;

    let linked_count = result.rows_affected() as u32;
    info!("Linked {} aircraft to devices", linked_count);

    Ok(linked_count)
}
