use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{PgConnection, QueryableByName, RunQueryDsl};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use soar::pull;
use std::env;
use std::fs;
use std::path::Path;
use tracing::Instrument;
use tracing::{info, warn};

use soar::aprs_client::{AprsClient, AprsClientConfigBuilder};
use soar::fix_processor::FixProcessor;
use soar::flight_tracker::FlightTracker;
use soar::instance_lock::InstanceLock;
use soar::packet_processors::{
    AircraftPositionProcessor, GenericProcessor, PacketRouter, PositionPacketProcessor,
    ReceiverPositionProcessor, ReceiverStatusProcessor, ServerStatusProcessor,
};
use soar::receiver_repo::ReceiverRepository;
use soar::receiver_status_repo::ReceiverStatusRepository;
use soar::server_messages_repo::ServerMessagesRepository;

// Embed migrations at compile time
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

#[derive(QueryableByName)]
struct LockResult {
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pg_try_advisory_lock: bool,
}

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
    /// Generate sitemap files for the website
    Sitemap {
        /// Static root directory where sitemap files will be stored (defaults to SITEMAP_ROOT env var)
        #[arg(long)]
        static_root: Option<String>,
    },
    /// Load aircraft model and registration data, receivers, and devices from files
    ///
    /// Aircraft registrations and models should come from the "releasable aircraft" FAA database.
    /// Airports and runways should come from "our airports" database.
    /// Receivers JSON file can be created from <https://github.com/hut8/ogn-rdb>
    /// Devices file should be the unified FlarmNet database from <https://turbo87.github.io/united-flarmnet/united.fln>
    LoadData {
        /// Path to the aircraft model data file (from ACFTREF.txt in the "releasable aircraft" FAA database
        /// <https://www.faa.gov/licenses_certificates/aircraft_certification/aircraft_registry/releasable_aircraft_download>)
        /// <https://registry.faa.gov/database/ReleasableAircraft.zip>
        #[arg(long)]
        aircraft_models: Option<String>,
        /// Path to the aircraft registrations data file (from MASTER.txt in the "releasable aircraft" FAA database)
        /// <https://www.faa.gov/licenses_certificates/aircraft_certification/aircraft_registry/releasable_aircraft_download>
        /// <https://registry.faa.gov/database/ReleasableAircraft.zip>
        #[arg(long)]
        aircraft_registrations: Option<String>,
        /// Path to the airports CSV file (from "our airports" database)
        /// <https://davidmegginson.github.io/ourairports-data/airports.csv>
        #[arg(long)]
        airports: Option<String>,
        /// Path to the runways CSV file (from "our airports" database)
        /// <https://davidmegginson.github.io/ourairports-data/runways.csv>
        #[arg(long)]
        runways: Option<String>,
        /// Path to the receivers JSON file (can be created from <https://github.com/hut8/ogn-rdb>)
        #[arg(long)]
        receivers: Option<String>,
        /// Path to the devices file (unified FlarmNet database from <https://turbo87.github.io/united-flarmnet/united.fln>)
        #[arg(long)]
        devices: Option<String>,
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
    /// Archive old fixes data to compressed CSV files and delete from database
    ///
    /// Archives fixes data one day at a time, starting from the oldest day in the database
    /// and working forward until the specified 'before' date (exclusive).
    /// Each day's data is written to a file named YYYYMMDD-fixes.csv.zst
    Archive {
        /// Archive fixes before this date (YYYY-MM-DD format, exclusive, UTC)
        /// Cannot be a future date. If today's date is used, archives all data up to (but not including) today.
        #[arg(value_name = "BEFORE_DATE")]
        before: String,

        /// Directory where archive files will be stored
        #[arg(long)]
        archive_path: String,
    },
    /// Verify runtime initialization (Sentry, tracing, tokio-console)
    ///
    /// Tests that the runtime can initialize without panicking. Used for CI/CD
    /// to catch configuration issues like missing tokio_unstable flag.
    VerifyRuntime {},
}

#[tracing::instrument]
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
        .map_err(|e| anyhow::anyhow!("Failed to create Diesel connection pool: {e}"))?;

    info!("Successfully created Diesel connection pool");

    // Run embedded migrations with a PostgreSQL advisory lock
    info!("Running database migrations...");
    let mut connection = pool
        .get()
        .map_err(|e| anyhow::anyhow!("Failed to get database connection for migrations: {e}"))?;

    // Use a fixed, unique lock ID for migrations (arbitrary but consistent)
    // This ensures only one process can run migrations at a time
    let migration_lock_id = 19150118; // Ordinal Positions of "SOAR" in English alphabet: S(19) O(15) A(1) R(18)

    // Use session-scoped advisory lock with retry logic
    info!("Attempting to acquire migration lock...");

    // Try to acquire the lock with retries (total ~30 seconds)
    let mut attempts = 0;
    let max_attempts = 30;
    let mut lock_acquired = false;

    while attempts < max_attempts && !lock_acquired {
        let lock_result =
            diesel::sql_query(format!("SELECT pg_try_advisory_lock({migration_lock_id})"))
                .get_result::<LockResult>(&mut connection)
                .map_err(|e| {
                    anyhow::anyhow!("Failed to attempt migration lock acquisition: {e}")
                })?;

        let result = lock_result.pg_try_advisory_lock;

        if result {
            lock_acquired = true;
            info!("Migration lock acquired on attempt {}", attempts + 1);
        } else {
            attempts += 1;
            if attempts < max_attempts {
                info!(
                    "Migration lock unavailable, retrying in 1 second... (attempt {}/{})",
                    attempts, max_attempts
                );
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    }

    if !lock_acquired {
        return Err(anyhow::anyhow!(
            "Failed to acquire migration lock after {max_attempts} attempts. Another migration process may be running."
        ));
    }

    info!("Migration lock acquired successfully");

    // Run migrations while holding the lock and handle result immediately
    match connection.run_pending_migrations(MIGRATIONS) {
        Ok(_) => {
            info!("Database migrations completed successfully");
            // Release the advisory lock after successful migrations
            diesel::sql_query(format!("SELECT pg_advisory_unlock({migration_lock_id})"))
                .execute(&mut connection)
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to release migration lock after successful migrations: {e}"
                    )
                })?;
            info!("Migration lock released");
        }
        Err(migration_error) => {
            // Release the advisory lock even if migrations failed
            let unlock_result =
                diesel::sql_query(format!("SELECT pg_advisory_unlock({migration_lock_id})"))
                    .execute(&mut connection);
            info!("Migration lock released after failure");

            // Log unlock error but prioritize migration error
            if let Err(unlock_error) = unlock_result {
                warn!("Also failed to release migration lock: {}", unlock_error);
            }

            return Err(anyhow::anyhow!(
                "Failed to run migrations: {migration_error}"
            ));
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
            Ok(()) => {
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
    let home_archive = format!("{home_dir}/soar-archive");

    // Create the directory if it doesn't exist
    fs::create_dir_all(&home_archive)
        .map_err(|e| anyhow::anyhow!("Failed to create archive directory {home_archive}: {e}"))?;

    info!("Using archive directory: {}", home_archive);
    Ok(home_archive)
}

#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip_all, fields(server = %server, port = %port))]
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
    sentry::configure_scope(|scope| {
        scope.set_tag("operation", "run");
    });
    info!("Starting APRS client with server: {}:{}", server, port);

    // Acquire instance lock to prevent multiple instances from running
    let is_production = env::var("SOAR_ENV")
        .map(|env| env == "production")
        .unwrap_or(false);

    // Start metrics server in the background
    if is_production {
        info!("Starting metrics server on port 9091");
        tokio::spawn(
            async {
                soar::metrics::start_metrics_server(9091).await;
            }
            .instrument(tracing::info_span!("metrics_server")),
        );
    }
    let lock_name = if is_production {
        "soar-run-production"
    } else {
        "soar-run-dev"
    };
    let _instance_lock = InstanceLock::new(lock_name)?;
    info!(
        "Acquired instance lock: {}",
        _instance_lock.path().display()
    );

    // Set up database connection
    let diesel_pool = setup_diesel_database().await?;

    // Determine flight tracker state file path
    let state_file_path = match env::var("FLIGHT_STATE_PATH") {
        Ok(path) => std::path::PathBuf::from(path),
        Err(_) => {
            // Use XDG state directory or fallback to ~/.local/state
            let state_dir = if let Ok(xdg_state_home) = env::var("XDG_STATE_HOME") {
                std::path::PathBuf::from(xdg_state_home)
            } else if let Ok(home) = env::var("HOME") {
                std::path::PathBuf::from(home).join(".local/state")
            } else {
                std::path::PathBuf::from("/tmp")
            };

            let env_suffix = if is_production { "production" } else { "dev" };

            state_dir
                .join("soar")
                .join(format!("flight-tracker-state-{}.json", env_suffix))
        }
    };

    // Ensure parent directory exists
    if let Some(parent) = state_file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    info!(
        "Flight tracker state will be saved to: {}",
        state_file_path.display()
    );

    // Determine and log elevation data storage path
    let _elevation_data_path = match env::var("ELEVATION_DATA_PATH") {
        Ok(path) => {
            info!("Elevation data path from ELEVATION_DATA_PATH: {}", path);
            std::path::PathBuf::from(path)
        }
        Err(_) => {
            use directories::BaseDirs;
            let base = BaseDirs::new().context("no home directory")?;
            let default_path = base
                .cache_dir()
                .join("elevation")
                .join("copernicus-dem-30m");
            info!("Elevation data path (default): {}", default_path.display());
            default_path
        }
    };

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

    // Create FlightTracker with state persistence
    let flight_tracker = FlightTracker::with_state_persistence(&diesel_pool, state_file_path);

    // Load saved state (if exists and < 24 hours old)
    if let Err(e) = flight_tracker.load_state().await {
        warn!("Failed to load flight tracker state: {}", e);
    }

    // Start periodic state saving (every 30 seconds)
    flight_tracker.start_periodic_state_saving(30);

    // Start flight timeout checker (every 60 seconds)
    flight_tracker.start_timeout_checker(60);

    // Create database fix processor to save all valid fixes to the database
    // Try to create with NATS first, fall back to without NATS if connection fails
    let fix_processor = match FixProcessor::with_flight_tracker_and_nats(
        diesel_pool.clone(),
        flight_tracker.clone(),
        &nats_url,
    )
    .await
    {
        Ok(processor_with_nats) => {
            info!("Created FixProcessor with NATS publisher and state persistence");
            processor_with_nats
        }
        Err(e) => {
            warn!(
                "Failed to create FixProcessor with NATS ({}), falling back to processor without NATS",
                e
            );
            FixProcessor::with_flight_tracker(diesel_pool.clone(), flight_tracker.clone())
        }
    };

    // Set up shutdown handler
    tokio::spawn(
        async move {
            match tokio::signal::ctrl_c().await {
                Ok(()) => {
                    info!("Received Ctrl+C, exiting...");
                    std::process::exit(0);
                }
                Err(err) => {
                    eprintln!("Unable to listen for shutdown signal: {}", err);
                }
            }
        }
        .instrument(tracing::info_span!("shutdown_handler")),
    );

    // Create server status processor for server messages
    let server_messages_repo = ServerMessagesRepository::new(diesel_pool.clone());
    let server_status_processor = ServerStatusProcessor::new(server_messages_repo);

    // Create repositories
    let receiver_repo = ReceiverRepository::new(diesel_pool.clone());
    let receiver_status_repo = ReceiverStatusRepository::new(diesel_pool.clone());
    let aprs_messages_repo =
        soar::aprs_messages_repo::AprsMessagesRepository::new(diesel_pool.clone());

    // Create GenericProcessor for receiver identification and APRS message insertion
    let generic_processor = GenericProcessor::new(receiver_repo.clone(), aprs_messages_repo);

    // Create receiver status processor for receiver status messages
    let receiver_status_processor =
        ReceiverStatusProcessor::new(receiver_status_repo, receiver_repo.clone());

    // Create receiver position processor for receiver position messages
    let receiver_position_processor = ReceiverPositionProcessor::new(receiver_repo.clone());

    // Create aircraft position processor
    // Note: FlightDetectionProcessor is now handled inside FixProcessor
    let aircraft_position_processor =
        AircraftPositionProcessor::new().with_fix_processor(fix_processor.clone());

    // Create position packet processor with BOTH aircraft and receiver processors
    let position_processor = PositionPacketProcessor::new()
        .with_aircraft_processor(aircraft_position_processor)
        .with_receiver_processor(receiver_position_processor);

    // Create PacketRouter with all processors
    let packet_router = if let Some(archive_path) = archive_dir.clone() {
        PacketRouter::with_archive(generic_processor, archive_path)
            .await?
            .with_position_processor(position_processor)
            .with_receiver_status_processor(receiver_status_processor)
            .with_server_status_processor(server_status_processor)
    } else {
        PacketRouter::new(generic_processor)
            .with_position_processor(position_processor)
            .with_receiver_status_processor(receiver_status_processor)
            .with_server_status_processor(server_status_processor)
    };

    info!(
        "Setting up APRS client with PacketRouter - archive directory: {:?}, NATS URL: {}",
        archive_dir, nats_url
    );

    // Create and start APRS client with PacketRouter
    let mut client = AprsClient::new(config, packet_router);

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
    // Load environment variables from .env file early
    dotenvy::dotenv().ok();

    // Check if we're in production mode
    let is_production = env::var("SOAR_ENV")
        .map(|env| env == "production")
        .unwrap_or(false);

    // Initialize Sentry for error tracking (errors only, no performance monitoring)
    let _guard = if let Ok(sentry_dsn) = env::var("SENTRY_DSN") {
        info!("Initializing Sentry with DSN");

        // Use SENTRY_RELEASE env var if set (for deployed versions with commit SHA),
        // otherwise fall back to CARGO_PKG_VERSION for local development
        let release = env::var("SENTRY_RELEASE")
            .ok()
            .or_else(|| Some(env!("CARGO_PKG_VERSION").to_string()))
            .map(Into::into);

        if let Some(ref r) = release {
            info!("Sentry release version: {}", r);
        }

        Some(sentry::init(sentry::ClientOptions {
            dsn: Some(sentry_dsn.parse().expect("Invalid SENTRY_DSN format")),
            sample_rate: 0.05,        // Sample 5% of error events
            traces_sample_rate: 0.05, // Sample 5% of performance traces
            attach_stacktrace: true,
            release,
            enable_logs: true,
            environment: env::var("SOAR_ENV").ok().map(Into::into),
            session_mode: sentry::SessionMode::Request,
            auto_session_tracking: true,
            before_send: Some(std::sync::Arc::new(
                move |event: sentry::protocol::Event<'static>| {
                    // Always capture error-level events
                    if event.level >= sentry::Level::Error {
                        Some(event)
                    } else {
                        // For non-error events, only capture in production
                        if is_production { Some(event) } else { None }
                    }
                },
            )),
            ..Default::default()
        }))
    } else {
        if is_production {
            eprintln!("ERROR: SENTRY_DSN environment variable is required in production mode");
            std::process::exit(1);
        }
        info!("SENTRY_DSN not configured, Sentry disabled");
        None
    };

    // Test Sentry integration if enabled
    if _guard.is_some() {
        info!("Sentry initialized successfully");

        // Set up panic hook to capture panics in Sentry
        std::panic::set_hook(Box::new(|panic_info| {
            let panic_msg = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };

            let location = if let Some(location) = panic_info.location() {
                format!(
                    "{}:{}:{}",
                    location.file(),
                    location.line(),
                    location.column()
                )
            } else {
                "Unknown location".to_string()
            };

            sentry::configure_scope(|scope| {
                scope.set_tag("panic.location", location);
            });

            sentry::capture_message(&format!("Panic: {panic_msg}"), sentry::Level::Fatal);

            // Flush Sentry before the process exits
            if let Some(client) = sentry::Hub::current().client() {
                let _ = client.flush(Some(std::time::Duration::from_secs(2)));
            }
        }));
    }

    let cli = Cli::parse();

    // Initialize tracing/tokio-console based on subcommand
    use tracing_subscriber::{EnvFilter, filter, layer::SubscriberExt, util::SubscriberInitExt};

    // Create separate filter for fmt_layer (console output)
    // Use RUST_LOG if set, otherwise default based on production mode
    let fmt_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        if is_production {
            EnvFilter::new("warn")
        } else {
            EnvFilter::new("debug")
        }
    });

    // Create filter for tokio-console layer (needs tokio=trace,runtime=trace for task visibility)
    let console_filter = EnvFilter::new("warn,tokio=trace,runtime=trace");

    let registry = tracing_subscriber::registry();

    let fmt_layer = filter::Filtered::new(tracing_subscriber::fmt::layer(), fmt_filter);

    match &cli.command {
        Commands::Run { .. } => {
            // Run subcommand uses tokio-console on port 6669 (default)
            let console_layer = filter::Filtered::new(
                console_subscriber::ConsoleLayer::builder()
                    .server_addr(([0, 0, 0, 0], 6669))
                    .spawn(),
                console_filter.clone(),
            );

            if let Some(sentry_layer) = _guard
                .as_ref()
                .map(|_| sentry::integrations::tracing::layer())
            {
                registry
                    .with(fmt_layer)
                    .with(console_layer)
                    .with(sentry_layer)
                    .init();
            } else {
                registry.with(fmt_layer).with(console_layer).init();
            }

            info!(
                "tokio-console subscriber initialized on port 6669 - connect with `tokio-console`"
            );
        }
        Commands::VerifyRuntime { .. } => {
            // VerifyRuntime uses tokio-console on port 7779 to avoid conflict with Run
            let console_layer = filter::Filtered::new(
                console_subscriber::ConsoleLayer::builder()
                    .server_addr(([0, 0, 0, 0], 7779))
                    .spawn(),
                console_filter.clone(),
            );

            if let Some(sentry_layer) = _guard
                .as_ref()
                .map(|_| sentry::integrations::tracing::layer())
            {
                registry
                    .with(fmt_layer)
                    .with(console_layer)
                    .with(sentry_layer)
                    .init();
            } else {
                registry.with(fmt_layer).with(console_layer).init();
            }

            info!(
                "tokio-console subscriber initialized on port 7779 - connect with `tokio-console http://localhost:7779`"
            );
        }
        Commands::Web { .. } => {
            // Web subcommand uses tokio-console on port 6670 to avoid conflict
            let console_layer = filter::Filtered::new(
                console_subscriber::ConsoleLayer::builder()
                    .server_addr(([0, 0, 0, 0], 6670))
                    .spawn(),
                console_filter.clone(),
            );

            if let Some(sentry_layer) = _guard
                .as_ref()
                .map(|_| sentry::integrations::tracing::layer())
            {
                registry
                    .with(fmt_layer)
                    .with(console_layer)
                    .with(sentry_layer)
                    .init();
            } else {
                registry.with(fmt_layer).with(console_layer).init();
            }

            info!(
                "tokio-console subscriber initialized on port 6670 - connect with `tokio-console http://localhost:6670`"
            );
        }
        _ => {
            // Other subcommands use regular tracing (no tokio-console overhead)
            if let Some(sentry_layer) = _guard
                .as_ref()
                .map(|_| sentry::integrations::tracing::layer())
            {
                registry.with(fmt_layer).with(sentry_layer).init();
            } else {
                registry.with(fmt_layer).init();
            }
        }
    }

    // Handle VerifyRuntime early - it doesn't need database access
    if matches!(cli.command, Commands::VerifyRuntime {}) {
        info!("Runtime verification successful:");
        info!("  ✓ Sentry integration initialized");
        info!("  ✓ Tracing subscriber initialized");
        info!("  ✓ tokio-console layer initialized (port 7779)");
        info!("  ✓ All runtime components ready");
        info!("Runtime verification PASSED");
        return Ok(());
    }

    // Set up database connection - Diesel for all repositories
    let diesel_pool = setup_diesel_database().await?;

    match cli.command {
        Commands::Sitemap { static_root } => {
            let sitemap_path = static_root.unwrap_or_else(|| {
                env::var("SITEMAP_ROOT").unwrap_or_else(|_| "/var/soar/sitemap".to_string())
            });
            soar::sitemap::handle_sitemap_generation(diesel_pool, sitemap_path).await
        }
        Commands::LoadData {
            aircraft_models,
            aircraft_registrations,
            airports,
            runways,
            receivers,
            devices,
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
                devices,
                geocode,
                link_home_bases,
            )
            .await
        }
        Commands::PullData {} => pull::handle_pull_data(diesel_pool).await,
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

            // Live fixes service is initialized inside start_web_server
            soar::web::start_web_server(interface, final_port, diesel_pool).await
        }
        Commands::Archive {
            before,
            archive_path,
        } => soar::archive::handle_archive(diesel_pool, before, archive_path).await,
        Commands::VerifyRuntime {} => {
            // This should never be reached due to early return above
            unreachable!("VerifyRuntime should be handled before database setup")
        }
    }
}
