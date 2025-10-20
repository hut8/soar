use anyhow::Result;
use chrono::Local;
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use std::{env, fs, io};
use tracing::{info, warn};

/// Get the data directory based on environment
///
/// In production (SOAR_ENV=production), uses /tmp/soar/data-{date}
/// In development, uses ~/.cache/soar/data-{date}
fn get_data_directory(date: &str) -> Result<String> {
    let is_production = env::var("SOAR_ENV")
        .map(|env| env == "production")
        .unwrap_or(false);

    if is_production {
        Ok(format!("/tmp/soar/data-{}", date))
    } else {
        // Use ~/.cache/soar for development
        let home_dir =
            env::var("HOME").map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
        Ok(format!("{}/.cache/soar/data-{}", home_dir, date))
    }
}

async fn download_with_retry(
    client: &reqwest::Client,
    url: &str,
    max_retries: u32,
) -> Result<reqwest::Response> {
    let mut last_error = None;

    for attempt in 1..=max_retries {
        match client.get(url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    return Ok(response);
                } else {
                    let status = response.status();
                    last_error = Some(anyhow::anyhow!("HTTP error: {} for URL: {}", status, url));
                    if attempt < max_retries {
                        warn!(
                            "HTTP error {} for URL: {}, retrying (attempt {}/{})",
                            status, url, attempt, max_retries
                        );
                        tokio::time::sleep(tokio::time::Duration::from_secs(2u64.pow(attempt - 1)))
                            .await;
                    }
                }
            }
            Err(e) => {
                last_error = Some(anyhow::anyhow!("Request failed for URL {}: {}", url, e));
                if attempt < max_retries {
                    warn!(
                        "Request failed for URL: {}, retrying (attempt {}/{}): {}",
                        url, attempt, max_retries, e
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(2u64.pow(attempt - 1)))
                        .await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All retry attempts failed for URL: {}", url)))
}

async fn download_file_atomically(
    client: &reqwest::Client,
    url: &str,
    final_path: &str,
    max_retries: u32,
) -> Result<()> {
    let temp_path = format!("{}.tmp", final_path);

    // Check if final file already exists (daily files should not be re-downloaded)
    if std::path::Path::new(final_path).exists() {
        info!("File already exists, skipping download: {}", final_path);
        return Ok(());
    }

    info!("Downloading {} to {}", url, final_path);

    // Clean up any existing temp file
    if std::path::Path::new(&temp_path).exists() {
        fs::remove_file(&temp_path)?;
    }

    match download_with_retry(client, url, max_retries).await {
        Ok(response) => {
            let content = response.bytes().await?;
            fs::write(&temp_path, content)?;

            // Atomically move temp file to final location
            fs::rename(&temp_path, final_path)?;
            info!("Successfully downloaded: {}", final_path);
            Ok(())
        }
        Err(e) => {
            // Clean up temp file on failure
            if std::path::Path::new(&temp_path).exists() {
                let _ = fs::remove_file(&temp_path);
            }
            Err(e)
        }
    }
}

async fn download_text_file_atomically(
    client: &reqwest::Client,
    url: &str,
    final_path: &str,
    max_retries: u32,
) -> Result<()> {
    let temp_path = format!("{}.tmp", final_path);

    // Check if final file already exists (daily files should not be re-downloaded)
    if std::path::Path::new(final_path).exists() {
        info!("File already exists, skipping download: {}", final_path);
        return Ok(());
    }

    info!("Downloading {} to {}", url, final_path);

    // Clean up any existing temp file
    if std::path::Path::new(&temp_path).exists() {
        fs::remove_file(&temp_path)?;
    }

    match download_with_retry(client, url, max_retries).await {
        Ok(response) => {
            let content = response.text().await?;
            fs::write(&temp_path, content)?;

            // Atomically move temp file to final location
            fs::rename(&temp_path, final_path)?;
            info!("Successfully downloaded: {}", final_path);
            Ok(())
        }
        Err(e) => {
            // Clean up temp file on failure
            if std::path::Path::new(&temp_path).exists() {
                let _ = fs::remove_file(&temp_path);
            }
            Err(e)
        }
    }
}

pub async fn handle_pull_data(diesel_pool: Pool<ConnectionManager<PgConnection>>) -> Result<()> {
    sentry::configure_scope(|scope| {
        scope.set_tag("operation", "pull-data");
    });
    info!("Starting pull-data operation");

    // Start metrics server on port 9092 for profiling during data pull
    tokio::spawn(async {
        crate::metrics::start_metrics_server(9092).await;
    });

    // Create temporary directory with date only (no time)
    let date = Local::now().format("%Y%m%d").to_string();
    let temp_dir = get_data_directory(&date)?;

    info!("Creating data directory: {}", temp_dir);
    fs::create_dir_all(&temp_dir)?;

    let client = reqwest::Client::new();
    let max_retries = 5;

    // Pull receiver data from OGN RDB
    let receivers_path = format!("{}/receivers.json", temp_dir);
    info!("Pulling receiver data from OGN RDB...");
    if !std::path::Path::new(&receivers_path).exists() {
        crate::fetch_receivers::fetch_receivers(&receivers_path).await?;
        info!("Receivers data saved to: {}", receivers_path);
    } else {
        info!(
            "Receivers file already exists, skipping: {}",
            receivers_path
        );
    }

    // Download airports.csv
    let airports_url = "https://davidmegginson.github.io/ourairports-data/airports.csv";
    let airports_path = format!("{}/airports.csv", temp_dir);
    download_text_file_atomically(&client, airports_url, &airports_path, max_retries).await?;

    // Download runways.csv
    let runways_url = "https://davidmegginson.github.io/ourairports-data/runways.csv";
    let runways_path = format!("{}/runways.csv", temp_dir);
    download_text_file_atomically(&client, runways_url, &runways_path, max_retries).await?;

    // Download FAA ReleasableAircraft.zip
    let faa_url = "https://registry.faa.gov/database/ReleasableAircraft.zip";
    let zip_path = format!("{}/ReleasableAircraft.zip", temp_dir);
    download_file_atomically(&client, faa_url, &zip_path, max_retries).await?;

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

    // Download unified FlarmNet database
    let flarmnet_url = "https://turbo87.github.io/united-flarmnet/united.fln";
    let flarmnet_path = format!("{}/united.fln", temp_dir);
    download_text_file_atomically(&client, flarmnet_url, &flarmnet_path, max_retries).await?;

    // Display the temporary directory
    info!("Data directory located at: {}", temp_dir);

    // Invoke handle_load_data with all downloaded files
    info!("Invoking load data procedures...");
    crate::loader::handle_load_data(
        diesel_pool,
        Some(acftref_path), // aircraft_models
        Some(master_path),  // aircraft_registrations
        Some(airports_path),
        Some(runways_path),
        Some(receivers_path),
        Some(flarmnet_path),
        true,
        true,
    )
    .await
}
