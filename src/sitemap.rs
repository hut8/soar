use anyhow::Result;
use diesel::prelude::*;
use sitemap_rs::{
    sitemap::Sitemap as SitemapFile, sitemap_index::SitemapIndex, url::Url, url_set::UrlSet,
};
use std::fs;
use std::path::Path;
use tracing::info;
use uuid::Uuid;

use crate::web::PgPool;

/// Maximum number of URLs per sitemap file (Google recommends 50,000)
const MAX_URLS_PER_SITEMAP: usize = 50000;

/// Base URL for the website
const BASE_URL: &str = "https://glider.flights";

/// Generate sitemap and robots.txt files
pub async fn handle_sitemap_generation(pool: PgPool, static_root: String) -> Result<()> {
    info!("Starting sitemap generation");
    info!("Static root directory: {}", static_root);

    // Ensure the static root directory exists
    if !Path::new(&static_root).exists() {
        info!("Creating static root directory: {}", static_root);
        fs::create_dir_all(&static_root)?;
    }

    // Generate static pages
    let static_urls = generate_static_pages()?;

    // Get all club IDs from database
    let club_ids = get_all_club_ids(pool.clone()).await?;
    info!("Found {} clubs for sitemap generation", club_ids.len());

    // Generate club URLs
    let club_urls = generate_club_urls(&club_ids)?;

    // Get all device addresses from database
    let device_addresses = get_all_device_addresses(pool).await?;
    info!(
        "Found {} devices for sitemap generation",
        device_addresses.len()
    );

    // Generate device URLs
    let device_urls = generate_device_urls(&device_addresses)?;

    // Combine all URLs
    let mut all_urls = static_urls;
    all_urls.extend(club_urls);
    all_urls.extend(device_urls);

    let total_urls = all_urls.len();
    info!("Total URLs to generate: {}", total_urls);

    // Determine if we need multiple sitemap files
    if total_urls <= MAX_URLS_PER_SITEMAP {
        // Single sitemap file
        generate_single_sitemap(&static_root, all_urls).await?;
    } else {
        // Multiple sitemap files with sitemap index
        generate_multiple_sitemaps(&static_root, all_urls).await?;
    }

    info!("Sitemap generation completed successfully");
    Ok(())
}

/// Generate static page entries for the sitemap
fn generate_static_pages() -> Result<Vec<Url>> {
    let urls = vec![
        // Main pages (high priority)
        Url::builder(format!("{}/", BASE_URL))
            .priority(0.9)
            .build()?,
        Url::builder(format!("{}/devices", BASE_URL))
            .priority(0.8)
            .build()?,
        Url::builder(format!("{}/clubs", BASE_URL))
            .priority(0.8)
            .build()?,
        Url::builder(format!("{}/airports", BASE_URL))
            .priority(0.8)
            .build()?,
        Url::builder(format!("{}/operations", BASE_URL))
            .priority(0.7)
            .build()?,
        // Authentication pages (lower priority)
        Url::builder(format!("{}/register", BASE_URL))
            .priority(0.3)
            .build()?,
        Url::builder(format!("{}/login", BASE_URL))
            .priority(0.2)
            .build()?,
        Url::builder(format!("{}/forgot-password", BASE_URL))
            .priority(0.1)
            .build()?,
        Url::builder(format!("{}/reset-password", BASE_URL))
            .priority(0.1)
            .build()?,
        Url::builder(format!("{}/verify-email", BASE_URL))
            .priority(0.1)
            .build()?,
    ];

    Ok(urls)
}

/// Generate club page URLs
fn generate_club_urls(club_ids: &[Uuid]) -> Result<Vec<Url>> {
    let mut urls = Vec::new();

    for club_id in club_ids {
        let club_url = format!("{}/clubs/{}", BASE_URL, club_id);
        let url = Url::builder(club_url).priority(0.6).build()?;
        urls.push(url);
    }

    Ok(urls)
}

/// Get all club IDs from the database
async fn get_all_club_ids(pool: PgPool) -> Result<Vec<Uuid>> {
    let result = tokio::task::spawn_blocking(move || {
        use crate::schema::clubs::dsl::*;

        let mut conn = pool.get()?;

        let club_ids: Vec<Uuid> = clubs
            .filter(is_soaring.eq(true))
            .order(id.asc())
            .select(id)
            .load::<Uuid>(&mut conn)?;

        Ok::<Vec<Uuid>, anyhow::Error>(club_ids)
    })
    .await??;

    Ok(result)
}

/// Generate device page URLs
fn generate_device_urls(device_addresses: &[String]) -> Result<Vec<Url>> {
    let mut urls = Vec::new();

    for address in device_addresses {
        let device_url = format!("{}/devices/{}", BASE_URL, address);
        let url = Url::builder(device_url).priority(0.5).build()?;
        urls.push(url);
    }

    Ok(urls)
}

/// Get all device addresses from the database
async fn get_all_device_addresses(pool: PgPool) -> Result<Vec<String>> {
    let result = tokio::task::spawn_blocking(move || {
        use crate::devices::AddressType;
        use crate::schema::devices::dsl::*;

        let mut conn = pool.get()?;

        // Get all devices with their address_type and address
        let device_data: Vec<(AddressType, i32)> = devices
            .order((address_type.asc(), address.asc()))
            .select((address_type, address))
            .load::<(AddressType, i32)>(&mut conn)?;

        // Format as "TYPE:ADDRESS" strings (e.g., "I:DD1234", "O:123456")
        let device_addresses: Vec<String> = device_data
            .into_iter()
            .map(|(addr_type_enum, addr)| {
                // Convert address to hex for ICAO (I) and FLARM (F) types
                match addr_type_enum {
                    AddressType::Icao | AddressType::Flarm => {
                        format!("{}:{:06X}", addr_type_enum, addr as u32)
                    }
                    _ => format!("{}:{}", addr_type_enum, addr),
                }
            })
            .collect();

        Ok::<Vec<String>, anyhow::Error>(device_addresses)
    })
    .await??;

    Ok(result)
}

/// Generate a single sitemap file
async fn generate_single_sitemap(static_root: &str, urls: Vec<Url>) -> Result<()> {
    info!("Generating single sitemap file");

    let url_set = UrlSet::new(urls)?;
    let url_count = url_set.urls.len();

    // Write sitemap file
    let sitemap_path = Path::new(static_root).join("sitemap.xml");
    let mut file = fs::File::create(&sitemap_path)?;
    url_set.write(&mut file)?;

    info!("Generated sitemap.xml with {} URLs", url_count);
    Ok(())
}

/// Generate multiple sitemap files with a sitemap index
async fn generate_multiple_sitemaps(static_root: &str, urls: Vec<Url>) -> Result<()> {
    info!("Generating multiple sitemap files");

    let mut sitemap_files = Vec::new();
    let mut sitemap_counter = 1;

    // Split URLs into chunks
    for chunk in urls.chunks(MAX_URLS_PER_SITEMAP) {
        let url_set = UrlSet::new(chunk.to_vec())?;
        let url_count = url_set.urls.len();

        // Write numbered sitemap file
        let filename = format!("sitemap-{}.xml", sitemap_counter);
        let sitemap_path = Path::new(static_root).join(&filename);
        let mut file = fs::File::create(&sitemap_path)?;
        url_set.write(&mut file)?;

        info!("Generated {} with {} URLs", filename, url_count);

        // Create sitemap reference for index
        let sitemap_url = format!("{}/{}", BASE_URL, filename);
        let sitemap_file = SitemapFile::new(sitemap_url, None);
        sitemap_files.push(sitemap_file);

        sitemap_counter += 1;
    }

    // Write sitemap index
    let sitemap_index = SitemapIndex::new(sitemap_files)?;
    let index_path = Path::new(static_root).join("sitemap.xml");
    let mut file = fs::File::create(&index_path)?;
    sitemap_index.write(&mut file)?;

    info!(
        "Generated sitemap index with {} sitemap files",
        sitemap_counter - 1
    );
    Ok(())
}
