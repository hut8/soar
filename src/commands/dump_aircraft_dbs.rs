use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use serde::Deserialize;
use std::fs::File;
use std::io::{Read, Write};
use tracing::{debug, info};

use soar::aircraft::read_flarmnet_file;
use soar::aircraft::{AddressType, Aircraft};
use soar::aircraft_types::{AircraftCategory, EngineType};

const DDB_URL_UNIFIED_FLARMNET: &str = "https://turbo87.github.io/united-flarmnet/united.fln";
const DDB_URL_ADSB_EXCHANGE: &str =
    "https://downloads.adsbexchange.com/downloads/basic-ac-db.json.gz";

/// ADS-B Exchange record structure (mirrors load_data/adsb_exchange.rs)
#[derive(Debug, Deserialize)]
struct AdsbExchangeRecord {
    icao: String,
    #[serde(rename = "reg")]
    registration: Option<String>,
    #[serde(rename = "icaotype")]
    icao_type_code: Option<String>,
    #[serde(rename = "ownop")]
    owner_operator: Option<String>,
    /// Year is a string in the JSON (e.g., "1970" or ""), not a number
    year: Option<String>,
    manufacturer: Option<String>,
    model: Option<String>,
    faa_pia: Option<bool>,
    faa_ladd: Option<bool>,
    short_type: Option<String>,
    #[serde(rename = "mil")]
    is_military: Option<bool>,
}

/// Parse short_type format: [Category][NumEngines][EngineType]
/// Example: "L2J" = Landplane, 2 engines, Jet
fn parse_short_type(
    short_type: &str,
) -> (Option<AircraftCategory>, Option<i16>, Option<EngineType>) {
    let chars: Vec<char> = short_type.chars().collect();

    if chars.len() < 3 {
        return (None, None, None);
    }

    let category = AircraftCategory::from_short_type_char(chars[0]);
    let engine_count = chars[1].to_digit(10).map(|n| n as i16);
    let engine_type = EngineType::from_short_type_char(chars[2]);

    (category, engine_count, engine_type)
}

/// Parse ICAO hex address
/// If ICAO starts with '~', it's a non-ICAO address (e.g., from TIS-B)
fn parse_icao_address(icao_hex: &str) -> Result<(u32, AddressType)> {
    let (cleaned_hex, address_type) = if let Some(stripped) = icao_hex.strip_prefix('~') {
        (stripped, AddressType::Unknown)
    } else {
        (icao_hex, AddressType::Icao)
    };

    let address = u32::from_str_radix(cleaned_hex, 16)
        .with_context(|| format!("Failed to parse ICAO hex address: {}", icao_hex))?;

    Ok((address, address_type))
}

/// Canonicalize registration using flydent
fn canonicalize_registration(registration: &str) -> String {
    let parser = flydent::Parser::new();
    match parser.parse(registration, false, false) {
        Some(r) => r.canonical_callsign().to_string(),
        None => registration.to_string(),
    }
}

/// Build aircraft model string from manufacturer and model
fn build_aircraft_model(manufacturer: Option<&String>, model: Option<&String>) -> String {
    match (manufacturer, model) {
        (Some(mfr), Some(mdl)) if !mfr.is_empty() && !mdl.is_empty() => {
            format!("{} {}", mfr, mdl)
        }
        (Some(mfr), None) if !mfr.is_empty() => mfr.clone(),
        (None, Some(mdl)) if !mdl.is_empty() => mdl.clone(),
        _ => String::new(),
    }
}

/// Convert an ADS-B Exchange record to an Aircraft struct
fn adsb_record_to_aircraft(record: &AdsbExchangeRecord) -> Option<Aircraft> {
    // Parse ICAO address
    let (address, address_type) = match parse_icao_address(&record.icao) {
        Ok(parsed) => parsed,
        Err(e) => {
            debug!("Skipping record with invalid ICAO {}: {}", record.icao, e);
            return None;
        }
    };

    // Validate icao_model_code is exactly 3 or 4 characters (per ICAO Doc 8643) if present
    let icao_model_code = record
        .icao_type_code
        .as_ref()
        .filter(|code| {
            let len = code.len();
            len == 3 || len == 4
        })
        .cloned();

    // Parse short_type into components
    let (category, engine_count, engine_type) = record
        .short_type
        .as_ref()
        .map(|st| parse_short_type(st))
        .unwrap_or((None, None, None));

    // Canonicalize registration if present
    let registration = record
        .registration
        .as_ref()
        .filter(|r| !r.is_empty())
        .map(|r| canonicalize_registration(r));

    // Build aircraft model from manufacturer and model
    let aircraft_model = build_aircraft_model(record.manufacturer.as_ref(), record.model.as_ref());

    // Parse year from string (e.g., "1970") to i16, skipping empty strings
    let year = record
        .year
        .as_ref()
        .filter(|y| !y.is_empty())
        .and_then(|y| y.parse::<i16>().ok());

    Some(Aircraft {
        id: None,
        address_type,
        address,
        aircraft_model,
        registration,
        competition_number: String::new(),
        tracked: true,
        identified: true,
        frequency_mhz: None,
        pilot_name: None,
        home_base_airport_ident: None,
        last_fix_at: None,
        club_id: None,
        icao_model_code,
        adsb_emitter_category: None,
        tracker_device_type: None,
        country_code: None,
        owner_operator: record.owner_operator.clone(),
        aircraft_category: category,
        engine_count,
        engine_type,
        faa_pia: record.faa_pia,
        faa_ladd: record.faa_ladd,
        year,
        is_military: record.is_military,
        from_ogn_ddb: Some(false),
        from_adsbx_ddb: Some(true),
        created_at: None,
        updated_at: None,
        latitude: None,
        longitude: None,
        current_fix: None,
    })
}

/// Download and parse ADS-B Exchange database
async fn fetch_adsb_exchange(
    source_path: Option<String>,
    output_path: &str,
) -> Result<Vec<Aircraft>> {
    let json_content = match source_path {
        Some(local_path) => {
            info!("Using local ADS-B Exchange database from: {}", local_path);
            if !std::path::Path::new(&local_path).exists() {
                return Err(anyhow::anyhow!(
                    "Local ADS-B Exchange file does not exist: {}",
                    local_path
                ));
            }
            // Check if it's gzipped
            if local_path.ends_with(".gz") {
                let file = File::open(&local_path)?;
                let mut decoder = GzDecoder::new(file);
                let mut content = String::new();
                decoder.read_to_string(&mut content)?;
                content
            } else {
                std::fs::read_to_string(&local_path)?
            }
        }
        None => {
            info!(
                "Downloading ADS-B Exchange database from {}",
                DDB_URL_ADSB_EXCHANGE
            );

            let response = reqwest::get(DDB_URL_ADSB_EXCHANGE).await?;

            if !response.status().is_success() {
                return Err(anyhow::anyhow!(
                    "Failed to download ADS-B Exchange database: HTTP {}",
                    response.status()
                ));
            }

            let bytes = response.bytes().await?;
            info!(
                "Downloaded ADS-B Exchange database ({} bytes compressed)",
                bytes.len()
            );

            // Save compressed file temporarily
            let gz_path = format!("{}.tmp.json.gz", output_path);
            std::fs::write(&gz_path, &bytes)?;

            // Decompress
            let file = File::open(&gz_path)?;
            let mut decoder = GzDecoder::new(file);
            let mut content = String::new();
            decoder.read_to_string(&mut content)?;

            // Clean up compressed file
            std::fs::remove_file(&gz_path)?;

            info!(
                "Decompressed ADS-B Exchange database ({} bytes)",
                content.len()
            );
            content
        }
    };

    // Parse NDJSON (newline-delimited JSON)
    let mut aircraft = Vec::new();
    let mut parse_errors = 0;
    let mut skipped_invalid = 0;

    for (line_num, line) in json_content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match serde_json::from_str::<AdsbExchangeRecord>(line) {
            Ok(record) => {
                if let Some(a) = adsb_record_to_aircraft(&record) {
                    aircraft.push(a);
                } else {
                    skipped_invalid += 1;
                }
            }
            Err(e) => {
                debug!(
                    "Failed to parse line {} from ADS-B Exchange: {}",
                    line_num + 1,
                    e
                );
                parse_errors += 1;
            }
        }
    }

    info!(
        "Parsed {} aircraft from ADS-B Exchange ({} parse errors, {} skipped invalid)",
        aircraft.len(),
        parse_errors,
        skipped_invalid
    );

    Ok(aircraft)
}

/// Download and parse unified FlarmNet database
async fn fetch_flarmnet(source_path: Option<String>, output_path: &str) -> Result<Vec<Aircraft>> {
    let (temp_path, cleanup_temp) = match source_path {
        Some(local_path) => {
            info!("Using local unified FlarmNet database from: {}", local_path);
            if !std::path::Path::new(&local_path).exists() {
                return Err(anyhow::anyhow!(
                    "Local FlarmNet file does not exist: {}",
                    local_path
                ));
            }
            (local_path, false)
        }
        None => {
            info!(
                "Downloading unified FlarmNet database from {}",
                DDB_URL_UNIFIED_FLARMNET
            );

            let response = reqwest::get(DDB_URL_UNIFIED_FLARMNET).await?;

            if !response.status().is_success() {
                return Err(anyhow::anyhow!(
                    "Failed to download unified FlarmNet database: HTTP {}",
                    response.status()
                ));
            }

            let content = response.text().await?;
            info!(
                "Downloaded unified FlarmNet database ({} bytes)",
                content.len()
            );

            // Save to temporary file for parsing
            let temp_path = format!("{}.tmp.fln", output_path);
            std::fs::write(&temp_path, &content)?;
            info!("Saved to temporary file: {}", temp_path);

            (temp_path, true)
        }
    };

    // Parse the FlarmNet file
    info!("Parsing unified FlarmNet database...");
    let devices = read_flarmnet_file(&temp_path)?;
    info!(
        "Successfully parsed {} devices from FlarmNet",
        devices.len()
    );

    // Clean up temp file only if we downloaded it
    if cleanup_temp {
        std::fs::remove_file(&temp_path)?;
    }

    Ok(devices)
}

/// Download aircraft databases (FlarmNet and ADS-B Exchange) and dump to JSONL file
pub async fn handle_dump_aircraft_dbs(
    output_path: String,
    flarmnet_source: Option<String>,
    adsb_source: Option<String>,
) -> Result<()> {
    // Fetch both data sources
    let flarmnet_aircraft = fetch_flarmnet(flarmnet_source, &output_path).await?;
    let adsb_aircraft = fetch_adsb_exchange(adsb_source, &output_path).await?;

    let total_count = flarmnet_aircraft.len() + adsb_aircraft.len();
    info!(
        "Total aircraft to write: {} ({} FlarmNet + {} ADS-B Exchange)",
        total_count,
        flarmnet_aircraft.len(),
        adsb_aircraft.len()
    );

    // Write all aircraft to JSONL file (one JSON object per line)
    info!(
        "Writing {} aircraft to JSONL file: {}",
        total_count, output_path
    );
    let mut output_file = File::create(&output_path)?;
    let mut written = 0;

    // Write FlarmNet aircraft first
    for device in &flarmnet_aircraft {
        let json_line = serde_json::to_string(device)?;
        writeln!(output_file, "{}", json_line)?;
        written += 1;

        if written % 10000 == 0 {
            info!("Written {} / {} aircraft", written, total_count);
        }
    }

    // Write ADS-B Exchange aircraft
    for device in &adsb_aircraft {
        let json_line = serde_json::to_string(device)?;
        writeln!(output_file, "{}", json_line)?;
        written += 1;

        if written % 10000 == 0 {
            info!("Written {} / {} aircraft", written, total_count);
        }
    }

    info!(
        "Successfully wrote {} aircraft to {}",
        total_count, output_path
    );
    Ok(())
}
