use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use diesel::connection::{Instrumentation, InstrumentationEvent, set_default_instrumentation};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{PgConnection, QueryableByName, RunQueryDsl};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

mod commands;
mod migration_email_reporter;

use commands::{
    handle_archive, handle_dump_unified_ddb, handle_ingest_adsb, handle_ingest_ogn,
    handle_load_data, handle_pull_airspaces, handle_pull_data, handle_resurrect, handle_run,
    handle_seed_test_data, handle_sitemap_generation,
};
use migration_email_reporter::{
    MigrationEmailConfig, MigrationReport, send_migration_email_report,
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
#[command(version = env!("VERGEN_GIT_DESCRIBE"))]
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
        /// Path to the ADS-B Exchange basic aircraft database JSON file
        /// <http://downloads.adsbexchange.com/downloads/basic-ac-db.json.gz>
        #[arg(long)]
        adsb_exchange: Option<String>,
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
    /// Pull airspace data from OpenAIP
    ///
    /// Downloads airspace boundaries from OpenAIP API and stores them in the database.
    /// Requires OPENAIP_API_KEY environment variable.
    /// Data is licensed under CC BY-NC 4.0 (non-commercial use only).
    PullAirspaces {
        /// Perform incremental sync (only fetch updated airspaces since last successful sync)
        #[arg(long)]
        incremental: bool,

        /// Filter by specific countries (ISO 3166-1 alpha-2 codes, comma-separated)
        /// Example: --countries US,CA,MX
        #[arg(long, value_delimiter = ',')]
        countries: Option<Vec<String>>,
    },
    /// Ingest OGN (APRS) messages into NATS (durable queue service)
    ///
    /// This service connects to OGN APRS-IS and publishes all messages to NATS.
    /// It is designed to run independently and survive restarts without dropping messages.
    IngestOgn {
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
    /// Ingest ADS-B (Beast) messages into NATS (durable queue service)
    ///
    /// This service connects to a Beast-format ADS-B server and publishes all messages to NATS.
    /// It is designed to run independently and survive restarts without dropping messages.
    IngestAdsb {
        /// Beast server hostname
        #[arg(long, default_value = "localhost")]
        server: String,

        /// Beast server port (standard Beast port is 30005)
        #[arg(long, default_value = "30005")]
        port: u16,

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

        /// APRS type(s) to suppress from processing (e.g., OGADSB, OGFLR)
        /// Can be specified multiple times to suppress multiple types
        #[arg(long)]
        suppress_aprs_type: Vec<String>,

        /// OGN aircraft type(s) to skip processing (e.g., JetTurboprop, Glider)
        /// Can be specified multiple times to skip multiple types
        #[arg(long)]
        skip_ogn_aircraft_type: Vec<String>,

        /// Disable APRS/OGN message consumption from NATS
        #[arg(long)]
        no_aprs: bool,

        /// Disable ADS-B (Beast) message consumption from NATS
        #[arg(long)]
        no_adsb: bool,

        /// Enable tokio-console for async task monitoring (port 6669)
        #[arg(long)]
        enable_tokio_console: bool,
    },
    /// Start the web server
    Web {
        /// Port to bind the web server to
        #[arg(long, default_value = "61225")]
        port: u16,

        /// Interface to bind the web server to
        #[arg(long, default_value = "localhost")]
        interface: String,

        /// Enable test mode (auto-generates JWT_SECRET, sets test database and NATS)
        #[arg(long)]
        test_mode: bool,

        /// Enable tokio-console for async task monitoring (port 6670)
        #[arg(long)]
        enable_tokio_console: bool,
    },
    /// Archive old data to compressed CSV files and delete from database
    ///
    /// Archives data with staggered retention to respect foreign key constraints:
    /// 1. Flights (before_date + 0 days)
    /// 2. Fixes and ReceiverStatuses (before_date + 1 day)
    /// 3. AprsMessages (before_date + 2 days)
    ///
    /// Default: Uses 21 days ago, which archives:
    /// - Flights: 21+ days old
    /// - Fixes and ReceiverStatuses: 22+ days old
    /// - AprsMessages: 23+ days old
    ///
    /// Each day's data is written to files named YYYYMMDD-{table}.csv.zst
    Archive {
        /// Archive data before this date (YYYY-MM-DD format, exclusive, UTC)
        /// Cannot be a future date. Flights are archived before this date,
        /// Fixes/ReceiverStatuses before date+1, AprsMessages before date+2.
        /// Defaults to 21 days ago if not specified.
        #[arg(long, value_name = "BEFORE_DATE")]
        before: Option<String>,

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
    VerifyRuntime {
        /// Enable tokio-console for async task monitoring (port 7779)
        #[arg(long)]
        enable_tokio_console: bool,
    },
    /// Run database migrations
    ///
    /// Runs all pending database migrations and exits. This is useful for deployment
    /// scripts to ensure migrations are applied before starting services.
    /// Migrations are also run automatically by other commands that need the database.
    Migrate {},
    /// Seed test data for E2E testing
    ///
    /// Creates a known set of test data for E2E tests:
    /// - Test user with known credentials (configurable via env vars)
    /// - Test club
    /// - Test devices
    ///
    /// Environment variables:
    /// - TEST_USER_EMAIL (default: test@example.com)
    /// - TEST_USER_PASSWORD (default: testpassword123)
    /// - TEST_USER_FIRST_NAME (default: Test)
    /// - TEST_USER_LAST_NAME (default: User)
    SeedTestData {},
    /// Aggregate position fixes into H3 coverage hexes
    ///
    /// Processes position fixes from the specified date range and aggregates them into H3
    /// hexagonal coverage zones. Updates coverage statistics including fix counts, timestamps,
    /// and altitude information for visualization and analysis.
    ///
    /// If start/end dates are omitted, automatically determines what needs aggregation by:
    /// - Finding the most recent coverage date in the database
    /// - Aggregating from (last coverage date + 1) to yesterday
    /// - If no coverage exists, starts from the oldest fix date
    ///
    /// Example: soar aggregate-coverage --start-date 2025-12-25 --end-date 2025-12-26 --resolutions 6,7,8
    /// Example: soar aggregate-coverage --resolutions 6,7,8  (auto-detect date range)
    AggregateCoverage {
        /// Start date for aggregation (YYYY-MM-DD). If omitted, auto-detects from database.
        #[arg(long)]
        start_date: Option<chrono::NaiveDate>,

        /// End date for aggregation (YYYY-MM-DD). If omitted, defaults to yesterday.
        #[arg(long)]
        end_date: Option<chrono::NaiveDate>,

        /// H3 resolutions to aggregate (comma-separated, e.g., "3,4,5,6,7,8")
        /// Resolution 3: ~12,400km² per hex, Resolution 4: ~1,770km² per hex, Resolution 5: ~252km² per hex, Resolution 6: ~36km² per hex, Resolution 7: ~5km² per hex, Resolution 8: ~0.7km² per hex
        #[arg(long, default_value = "3,4,5,6,7,8", value_delimiter = ',')]
        resolutions: Vec<i16>,
    },
    /// Dump unified FlarmNet device database to JSONL file
    ///
    /// Downloads the unified FlarmNet database from <https://turbo87.github.io/united-flarmnet/united.fln>
    /// and exports all device records to a JSONL (JSON Lines) file for debugging.
    /// Each line contains one complete device record with all fields.
    DumpUnifiedDdb {
        /// Output file path for JSONL export
        #[arg(value_name = "OUTPUT_FILE")]
        output: String,
        /// Optional local source file (if not specified, downloads from remote)
        #[arg(long)]
        source: Option<String>,
    },
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

/// Find the git repository root by walking up the directory tree
fn find_git_root() -> Result<PathBuf> {
    let current_dir = env::current_dir().context("Failed to get current directory")?;

    let mut path = current_dir.as_path();
    loop {
        let git_dir = path.join(".git");
        if git_dir.exists() {
            return Ok(path.to_path_buf());
        }

        match path.parent() {
            Some(parent) => path = parent,
            None => {
                return Err(anyhow::anyhow!(
                    "Could not find .git directory. Started from: {}",
                    current_dir.display()
                ));
            }
        }
    }
}

/// Dump database schema to schema.sql if not in production, staging, test, or CI
fn dump_schema_if_non_production(database_url: &str) -> Result<()> {
    // Check environment
    let soar_env = env::var("SOAR_ENV").unwrap_or_default();
    let is_production = soar_env == "production";
    let is_staging = soar_env == "staging";
    let is_test = soar_env == "test";
    let is_ci = env::var("CI").is_ok();

    if is_production {
        info!("Skipping schema dump in production environment");
        return Ok(());
    }

    if is_staging {
        info!("Skipping schema dump in staging environment");
        return Ok(());
    }

    if is_test {
        info!("Skipping schema dump in test environment");
        return Ok(());
    }

    if is_ci {
        info!("Skipping schema dump in CI environment");
        return Ok(());
    }

    info!("Dumping database schema to schema.sql...");

    // Run pg_dump with flags to ensure deterministic output
    let output = std::process::Command::new("pg_dump")
        .arg("--schema-only") // Only dump schema, not data
        .arg("--no-owner") // Don't include ownership commands
        .arg("--no-privileges") // Don't include GRANT/REVOKE
        .arg("--no-tablespaces") // Don't include tablespace assignments
        .arg("--no-comments") // Don't include comments (may contain timestamps)
        .arg("--restrict-key=SOAR") // Use fixed key to prevent random \restrict hash changes
        .arg(database_url)
        .output()
        .context("Failed to execute pg_dump - is PostgreSQL client installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("pg_dump failed: {}", stderr));
    }

    // Write output to schema.sql in repository root
    // Find the repository root by looking for .git directory
    let repo_root = find_git_root().context("Failed to find repository root (.git directory)")?;
    let schema_path = repo_root.join("schema.sql");
    std::fs::write(&schema_path, &output.stdout).context("Failed to write schema.sql")?;

    info!("Successfully dumped schema to {}", schema_path.display());
    Ok(())
}

/// Migration result containing the pool and migration information
pub struct MigrationResult {
    pub pool: Pool<ConnectionManager<PgConnection>>,
    pub applied_migrations: Vec<String>,
    pub duration_secs: f64,
}

async fn setup_diesel_database(app_name_prefix: &str) -> Result<MigrationResult> {
    let migration_start = std::time::Instant::now();

    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Get the database URL from environment variables
    let mut database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL must be set in environment variables");

    // Construct application_name from command and environment
    let soar_env = env::var("SOAR_ENV").unwrap_or_default();
    let app_name = match soar_env.as_str() {
        "staging" => format!("{}-staging", app_name_prefix),
        "" => format!("{}-dev", app_name_prefix),
        _ => app_name_prefix.to_string(), // production or other
    };

    // Append application_name to DATABASE_URL for PostgreSQL connection tracking
    let separator = if database_url.contains('?') { '&' } else { '?' };
    database_url = format!("{}{}application_name={}", database_url, separator, app_name);

    info!(
        "Connecting to PostgreSQL with application_name: {}",
        app_name
    );

    // Clone for schema dump later (database_url will be moved into ConnectionManager)
    let database_url_for_dump = database_url.clone();

    // Create a Diesel connection pool with conservative sizing to prevent "too many clients" errors
    // PostgreSQL max_connections: 200
    // Pool size: 20 per instance (allows up to 10 concurrent service instances safely)
    // Handles:
    // - 5 APRS workers
    // - 8 elevation workers
    // - 1 batch writer
    // - Various background tasks and web requests
    // Reduced from 50 to 20 after Dec 2025 connection exhaustion incident
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
        .max_size(20)
        .build(manager)
        .map_err(|e| anyhow::anyhow!("Failed to create Diesel connection pool: {e}"))?;

    info!("Successfully created Diesel connection pool (max connections: 20)");

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
    let applied_migration_names = match connection.run_pending_migrations(MIGRATIONS) {
        Ok(applied_migrations) => {
            let migration_names: Vec<String> =
                applied_migrations.iter().map(|m| m.to_string()).collect();

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

            // Dump schema to schema.sql (non-production only)
            dump_schema_if_non_production(&database_url_for_dump)?;

            // Release the advisory lock after successful migrations
            diesel::sql_query(format!("SELECT pg_advisory_unlock({migration_lock_id})"))
                .execute(&mut connection)
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to release migration lock after successful migrations: {e}"
                    )
                })?;
            info!("Migration lock released");

            migration_names
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
    };

    let duration_secs = migration_start.elapsed().as_secs_f64();

    Ok(MigrationResult {
        pool,
        applied_migrations: applied_migration_names,
        duration_secs,
    })
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

    // Check if we're in production or staging mode
    let soar_env = env::var("SOAR_ENV").unwrap_or_default();
    let is_production = soar_env == "production";
    let is_staging = soar_env == "staging";

    // Initialize Sentry for error tracking (errors only, no performance monitoring)
    let _guard = if let Ok(sentry_dsn) = env::var("SENTRY_DSN") {
        // Skip Sentry initialization if DSN is empty or invalid
        if sentry_dsn.is_empty() {
            info!("SENTRY_DSN is empty, Sentry disabled");
            None
        } else if let Ok(parsed_dsn) = sentry_dsn.parse() {
            info!("Initializing Sentry with DSN");

            // Use SENTRY_RELEASE env var if set (for deployed versions),
            // otherwise fall back to VERGEN_GIT_DESCRIBE (git-derived version from build.rs)
            let release = env::var("SENTRY_RELEASE")
                .ok()
                .or_else(|| Some(env!("VERGEN_GIT_DESCRIBE").to_string()))
                .map(Into::into);

            if let Some(ref r) = release {
                info!("Sentry release version: {}", r);
            }

            Some(sentry::init(sentry::ClientOptions {
                dsn: Some(parsed_dsn),
                sample_rate: 0.05,       // Sample 5% of error events
                traces_sample_rate: 0.1, // Sample 10% of performance traces (increased for better visibility)
                attach_stacktrace: true,
                release,
                enable_logs: true,
                environment: env::var("SOAR_ENV").ok().map(Into::into),
                session_mode: sentry::SessionMode::Request,
                auto_session_tracking: true,
                // Note: Continuous profiling not available in sentry-rust 0.45.0
                // Using increased traces_sample_rate for better performance visibility
                before_send: Some(std::sync::Arc::new(
                    move |event: sentry::protocol::Event<'static>| {
                        // Always capture error-level events
                        if event.level >= sentry::Level::Error {
                            Some(event)
                        } else {
                            // For non-error events, only capture in production/staging
                            if is_production || is_staging {
                                Some(event)
                            } else {
                                None
                            }
                        }
                    },
                )),
                ..Default::default()
            }))
        } else {
            eprintln!("WARNING: Invalid SENTRY_DSN format, Sentry disabled");
            None
        }
    } else {
        if is_production || is_staging {
            eprintln!(
                "ERROR: SENTRY_DSN environment variable is required in production and staging modes"
            );
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
    // Use RUST_LOG if set, otherwise default based on environment
    // Note: async_nats is set to warn to suppress "slow consumers" INFO logs during high load
    let fmt_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        if is_production {
            EnvFilter::new("warn,hyper_util=info,rustls=info,async_nats=warn")
        } else if is_staging {
            EnvFilter::new("info,hyper_util=info,rustls=info,async_nats=warn")
        } else {
            EnvFilter::new("debug,hyper_util=info,rustls=info,async_nats=warn")
        }
    });

    // Create filter for tokio-console layer (needs tokio=trace,runtime=trace for task visibility)
    let console_filter = EnvFilter::new("warn,tokio=trace,runtime=trace");

    let registry = tracing_subscriber::registry();

    let fmt_layer = filter::Filtered::new(tracing_subscriber::fmt::layer(), fmt_filter);

    match &cli.command {
        Commands::Run {
            enable_tokio_console,
            ..
        } => {
            if *enable_tokio_console {
                // Run subcommand uses tokio-console on port 6669
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
                    "tokio-console subscriber initialized on port 6669 - connect with `tokio-console http://localhost:6669`"
                );
            } else {
                // No tokio-console overhead
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
        Commands::VerifyRuntime {
            enable_tokio_console,
        } => {
            if *enable_tokio_console {
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
            } else {
                // No tokio-console overhead
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
        Commands::Web {
            enable_tokio_console,
            ..
        } => {
            // Skip console subscriber in test mode to avoid port conflicts
            let is_test_mode = env::var("SOAR_ENV").map(|v| v == "test").unwrap_or(false);

            if is_test_mode {
                // Test mode: skip console subscriber
                if let Some(sentry_layer) = _guard
                    .as_ref()
                    .map(|_| sentry::integrations::tracing::layer())
                {
                    registry.with(fmt_layer).with(sentry_layer).init();
                } else {
                    registry.with(fmt_layer).init();
                }
                info!("Running in test mode - console subscriber disabled");
            } else if *enable_tokio_console {
                // Production/development mode with tokio-console enabled
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
            } else {
                // No tokio-console overhead
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
        Commands::VerifyRuntime {
            enable_tokio_console,
        } => {
            info!("Runtime verification successful:");
            info!("  ✓ Sentry integration initialized");
            info!("  ✓ Tracing subscriber initialized");
            if *enable_tokio_console {
                info!("  ✓ tokio-console layer initialized (port 7779)");
            }
            info!("  ✓ All runtime components ready");
            info!("Runtime verification PASSED");
            return Ok(());
        }
        Commands::IngestOgn {
            server,
            port,
            callsign,
            filter,
            max_retries,
            retry_delay,
            nats_url,
        } => {
            // IngestOgn only uses NATS, doesn't need database
            return handle_ingest_ogn(
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
        Commands::IngestAdsb {
            server,
            port,
            max_retries,
            retry_delay,
            nats_url,
        } => {
            // IngestAdsb only uses NATS, doesn't need database
            return handle_ingest_adsb(
                server.clone(),
                *port,
                *max_retries,
                *retry_delay,
                nats_url.clone(),
            )
            .await;
        }
        Commands::DumpUnifiedDdb { output, source } => {
            // DumpUnifiedDdb only downloads and exports data, doesn't need database
            return handle_dump_unified_ddb(output.clone(), source.clone()).await;
        }
        _ => {
            // All other commands need database access
        }
    }

    // Check if we're in test mode and configure environment BEFORE database setup
    if let Commands::Web { test_mode, .. } = &cli.command
        && *test_mode
    {
        info!("Test mode enabled: configuring test environment");

        // Set JWT_SECRET for authentication
        // SAFETY: We're setting environment variables during startup before any threads are spawned.
        // This is safe because:
        // 1. We're in main() and haven't started the async runtime yet
        // 2. No other threads exist that could be reading these variables
        // 3. These are only set in test mode, not in production
        unsafe {
            env::set_var("JWT_SECRET", "test-jwt-secret-for-e2e-tests");
        }
        info!("  ✓ JWT_SECRET configured");

        // Set test database URL (if not already set)
        if env::var("DATABASE_URL").is_err() {
            unsafe {
                env::set_var(
                    "DATABASE_URL",
                    "postgres://postgres:postgres@localhost:5432/soar_test",
                );
            }
            info!("  ✓ DATABASE_URL set to test database");
        }

        // Set NATS URL (if not already set)
        if env::var("NATS_URL").is_err() {
            unsafe {
                env::set_var("NATS_URL", "nats://localhost:4222");
            }
            info!("  ✓ NATS_URL configured");
        }

        // Set SOAR_ENV to test (if not already set)
        if env::var("SOAR_ENV").is_err() {
            unsafe {
                env::set_var("SOAR_ENV", "test");
            }
            info!("  ✓ SOAR_ENV set to 'test'");
        }

        info!("Test environment configuration complete");
    }

    // Enable SQL query logging only for the migrate command
    if matches!(cli.command, Commands::Migrate {}) {
        set_default_instrumentation(|| Some(Box::new(QueryLogger)))
            .expect("Failed to set default instrumentation");
    }

    // Set up database connection for commands that need it
    // This also runs migrations automatically
    // Determine application name prefix based on command
    let app_name_prefix = match &cli.command {
        Commands::Run { .. } => "soar-run",
        Commands::Web { .. } => "soar-web",
        Commands::Archive { .. } => "soar-archive",
        Commands::Resurrect { .. } => "soar-resurrect",
        Commands::LoadData { .. } => "soar-load-data",
        Commands::PullData {} => "soar-pull-data",
        Commands::PullAirspaces { .. } => "soar-pull-airspaces",
        Commands::Sitemap { .. } => "soar-sitemap",
        Commands::Migrate {} => "soar-migrate",
        Commands::SeedTestData {} => "soar-seed-test-data",
        Commands::AggregateCoverage { .. } => "soar-aggregate-coverage",
        // These should not reach here due to early returns
        Commands::IngestOgn { .. } => unreachable!(),
        Commands::IngestAdsb { .. } => unreachable!(),
        Commands::VerifyRuntime { .. } => unreachable!(),
        Commands::DumpUnifiedDdb { .. } => unreachable!(),
    };

    // For Migrate command, handle errors specially to send notifications
    let (diesel_pool, migration_info) = if matches!(cli.command, Commands::Migrate {}) {
        match setup_diesel_database(app_name_prefix).await {
            Ok(result) => (
                result.pool,
                Some((result.applied_migrations, result.duration_secs)),
            ),
            Err(e) => {
                let duration_secs = std::time::Instant::now().elapsed().as_secs_f64();
                let error_message = format!("{:#}", e);

                // Send failure email
                if let Ok(email_config) = MigrationEmailConfig::from_env() {
                    let report =
                        MigrationReport::failure(error_message.clone(), None, duration_secs);
                    if let Err(email_err) = send_migration_email_report(&email_config, &report) {
                        warn!("Failed to send migration failure email: {}", email_err);
                    }
                } else {
                    warn!(
                        "Email configuration not available, skipping migration failure notification"
                    );
                }

                // Send Sentry error event
                if sentry::Hub::current().client().is_some() {
                    sentry::configure_scope(|scope| {
                        scope.set_tag("migration", "true");
                        scope.set_tag("environment", env::var("SOAR_ENV").unwrap_or_default());
                        scope.set_tag("type", "database_migration");
                    });
                    sentry::capture_message(
                        &format!(
                            "Database migration failed for {} (error: {})",
                            env::var("SOAR_ENV").unwrap_or_else(|_| "development".to_string()),
                            error_message
                        ),
                        sentry::Level::Error,
                    );
                }

                return Err(e);
            }
        }
    } else {
        let result = setup_diesel_database(app_name_prefix).await?;
        (result.pool, None)
    };

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
            adsb_exchange,
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
                adsb_exchange,
                geocode,
                link_home_bases,
            )
            .await
        }
        Commands::PullData {} => handle_pull_data(diesel_pool).await,
        Commands::PullAirspaces {
            incremental,
            countries,
        } => handle_pull_airspaces(diesel_pool, incremental, countries).await,
        Commands::IngestOgn { .. } => {
            // This should never be reached due to early return above
            unreachable!("IngestOgn should be handled before database setup")
        }
        Commands::IngestAdsb { .. } => {
            // This should never be reached due to early return above
            unreachable!("IngestAdsb should be handled before database setup")
        }
        Commands::Run {
            archive_dir,
            archive,
            nats_url,
            suppress_aprs_type,
            skip_ogn_aircraft_type,
            no_aprs,
            no_adsb,
            enable_tokio_console: _,
        } => {
            // Determine archive directory if --archive flag is used
            let final_archive_dir = if archive {
                Some(determine_archive_dir()?)
            } else {
                archive_dir
            };

            handle_run(
                final_archive_dir,
                nats_url,
                &suppress_aprs_type,
                &skip_ogn_aircraft_type,
                no_aprs,
                no_adsb,
                diesel_pool,
            )
            .await
        }
        Commands::Web {
            interface,
            port,
            test_mode: _,
            enable_tokio_console: _,
        } => {
            // Test mode environment is configured earlier, before database setup
            // Check SOAR_ENV and override port only for development mode
            let final_port = match env::var("SOAR_ENV") {
                Ok(soar_env) if soar_env == "production" => {
                    info!("Running in production mode on port {}", port);
                    port
                }
                Ok(soar_env) if soar_env == "staging" => {
                    info!("Running in staging mode on port {}", port);
                    port
                }
                Ok(soar_env) if soar_env == "test" => {
                    info!("Running in test mode on port {}", port);
                    port
                }
                Ok(soar_env) => {
                    info!(
                        "Running in {} mode, overriding port to 1337 (development default)",
                        soar_env
                    );
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
        Commands::VerifyRuntime { .. } => {
            // This should never be reached due to early return above
            unreachable!("VerifyRuntime should be handled before database setup")
        }
        Commands::Migrate {} => {
            // Migrations are already run by setup_diesel_database()
            // Send email notification and Sentry event
            info!("Database migrations completed successfully");
            info!("All pending migrations have been applied");

            // Query and display the latest migration version
            use diesel::prelude::*;
            match diesel_pool.get() {
                Ok(mut conn) => {
                    #[derive(QueryableByName)]
                    struct MigrationVersion {
                        #[diesel(sql_type = diesel::sql_types::Text)]
                        version: String,
                    }

                    match diesel::sql_query(
                        "SELECT version FROM __diesel_schema_migrations ORDER BY version DESC LIMIT 1",
                    )
                    .get_result::<MigrationVersion>(&mut conn)
                    {
                        Ok(latest) => {
                            info!("Latest migration applied: {}", latest.version);
                        }
                        Err(e) => {
                            warn!("Could not query latest migration version: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Could not get database connection to query latest migration: {}",
                        e
                    );
                }
            }

            // Send success notifications (email and Sentry)
            if let Ok(email_config) = MigrationEmailConfig::from_env() {
                let (applied_migrations, duration_secs) = migration_info.unwrap_or((vec![], 0.0));
                let report = MigrationReport::success(applied_migrations, duration_secs);
                if let Err(e) = send_migration_email_report(&email_config, &report) {
                    warn!("Failed to send migration success email: {}", e);
                }
            } else {
                warn!("Email configuration not available, skipping migration email notification");
            }

            // Send Sentry success event
            if sentry::Hub::current().client().is_some() {
                sentry::configure_scope(|scope| {
                    scope.set_tag("migration", "true");
                    scope.set_tag("environment", env::var("SOAR_ENV").unwrap_or_default());
                    scope.set_tag("type", "database_migration");
                });
                sentry::capture_message(
                    &format!(
                        "Database migration completed successfully for {}",
                        env::var("SOAR_ENV").unwrap_or_else(|_| "development".to_string())
                    ),
                    sentry::Level::Info,
                );
            }

            Ok(())
        }
        Commands::AggregateCoverage {
            start_date,
            end_date,
            resolutions,
        } => {
            commands::aggregate_coverage(diesel_pool, start_date, end_date, resolutions.clone())
                .await
        }
        Commands::SeedTestData {} => handle_seed_test_data(&diesel_pool).await,
        Commands::DumpUnifiedDdb { .. } => {
            // This should never be reached due to early return above
            unreachable!("DumpUnifiedDdb should be handled before database setup")
        }
    }
}
