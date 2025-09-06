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

fn int_to_bool(s: &str) -> bool {
    match s.trim() {
        "1" => true,
        "0" => false,
        _ => false, // Default to false for any other value
    }
}

#[derive(Debug, Clone)]
pub struct Runway {
    pub id: i32,                 // Internal OurAirports ID
    pub airport_ref: i32,        // Foreign key to airports table (id)
    pub airport_ident: String,   // Airport identifier (matches airports.ident)
    pub length_ft: Option<i32>,  // Length of runway in feet
    pub width_ft: Option<i32>,   // Width of runway in feet
    pub surface: Option<String>, // Surface type (ASP, CON, TURF, etc.)
    pub lighted: bool,           // Whether runway is lighted
    pub closed: bool,            // Whether runway is closed

    // Low-numbered end of runway
    pub le_ident: Option<String>,     // Low end identifier (e.g., "08L")
    pub le_latitude_deg: Option<f64>, // Low end latitude
    pub le_longitude_deg: Option<f64>, // Low end longitude
    pub le_elevation_ft: Option<i32>, // Low end elevation
    pub le_heading_degt: Option<f64>, // Low end heading in degrees true
    pub le_displaced_threshold_ft: Option<i32>, // Low end displaced threshold

    // High-numbered end of runway
    pub he_ident: Option<String>, // High end identifier (e.g., "26R")
    pub he_latitude_deg: Option<f64>, // High end latitude
    pub he_longitude_deg: Option<f64>, // High end longitude
    pub he_elevation_ft: Option<i32>, // High end elevation
    pub he_heading_degt: Option<f64>, // High end heading in degrees true
    pub he_displaced_threshold_ft: Option<i32>, // High end displaced threshold
}

impl Runway {
    /// Parse a Runway from a CSV line with the OurAirports format
    /// Expected CSV columns (0-based indices):
    /// 0: id, 1: airport_ref, 2: airport_ident, 3: length_ft, 4: width_ft, 5: surface,
    /// 6: lighted, 7: closed, 8: le_ident, 9: le_latitude_deg, 10: le_longitude_deg,
    /// 11: le_elevation_ft, 12: le_heading_degT, 13: le_displaced_threshold_ft,
    /// 14: he_ident, 15: he_latitude_deg, 16: he_longitude_deg, 17: he_elevation_ft,
    /// 18: he_heading_degT, 19: he_displaced_threshold_ft
    pub fn from_csv_line(line: &str) -> Result<Self> {
        // Parse CSV with proper quote handling
        let fields = parse_csv_line(line)?;

        if fields.len() < 20 {
            return Err(anyhow!(
                "CSV line has insufficient fields: expected at least 20, got {}",
                fields.len()
            ));
        }

        let id = fields[0]
            .trim()
            .parse::<i32>()
            .with_context(|| format!("Failed to parse runway ID: '{}'", fields[0]))?;

        let airport_ref = fields[1]
            .trim()
            .parse::<i32>()
            .with_context(|| format!("Failed to parse airport_ref: '{}'", fields[1]))?;

        let airport_ident = to_string_trim(&fields[2]);
        if airport_ident.is_empty() {
            return Err(anyhow!("Missing airport identifier in CSV"));
        }

        let length_ft = to_opt_i32(&fields[3]);
        let width_ft = to_opt_i32(&fields[4]);
        let surface = to_opt_string(&fields[5]);
        let lighted = int_to_bool(&fields[6]);
        let closed = int_to_bool(&fields[7]);

        // Low end fields
        let le_ident = to_opt_string(&fields[8]);
        let le_latitude_deg = to_opt_f64(&fields[9]);
        let le_longitude_deg = to_opt_f64(&fields[10]);
        let le_elevation_ft = to_opt_i32(&fields[11]);
        let le_heading_degt = to_opt_f64(&fields[12]);
        let le_displaced_threshold_ft = to_opt_i32(&fields[13]);

        // High end fields
        let he_ident = to_opt_string(&fields[14]);
        let he_latitude_deg = to_opt_f64(&fields[15]);
        let he_longitude_deg = to_opt_f64(&fields[16]);
        let he_elevation_ft = to_opt_i32(&fields[17]);
        let he_heading_degt = to_opt_f64(&fields[18]);
        let he_displaced_threshold_ft = to_opt_i32(&fields[19]);

        Ok(Runway {
            id,
            airport_ref,
            airport_ident,
            length_ft,
            width_ft,
            surface,
            lighted,
            closed,
            le_ident,
            le_latitude_deg,
            le_longitude_deg,
            le_elevation_ft,
            le_heading_degt,
            le_displaced_threshold_ft,
            he_ident,
            he_latitude_deg,
            he_longitude_deg,
            he_elevation_ft,
            he_heading_degt,
            he_displaced_threshold_ft,
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

/// Read a CSV OurAirports runways file and parse all rows.
/// Automatically skips the first line (header) and any blank lines.
/// Returns an error on the first malformed line.
pub fn read_runways_csv_file<P: AsRef<Path>>(path: P) -> Result<Vec<Runway>> {
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

        let rec = Runway::from_csv_line(trimmed)
            .with_context(|| format!("Parsing CSV line {}", lineno + 1))?;
        out.push(rec);
    }

    Ok(out)
}

/// Read only the first N runways from a CSV file (useful for large files)
pub fn read_runways_csv_sample<P: AsRef<Path>>(path: P, limit: usize) -> Result<Vec<Runway>> {
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

        let rec = Runway::from_csv_line(trimmed)
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
        let csv_line = r#"269408,6523,"00A",80,80,"ASPH-G",1,0,"H1",,,,,,,,,,,"#;

        let runway = Runway::from_csv_line(csv_line).expect("Failed to parse runway");

        assert_eq!(runway.id, 269408);
        assert_eq!(runway.airport_ref, 6523);
        assert_eq!(runway.airport_ident, "00A");
        assert_eq!(runway.length_ft, Some(80));
        assert_eq!(runway.width_ft, Some(80));
        assert_eq!(runway.surface, Some("ASPH-G".to_string()));
        assert!(runway.lighted);
        assert!(!runway.closed);
        assert_eq!(runway.le_ident, Some("H1".to_string()));
        assert_eq!(runway.le_latitude_deg, None);
        assert_eq!(runway.le_longitude_deg, None);
        assert_eq!(runway.he_ident, None);
    }

    #[test]
    fn test_int_to_bool() {
        assert!(int_to_bool("1"));
        assert!(!int_to_bool("0"));
        assert!(!int_to_bool(""));
        assert!(!int_to_bool("2"));
        assert!(!int_to_bool("yes"));
    }

    #[test]
    fn test_csv_line_parsing() {
        let line = r#"123,456,"test","quoted field","field with, comma",1,0"#;
        let fields = parse_csv_line(line).expect("Failed to parse CSV line");

        assert_eq!(fields.len(), 7);
        assert_eq!(fields[0], "123");
        assert_eq!(fields[1], "456");
        assert_eq!(fields[2], "test");
        assert_eq!(fields[3], "quoted field");
        assert_eq!(fields[4], "field with, comma");
        assert_eq!(fields[5], "1");
        assert_eq!(fields[6], "0");
    }

    #[test]
    fn test_empty_fields() {
        let line = r#"123,456,"test",,,"",1,0"#;
        let fields = parse_csv_line(line).expect("Failed to parse CSV line");

        assert_eq!(fields.len(), 8);
        assert_eq!(fields[0], "123");
        assert_eq!(fields[1], "456");
        assert_eq!(fields[2], "test");
        assert_eq!(fields[3], "");
        assert_eq!(fields[4], "");
        assert_eq!(fields[5], "");
        assert_eq!(fields[6], "1");
        assert_eq!(fields[7], "0");
    }

    #[test]
    fn test_runway_with_coordinates() {
        let csv_line = r#"255155,6524,"00AK",2500,70,"GRVL",0,0,"N",59.947733,-151.692524,450,360,,"S",59.947733,-151.692524,450,180,"#;

        let runway = Runway::from_csv_line(csv_line).expect("Failed to parse runway");

        assert_eq!(runway.id, 255155);
        assert_eq!(runway.airport_ref, 6524);
        assert_eq!(runway.airport_ident, "00AK");
        assert_eq!(runway.length_ft, Some(2500));
        assert_eq!(runway.width_ft, Some(70));
        assert_eq!(runway.surface, Some("GRVL".to_string()));
        assert!(!runway.lighted);
        assert!(!runway.closed);
        assert_eq!(runway.le_ident, Some("N".to_string()));
        assert_eq!(runway.le_latitude_deg, Some(59.947733));
        assert_eq!(runway.le_longitude_deg, Some(-151.692524));
        assert_eq!(runway.le_elevation_ft, Some(450));
        assert_eq!(runway.le_heading_degt, Some(360.0));
        assert_eq!(runway.he_ident, Some("S".to_string()));
        assert_eq!(runway.he_latitude_deg, Some(59.947733));
        assert_eq!(runway.he_longitude_deg, Some(-151.692524));
        assert_eq!(runway.he_elevation_ft, Some(450));
        assert_eq!(runway.he_heading_degt, Some(180.0));
    }
}
