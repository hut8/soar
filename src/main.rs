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
use soar::elevation::ElevationDB;
use soar::fix_processor::FixProcessor;
use soar::fixes_repo::FixesRepository;
use soar::flight_tracker::FlightTracker;
use soar::instance_lock::InstanceLock;
use soar::packet_processors::{
    AircraftPositionProcessor, GenericProcessor, PacketRouter, ReceiverPositionProcessor,
    ReceiverStatusProcessor, ServerStatusProcessor,
};
use soar::queue_config::{
    AGL_DATABASE_QUEUE_SIZE, AIRCRAFT_QUEUE_SIZE, ELEVATION_QUEUE_SIZE,
    RECEIVER_POSITION_QUEUE_SIZE, RECEIVER_STATUS_QUEUE_SIZE, SERVER_STATUS_QUEUE_SIZE,
    queue_warning_threshold,
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
/// Handle the ingest-aprs subcommand - connect to APRS-IS and publish to NATS JetStream
#[tracing::instrument(skip_all)]
async fn handle_ingest_aprs(
    server: String,
    port: u16,
    callsign: String,
    filter: Option<String>,
    max_retries: u32,
    retry_delay: u64,
    nats_url: String,
) -> Result<()> {
    use soar::queue_config::{
        APRS_RAW_STREAM, APRS_RAW_STREAM_STAGING, APRS_RAW_SUBJECT, APRS_RAW_SUBJECT_STAGING,
    };

    sentry::configure_scope(|scope| {
        scope.set_tag("operation", "ingest-aprs");
    });

    // Determine environment and use appropriate stream/subject names
    // Production: "APRS_RAW" and "aprs.raw"
    // Staging: "STAGING_APRS_RAW" and "staging.aprs.raw"
    let is_production = env::var("SOAR_ENV")
        .map(|env| env == "production")
        .unwrap_or(false);

    let (final_stream_name, final_subject) = if is_production {
        (APRS_RAW_STREAM.to_string(), APRS_RAW_SUBJECT.to_string())
    } else {
        (
            APRS_RAW_STREAM_STAGING.to_string(),
            APRS_RAW_SUBJECT_STAGING.to_string(),
        )
    };

    info!(
        "Starting APRS ingestion service - server: {}:{}, NATS: {}, stream: {}, subject: {}",
        server, port, nats_url, final_stream_name, final_subject
    );

    info!(
        "Environment: {}, using stream '{}' and subject '{}'",
        if is_production {
            "production"
        } else {
            "staging"
        },
        final_stream_name,
        final_subject
    );

    // Start metrics server in production mode
    if is_production {
        info!("Starting metrics server on port 9093");
        tokio::spawn(
            async {
                soar::metrics::start_metrics_server(9093).await;
            }
            .instrument(tracing::info_span!("metrics_server")),
        );

        // Initialize all metrics to zero so they appear in Prometheus even if no events occur
        soar::metrics::initialize_aprs_ingest_metrics();
        info!("Initialized APRS ingest metrics to default values");
    }

    // Initialize all APRS ingester metrics to zero so they appear in Grafana even before events occur
    info!("Initializing APRS ingester metrics...");

    // Connection metrics
    metrics::counter!("aprs.connection.established").absolute(0);
    metrics::counter!("aprs.connection.ended").absolute(0);
    metrics::counter!("aprs.connection.failed").absolute(0);
    metrics::counter!("aprs.connection.operation_failed").absolute(0);
    metrics::gauge!("aprs.connection.connected").set(0.0);

    // Keepalive metrics
    metrics::counter!("aprs.keepalive.sent").absolute(0);

    // Message processing metrics
    metrics::counter!("aprs.raw_message.processed").absolute(0);
    metrics::counter!("aprs.raw_message_queue.full").absolute(0);
    metrics::gauge!("aprs.raw_message_queue.depth").set(0.0);

    // JetStream publishing metrics
    metrics::counter!("aprs.jetstream.published").absolute(0);
    metrics::counter!("aprs.jetstream.publish_error").absolute(0);
    metrics::gauge!("aprs.jetstream.queue_depth").set(0.0);

    info!("APRS ingester metrics initialized");

    // Acquire instance lock to prevent multiple ingest instances from running
    let lock_name = if is_production {
        "aprs-ingest-production"
    } else {
        "aprs-ingest-dev"
    };
    let _lock = InstanceLock::new(lock_name)
        .context("Failed to acquire instance lock - is another aprs-ingest instance running?")?;
    info!("Instance lock acquired for {}", lock_name);

    // Connect to NATS and set up JetStream
    info!("Connecting to NATS at {}...", nats_url);
    let nats_client = async_nats::ConnectOptions::new()
        .name("soar-aprs-ingester")
        .connect(&nats_url)
        .await
        .context("Failed to connect to NATS")?;
    info!("Connected to NATS successfully");

    let jetstream = async_nats::jetstream::new(nats_client);

    // Create or get the stream for raw APRS messages
    info!(
        "Setting up JetStream stream '{}' for subject '{}'...",
        final_stream_name, final_subject
    );

    let stream = match jetstream.get_stream(&final_stream_name).await {
        Ok(stream) => {
            info!("JetStream stream '{}' already exists", final_stream_name);
            stream
        }
        Err(_) => {
            info!("Creating new JetStream stream '{}'...", final_stream_name);
            jetstream
                .create_stream(async_nats::jetstream::stream::Config {
                    name: final_stream_name.clone(),
                    subjects: vec![final_subject.clone()],
                    max_messages: 10_000_000, // Store up to 10M messages
                    max_bytes: 10 * 1024 * 1024 * 1024, // 10GB max
                    max_age: std::time::Duration::from_secs(24 * 60 * 60), // 24 hours retention
                    storage: async_nats::jetstream::stream::StorageType::File,
                    num_replicas: 1,
                    ..Default::default()
                })
                .await
                .context("Failed to create JetStream stream")?
        }
    };

    info!(
        "JetStream stream ready - will publish to subject '{}'",
        final_subject
    );

    // Create a simplified APRS client that publishes to JetStream
    let config = AprsClientConfigBuilder::new()
        .server(server)
        .port(port)
        .callsign(callsign)
        .filter(filter)
        .max_retries(max_retries)
        .retry_delay_seconds(retry_delay)
        .build();

    // Create a custom "router" that just publishes to JetStream
    let jetstream_publisher = soar::aprs_jetstream_publisher::JetStreamPublisher::new(
        jetstream,
        final_subject.clone(),
        stream,
    );

    let mut client = AprsClient::new(config);

    info!("Starting APRS client for ingestion...");
    client
        .start_jetstream(jetstream_publisher)
        .await
        .context("APRS ingestion client failed")?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip_all)]
async fn handle_run(archive_dir: Option<String>, nats_url: String) -> Result<()> {
    use soar::queue_config::{
        APRS_RAW_STREAM, APRS_RAW_STREAM_STAGING, APRS_RAW_SUBJECT, APRS_RAW_SUBJECT_STAGING,
        SOAR_RUN_CONSUMER, SOAR_RUN_CONSUMER_STAGING,
    };

    sentry::configure_scope(|scope| {
        scope.set_tag("operation", "run");
    });

    // Determine environment and use appropriate stream/subject/consumer names
    let is_production = env::var("SOAR_ENV")
        .map(|env| env == "production")
        .unwrap_or(false);

    let (final_stream_name, final_subject, final_consumer_name) = if is_production {
        (
            APRS_RAW_STREAM.to_string(),
            APRS_RAW_SUBJECT.to_string(),
            SOAR_RUN_CONSUMER.to_string(),
        )
    } else {
        (
            APRS_RAW_STREAM_STAGING.to_string(),
            APRS_RAW_SUBJECT_STAGING.to_string(),
            SOAR_RUN_CONSUMER_STAGING.to_string(),
        )
    };

    info!(
        "Starting APRS processing service consuming from JetStream stream: {}, subject: {}, consumer: {}",
        final_stream_name, final_subject, final_consumer_name
    );

    // Start metrics server in the background
    if is_production {
        info!("Starting metrics server on port 9091");
        tokio::spawn(
            async {
                soar::metrics::start_metrics_server(9091).await;
            }
            .instrument(tracing::info_span!("metrics_server")),
        );

        // Initialize all metrics to zero so they appear in Prometheus even if no events occur
        soar::metrics::initialize_run_metrics();
        info!("Initialized metrics to default values");
    }

    // Initialize all soar-run metrics to zero so they appear in Grafana even before events occur
    info!("Initializing soar-run metrics...");

    // JetStream consumer metrics
    metrics::counter!("aprs.jetstream.consumed").absolute(0);
    metrics::counter!("aprs.jetstream.consumer_error").absolute(0);

    // Receiver cache metrics
    metrics::counter!("generic_processor.receiver_cache.hit").absolute(0);
    metrics::counter!("generic_processor.receiver_cache.miss").absolute(0);

    // Flight tracker metrics
    metrics::counter!("flight_tracker_timeouts_detected").absolute(0);

    info!("soar-run metrics initialized");

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

    // Log elevation data storage path
    let elevation_path =
        env::var("ELEVATION_DATA_PATH").unwrap_or_else(|_| "/var/soar/elevation".to_string());
    info!("Elevation data path: {}", elevation_path);

    info!(
        "Environment: {}, using stream '{}', subject '{}', consumer '{}'",
        if is_production {
            "production"
        } else {
            "staging"
        },
        final_stream_name,
        final_subject,
        final_consumer_name
    );

    // Create FlightTracker
    let flight_tracker = FlightTracker::new(&diesel_pool);

    // Initialize flight tracker from database:
    // 1. Timeout old incomplete flights (older than 1 hour)
    // 2. Load recent active flights into memory
    let timeout_duration = chrono::Duration::hours(1);
    match flight_tracker
        .initialize_from_database(timeout_duration)
        .await
    {
        Ok((timed_out, loaded)) => {
            info!(
                "Flight tracker initialized: {} flights timed out, {} flights loaded",
                timed_out, loaded
            );
        }
        Err(e) => {
            warn!("Failed to initialize flight tracker from database: {}", e);
        }
    }

    // Start flight timeout checker (every 60 seconds)
    flight_tracker.start_timeout_checker(60);

    // Create separate bounded channel for elevation/AGL calculations
    // This prevents elevation lookups (which can be slow) from blocking the main processing queue
    let (elevation_tx, elevation_rx) =
        tokio::sync::mpsc::channel::<soar::elevation::ElevationTask>(ELEVATION_QUEUE_SIZE);

    info!("Created bounded elevation processing queue with capacity 1,000");

    // Create separate bounded channel for AGL database updates
    // This separates the fast elevation calculation from the slower database updates
    // and allows batching of database writes for much better throughput
    let (agl_db_tx, agl_db_rx) =
        tokio::sync::mpsc::channel::<soar::elevation::AglDatabaseTask>(AGL_DATABASE_QUEUE_SIZE);

    info!("Created bounded AGL database update queue with capacity 10,000");

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
            info!("Created FixProcessor with NATS publisher");
            processor_with_nats.with_elevation_channel(elevation_tx.clone())
        }
        Err(e) => {
            warn!(
                "Failed to create FixProcessor with NATS ({}), falling back to processor without NATS",
                e
            );
            FixProcessor::with_flight_tracker(diesel_pool.clone(), flight_tracker.clone())
                .with_elevation_channel(elevation_tx.clone())
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

    // Create GenericProcessor for archiving, receiver identification, and APRS message insertion
    let generic_processor = if let Some(archive_path) = archive_dir.clone() {
        let archive_service = soar::aprs_client::ArchiveService::new(archive_path).await?;
        GenericProcessor::new(receiver_repo.clone(), aprs_messages_repo)
            .with_archive_service(archive_service)
    } else {
        GenericProcessor::new(receiver_repo.clone(), aprs_messages_repo)
    };

    // Create receiver status processor for receiver status messages
    let receiver_status_processor =
        ReceiverStatusProcessor::new(receiver_status_repo, receiver_repo.clone());

    // Create receiver position processor for receiver position messages
    let receiver_position_processor = ReceiverPositionProcessor::new(receiver_repo.clone());

    // Create aircraft position processor
    // Note: FlightDetectionProcessor is now handled inside FixProcessor
    let aircraft_position_processor =
        AircraftPositionProcessor::new().with_fix_processor(fix_processor.clone());

    info!(
        "Setting up APRS client with PacketRouter - archive directory: {:?}, NATS URL: {}",
        archive_dir, nats_url
    );

    // Create bounded channels for per-processor queues
    // Aircraft positions: highest capacity due to high volume and heavy processing
    let (aircraft_tx, aircraft_rx) = tokio::sync::mpsc::channel(AIRCRAFT_QUEUE_SIZE);
    info!(
        "Created aircraft position queue with capacity {}",
        AIRCRAFT_QUEUE_SIZE
    );

    // Receiver status: high capacity
    let (receiver_status_tx, receiver_status_rx) =
        tokio::sync::mpsc::channel(RECEIVER_STATUS_QUEUE_SIZE);
    info!(
        "Created receiver status queue with capacity {}",
        RECEIVER_STATUS_QUEUE_SIZE
    );

    // Receiver position: medium capacity
    let (receiver_position_tx, receiver_position_rx) =
        tokio::sync::mpsc::channel(RECEIVER_POSITION_QUEUE_SIZE);
    info!(
        "Created receiver position queue with capacity {}",
        RECEIVER_POSITION_QUEUE_SIZE
    );

    // Server status: low capacity (rare messages)
    let (server_status_tx, server_status_rx) = tokio::sync::mpsc::channel(SERVER_STATUS_QUEUE_SIZE);
    info!(
        "Created server status queue with capacity {}",
        SERVER_STATUS_QUEUE_SIZE
    );

    // Create PacketRouter with per-processor queues
    let packet_router = PacketRouter::new(generic_processor)
        .with_aircraft_position_queue(aircraft_tx)
        .with_receiver_status_queue(receiver_status_tx)
        .with_receiver_position_queue(receiver_position_tx)
        .with_server_status_queue(server_status_tx);

    info!("Created PacketRouter with per-processor queues");

    // Initialize queue drop counters so they're always exported to Prometheus
    // even when queues are healthy and not dropping packets
    metrics::counter!("aprs.raw_message_queue.full").absolute(0);
    metrics::counter!("aprs.aircraft_queue.full").absolute(0);
    metrics::counter!("aprs.aircraft_queue.closed").absolute(0);
    metrics::counter!("aprs.receiver_status_queue.full").absolute(0);
    metrics::counter!("aprs.receiver_status_queue.closed").absolute(0);
    metrics::counter!("aprs.receiver_position_queue.full").absolute(0);
    metrics::counter!("aprs.receiver_position_queue.closed").absolute(0);
    metrics::counter!("aprs.server_status_queue.full").absolute(0);
    metrics::counter!("aprs.server_status_queue.closed").absolute(0);

    // Initialize APRS connection metrics
    metrics::gauge!("aprs.connection.connected").set(0.0);
    metrics::counter!("aprs.connection.established").absolute(0);
    metrics::counter!("aprs.connection.ended").absolute(0);
    metrics::counter!("aprs.connection.failed").absolute(0);
    metrics::counter!("aprs.connection.operation_failed").absolute(0);
    metrics::counter!("aprs.keepalive.sent").absolute(0);

    // Initialize JetStream metrics
    metrics::gauge!("aprs.jetstream.queue_depth").set(0.0);
    metrics::counter!("aprs.jetstream.published").absolute(0);
    metrics::counter!("aprs.jetstream.publish_error").absolute(0);
    metrics::counter!("aprs.jetstream.consumed").absolute(0);
    metrics::counter!("aprs.jetstream.decode_error").absolute(0);
    metrics::counter!("aprs.jetstream.ack_error").absolute(0);
    metrics::counter!("aprs.jetstream.process_error").absolute(0);
    metrics::counter!("aprs.jetstream.receive_error").absolute(0);

    // Initialize NATS publisher error counter
    metrics::counter!("nats_publisher_errors").absolute(0);

    // Spawn AGL batch database writer
    // This worker receives calculated AGL values and writes them to database in batches
    // Batching dramatically reduces database load (100+ individual UPDATEs become 1 batch UPDATE)
    let batch_writer_fixes_repo = FixesRepository::new(diesel_pool.clone());
    tokio::spawn(
        async move {
            soar::agl_batch_writer::batch_writer_task(agl_db_rx, batch_writer_fixes_repo).await;
        }
        .instrument(tracing::info_span!("agl_batch_writer")),
    );
    info!("Spawned AGL batch database writer (batch size: 100, timeout: 5s)");

    // Spawn multiple dedicated elevation processing workers
    // These workers handle AGL calculations separately to prevent them from blocking main processing
    // Using multiple workers allows parallel elevation lookups, which can be I/O intensive
    // The ElevationDB uses Arc<Mutex<LruCache>> internally, so all workers share the same cache
    let num_elevation_workers = 8;
    let elevation_db = flight_tracker.elevation_db().clone();

    // Wrap elevation receiver in Arc<Mutex> to share among elevation workers
    let shared_elevation_rx = std::sync::Arc::new(tokio::sync::Mutex::new(elevation_rx));

    for elevation_worker_id in 0..num_elevation_workers {
        let worker_elevation_rx = shared_elevation_rx.clone();
        let worker_elevation_db = elevation_db.clone();
        let worker_agl_db_tx = agl_db_tx.clone();

        tokio::spawn(
            async move {
                loop {
                    // Lock the receiver and try to get an elevation task
                    let task = {
                        let mut rx = worker_elevation_rx.lock().await;
                        rx.recv().await
                    };

                    match task {
                        Some(task) => {
                            let start = std::time::Instant::now();

                            // Calculate AGL (no database update here, just calculation)
                            let agl = soar::flight_tracker::altitude::calculate_altitude_agl(
                                &worker_elevation_db,
                                &task.fix,
                            )
                            .await;

                            let duration = start.elapsed();
                            metrics::histogram!("aprs.elevation.duration_ms")
                                .record(duration.as_millis() as f64);
                            metrics::counter!("aprs.elevation.processed", "worker_id" => elevation_worker_id.to_string()).increment(1);

                            // Send calculated AGL to database batch writer
                            let agl_task = soar::elevation::AglDatabaseTask {
                                fix_id: task.fix_id,
                                altitude_agl_feet: agl,
                            };

                            if let Err(e) = worker_agl_db_tx.try_send(agl_task) {
                                match e {
                                    tokio::sync::mpsc::error::TrySendError::Full(_) => {
                                        warn!("AGL database queue is FULL (10,000 tasks) - dropping database update for fix {}", task.fix_id);
                                        metrics::counter!("agl_db_queue.dropped_total").increment(1);
                                    }
                                    tokio::sync::mpsc::error::TrySendError::Closed(_) => {
                                        warn!("AGL database queue is closed");
                                    }
                                }
                            }
                        }
                        None => {
                            // Channel closed, exit worker
                            break;
                        }
                    }
                }
            }
            .instrument(tracing::info_span!("elevation_worker", worker_id = elevation_worker_id)),
        );
    }

    info!(
        "Spawned {} dedicated elevation processing workers (sharing elevation and dataset caches)",
        num_elevation_workers
    );

    // Spawn AGL backfill background task
    // This task runs continuously, backfilling AGL altitudes for old fixes that were missed
    // It gets its own dedicated ElevationDB with a larger GDAL dataset cache (1000 vs 100)
    // to avoid contention with real-time elevation processing
    let backfill_fixes_repo = FixesRepository::new(diesel_pool.clone());
    let backfill_elevation_db = ElevationDB::with_custom_cache_sizes(500_000, 1000)
        .context("Failed to create dedicated ElevationDB for AGL backfill")?;
    info!(
        "Created dedicated ElevationDB for AGL backfill (dataset cache: 1000, elevation cache: 500,000)"
    );
    tokio::spawn(
        async move {
            soar::agl_backfill::agl_backfill_task(backfill_fixes_repo, backfill_elevation_db).await;
        }
        .instrument(tracing::info_span!("agl_backfill")),
    );
    info!("Spawned AGL backfill background task with dedicated elevation database");

    // Spawn dedicated worker pools for each processor type
    // Aircraft position workers (20 workers - heaviest processing due to FixProcessor + flight tracking)
    let num_aircraft_workers = 20;
    info!(
        "Spawning {} aircraft position workers",
        num_aircraft_workers
    );
    let shared_aircraft_rx = std::sync::Arc::new(tokio::sync::Mutex::new(aircraft_rx));
    for worker_id in 0..num_aircraft_workers {
        let worker_rx = shared_aircraft_rx.clone();
        let processor = aircraft_position_processor.clone();
        tokio::spawn(
            async move {
                loop {
                    let task = {
                        let mut rx = worker_rx.lock().await;
                        rx.recv().await
                    };
                    match task {
                        Some((packet, context)) => {
                            let start = std::time::Instant::now();
                            processor.process_aircraft_position(&packet, context).await;
                            let duration = start.elapsed();
                            metrics::histogram!("aprs.aircraft.duration_ms")
                                .record(duration.as_millis() as f64);
                            metrics::counter!("aprs.aircraft.processed").increment(1);
                        }
                        None => break,
                    }
                }
            }
            .instrument(tracing::info_span!("aircraft_worker", worker_id)),
        );
    }

    // Receiver status workers (6 workers - medium processing)
    let num_receiver_status_workers = 6;
    info!(
        "Spawning {} receiver status workers",
        num_receiver_status_workers
    );
    let shared_receiver_status_rx =
        std::sync::Arc::new(tokio::sync::Mutex::new(receiver_status_rx));
    for worker_id in 0..num_receiver_status_workers {
        let worker_rx = shared_receiver_status_rx.clone();
        let processor = receiver_status_processor.clone();
        tokio::spawn(
            async move {
                loop {
                    let task = {
                        let mut rx = worker_rx.lock().await;
                        rx.recv().await
                    };
                    match task {
                        Some((packet, context)) => {
                            let start = std::time::Instant::now();
                            processor.process_status_packet(&packet, context).await;
                            let duration = start.elapsed();
                            metrics::histogram!("aprs.receiver_status.duration_ms")
                                .record(duration.as_millis() as f64);
                            metrics::counter!("aprs.receiver_status.processed").increment(1);
                        }
                        None => break,
                    }
                }
            }
            .instrument(tracing::info_span!("receiver_status_worker", worker_id)),
        );
    }

    // Receiver position workers (4 workers - light processing)
    let num_receiver_position_workers = 4;
    info!(
        "Spawning {} receiver position workers",
        num_receiver_position_workers
    );
    let shared_receiver_position_rx =
        std::sync::Arc::new(tokio::sync::Mutex::new(receiver_position_rx));
    for worker_id in 0..num_receiver_position_workers {
        let worker_rx = shared_receiver_position_rx.clone();
        let processor = receiver_position_processor.clone();
        tokio::spawn(
            async move {
                loop {
                    let task = {
                        let mut rx = worker_rx.lock().await;
                        rx.recv().await
                    };
                    match task {
                        Some((packet, context)) => {
                            let start = std::time::Instant::now();
                            processor.process_receiver_position(&packet, context).await;
                            let duration = start.elapsed();
                            metrics::histogram!("aprs.receiver_position.duration_ms")
                                .record(duration.as_millis() as f64);
                            metrics::counter!("aprs.receiver_position.processed").increment(1);
                        }
                        None => break,
                    }
                }
            }
            .instrument(tracing::info_span!("receiver_position_worker", worker_id)),
        );
    }

    // Server status workers (2 workers - very light processing)
    info!("Spawning 2 server status workers");
    let shared_server_status_rx = std::sync::Arc::new(tokio::sync::Mutex::new(server_status_rx));
    for worker_id in 0..2 {
        let worker_rx = shared_server_status_rx.clone();
        let processor = server_status_processor.clone();
        tokio::spawn(
            async move {
                loop {
                    let task = {
                        let mut rx = worker_rx.lock().await;
                        rx.recv().await
                    };
                    match task {
                        Some(message) => {
                            let start = std::time::Instant::now();
                            processor.process_server_message(&message).await;
                            let duration = start.elapsed();
                            metrics::histogram!("aprs.server_status.duration_ms")
                                .record(duration.as_millis() as f64);
                            metrics::counter!("aprs.server_status.processed").increment(1);
                        }
                        None => break,
                    }
                }
            }
            .instrument(tracing::info_span!("server_status_worker", worker_id)),
        );
    }

    // Spawn queue depth metrics reporter
    // Reports the depth of all processing queues to Prometheus every 10 seconds
    let metrics_aircraft_rx = shared_aircraft_rx.clone();
    let metrics_receiver_status_rx = shared_receiver_status_rx.clone();
    let metrics_receiver_position_rx = shared_receiver_position_rx.clone();
    let metrics_server_status_rx = shared_server_status_rx.clone();
    let metrics_elevation_rx = shared_elevation_rx.clone();
    tokio::spawn(
        async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            interval.tick().await; // First tick completes immediately

            loop {
                interval.tick().await;

                // Sample queue depths (non-blocking reads)
                let aircraft_depth = {
                    if let Ok(rx) = metrics_aircraft_rx.try_lock() {
                        rx.len()
                    } else {
                        0 // Skip if locked
                    }
                };
                let receiver_status_depth = {
                    if let Ok(rx) = metrics_receiver_status_rx.try_lock() {
                        rx.len()
                    } else {
                        0
                    }
                };
                let receiver_position_depth = {
                    if let Ok(rx) = metrics_receiver_position_rx.try_lock() {
                        rx.len()
                    } else {
                        0
                    }
                };
                let server_status_depth = {
                    if let Ok(rx) = metrics_server_status_rx.try_lock() {
                        rx.len()
                    } else {
                        0
                    }
                };
                let elevation_depth = {
                    if let Ok(rx) = metrics_elevation_rx.try_lock() {
                        rx.len()
                    } else {
                        0
                    }
                };

                // Report to Prometheus
                metrics::gauge!("aprs.aircraft_queue.depth").set(aircraft_depth as f64);
                metrics::gauge!("aprs.receiver_status_queue.depth")
                    .set(receiver_status_depth as f64);
                metrics::gauge!("aprs.receiver_position_queue.depth")
                    .set(receiver_position_depth as f64);
                metrics::gauge!("aprs.server_status_queue.depth").set(server_status_depth as f64);
                metrics::gauge!("aprs.elevation_queue.depth").set(elevation_depth as f64);

                // Warn if queues are building up (50% full)
                if aircraft_depth > queue_warning_threshold(AIRCRAFT_QUEUE_SIZE) {
                    warn!(
                        "Aircraft position queue building up: {} messages (50% full)",
                        aircraft_depth
                    );
                }
                if receiver_status_depth > queue_warning_threshold(RECEIVER_STATUS_QUEUE_SIZE) {
                    warn!(
                        "Receiver status queue building up: {} messages (50% full)",
                        receiver_status_depth
                    );
                }
                if receiver_position_depth > queue_warning_threshold(RECEIVER_POSITION_QUEUE_SIZE) {
                    warn!(
                        "Receiver position queue building up: {} messages (50% full)",
                        receiver_position_depth
                    );
                }
                if elevation_depth > queue_warning_threshold(ELEVATION_QUEUE_SIZE) {
                    warn!(
                        "Elevation queue building up: {} tasks (50% full)",
                        elevation_depth
                    );
                }
            }
        }
        .instrument(tracing::info_span!("queue_metrics_reporter")),
    );
    info!("Spawned queue depth metrics reporter (reports every 10 seconds to Prometheus)");

    // Set up JetStream consumer to read from durable queue
    info!("Connecting to NATS at {}...", nats_url);
    let nats_client = async_nats::ConnectOptions::new()
        .name("soar-run")
        .connect(&nats_url)
        .await
        .context("Failed to connect to NATS")?;
    info!("Connected to NATS successfully");

    let jetstream = async_nats::jetstream::new(nats_client);

    // Update JetStream connection status metric
    if is_production {
        metrics::gauge!("jetstream_connected").set(1.0);
    }

    // Create JetStream consumer
    info!(
        "Creating JetStream consumer '{}' for stream '{}', subject '{}'...",
        final_consumer_name, final_stream_name, final_subject
    );
    let consumer = soar::aprs_jetstream_consumer::JetStreamConsumer::new(
        jetstream,
        final_stream_name.clone(),
        final_subject.clone(),
        final_consumer_name.clone(),
    )
    .await
    .context("Failed to create JetStream consumer")?;

    info!("JetStream consumer ready, starting message processing...");
    info!("APRS client started. Press Ctrl+C to stop.");

    // Start consuming messages from JetStream
    // This runs indefinitely until the stream ends or an error occurs
    // Each message will be parsed and routed through PacketRouter
    consumer
        .consume(move |message| {
            let packet_router = packet_router.clone();
            async move {
                // Parse the APRS message using the same parser as the APRS client
                AprsClient::process_message(&message, &packet_router).await;
                Ok(())
            }
        })
        .await?;

    // If we reach here, the consumer stream ended unexpectedly
    warn!("JetStream consumer stopped unexpectedly");
    Ok(())
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
        Commands::IngestAprs {
            server,
            port,
            callsign,
            filter,
            max_retries,
            retry_delay,
            nats_url,
        } => {
            handle_ingest_aprs(
                server,
                port,
                callsign,
                filter,
                max_retries,
                retry_delay,
                nats_url,
            )
            .await
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

            handle_run(final_archive_dir, nats_url).await
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
        Commands::Resurrect { date, archive_path } => {
            soar::archive::handle_resurrect(diesel_pool, date, archive_path).await
        }
        Commands::VerifyRuntime {} => {
            // This should never be reached due to early return above
            unreachable!("VerifyRuntime should be handled before database setup")
        }
    }
}
