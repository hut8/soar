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
    /// Load FAA aircraft registrations from CSV file
    LoadRegistrations {
        /// Path to the FAA registrations CSV file
        #[arg(long)]
        faa_registrations: String,
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

async fn handle_load_registrations(faa_registrations_path: String) -> Result<()> {
    info!("Loading FAA registrations from: {}", faa_registrations_path);

    // Set up database connection
    let _pool = setup_database().await?;

    // Read the aircraft registrations file
    match read_aircraft_file(&faa_registrations_path) {
        Ok(aircraft_list) => {
            info!("Successfully loaded {} aircraft registrations", aircraft_list.len());

            // For now, just discard the results as requested
            // In the future, this is where we would insert into the database
            info!("Discarding results as requested");

            // Print a few examples for verification
            if !aircraft_list.is_empty() {
                info!("Sample aircraft records:");
                for (i, aircraft) in aircraft_list.iter().take(3).enumerate() {
                    info!("  {}. N-Number: {}, Serial: {}, Manufacturer: {:?}",
                          i + 1,
                          aircraft.n_number,
                          aircraft.serial_number,
                          aircraft.mfr_mdl_code);
                }
            }
        }
        Err(e) => {
            error!("Failed to read aircraft registrations file: {}", e);
            return Err(e);
        }
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
        Commands::LoadRegistrations { faa_registrations } => {
            handle_load_registrations(faa_registrations).await
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
