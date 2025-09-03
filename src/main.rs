pub mod ddb;
pub mod device_repo;
pub mod ogn_aprs_aircraft;
pub mod aprs_client;
pub mod faa;

use anyhow::Result;
use clap::{Parser, Subcommand};
use sqlx::postgres::PgPool;
use std::env;
use std::sync::Arc;
use tracing::{info, error};

use crate::aprs_client::{AprsClient, AprsClientConfigBuilder, MessageProcessor};
use crate::ddb::DeviceDatabase;
use crate::device_repo::DeviceRepository;
use crate::faa::aircraft_registrations::read_aircraft_file;
use crate::faa::aircraft_models::read_aircraft_models_file;
use crate::faa::aircraft_model_repo::AircraftModelRepository;
use crate::faa::aircraft_registrations_repo::AircraftRegistrationsRepository;
use soar::airports::read_airports_csv_file;
use soar::airports_repo::AirportsRepository;
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
    /// Load aircraft model and registration data
    ///
    /// Aircraft registrations and models should come from the "releasable aircraft" FAA database.
    /// Airports and runways should come from "our airports" database.
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
    },
    /// Pull devices from DDB and upsert them into the database
    PullDevices,
    /// Run the main APRS client
    Run {
        /// APRS server hostname
        #[arg(long, default_value = "aprs.glidernet.org")]
        server: String,

        /// APRS server port
        #[arg(long, default_value = "14580")]
        port: u16,

        /// Callsign for APRS authentication
        #[arg(long, default_value = "N0CALL")]
        callsign: String,

        /// APRS filter string (e.g., "r/47.0/-122.0/100" for radius filter)
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
    },
}

/// Simple message processor that logs received APRS packets
struct LoggingMessageProcessor;

impl MessageProcessor for LoggingMessageProcessor {
    fn process_message(&self, message: ogn_parser::AprsPacket) {
        info!("Received APRS packet: {:?}", message);
    }
}

async fn setup_database() -> Result<PgPool> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Get the database URL from environment variables
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in environment variables");

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

async fn handle_load_data(
    aircraft_models_path: Option<String>,
    aircraft_registrations_path: Option<String>,
    airports_path: Option<String>,
    runways_path: Option<String>
) -> Result<()> {
    info!("Loading data - Models: {:?}, Registrations: {:?}, Airports: {:?}, Runways: {:?}",
          aircraft_models_path, aircraft_registrations_path, airports_path, runways_path);

    // Set up database connection
    let pool = setup_database().await?;

    // Load aircraft models first (if provided)
    if let Some(aircraft_models_path) = aircraft_models_path {
        info!("Loading aircraft models from: {}", aircraft_models_path);
        match read_aircraft_models_file(&aircraft_models_path) {
            Ok(aircraft_models) => {
                info!("Successfully loaded {} aircraft models", aircraft_models.len());

                // Create aircraft model repository and upsert models
                let model_repo = AircraftModelRepository::new(pool.clone());
                info!("Upserting {} aircraft models into database...", aircraft_models.len());

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
        info!("Loading aircraft registrations from: {}", aircraft_registrations_path);
        match read_aircraft_file(&aircraft_registrations_path) {
            Ok(aircraft_list) => {
                info!("Successfully loaded {} aircraft registrations", aircraft_list.len());

                // Create aircraft registrations repository and upsert registrations
                let registrations_repo = AircraftRegistrationsRepository::new(pool.clone());
                info!("Upserting {} aircraft registrations into database...", aircraft_list.len());

                match registrations_repo.upsert_aircraft_registrations(aircraft_list).await {
                    Ok(upserted_count) => {
                        info!("Successfully upserted {} aircraft registrations", upserted_count);

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
                info!("Upserting {} airports into database...", airports_list.len());

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
                let runways_repo = RunwaysRepository::new(pool);
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

    Ok(())
}

async fn handle_pull_devices() -> Result<()> {
    info!("Starting device pull from DDB...");

    // Set up database connection
    let pool = setup_database().await?;

    // Create device database and fetch devices
    let mut device_db = DeviceDatabase::new();
    info!("Fetching devices from DDB...");

    match device_db.fetch().await {
        Ok(_) => {
            let device_count = device_db.device_count();
            info!("Successfully fetched {} devices from DDB", device_count);

            if device_count == 0 {
                info!("No devices found in DDB response");
                return Ok(());
            }

            // Create device repository and upsert devices
            let device_repo = DeviceRepository::new(pool);
            let devices: Vec<_> = device_db.get_all_devices().values().cloned().collect();

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
        Err(e) => {
            error!("Failed to fetch devices from DDB: {}", e);
            return Err(anyhow::anyhow!("Failed to fetch devices from DDB: {}", e));
        }
    }

    Ok(())
}

async fn handle_run(
    server: String,
    port: u16,
    callsign: String,
    filter: Option<String>,
    max_retries: u32,
    retry_delay: u64,
    archive_dir: Option<String>,
) -> Result<()> {
    info!("Starting APRS client with server: {}:{}", server, port);

    // Set up database connection
    let _pool = setup_database().await?;

    // Create APRS client configuration
    let config = AprsClientConfigBuilder::new()
        .server(server)
        .port(port)
        .callsign(callsign)
        .filter(filter)
        .max_retries(max_retries)
        .retry_delay_seconds(retry_delay)
        .archive_base_dir(archive_dir)
        .build();

    // Create message processor
    let processor: Arc<dyn MessageProcessor> = Arc::new(LoggingMessageProcessor);

    // Create and start APRS client
    let mut client = AprsClient::new(config, processor);

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
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::LoadData { aircraft_models, aircraft_registrations, airports, runways } => {
            handle_load_data(aircraft_models, aircraft_registrations, airports, runways).await
        }
        Commands::PullDevices => {
            handle_pull_devices().await
        }
        Commands::Run {
            server,
            port,
            callsign,
            filter,
            max_retries,
            retry_delay,
            archive_dir,
        } => {
            handle_run(
                server,
                port,
                callsign,
                filter,
                max_retries,
                retry_delay,
                archive_dir,
            ).await
        }
    }
}
