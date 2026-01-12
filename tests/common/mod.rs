//! Common test utilities for database-backed integration tests
//!
//! This module provides helpers for creating isolated test databases
//! using PostgreSQL templates for fast, parallel test execution.
//!
//! # Overview
//!
//! The `TestDatabase` struct creates a unique PostgreSQL database for each test
//! from the `soar_test_template` template database. This provides complete isolation
//! between tests, allowing them to run in parallel without interference.
//!
//! Migrations are automatically run on the template database before the first test,
//! ensuring the schema is always up-to-date.
//!
//! # Usage
//!
//! ```no_run
//! use common::TestDatabase;
//!
//! #[tokio::test]
//! async fn my_test() {
//!     let test_db = TestDatabase::new()
//!         .await
//!         .expect("Failed to create test database");
//!     let pool = test_db.pool();
//!
//!     // Use the pool for test operations
//!     // Database is automatically dropped when test_db goes out of scope
//! }
//! ```

use anyhow::{Context, Result};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use std::sync::Once;
use std::thread;
use std::time::Duration;

// Embed migrations at compile time
const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

// Ensure migrations only run once per test session
static MIGRATIONS_RUN: Once = Once::new();

type PgPool = Pool<ConnectionManager<PgConnection>>;

/// Ensures the template database exists and has the latest migrations applied.
/// This is called automatically by `TestDatabase::new()` and only runs once per test session.
fn ensure_template_migrated() {
    MIGRATIONS_RUN.call_once(|| {
        dotenvy::dotenv().ok();

        let base_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/soar_test".to_string());

        // Connect to postgres database for admin operations
        let admin_url = base_url
            .replace("/soar_test", "/postgres")
            .replace("/soar_test_template", "/postgres");

        let template_url = base_url.replace("/soar_test", "/soar_test_template");

        // Create template database if it doesn't exist
        if let Ok(mut admin_conn) = PgConnection::establish(&admin_url) {
            // Check if template exists
            let exists: Result<bool, _> = diesel::sql_query(
                "SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = 'soar_test_template')",
            )
            .get_result::<TemplateExists>(&mut admin_conn)
            .map(|r| r.exists);

            if exists != Ok(true) {
                // Create template database
                let _ = diesel::sql_query("CREATE DATABASE soar_test_template")
                    .execute(&mut admin_conn);

                // Create PostGIS extension
                if let Ok(mut template_conn) = PgConnection::establish(&template_url) {
                    let _ = diesel::sql_query("CREATE EXTENSION IF NOT EXISTS postgis")
                        .execute(&mut template_conn);
                    // Explicitly close connection
                    drop(template_conn);
                }
            }

            // Unmark as template temporarily to allow connections for migrations
            let _ = diesel::sql_query(
                "UPDATE pg_database SET datistemplate = FALSE, datallowconn = TRUE \
                 WHERE datname = 'soar_test_template'",
            )
            .execute(&mut admin_conn);

            // Explicitly close admin connection
            drop(admin_conn);
        }

        // Run pending migrations on template
        if let Ok(mut template_conn) = PgConnection::establish(&template_url) {
            match template_conn.run_pending_migrations(MIGRATIONS) {
                Ok(applied) => {
                    if !applied.is_empty() {
                        eprintln!("Applied {} migration(s) to test template", applied.len());
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to run migrations on template: {}", e);
                }
            }

            // Explicitly drop the connection to ensure it's closed before re-marking the template
            drop(template_conn);
        }

        // Small delay to ensure the connection is fully cleaned up by PostgreSQL
        // This prevents "source database is being accessed by other users" errors
        // when tests run in parallel
        thread::sleep(Duration::from_millis(50));

        // Re-mark as template
        if let Ok(mut admin_conn) = PgConnection::establish(&admin_url) {
            let _ = diesel::sql_query(
                "UPDATE pg_database SET datistemplate = TRUE, datallowconn = FALSE \
                 WHERE datname = 'soar_test_template'",
            )
            .execute(&mut admin_conn);

            // Explicitly close admin connection
            drop(admin_conn);
        }

        // Final delay to ensure template marking is fully processed
        thread::sleep(Duration::from_millis(20));
    });
}

#[derive(QueryableByName)]
struct TemplateExists {
    #[diesel(sql_type = diesel::sql_types::Bool)]
    exists: bool,
}

/// Manages an isolated test database created from a template.
///
/// Each `TestDatabase` instance creates a unique database from the
/// `soar_test_template` template database. The database is automatically
/// dropped when this struct is dropped, ensuring cleanup even on test panic.
///
/// # Template Database
///
/// The template database must exist and be initialized with migrations before
/// running tests. Create it by running:
///
/// ```bash
/// ./scripts/setup-test-template.sh
/// ```
///
/// # Database Lifecycle
///
/// 1. `new()` creates database: `CREATE DATABASE soar_test_<random> TEMPLATE soar_test_template`
/// 2. Test runs with isolated database
/// 3. `Drop` executes: `DROP DATABASE soar_test_<random> WITH (FORCE)`
///
/// # PostgreSQL Version
///
/// Requires PostgreSQL 13+ for `DROP DATABASE ... WITH (FORCE)` support.
pub struct TestDatabase {
    /// The name of the test database (e.g., "soar_test_a7b3f9x2k4m1")
    db_name: String,
    /// Connection pool for the test database
    pool: PgPool,
    /// Admin database URL for cleanup operations (connects to 'postgres' database)
    admin_url: String,
}

impl TestDatabase {
    /// Creates a new isolated test database from the template.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Template database `soar_test_template` doesn't exist
    /// - Database creation fails (permissions, disk space, etc.)
    /// - Connection to new database fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// let test_db = TestDatabase::new().await?;
    /// let pool = test_db.pool();
    /// ```
    pub async fn new() -> Result<Self> {
        // Ensure template database has latest migrations (runs once per test session)
        ensure_template_migrated();

        // Load environment variables
        dotenvy::dotenv().ok();

        // Get base database URL from environment
        let base_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/soar_test".to_string());

        // Parse the URL to extract components
        let (admin_url, db_name) = Self::generate_database_info(&base_url)?;

        // Create the database from template (blocking operation)
        Self::create_database(&admin_url, &db_name)
            .await
            .context("Failed to create test database from template")?;

        // Build connection URL for the new database
        let test_db_url = Self::build_database_url(&base_url, &db_name);

        // Create connection pool for the test database
        let manager = ConnectionManager::<PgConnection>::new(&test_db_url);
        let pool = Pool::builder()
            .max_size(10) // Reasonable pool size for tests
            .build(manager)
            .with_context(|| format!("Failed to create connection pool for {}", db_name))?;

        Ok(TestDatabase {
            db_name,
            pool,
            admin_url,
        })
    }

    /// Returns a clone of the connection pool for this test database.
    ///
    /// The pool can be cloned and passed around within the test.
    pub fn pool(&self) -> PgPool {
        self.pool.clone()
    }

    /// Returns the database name for debugging purposes.
    #[allow(dead_code)]
    pub fn name(&self) -> &str {
        &self.db_name
    }

    /// Generates a unique database name and admin URL.
    ///
    /// Returns (admin_url, db_name) tuple.
    fn generate_database_info(base_url: &str) -> Result<(String, String)> {
        // Generate a random 16-character hex suffix using hex (simple and portable)
        use rand::RngCore;
        let mut rng = rand::rng();
        let random_bytes: u64 = rng.next_u64();
        let suffix = format!("{:016x}", random_bytes);

        let db_name = format!("soar_test_{}", suffix);

        // Build admin URL (replaces database name with 'postgres')
        let admin_url = base_url
            .replace("/soar_test", "/postgres")
            .replace("/soar_test_template", "/postgres");

        Ok((admin_url, db_name))
    }

    /// Builds a database URL for the test database.
    fn build_database_url(base_url: &str, db_name: &str) -> String {
        // Replace the database name in the base URL
        base_url
            .replace("/soar_test", &format!("/{}", db_name))
            .replace("/soar_test_template", &format!("/{}", db_name))
    }

    /// Creates a new database from the template.
    ///
    /// Uses a file-based lock to ensure only one database is created at a time.
    /// This prevents "source database is being accessed by other users" errors
    /// when multiple tests try to clone from the template simultaneously.
    async fn create_database(admin_url: &str, db_name: &str) -> Result<()> {
        use diesel::Connection;
        use fs2::FileExt;
        use std::fs::OpenOptions;

        let admin_url = admin_url.to_string();
        let db_name = db_name.to_string();

        // Run blocking database creation in a blocking task
        tokio::task::spawn_blocking(move || {
            // Acquire file-based lock to serialize template cloning
            let lock_path = std::env::temp_dir().join("soar_test_template.lock");
            let lock_file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(false)
                .open(&lock_path)
                .context("Failed to create lock file for template database cloning")?;

            // Acquire exclusive lock (blocks until available)
            lock_file
                .lock_exclusive()
                .context("Failed to acquire lock for template database cloning")?;

            // Connect to postgres database for admin operations
            let mut conn = PgConnection::establish(&admin_url).context(
                "Failed to connect to PostgreSQL for database creation. Is PostgreSQL running?",
            )?;

            // Terminate all connections to the template database before cloning
            // This prevents "source database is being accessed by other users" errors
            let terminate_sql = "
                SELECT pg_terminate_backend(pg_stat_activity.pid)
                FROM pg_stat_activity
                WHERE pg_stat_activity.datname = 'soar_test_template'
                  AND pid <> pg_backend_pid()
            ";

            diesel::sql_query(terminate_sql)
                .execute(&mut conn)
                .context("Failed to terminate connections to template database")?;

            // Create database from template
            // Note: db_name is randomly generated alphanumeric, safe from SQL injection
            let create_sql = format!(
                "CREATE DATABASE \"{}\" TEMPLATE soar_test_template",
                db_name
            );

            let result = diesel::sql_query(&create_sql)
                .execute(&mut conn)
                .with_context(|| {
                    format!(
                        "Failed to create database '{}' from template.\n\
                         \n\
                         The template database 'soar_test_template' may not exist.\n\
                         Run: ./scripts/setup-test-template.sh\n\
                         \n\
                         This creates the template database with all migrations applied.",
                        db_name
                    )
                });

            // Lock is automatically released when lock_file is dropped
            drop(lock_file);

            result?;
            Ok::<(), anyhow::Error>(())
        })
        .await
        .context("Database creation task panicked")?
    }

    /// Drops the test database.
    ///
    /// This is called automatically by the Drop trait, but can also be called
    /// explicitly if early cleanup is desired.
    fn cleanup(&self) {
        use diesel::Connection;
        use std::panic::AssertUnwindSafe;

        // Attempt cleanup but don't panic on failure
        let db_name = self.db_name.clone();
        let admin_url = self.admin_url.clone();

        let result = std::panic::catch_unwind(AssertUnwindSafe(move || {
            let mut conn = PgConnection::establish(&admin_url).ok()?;

            // PostgreSQL 13+ supports WITH (FORCE) to terminate active connections
            // Note: db_name is randomly generated alphanumeric, safe from SQL injection
            let drop_sql = format!("DROP DATABASE IF EXISTS \"{}\" WITH (FORCE)", db_name);

            diesel::sql_query(&drop_sql).execute(&mut conn).ok()
        }));

        if result.is_err() {
            eprintln!(
                "Warning: Failed to drop test database '{}'. \
                 You may need to manually clean up: DROP DATABASE {};",
                self.db_name, self.db_name
            );
        }
    }
}

impl Drop for TestDatabase {
    /// Automatically drops the test database when TestDatabase goes out of scope.
    ///
    /// This ensures cleanup happens even if the test panics. Uses `WITH (FORCE)`
    /// to forcibly disconnect any active connections (requires PostgreSQL 13+).
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_database_info() {
        let base_url = "postgresql://user:pass@localhost:5432/soar_test";
        let (admin_url, db_name) = TestDatabase::generate_database_info(base_url).unwrap();

        assert_eq!(admin_url, "postgresql://user:pass@localhost:5432/postgres");
        assert!(db_name.starts_with("soar_test_"));
        assert_eq!(db_name.len(), "soar_test_".len() + 16); // 16 hex characters
    }

    #[test]
    fn test_build_database_url() {
        let base_url = "postgresql://user:pass@localhost:5432/soar_test";
        let db_name = "soar_test_abc123def456";
        let result = TestDatabase::build_database_url(base_url, db_name);

        assert_eq!(
            result,
            "postgresql://user:pass@localhost:5432/soar_test_abc123def456"
        );
    }

    #[test]
    fn test_generate_unique_names() {
        let base_url = "postgresql://localhost/soar_test";
        let (_, name1) = TestDatabase::generate_database_info(base_url).unwrap();
        let (_, name2) = TestDatabase::generate_database_info(base_url).unwrap();

        // Names should be different (extremely high probability)
        assert_ne!(name1, name2);
    }
}
