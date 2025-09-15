use anyhow::Result;
use chrono::Local;
use clap::{Parser, Subcommand};
use diesel::{PgConnection, RunQueryDsl};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

use soar::aprs_client::{AprsClient, AprsClientConfigBuilder, FixProcessor, MessageProcessor};
use soar::database_fix_processor::DatabaseFixProcessor;
use soar::live_fixes::LiveFixService;

// Embed migrations at compile time
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

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
        /// https://registry.faa.gov/database/ReleasableAircraft.zip
        #[arg(long)]
        aircraft_models: Option<String>,
        /// Path to the aircraft registrations data file (from MASTER.txt in the "releasable aircraft" FAA database)
        /// https://www.faa.gov/licenses_certificates/aircraft_certification/aircraft_registry/releasable_aircraft_download
        /// https://registry.faa.gov/database/ReleasableAircraft.zip
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
        /// Link soaring clubs to their nearest airports (within 10 miles) as home bases
        #[arg(long)]
        link_home_bases: bool,
    },
    /// Pull data from HTTP sources and then load it
    ///
    /// Downloads airports and runways data from ourairports-data, creates a temporary directory,
    /// and then invokes the same procedures as load-data.
    PullData {},
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

        /// Path to CSV log file for unparsed APRS fragments (optional)
        #[arg(long)]
        unparsed_log: Option<String>,
    },
    /// Start the web server
    Web {
        /// Port to bind the web server to
        #[arg(long, default_value = "61225")]
        port: u16,

        /// Interface to bind the web server to
        #[arg(long, default_value = "localhost")]
        interface: String,
    },
}

async fn setup_diesel_database() -> Result<Pool<ConnectionManager<PgConnection>>> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Get the database URL from environment variables
    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL must be set in environment variables");

    // Create a Diesel connection pool
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
        .build(manager)
        .map_err(|e| anyhow::anyhow!("Failed to create Diesel connection pool: {}", e))?;

    info!("Successfully created Diesel connection pool");

    // Run embedded migrations with a PostgreSQL advisory lock
    info!("Running database migrations...");
    let mut connection = pool
        .get()
        .map_err(|e| anyhow::anyhow!("Failed to get database connection for migrations: {}", e))?;

    // Use a fixed, unique lock ID for migrations (arbitrary but consistent)
    // This ensures only one process can run migrations at a time
    let migration_lock_id = 19150118; // Ordinal Positions of "SOAR" in English alphabet: S(19) O(15) A(1) R(18)

    // Blocking advisory lock with timeout
    info!("Waiting to acquire migration lock...");
    diesel::sql_query(format!(
        "SELECT pg_advisory_xact_lock({})",
        migration_lock_id
    ))
    .execute(&mut connection)
    .map_err(|e| anyhow::anyhow!("Failed to acquire migration lock: {}", e))?;

    info!("Migration lock acquired");

    connection
        .run_pending_migrations(MIGRATIONS)
        .map_err(|e| anyhow::anyhow!("Failed to run migrations: {}", e))?;

    info!("Database migrations completed successfully");

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
    unparsed_log: Option<String>,
) -> Result<()> {
    info!("Starting APRS client with server: {}:{}", server, port);

    // Set up database connection
    let diesel_pool = setup_diesel_database().await?;

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
        .unparsed_log_path(unparsed_log)
        .build();

    // Create composite processor (NATS + file streaming)
    info!(
        "Setting up message processors - writing to directory: {:?}, NATS URL: {}",
        archive_dir, nats_url
    );
    let archive_processor: Arc<dyn MessageProcessor> = Arc::new(
        soar::message_processors::ArchiveMessageProcessor::new(archive_dir),
    );

    // Create database fix processor to save all valid fixes to the database
    let db_fix_processor: Arc<dyn FixProcessor> =
        Arc::new(DatabaseFixProcessor::new(diesel_pool.clone()));

    // Create and start APRS client with both message and fix processors
    let mut client =
        AprsClient::new_with_fix_processor(config, archive_processor, db_fix_processor);

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

async fn handle_pull_data(diesel_pool: Pool<ConnectionManager<PgConnection>>) -> Result<()> {
    info!("Starting pull-data operation");

    // Create temporary directory with timestamp
    let timestamp = Local::now().format("%Y%m%d-%H%M%S");
    let temp_dir = format!("/tmp/soar/data-{}", timestamp);

    info!("Creating temporary directory: {}", temp_dir);
    fs::create_dir_all(&temp_dir)?;

    // Download airports.csv
    let airports_url = "https://davidmegginson.github.io/ourairports-data/airports.csv";
    let airports_path = format!("{}/airports.csv", temp_dir);
    info!("Downloading airports data from: {}", airports_url);

    let client = reqwest::Client::new();
    let airports_response = client.get(airports_url).send().await?;
    let airports_content = airports_response.text().await?;
    fs::write(&airports_path, airports_content)?;
    info!("Airports data saved to: {}", airports_path);

    // Download runways.csv
    let runways_url = "https://davidmegginson.github.io/ourairports-data/runways.csv";
    let runways_path = format!("{}/runways.csv", temp_dir);
    info!("Downloading runways data from: {}", runways_url);

    let runways_response = client.get(runways_url).send().await?;
    let runways_content = runways_response.text().await?;
    fs::write(&runways_path, runways_content)?;
    info!("Runways data saved to: {}", runways_path);

    // Download FAA ReleasableAircraft.zip
    let faa_url = "https://registry.faa.gov/database/ReleasableAircraft.zip";
    let zip_path = format!("{}/ReleasableAircraft.zip", temp_dir);
    info!("Downloading FAA aircraft data from: {}", faa_url);

    let faa_response = client.get(faa_url).send().await?;
    let zip_content = faa_response.bytes().await?;
    fs::write(&zip_path, zip_content)?;
    info!("FAA zip file saved to: {}", zip_path);

    // Extract ACFTREF.txt and MASTER.txt from the zip file
    info!("Extracting aircraft files from zip...");
    let zip_file = fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(zip_file)?;

    // Extract ACFTREF.txt (aircraft models)
    let acftref_path = format!("{}/ACFTREF.txt", temp_dir);
    {
        let mut acftref_file = archive.by_name("ACFTREF.txt")?;
        let mut acftref_output = fs::File::create(&acftref_path)?;
        io::copy(&mut acftref_file, &mut acftref_output)?;
    }
    info!("Aircraft models data extracted to: {}", acftref_path);

    // Extract MASTER.txt (aircraft registrations)
    let master_path = format!("{}/MASTER.txt", temp_dir);
    {
        let mut master_file = archive.by_name("MASTER.txt")?;
        let mut master_output = fs::File::create(&master_path)?;
        io::copy(&mut master_file, &mut master_output)?;
    }
    info!("Aircraft registrations data extracted to: {}", master_path);

    // Display the temporary directory
    info!("Temporary directory created at: {}", temp_dir);

    // Invoke handle_load_data with all downloaded files
    info!("Invoking load data procedures...");
    soar::loader::handle_load_data(
        diesel_pool,
        Some(acftref_path), // aircraft_models
        Some(master_path),  // aircraft_registrations
        Some(airports_path),
        Some(runways_path),
        None, // receivers
        true,
        true,
        true,
    )
    .await
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file early
    dotenvy::dotenv().ok();

    // Initialize Sentry for error tracking (errors only, no performance monitoring)
    let _guard = if let Ok(sentry_dsn) = env::var("SENTRY_DSN") {
        info!("Initializing Sentry with DSN");
        Some(sentry::init(sentry::ClientOptions {
            dsn: Some(sentry_dsn.parse().expect("Invalid SENTRY_DSN format")),
            traces_sample_rate: 0.0, // Disable performance monitoring
            session_mode: sentry::SessionMode::Request,
            auto_session_tracking: false, // Disable session tracking
            ..Default::default()
        }))
    } else {
        info!("SENTRY_DSN not configured, Sentry disabled");
        None
    };

    // Initialize tracing with TRACE level by default, but silence async_nats TRACE/DEBUG
    use tracing_subscriber::{EnvFilter, FmtSubscriber};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        // Default filter: TRACE for soar, INFO for async_nats, WARN for everything else
        EnvFilter::new("info,soar=debug,async_nats=warn,soar::nats_publisher=warn")
    });

    FmtSubscriber::builder().with_env_filter(filter).init();

    let cli = Cli::parse();

    // Set up database connection - Diesel for all repositories
    let diesel_pool = setup_diesel_database().await?;

    match cli.command {
        Commands::LoadData {
            aircraft_models,
            aircraft_registrations,
            airports,
            runways,
            receivers,
            pull_devices,
            geocode,
            link_home_bases,
        } => {
            soar::loader::handle_load_data(
                diesel_pool,
                aircraft_models,
                aircraft_registrations,
                airports,
                runways,
                receivers,
                pull_devices,
                geocode,
                link_home_bases,
            )
            .await
        }
        Commands::PullData {} => handle_pull_data(diesel_pool).await,
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
            unparsed_log,
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
                unparsed_log,
            )
            .await
        }
        Commands::Web { interface, port } => {
            // Check SOAR_ENV and override port if not production
            let final_port = match env::var("SOAR_ENV") {
                Ok(soar_env) if soar_env == "production" => {
                    info!("Running in production mode on port {}", port);
                    port
                }
                Ok(soar_env) => {
                    info!("Running in {} mode, overriding port to 1337", soar_env);
                    1337
                }
                Err(_) => {
                    info!("SOAR_ENV not set, defaulting to development mode on port 1337");
                    1337
                }
            };

            // Start live fixes service if NATS URL is configured
            if let Ok(nats_url) = env::var("NATS_URL") {
                info!("Starting live fixes service with NATS URL: {}", nats_url);
                let live_fix_service = LiveFixService::new(&nats_url).await?;
                live_fix_service.start_listening().await?;
            } else {
                warn!("NATS_URL not configured, live fixes will not be available");
            }

            soar::web::start_web_server(interface, final_port, diesel_pool).await
        }
    }
}
