use anyhow::{Context, Result, anyhow};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

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

fn to_opt_f64(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    t.parse::<f64>().ok()
}

fn yes_no_to_bool(s: &str) -> bool {
    match s.trim().to_lowercase().as_str() {
        "yes" => true,
        "no" => false,
        _ => false, // Default to false for any other value
    }
}

#[derive(Debug, Clone)]
pub struct Airport {
    pub id: i32,                        // Internal OurAirports ID
    pub ident: String,                  // Airport identifier (ICAO or local code)
    pub airport_type: String,           // Type of airport (large_airport, small_airport, etc.)
    pub name: String,                   // Official airport name
    pub latitude_deg: Option<f64>,      // Latitude in decimal degrees
    pub longitude_deg: Option<f64>,     // Longitude in decimal degrees
    pub elevation_ft: Option<i32>,      // Elevation above MSL in feet
    pub continent: Option<String>,      // Continent code (NA, EU, etc.)
    pub iso_country: Option<String>,    // ISO 3166-1 alpha-2 country code
    pub iso_region: Option<String>,     // ISO 3166-2 region code
    pub municipality: Option<String>,   // Primary municipality served
    pub scheduled_service: bool,        // Whether airport has scheduled service
    pub icao_code: Option<String>,      // ICAO code
    pub iata_code: Option<String>,      // IATA code
    pub gps_code: Option<String>,       // GPS code
    pub local_code: Option<String>,     // Local country code
    pub home_link: Option<String>,      // Airport website URL
    pub wikipedia_link: Option<String>, // Wikipedia article URL
    pub keywords: Option<String>,       // Search keywords
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
        let latitude_deg = to_opt_f64(&fields[4]);
        let longitude_deg = to_opt_f64(&fields[5]);
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

    #[test]
    fn test_csv_parsing() {
        let csv_line = r#"6523,"00A","heliport","Total RF Heliport",40.070985,-74.933689,11,"NA","US","US-PA","Bensalem","no",,,"K00A","00A","https://www.penndot.pa.gov/TravelInPA/airports-pa/Pages/Total-RF-Heliport.aspx",,"#;

        let airport = Airport::from_csv_line(csv_line).expect("Failed to parse airport");

        assert_eq!(airport.id, 6523);
        assert_eq!(airport.ident, "00A");
        assert_eq!(airport.airport_type, "heliport");
        assert_eq!(airport.name, "Total RF Heliport");
        assert_eq!(airport.latitude_deg, Some(40.070985));
        assert_eq!(airport.longitude_deg, Some(-74.933689));
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
