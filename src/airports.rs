use anyhow::{Context, Result, anyhow};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use uuid::Uuid;

fn to_opt_string(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn to_string_trim(s: &str) -> String {
    s.trim().to_string()
}

fn to_opt_i32(s: &str) -> Option<i32> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    t.parse::<i32>().ok()
}

fn to_opt_bigdecimal(s: &str) -> Option<BigDecimal> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    BigDecimal::parse_bytes(t.as_bytes(), 10)
}

fn yes_no_to_bool(s: &str) -> bool {
    match s.trim().to_lowercase().as_str() {
        "yes" => true,
        "no" => false,
        _ => false, // Default to false for any other value
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Airport {
    pub id: i32,                           // Internal OurAirports ID
    pub ident: String,                     // Airport identifier (ICAO or local code)
    pub airport_type: String,              // Type of airport (large_airport, small_airport, etc.)
    pub name: String,                      // Official airport name
    pub latitude_deg: Option<BigDecimal>,  // Latitude in decimal degrees
    pub longitude_deg: Option<BigDecimal>, // Longitude in decimal degrees
    pub elevation_ft: Option<i32>,         // Elevation above MSL in feet
    pub continent: Option<String>,         // Continent code (NA, EU, etc.)
    pub iso_country: Option<String>,       // ISO 3166-1 alpha-2 country code
    pub iso_region: Option<String>,        // ISO 3166-2 region code
    pub municipality: Option<String>,      // Primary municipality served
    pub scheduled_service: bool,           // Whether airport has scheduled service
    pub icao_code: Option<String>,         // ICAO code
    pub iata_code: Option<String>,         // IATA code
    pub gps_code: Option<String>,          // GPS code
    pub local_code: Option<String>,        // Local country code
    pub home_link: Option<String>,         // Airport website URL
    pub wikipedia_link: Option<String>,    // Wikipedia article URL
    pub keywords: Option<String>,          // Search keywords
    pub location_id: Option<Uuid>,         // Foreign key to locations table
}

/// Diesel model for the airports table - used for database operations
#[derive(Debug, Clone, Queryable, QueryableByName, Selectable, Serialize, Deserialize)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = crate::schema::airports)]
pub struct AirportModel {
    pub id: i32,
    pub ident: String,
    pub type_: String,
    pub name: String,
    pub latitude_deg: Option<BigDecimal>,
    pub longitude_deg: Option<BigDecimal>,
    pub elevation_ft: Option<i32>,
    pub continent: Option<String>,
    pub iso_country: Option<String>,
    pub iso_region: Option<String>,
    pub municipality: Option<String>,
    pub scheduled_service: bool,
    pub gps_code: Option<String>,
    pub icao_code: Option<String>,
    pub iata_code: Option<String>,
    pub local_code: Option<String>,
    pub home_link: Option<String>,
    pub wikipedia_link: Option<String>,
    pub keywords: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub location_id: Option<Uuid>,
}

/// Insert model for new airports (without created_at, updated_at, location)
#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::airports)]
pub struct NewAirportModel {
    pub id: i32,
    pub ident: String,
    pub type_: String,
    pub name: String,
    pub latitude_deg: Option<BigDecimal>,
    pub longitude_deg: Option<BigDecimal>,
    pub elevation_ft: Option<i32>,
    pub continent: Option<String>,
    pub iso_country: Option<String>,
    pub iso_region: Option<String>,
    pub municipality: Option<String>,
    pub scheduled_service: bool,
    pub icao_code: Option<String>,
    pub iata_code: Option<String>,
    pub gps_code: Option<String>,
    pub local_code: Option<String>,
    pub home_link: Option<String>,
    pub wikipedia_link: Option<String>,
    pub keywords: Option<String>,
}

/// Conversion from Airport (API model) to AirportModel (database model)
impl From<Airport> for AirportModel {
    fn from(airport: Airport) -> Self {
        Self {
            id: airport.id,
            ident: airport.ident,
            type_: airport.airport_type,
            name: airport.name,
            latitude_deg: airport.latitude_deg,
            longitude_deg: airport.longitude_deg,
            elevation_ft: airport.elevation_ft,
            continent: airport.continent,
            iso_country: airport.iso_country,
            iso_region: airport.iso_region,
            municipality: airport.municipality,
            scheduled_service: airport.scheduled_service,
            gps_code: airport.gps_code,
            icao_code: airport.icao_code,
            iata_code: airport.iata_code,
            local_code: airport.local_code,
            home_link: airport.home_link,
            wikipedia_link: airport.wikipedia_link,
            keywords: airport.keywords,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            location_id: airport.location_id,
        }
    }
}

/// Conversion from Airport (API model) to NewAirportModel (insert model)
impl From<Airport> for NewAirportModel {
    fn from(airport: Airport) -> Self {
        Self {
            id: airport.id,
            ident: airport.ident,
            name: airport.name,
            latitude_deg: airport.latitude_deg,
            longitude_deg: airport.longitude_deg,
            elevation_ft: airport.elevation_ft,
            continent: airport.continent,
            iso_country: airport.iso_country,
            iso_region: airport.iso_region,
            municipality: airport.municipality,
            scheduled_service: airport.scheduled_service,
            icao_code: airport.icao_code,
            iata_code: airport.iata_code,
            gps_code: airport.gps_code,
            local_code: airport.local_code,
            home_link: airport.home_link,
            wikipedia_link: airport.wikipedia_link,
            keywords: airport.keywords,
            type_: airport.airport_type,
        }
    }
}

/// Conversion from AirportModel (database model) to Airport (API model)
impl From<AirportModel> for Airport {
    fn from(model: AirportModel) -> Self {
        Self {
            id: model.id,
            ident: model.ident,
            airport_type: model.type_,
            name: model.name,
            latitude_deg: model.latitude_deg,
            longitude_deg: model.longitude_deg,
            elevation_ft: model.elevation_ft,
            continent: model.continent,
            iso_country: model.iso_country,
            iso_region: model.iso_region,
            municipality: model.municipality,
            scheduled_service: model.scheduled_service,
            icao_code: model.icao_code,
            iata_code: model.iata_code,
            gps_code: model.gps_code,
            local_code: model.local_code,
            home_link: model.home_link,
            wikipedia_link: model.wikipedia_link,
            keywords: model.keywords,
            location_id: model.location_id,
        }
    }
}

impl Airport {
    /// Parse an Airport from a CSV line with the OurAirports format
    /// Expected CSV columns (0-based indices):
    /// 0: id, 1: ident, 2: type, 3: name, 4: latitude_deg, 5: longitude_deg,
    /// 6: elevation_ft, 7: continent, 8: iso_country, 9: iso_region,
    /// 10: municipality, 11: scheduled_service, 12: icao_code, 13: iata_code,
    /// 14: gps_code, 15: local_code, 16: home_link, 17: wikipedia_link, 18: keywords
    pub fn from_csv_line(line: &str) -> Result<Self> {
        // Parse CSV with proper quote handling
        let fields = parse_csv_line(line)?;

        if fields.len() < 19 {
            return Err(anyhow!(
                "CSV line has insufficient fields: expected at least 19, got {}",
                fields.len()
            ));
        }

        let id = fields[0]
            .trim()
            .parse::<i32>()
            .with_context(|| format!("Failed to parse airport ID: '{}'", fields[0]))?;

        let ident = to_string_trim(&fields[1]);
        if ident.is_empty() {
            return Err(anyhow!("Missing airport identifier in CSV"));
        }

        let airport_type = to_string_trim(&fields[2]);
        let name = to_string_trim(&fields[3]);
        let latitude_deg = to_opt_bigdecimal(&fields[4]);
        let longitude_deg = to_opt_bigdecimal(&fields[5]);
        let elevation_ft = to_opt_i32(&fields[6]);
        let continent = to_opt_string(&fields[7]);
        let iso_country = to_opt_string(&fields[8]);
        let iso_region = to_opt_string(&fields[9]);
        let municipality = to_opt_string(&fields[10]);
        let scheduled_service = yes_no_to_bool(&fields[11]);
        let icao_code = to_opt_string(&fields[12]);
        let iata_code = to_opt_string(&fields[13]);
        let gps_code = to_opt_string(&fields[14]);
        let local_code = to_opt_string(&fields[15]);
        let home_link = to_opt_string(&fields[16]);
        let wikipedia_link = to_opt_string(&fields[17]);
        let keywords = to_opt_string(&fields[18]);

        Ok(Airport {
            id,
            ident,
            airport_type,
            name,
            latitude_deg,
            longitude_deg,
            elevation_ft,
            continent,
            iso_country,
            iso_region,
            municipality,
            scheduled_service,
            icao_code,
            iata_code,
            gps_code,
            local_code,
            home_link,
            wikipedia_link,
            keywords,
            location_id: None,
        })
    }
}

/// Simple CSV parser that handles quoted fields
fn parse_csv_line(line: &str) -> Result<Vec<String>> {
    let mut fields = Vec::new();
    let mut current_field = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes {
                    // Check if this is an escaped quote (double quote)
                    if chars.peek() == Some(&'"') {
                        chars.next(); // consume the second quote
                        current_field.push('"');
                    } else {
                        in_quotes = false;
                    }
                } else {
                    in_quotes = true;
                }
            }
            ',' if !in_quotes => {
                fields.push(current_field.clone());
                current_field.clear();
            }
            _ => {
                current_field.push(ch);
            }
        }
    }

    // Add the last field
    fields.push(current_field);

    Ok(fields)
}

/// Read a CSV OurAirports file and parse all rows.
/// Automatically skips the first line (header) and any blank lines.
/// Returns an error on the first malformed line.
pub fn read_airports_csv_file<P: AsRef<Path>>(path: P) -> Result<Vec<Airport>> {
    let f = File::open(path.as_ref()).with_context(|| format!("Opening {:?}", path.as_ref()))?;
    let reader = BufReader::new(f);
    let mut out = Vec::new();
    let mut is_first_line = true;

    for (lineno, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("Reading line {}", lineno + 1))?;
        let trimmed = line.trim_end_matches(&['\r', '\n'][..]);

        // Skip header line (first line)
        if is_first_line {
            is_first_line = false;
            continue;
        }

        // Skip blank lines
        if trimmed.trim().is_empty() {
            continue;
        }

        let rec = Airport::from_csv_line(trimmed)
            .with_context(|| format!("Parsing CSV line {}", lineno + 1))?;
        out.push(rec);
    }

    Ok(out)
}

/// Read only the first N airports from a CSV file (useful for large files)
pub fn read_airports_csv_sample<P: AsRef<Path>>(path: P, limit: usize) -> Result<Vec<Airport>> {
    let f = File::open(path.as_ref()).with_context(|| format!("Opening {:?}", path.as_ref()))?;
    let reader = BufReader::new(f);
    let mut out = Vec::new();
    let mut is_first_line = true;
    let mut count = 0;

    for (lineno, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("Reading line {}", lineno + 1))?;
        let trimmed = line.trim_end_matches(&['\r', '\n'][..]);

        // Skip header line (first line)
        if is_first_line {
            is_first_line = false;
            continue;
        }

        // Skip blank lines
        if trimmed.trim().is_empty() {
            continue;
        }

        if count >= limit {
            break;
        }

        let rec = Airport::from_csv_line(trimmed)
            .with_context(|| format!("Parsing CSV line {}", lineno + 1))?;
        out.push(rec);
        count += 1;
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_csv_parsing() {
        let csv_line = r#"6523,"00A","heliport","Total RF Heliport",40.070985,-74.933689,11,"NA","US","US-PA","Bensalem","no",,,"K00A","00A","https://www.penndot.pa.gov/TravelInPA/airports-pa/Pages/Total-RF-Heliport.aspx",,"#;

        let airport = Airport::from_csv_line(csv_line).expect("Failed to parse airport");

        assert_eq!(airport.id, 6523);
        assert_eq!(airport.ident, "00A");
        assert_eq!(airport.airport_type, "heliport");
        assert_eq!(airport.name, "Total RF Heliport");
        assert_eq!(
            airport.latitude_deg,
            Some(BigDecimal::from_str("40.070985").unwrap())
        );
        assert_eq!(
            airport.longitude_deg,
            Some(BigDecimal::from_str("-74.933689").unwrap())
        );
        assert_eq!(airport.elevation_ft, Some(11));
        assert_eq!(airport.continent, Some("NA".to_string()));
        assert_eq!(airport.iso_country, Some("US".to_string()));
        assert_eq!(airport.iso_region, Some("US-PA".to_string()));
        assert_eq!(airport.municipality, Some("Bensalem".to_string()));
        assert!(!airport.scheduled_service);
        assert_eq!(airport.icao_code, None);
        assert_eq!(airport.iata_code, None);
        assert_eq!(airport.gps_code, Some("K00A".to_string()));
        assert_eq!(airport.local_code, Some("00A".to_string()));
        assert!(airport.home_link.is_some());
        assert_eq!(airport.wikipedia_link, None);
        assert_eq!(airport.keywords, None);
    }

    #[test]
    fn test_yes_no_to_bool() {
        assert!(yes_no_to_bool("yes"));
        assert!(!yes_no_to_bool("no"));
        assert!(yes_no_to_bool("YES"));
        assert!(!yes_no_to_bool("NO"));
        assert!(!yes_no_to_bool(""));
        assert!(!yes_no_to_bool("maybe"));
    }

    #[test]
    fn test_csv_line_parsing() {
        let line = r#"123,"test","quoted field","field with, comma",45.6,-123.4"#;
        let fields = parse_csv_line(line).expect("Failed to parse CSV line");

        assert_eq!(fields.len(), 6);
        assert_eq!(fields[0], "123");
        assert_eq!(fields[1], "test");
        assert_eq!(fields[2], "quoted field");
        assert_eq!(fields[3], "field with, comma");
        assert_eq!(fields[4], "45.6");
        assert_eq!(fields[5], "-123.4");
    }

    #[test]
    fn test_empty_fields() {
        let line = r#"123,"test",,,"",45.6"#;
        let fields = parse_csv_line(line).expect("Failed to parse CSV line");

        assert_eq!(fields.len(), 6);
        assert_eq!(fields[0], "123");
        assert_eq!(fields[1], "test");
        assert_eq!(fields[2], "");
        assert_eq!(fields[3], "");
        assert_eq!(fields[4], "");
        assert_eq!(fields[5], "45.6");
    }
}
