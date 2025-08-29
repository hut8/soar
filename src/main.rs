pub mod ddb;
pub mod ogn_aprs_aircraft;
pub mod aprs_client;
pub mod faa_data;

use sqlx::postgres::PgPool;
use std::env;

// Embed migrations into the binary
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Get the database URL from environment variables
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in environment variables");

    let mut device_db = ddb::DeviceDatabase::new();

    // Create a connection pool to the PostgreSQL database
    let pool = match PgPool::connect(&database_url).await {
        Ok(pool) => {
            println!("Successfully connected to PostgreSQL database");
            pool
        }
        Err(e) => {
            eprintln!("Failed to connect to PostgreSQL database: {e}");
            std::process::exit(1);
        }
    };

    // Run pending migrations
    println!("Running database migrations...");
    match MIGRATOR.run(&pool).await {
        Ok(_) => {
            println!("Database migrations completed successfully");
        }
        Err(e) => {
            eprintln!("Failed to run database migrations: {e}");
            std::process::exit(1);
        }
    }

    // Your application logic goes here
    println!("Application started with database connection");

    // Close the connection pool when done
    pool.close().await;
}
