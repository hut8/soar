mod ddb;
mod ogn_aprs_aircraft;
mod aprs_client;

use sqlx::postgres::PgPool;
use std::env;

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

    // Your application logic goes here
    println!("Application started with database connection");

    // Close the connection pool when done
    pool.close().await;
}
