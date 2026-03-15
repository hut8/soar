use anyhow::{Context, Result};
use diesel::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::info;

/// A station entry from the FAI glider tracking site.
#[derive(Debug, Clone, Deserialize)]
struct FaiStation {
    /// Callsign
    s: String,
    /// Latitude (string, parseable as f64)
    lt: String,
    /// Longitude (string, parseable as f64)
    lg: String,
    /// Last update timestamp ("YYYY-MM-DD HH:MM:SS")
    ut: String,
    /// Status: "U" (up) or "D" (down)
    u: String,
    /// Altitude in meters
    #[serde(default)]
    alti: Option<i64>,
    /// Software version
    #[serde(default)]
    version: Option<String>,
}

/// Root JSON response from the FAI stations endpoint.
#[derive(Debug, Deserialize)]
struct FaiStationsResponse {
    stations: Vec<FaiStation>,
}

/// Minimal receiver data loaded from our database.
struct SoarReceiver {
    callsign: String,
    latitude: Option<f64>,
    longitude: Option<f64>,
    latest_packet_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// A station present in both FAI and SOAR with a location mismatch.
struct LocationMismatch {
    callsign: String,
    fai_lat: f64,
    fai_lng: f64,
    soar_lat: f64,
    soar_lng: f64,
    delta: f64,
}

const FAI_STATIONS_URL: &str = "https://glidertracking.fai.org/stations.php";

/// Threshold in degrees (~1 km) above which we flag a location mismatch.
const LOCATION_THRESHOLD_DEG: f64 = 0.01;

pub async fn handle_station_xref(
    pool: Pool<ConnectionManager<PgConnection>>,
    verbose: bool,
) -> Result<()> {
    // 1. Fetch FAI station data
    info!("Fetching FAI station data from {}", FAI_STATIONS_URL);
    let client = reqwest::Client::new();
    let response = client
        .get(FAI_STATIONS_URL)
        .send()
        .await
        .context("Failed to fetch FAI station data")?;
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("FAI stations endpoint returned HTTP {}", status);
    }
    let fai_response: FaiStationsResponse = response
        .json()
        .await
        .context("Failed to parse FAI stations JSON")?;
    let fai_stations = fai_response.stations;
    info!("Fetched {} FAI stations", fai_stations.len());

    // 2. Load receivers from our database
    info!("Loading receivers from database...");
    let soar_receivers = {
        use soar::schema::receivers::dsl::*;
        let mut conn = pool.get().context("Failed to get database connection")?;
        receivers
            .select((callsign, latitude, longitude, latest_packet_at))
            .load::<(
                String,
                Option<f64>,
                Option<f64>,
                Option<chrono::DateTime<chrono::Utc>>,
            )>(&mut conn)
            .context("Failed to load receivers from database")?
            .into_iter()
            .map(|(cs, lat, lng, pkt)| SoarReceiver {
                callsign: cs,
                latitude: lat,
                longitude: lng,
                latest_packet_at: pkt,
            })
            .collect::<Vec<_>>()
    };
    info!("Loaded {} SOAR receivers", soar_receivers.len());

    // 3. Build lookup maps
    let fai_map: HashMap<&str, &FaiStation> =
        fai_stations.iter().map(|s| (s.s.as_str(), s)).collect();
    let soar_map: HashMap<&str, &SoarReceiver> = soar_receivers
        .iter()
        .map(|r| (r.callsign.as_str(), r))
        .collect();

    // 4. Compute sets
    let mut only_in_fai: Vec<&FaiStation> = Vec::new();
    let mut only_in_soar: Vec<&SoarReceiver> = Vec::new();
    let mut mismatches: Vec<LocationMismatch> = Vec::new();
    let mut matched_count: usize = 0;

    for station in &fai_stations {
        if !soar_map.contains_key(station.s.as_str()) {
            only_in_fai.push(station);
        }
    }

    for receiver in &soar_receivers {
        if !fai_map.contains_key(receiver.callsign.as_str()) {
            only_in_soar.push(receiver);
        }
    }

    // Check location mismatches for stations in both
    for station in &fai_stations {
        if let Some(receiver) = soar_map.get(station.s.as_str()) {
            let fai_lat: f64 = match station.lt.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let fai_lng: f64 = match station.lg.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            if let (Some(soar_lat), Some(soar_lng)) = (receiver.latitude, receiver.longitude) {
                let dlat = f64::abs(fai_lat - soar_lat);
                let dlng = f64::abs(fai_lng - soar_lng);
                let delta = dlat.max(dlng);
                if delta > LOCATION_THRESHOLD_DEG {
                    mismatches.push(LocationMismatch {
                        callsign: station.s.clone(),
                        fai_lat,
                        fai_lng,
                        soar_lat,
                        soar_lng,
                        delta,
                    });
                } else {
                    matched_count += 1;
                }
            } else {
                // SOAR has no location — count as matched (no mismatch to report)
                matched_count += 1;
            }
        }
    }

    // Sort for deterministic output
    only_in_fai.sort_by(|a, b| a.s.cmp(&b.s));
    only_in_soar.sort_by(|a, b| a.callsign.cmp(&b.callsign));
    mismatches.sort_by(|a, b| {
        b.delta
            .partial_cmp(&a.delta)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 5. Version distribution
    let mut version_counts: HashMap<&str, usize> = HashMap::new();
    for station in &fai_stations {
        let ver = station.version.as_deref().unwrap_or("unknown");
        *version_counts.entry(ver).or_insert(0) += 1;
    }
    let mut version_list: Vec<(&&str, &usize)> = version_counts.iter().collect();
    version_list.sort_by(|a, b| b.1.cmp(a.1));

    // 6. Print report
    println!();
    println!("FAI Station Cross-Reference Report");
    println!("===================================");
    println!("FAI stations: {}", fai_stations.len());
    println!("SOAR receivers: {}", soar_receivers.len());
    println!();

    // Only in FAI
    println!("Only in FAI (not in SOAR): {}", only_in_fai.len());
    if verbose {
        for s in &only_in_fai {
            let alt_str = s.alti.map(|a| format!("  alt={}m", a)).unwrap_or_default();
            let ver_str = s
                .version
                .as_deref()
                .map(|v| format!("  ver={}", v))
                .unwrap_or_default();
            println!(
                "  {:<12} lat={:<10} lng={:<11}{}{} status={}  last={}",
                s.s, s.lt, s.lg, alt_str, ver_str, s.u, s.ut
            );
        }
    }
    println!();

    // Only in SOAR
    println!("Only in SOAR (not in FAI): {}", only_in_soar.len());
    if verbose {
        for r in &only_in_soar {
            let lat_str = r
                .latitude
                .map(|l| format!("{:.4}", l))
                .unwrap_or_else(|| "N/A".to_string());
            let lng_str = r
                .longitude
                .map(|l| format!("{:.4}", l))
                .unwrap_or_else(|| "N/A".to_string());
            let pkt_str = r
                .latest_packet_at
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "never".to_string());
            println!(
                "  {:<12} lat={:<10} lng={:<11} last={}",
                r.callsign, lat_str, lng_str, pkt_str
            );
        }
    }
    println!();

    // Location mismatches
    println!(
        "Location mismatch (>{:.2}\u{00b0}): {}",
        LOCATION_THRESHOLD_DEG,
        mismatches.len()
    );
    if verbose {
        for m in &mismatches {
            println!(
                "  {:<12} FAI=({:.4}, {:.4})  SOAR=({:.4}, {:.4})  delta={:.3}\u{00b0}",
                m.callsign, m.fai_lat, m.fai_lng, m.soar_lat, m.soar_lng, m.delta
            );
        }
    }
    println!();

    println!("Matched: {}", matched_count);
    println!();

    // Version distribution
    println!("FAI version distribution:");
    for (ver, count) in &version_list {
        println!("  {:<16} {}", ver, count);
    }
    println!();

    Ok(())
}
