use anyhow::Result;
use clap::{Parser, Subcommand};
use sqlx::postgres::PgPool;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::{error, info, warn};

use soar::airports::read_airports_csv_file;
use soar::airports_repo::AirportsRepository;
use soar::aprs_client::{AprsClient, AprsClientConfigBuilder, MessageProcessor};
use soar::geocoding::geocode_components;
use soar::device_repo::DeviceRepository;
use soar::devices::DeviceFetcher;
use soar::faa::aircraft_model_repo::AircraftModelRepository;
use soar::faa::aircraft_models::read_aircraft_models_file;
use soar::faa::aircraft_registrations::read_aircraft_file;
use soar::faa::aircraft_registrations_repo::AircraftRegistrationsRepository;
use soar::receiver_repo::ReceiverRepository;
use soar::receivers::read_receivers_file;
use soar::runways::read_runways_csv_file;
use soar::runways_repo::RunwaysRepository;

// Embed migrations into the binary
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

#[derive(Parser)]
#[command(name = "soar")]
#[command(about = "SOAR - Soaring Observation And Records")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Load aircraft model and registration data, receivers, and optionally pull devices from DDB
    ///
    /// Aircraft registrations and models should come from the "releasable aircraft" FAA database.
    /// Airports and runways should come from "our airports" database.
    /// Receivers JSON file can be created from https://github.com/hut8/ogn-rdb
    LoadData {
        /// Path to the aircraft model data file (from ACFTREF.txt in the "releasable aircraft" FAA database
        /// https://www.faa.gov/licenses_certificates/aircraft_certification/aircraft_registry/releasable_aircraft_download)
        #[arg(long)]
        aircraft_models: Option<String>,
        /// Path to the aircraft registrations data file (from MASTER.txt in the "releasable aircraft" FAA database)
        /// https://www.faa.gov/licenses_certificates/aircraft_certification/aircraft_registry/releasable_aircraft_download
        #[arg(long)]
        aircraft_registrations: Option<String>,
        /// Path to the airports CSV file (from "our airports" database)
        /// https://davidmegginson.github.io/ourairports-data/airports.csv
        #[arg(long)]
        airports: Option<String>,
        /// Path to the runways CSV file (from "our airports" database)
        /// https://davidmegginson.github.io/ourairports-data/runways.csv
        #[arg(long)]
        runways: Option<String>,
        /// Path to the receivers JSON file (can be created from https://github.com/hut8/ogn-rdb)
        #[arg(long)]
        receivers: Option<String>,
        /// Also pull devices from DDB (Device Database) and upsert them into the database
        #[arg(long)]
        pull_devices: bool,
        /// Geocode registered addresses of aircraft belonging to clubs that haven't been geocoded yet
        #[arg(long)]
        geocode: bool,
    },
    /// Run the main APRS client
    Run {
        /// APRS server hostname
        #[arg(long, default_value = "aprs.glidernet.org")]
        server: String,

        /// APRS server port (automatically switches to 10152 for full feed if no filter specified)
        #[arg(long, default_value = "14580")]
        port: u16,

        /// Callsign for APRS authentication
        #[arg(long, default_value = "N0CALL")]
        callsign: String,

        /// APRS filter string (omit for full global feed via port 10152, or specify filter for port 14580)
        #[arg(long)]
        filter: Option<String>,

        /// Maximum number of connection retry attempts
        #[arg(long, default_value = "5")]
        max_retries: u32,

        /// Delay between reconnection attempts in seconds
        #[arg(long, default_value = "5")]
        retry_delay: u64,

        /// Base directory for message archive (optional)
        #[arg(long)]
        archive_dir: Option<String>,

        /// Enable automatic archiving (uses /var/soar/archive if writable, otherwise $HOME/soar-archive/)
        #[arg(long)]
        archive: bool,

        /// NATS server URL for pub/sub
        #[arg(long, default_value = "nats://localhost:4222")]
        nats_url: String,
    },
    /// Start the web server
    Web {
        /// Port to bind the web server to
        #[arg(long, default_value = "1337")]
        port: u16,

        /// Interface to bind the web server to
        #[arg(long, default_value = "localhost")]
        interface: String,
    },
}

async fn setup_database() -> Result<PgPool> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Get the database URL from environment variables
    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL must be set in environment variables");

    // Create a connection pool to the PostgreSQL database
    let pool = match PgPool::connect(&database_url).await {
        Ok(pool) => {
            info!("Successfully connected to PostgreSQL database");
            pool
        }
        Err(e) => {
            error!("Failed to connect to PostgreSQL database: {e}");
            std::process::exit(1);
        }
    };

    // Run pending migrations
    info!("Running database migrations...");
    match MIGRATOR.run(&pool).await {
        Ok(_) => {
            info!("Database migrations completed successfully");
        }
        Err(e) => {
            error!("Failed to run database migrations: {e}");
            std::process::exit(1);
        }
    }

    Ok(pool)
}

/// Determine the best archive directory to use
fn determine_archive_dir() -> Result<String> {
    let var_soar_archive = "/var/soar/archive";

    // Check if /var/soar/archive exists and is writable
    if Path::new(var_soar_archive).exists() {
        // Test if we can write to it by trying to create a temporary file
        let test_file = format!("{}/test_write_{}", var_soar_archive, std::process::id());
        match fs::write(&test_file, b"test") {
            Ok(_) => {
                // Clean up test file
                let _ = fs::remove_file(&test_file);
                info!("Using archive directory: {}", var_soar_archive);
                return Ok(var_soar_archive.to_string());
            }
            Err(_) => {
                info!(
                    "/var/soar/archive exists but is not writable, falling back to $HOME/soar-archive/"
                );
            }
        }
    }

    // Fallback to $HOME/soar-archive/
    let home_dir =
        env::var("HOME").map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
    let home_archive = format!("{}/soar-archive", home_dir);

    // Create the directory if it doesn't exist
    fs::create_dir_all(&home_archive).map_err(|e| {
        anyhow::anyhow!("Failed to create archive directory {}: {}", home_archive, e)
    })?;

    info!("Using archive directory: {}", home_archive);
    Ok(home_archive)
}





async fn handle_load_data(
    aircraft_models_path: Option<String>,
    aircraft_registrations_path: Option<String>,
    airports_path: Option<String>,
    runways_path: Option<String>,
    receivers_path: Option<String>,
    pull_devices: bool,
    geocode: bool,
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

    // Set up database connection
    let pool = setup_database().await?;

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
                let model_repo = AircraftModelRepository::new(pool.clone());
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
    if let Some(aircraft_registrations_path) = aircraft_registrations_path {
        info!(
            "Loading aircraft registrations from: {}",
            aircraft_registrations_path
        );
        match read_aircraft_file(&aircraft_registrations_path) {
            Ok(aircraft_list) => {
                info!(
                    "Successfully loaded {} aircraft registrations",
                    aircraft_list.len()
                );

                // Create aircraft registrations repository and upsert registrations
                let registrations_repo = AircraftRegistrationsRepository::new(pool.clone());
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
                let airports_repo = AirportsRepository::new(pool.clone());
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
                let runways_repo = RunwaysRepository::new(pool.clone());
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
                let receivers_repo = ReceiverRepository::new(pool.clone());
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
                    let device_repo = DeviceRepository::new(pool.clone());

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

    // Geocode aircraft addresses if requested
    if geocode {
        info!("Starting geocoding of aircraft registrations...");

        // Create aircraft registrations repository
        let aircraft_repo = AircraftRegistrationsRepository::new(pool.clone());

        // Get aircraft that need geocoding
        match aircraft_repo.get_aircraft_for_geocoding().await {
            Ok(aircraft_components) => {
                let aircraft_count = aircraft_components.len();
                
                if aircraft_count == 0 {
                    info!("No aircraft registrations need geocoding");
                } else {
                    info!("Found {} aircraft registrations that need geocoding", aircraft_count);
                    
                    let mut successful_geocodes = 0;
                    let mut failed_geocodes = 0;
                    
                    for (registration_number, street1, street2, city, state, zip_code, country) in aircraft_components {
                        // Check if street1 is a PO Box and skip it for initial geocoding attempt
                        let (use_street1, use_street2) = if let Some(street1_val) = &street1 {
                            let street1_upper = street1_val.to_uppercase();
                            if street1_upper.contains("PO BOX") || street1_upper.contains("P.O. BOX") || street1_upper.starts_with("BOX ") {
                                // Skip PO Box addresses, use only city/state/zip/country
                                (None, street2.clone()) 
                            } else {
                                (street1.clone(), street2.clone())
                            }
                        } else {
                            (street1.clone(), street2.clone())
                        };

                        info!("Geocoding {} - {}, {}, {}, {}", 
                            registration_number,
                            use_street1.as_deref().unwrap_or(""),
                            city.as_deref().unwrap_or(""),
                            state.as_deref().unwrap_or(""),
                            country.as_deref().unwrap_or("")
                        );
                        
                        // First attempt: Try full address (excluding PO boxes)
                        let result = geocode_components(
                            use_street1.as_deref(),
                            use_street2.as_deref(), 
                            city.as_deref(),
                            state.as_deref(),
                            zip_code.as_deref(),
                            country.as_deref()
                        ).await;

                        let final_result = match result {
                            Ok(point) => Ok(point),
                            Err(_) => {
                                // Fallback: Try with just city, state, zip, country
                                info!("Full address failed for {}, trying city/state/zip fallback", registration_number);
                                geocode_components(
                                    None, // No street
                                    None, // No street2
                                    city.as_deref(),
                                    state.as_deref(),
                                    zip_code.as_deref(),
                                    country.as_deref()
                                ).await
                            }
                        };

                        match final_result {
                            Ok(point) => {
                                // Update the database with the geocoded location
                                match aircraft_repo.update_registered_location(&registration_number, point.latitude, point.longitude).await {
                                    Ok(_) => {
                                        info!("Successfully geocoded {} to ({}, {})", registration_number, point.latitude, point.longitude);
                                        successful_geocodes += 1;
                                    }
                                    Err(e) => {
                                        error!("Failed to update location for {}: {}", registration_number, e);
                                        failed_geocodes += 1;
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to geocode {} (city: {}, state: {}): {}", 
                                    registration_number, 
                                    city.as_deref().unwrap_or("?"), 
                                    state.as_deref().unwrap_or("?"), 
                                    e
                                );
                                failed_geocodes += 1;
                            }
                        }

                        // Rate limiting: wait 1.1 seconds between requests to be respectful to Nominatim
                        tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;
                    }
                    
                    info!("Geocoding completed: {} successful, {} failed", successful_geocodes, failed_geocodes);
                }
            }
            Err(e) => {
                error!("Failed to get aircraft for geocoding: {}", e);
                return Err(e);
            }
        }
    } else {
        info!("Skipping geocoding - not requested");
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_run(
    server: String,
    port: u16,
    callsign: String,
    filter: Option<String>,
    max_retries: u32,
    retry_delay: u64,
    archive_dir: Option<String>,
    nats_url: String,
) -> Result<()> {
    info!("Starting APRS client with server: {}:{}", server, port);

    // Set up database connection
    let _pool = setup_database().await?;

    // Use port 10152 (full feed) if no filter is specified, otherwise use specified port
    let actual_port = if filter.is_none() {
        info!("No filter specified, using full feed port 10152");
        10152
    } else {
        port
    };

    // Create APRS client configuration
    let config = AprsClientConfigBuilder::new()
        .server(server)
        .port(actual_port)
        .callsign(callsign)
        .filter(filter)
        .max_retries(max_retries)
        .retry_delay_seconds(retry_delay)
        .build();

    // Create composite processor (NATS + file streaming)
    info!(
        "Setting up message processors - writing to directory: {:?}, NATS URL: {}",
        archive_dir, nats_url
    );
    let archive_processor: Arc<dyn MessageProcessor> = Arc::new(
        soar::message_processors::ArchiveMessageProcessor::new(archive_dir),
    );

    // Create and start APRS client
    let mut client = AprsClient::new(config, archive_processor);

    info!("Starting APRS client...");
    client.start().await?;

    // Keep the main thread alive
    // In a real application, you might want to handle shutdown signals here
    info!("APRS client started. Press Ctrl+C to stop.");

    // Wait indefinitely (in practice, you'd handle shutdown signals)
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with TRACE level by default, but silence async_nats TRACE/DEBUG
    use tracing_subscriber::{EnvFilter, FmtSubscriber};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        // Default filter: TRACE for soar, INFO for async_nats, WARN for everything else
        EnvFilter::new("warn,soar=trace,async_nats=info,soar::nats_publisher=warn")
    });

    FmtSubscriber::builder().with_env_filter(filter).init();

    let cli = Cli::parse();

    match cli.command {
        Commands::LoadData {
            aircraft_models,
            aircraft_registrations,
            airports,
            runways,
            receivers,
            pull_devices,
            geocode,
        } => {
            handle_load_data(
                aircraft_models,
                aircraft_registrations,
                airports,
                runways,
                receivers,
                pull_devices,
                geocode,
            )
            .await
        }
        Commands::Run {
            server,
            port,
            callsign,
            filter,
            max_retries,
            retry_delay,
            archive_dir,
            archive,
            nats_url,
        } => {
            // Determine archive directory if --archive flag is used
            let final_archive_dir = if archive {
                Some(determine_archive_dir()?)
            } else {
                archive_dir
            };

            handle_run(
                server,
                port,
                callsign,
                filter,
                max_retries,
                retry_delay,
                final_archive_dir,
                nats_url,
            )
            .await
        }
        Commands::Web { interface, port } => {
            let pool = setup_database().await?;
            soar::web::start_web_server(interface, port, pool).await
        },
    }
}
