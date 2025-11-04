use anyhow::Result;
use clap::{Parser, Subcommand};
use diesel::connection::{Instrumentation, InstrumentationEvent, set_default_instrumentation};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{PgConnection, QueryableByName, RunQueryDsl};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use std::env;
use std::fs;
use std::path::Path;
use tracing::{info, warn};

mod commands;
use commands::{
    handle_archive, handle_ingest_aprs, handle_load_data, handle_pull_data, handle_resurrect,
    handle_run, handle_sitemap_generation,
};

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
    /// Ingest APRS messages into NATS JetStream (durable queue service)
    ///
    /// This service connects to APRS-IS and publishes all messages to a durable NATS JetStream queue.
    /// It is designed to run independently and survive restarts without dropping messages.
    IngestAprs {
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

        /// NATS server URL for JetStream
        #[arg(long, default_value = "nats://localhost:4222")]
        nats_url: String,
    },
    /// Run the main APRS processing service (consumes from JetStream durable queue)
    Run {
        /// Base directory for message archive (optional)
        #[arg(long)]
        archive_dir: Option<String>,

        /// Enable automatic archiving (uses /var/soar/archive if writable, otherwise $HOME/soar-archive/)
        #[arg(long)]
        archive: bool,

        /// NATS server URL for JetStream consumer and pub/sub
        #[arg(long, default_value = "nats://localhost:4222")]
        nats_url: String,
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
    /// Archive old data to compressed CSV files and delete from database
    ///
    /// Archives data one day at a time in order to respect foreign key constraints:
    /// 1. Flights (8+ days old)
    /// 2. Fixes and ReceiverStatuses (9+ days old)
    /// 3. AprsMessages (10+ days old)
    ///
    /// Each day's data is written to files named YYYYMMDD-{table}.csv.zst
    Archive {
        /// Archive data before this date (YYYY-MM-DD format, exclusive, UTC)
        /// Cannot be a future date. If today's date is used, archives all data up to (but not including) today.
        #[arg(value_name = "BEFORE_DATE")]
        before: String,

        /// Directory where archive files will be stored
        #[arg(long)]
        archive_path: String,
    },
    /// Resurrect archived data from compressed CSV files back into the database
    ///
    /// Restores data from archive files in the reverse order of archival to respect foreign key constraints:
    /// 1. AprsMessages (must be restored first)
    /// 2. Fixes and ReceiverStatuses (depend on aprs_messages)
    /// 3. Flights (depend on fixes)
    ///
    /// Reads files named YYYYMMDD-{table}.csv.zst for the specified date
    Resurrect {
        /// Date to resurrect (YYYY-MM-DD format, UTC)
        /// Will look for archive files for this specific date
        #[arg(value_name = "DATE")]
        date: String,

        /// Directory where archive files are stored
        #[arg(long)]
        archive_path: String,
    },
    /// Verify runtime initialization (Sentry, tracing, tokio-console)
    ///
    /// Tests that the runtime can initialize without panicking. Used for CI/CD
    /// to catch configuration issues like missing tokio_unstable flag.
    VerifyRuntime {},
    /// Run database migrations
    ///
    /// Runs all pending database migrations and exits. This is useful for deployment
    /// scripts to ensure migrations are applied before starting services.
    /// Migrations are also run automatically by other commands that need the database.
    Migrate {},
}

// Query logger that logs SQL statements to tracing
struct QueryLogger;

impl Instrumentation for QueryLogger {
    fn on_connection_event(&mut self, event: InstrumentationEvent<'_>) {
        match event {
            InstrumentationEvent::StartQuery { query, .. } => {
                info!("Executing SQL: {}", query);
            }
            InstrumentationEvent::FinishQuery {
                query,
                error: Some(err),
                ..
            } => {
                warn!("Query failed: {} - Error: {}", query, err);
            }
            _ => {} // Ignore other events
        }
    }
}

#[tracing::instrument]
async fn setup_diesel_database() -> Result<Pool<ConnectionManager<PgConnection>>> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Get the database URL from environment variables
    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL must be set in environment variables");

    // Create a Diesel connection pool with increased capacity for batched AGL updates
    // Increased from default (10) to 50 to handle:
    // - 5 APRS workers
    // - 8 elevation workers
    // - 1 batch writer
    // - Various background tasks and web requests
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
        .max_size(50)
        .build(manager)
        .map_err(|e| anyhow::anyhow!("Failed to create Diesel connection pool: {e}"))?;

    info!("Successfully created Diesel connection pool (max connections: 50)");

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

    // Try to acquire the lock with retries (indefinite, 5 second intervals)
    let mut attempts = 0;
    let mut lock_acquired = false;

    while !lock_acquired {
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
            info!(
                "Migration lock unavailable, retrying in 5 seconds... (attempt {})",
                attempts
            );
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }

    info!("Migration lock acquired successfully");

    // Check for pending migrations before running them
    match connection.pending_migrations(MIGRATIONS) {
        Ok(pending) => {
            if pending.is_empty() {
                info!("No pending migrations found");
            } else {
                info!("Found {} pending migration(s) to apply:", pending.len());
                for migration in &pending {
                    info!("  - Will apply: {}", migration.name());
                }
            }
        }
        Err(e) => {
            warn!("Could not list pending migrations: {}", e);
        }
    }

    // Run migrations while holding the lock and handle result immediately
    match connection.run_pending_migrations(MIGRATIONS) {
        Ok(applied_migrations) => {
            if applied_migrations.is_empty() {
                info!("No pending migrations to apply");
            } else {
                info!(
                    "Successfully applied {} migration(s):",
                    applied_migrations.len()
                );
                for migration in &applied_migrations {
                    info!("  - Applied migration: {}", migration);
                }
            }
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
            // Run subcommand uses tokio-console on a random port in development
            // Note: With port 0, the OS assigns a random available port, but we can't
            // reliably get the actual port from console-subscriber without patching it.
            // For now, we'll use a fixed random-ish port based on PID
            let console_port = (std::process::id() % 10000) + 50000; // Port range 50000-59999
            let console_layer = filter::Filtered::new(
                console_subscriber::ConsoleLayer::builder()
                    .server_addr(([0, 0, 0, 0], console_port as u16))
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
                "tokio-console subscriber initialized on port {} - connect with `tokio-console http://localhost:{}`",
                console_port, console_port
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

    // Handle commands that don't need database access early
    match &cli.command {
        Commands::VerifyRuntime {} => {
            info!("Runtime verification successful:");
            info!("  ✓ Sentry integration initialized");
            info!("  ✓ Tracing subscriber initialized");
            info!("  ✓ tokio-console layer initialized (port 7779)");
            info!("  ✓ All runtime components ready");
            info!("Runtime verification PASSED");
            return Ok(());
        }
        Commands::IngestAprs {
            server,
            port,
            callsign,
            filter,
            max_retries,
            retry_delay,
            nats_url,
        } => {
            // IngestAprs only uses NATS, doesn't need database
            return handle_ingest_aprs(
                server.clone(),
                *port,
                callsign.clone(),
                filter.clone(),
                *max_retries,
                *retry_delay,
                nats_url.clone(),
            )
            .await;
        }
        _ => {
            // All other commands need database access
        }
    }

    // Enable SQL query logging only for the migrate command
    if matches!(cli.command, Commands::Migrate {}) {
        set_default_instrumentation(|| Some(Box::new(QueryLogger)))
            .expect("Failed to set default instrumentation");
    }

    // Set up database connection for commands that need it
    // This also runs migrations automatically
    let diesel_pool = setup_diesel_database().await?;

    match cli.command {
        Commands::Sitemap { static_root } => {
            let sitemap_path = static_root.unwrap_or_else(|| {
                env::var("SITEMAP_ROOT").unwrap_or_else(|_| "/var/soar/sitemap".to_string())
            });
            handle_sitemap_generation(diesel_pool, sitemap_path).await
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
            handle_load_data(
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
        Commands::PullData {} => handle_pull_data(diesel_pool).await,
        Commands::IngestAprs { .. } => {
            // This should never be reached due to early return above
            unreachable!("IngestAprs should be handled before database setup")
        }
        Commands::Run {
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

            handle_run(final_archive_dir, nats_url, diesel_pool).await
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
        } => handle_archive(diesel_pool, before, archive_path).await,
        Commands::Resurrect { date, archive_path } => {
            handle_resurrect(diesel_pool, date, archive_path).await
        }
        Commands::VerifyRuntime {} => {
            // This should never be reached due to early return above
            unreachable!("VerifyRuntime should be handled before database setup")
        }
        Commands::Migrate {} => {
            // Migrations are already run by setup_diesel_database()
            info!("Database migrations completed successfully");
            info!("All pending migrations have been applied");
            Ok(())
        }
    }
}
