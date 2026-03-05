use anyhow::{Context, Result, bail};
use shapefile::dbase::FieldValue;
use shapefile::{Shape, ShapeReader};
use std::io::{Read, Seek};
use tracing::{debug, info};

use crate::airspace::{
    AirspaceClass, AirspaceSource, AirspaceType, AltitudeReference, NewAirspace,
};

/// Result of NASR subscription URL discovery
pub struct NasrSubscription {
    /// The edition date (e.g., "2026-02-19")
    pub edition_date: String,
    /// The download URL for the airspace shapefiles
    pub url: String,
}

/// Discover the current NASR 28-day subscription URL from the FAA website.
///
/// Parses the NASR subscription page to find the current edition date,
/// then constructs the download URL for the airspace shapefiles.
pub async fn discover_nasr_url(client: &reqwest::Client) -> Result<NasrSubscription> {
    let page_url =
        "https://www.faa.gov/air_traffic/flight_info/aeronav/aero_data/NASR_Subscription/";

    let html = client
        .get(page_url)
        .send()
        .await
        .context("Failed to fetch NASR subscription page")?
        .text()
        .await
        .context("Failed to read NASR subscription page body")?;

    let document = scraper::Html::parse_document(&html);

    // Look for dates in the format YYYY-MM-DD in links that follow "Current" heading
    // The page has links like "2026-02-19" under a "Current" section
    let link_selector = scraper::Selector::parse("a").expect("valid selector");

    let date_re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").expect("valid regex");

    // Find a link whose text matches a date pattern (YYYY-MM-DD)
    // The first date link on the page is typically the current subscription
    for element in document.select(&link_selector) {
        let text = element.text().collect::<String>().trim().to_string();
        if date_re.is_match(&text) {
            let url = format!(
                "https://nfdc.faa.gov/webContent/28DaySub/{}/class_airspace_shape_files.zip",
                text
            );
            info!(date = %text, url = %url, "Discovered current NASR subscription");
            return Ok(NasrSubscription {
                edition_date: text,
                url,
            });
        }
    }

    bail!("Could not find current NASR subscription date on FAA website")
}

/// Parse a FAA NASR Class_Airspace shapefile and convert to NewAirspace records.
///
/// The shapefile contains polygons with DBF attributes describing US airspace.
/// Returns a Vec of (NewAirspace, GeoJSON geometry) tuples ready for database insertion.
pub fn parse_shapefile<R: Read + Seek>(
    shp_reader: ShapeReader<R>,
    dbf_reader: shapefile::dbase::Reader<R>,
) -> Result<Vec<(NewAirspace, serde_json::Value)>> {
    let mut reader = shapefile::Reader::new(shp_reader, dbf_reader);
    let mut results = Vec::new();
    let mut skipped = 0;

    for (index, result) in reader.iter_shapes_and_records().enumerate() {
        let (shape, record) = result.context("Failed to read shapefile record")?;

        // Extract fields from DBF record
        let ident = get_field_string(&record, "IDENT").unwrap_or_default();
        let name = get_field_string(&record, "NAME").unwrap_or_default();
        let class = get_field_string(&record, "CLASS").unwrap_or_default();
        let local_type = get_field_string(&record, "LOCAL_TYPE").unwrap_or_default();
        let sector = get_field_string(&record, "SECTOR").unwrap_or_default();
        let upper_desc = get_field_string(&record, "UPPER_DESC").unwrap_or_default();
        let upper_val = get_field_numeric(&record, "UPPER_VAL");
        let upper_uom = get_field_string(&record, "UPPER_UOM").unwrap_or_default();
        let upper_code = get_field_string(&record, "UPPER_CODE").unwrap_or_default();
        let lower_desc = get_field_string(&record, "LOWER_DESC").unwrap_or_default();
        let lower_val = get_field_numeric(&record, "LOWER_VAL");
        let lower_uom = get_field_string(&record, "LOWER_UOM").unwrap_or_default();
        let lower_code = get_field_string(&record, "LOWER_CODE").unwrap_or_default();

        // Map CLASS field to AirspaceClass
        let airspace_class = match class.as_str() {
            "B" => Some(AirspaceClass::B),
            "C" => Some(AirspaceClass::C),
            "D" => Some(AirspaceClass::D),
            "E" => Some(AirspaceClass::E),
            other => {
                debug!(class = other, name = %name, "Unknown airspace class, skipping");
                skipped += 1;
                continue;
            }
        };

        // Map LOCAL_TYPE to AirspaceType
        let airspace_type = map_local_type_to_airspace_type(&local_type);

        // Convert geometry to GeoJSON MultiPolygon
        let geojson = match shape_to_geojson_multipolygon(&shape) {
            Some(g) => g,
            None => {
                debug!(ident = %ident, name = %name, "Could not convert geometry, skipping");
                skipped += 1;
                continue;
            }
        };

        // Convert altitude limits
        let (upper_value, upper_unit, upper_reference) =
            convert_faa_altitude(upper_val, &upper_uom, &upper_code, &upper_desc);
        let (lower_value, lower_unit, lower_reference) =
            convert_faa_altitude(lower_val, &lower_uom, &lower_code, &lower_desc);

        // Generate source_id: combine local_type, ident, and sector for uniqueness
        let source_id = if sector.is_empty() {
            format!("{}:{}", local_type, ident)
        } else {
            format!("{}:{}:{}", local_type, ident, sector)
        };

        let display_name = if name.is_empty() {
            ident.clone()
        } else {
            name.clone()
        };

        let airspace = NewAirspace {
            openaip_id: None,
            name: display_name,
            airspace_class,
            airspace_type,
            country_code: Some("US".to_string()),
            lower_value,
            lower_unit,
            lower_reference,
            upper_value,
            upper_unit,
            upper_reference,
            remarks: None,
            activity_type: None,
            openaip_updated_at: None,
            source: AirspaceSource::FaaNasr,
            source_id,
        };

        results.push((airspace, geojson));

        if (index + 1) % 1000 == 0 {
            debug!("Parsed {} shapefile records", index + 1);
        }
    }

    info!(
        "Parsed {} airspace records from shapefile ({} skipped)",
        results.len(),
        skipped
    );
    Ok(results)
}

/// Map FAA LOCAL_TYPE to our AirspaceType enum
fn map_local_type_to_airspace_type(local_type: &str) -> AirspaceType {
    match local_type {
        "CLASS_B" => AirspaceType::Ctr,
        "CLASS_C" => AirspaceType::Ctr,
        "CLASS_D" => AirspaceType::Ctr,
        // Class E subtypes - map to appropriate types
        "CLASS_E2" => AirspaceType::Other, // Surface-based Class E
        "CLASS_E3" => AirspaceType::Other, // Transition area
        "CLASS_E4" => AirspaceType::Other, // Federal airway
        "CLASS_E5" => AirspaceType::Other, // 1200ft AGL floor
        "CLASS_E6" => AirspaceType::Other, // 14500ft MSL floor
        "CLASS_E7" => AirspaceType::Other, // Offshore transition
        _ => AirspaceType::Other,
    }
}

/// Convert a shapefile Shape to a GeoJSON MultiPolygon value
fn shape_to_geojson_multipolygon(shape: &Shape) -> Option<serde_json::Value> {
    match shape {
        Shape::Polygon(polygon) => {
            let rings: Vec<Vec<[f64; 2]>> = polygon
                .rings()
                .iter()
                .map(|ring| ring.points().iter().map(|p| [p.x, p.y]).collect::<Vec<_>>())
                .collect();

            Some(serde_json::json!({
                "type": "MultiPolygon",
                "coordinates": [rings]
            }))
        }
        Shape::NullShape => None,
        other => {
            debug!("Unexpected shape type: {:?}", std::mem::discriminant(other));
            None
        }
    }
}

/// Convert FAA altitude fields to our database format
///
/// FAA uses:
/// - UPPER_VAL / LOWER_VAL: numeric value (e.g., 3000, 180, -9998 for special)
/// - UPPER_UOM / LOWER_UOM: "FT" or "FL"
/// - UPPER_CODE / LOWER_CODE: "MSL", "AGL", "SFC"
/// - UPPER_DESC / LOWER_DESC: "AA" (above to Class A), "TI" (to and including), etc.
fn convert_faa_altitude(
    value: Option<f64>,
    uom: &str,
    code: &str,
    desc: &str,
) -> (Option<i32>, Option<String>, Option<AltitudeReference>) {
    // Handle special values
    match value {
        Some(v) if v <= -9998.0 => {
            // -9998 means "up to but not including Class A" = 18000 FT MSL
            if desc == "AA" {
                return (
                    Some(18000),
                    Some("FT".to_string()),
                    Some(AltitudeReference::Msl),
                );
            }
            return (None, None, None);
        }
        None => return (None, None, None),
        _ => {}
    }

    let val = value.unwrap() as i32;

    // Map UOM
    let unit = match uom {
        "FT" => "FT".to_string(),
        "FL" => "FL".to_string(),
        "" => return (Some(val), None, None),
        other => other.to_string(),
    };

    // Map code to altitude reference
    let reference = match code {
        "MSL" => Some(AltitudeReference::Msl),
        "AGL" => Some(AltitudeReference::Agl),
        "SFC" => Some(AltitudeReference::Gnd),
        _ => {
            if unit == "FL" {
                Some(AltitudeReference::Std)
            } else {
                None
            }
        }
    };

    (Some(val), Some(unit), reference)
}

/// Extract a string field from a DBF record
fn get_field_string(record: &shapefile::dbase::Record, field_name: &str) -> Option<String> {
    record.get(field_name).and_then(|v| match v {
        FieldValue::Character(Some(s)) => {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }
        _ => None,
    })
}

/// Extract a numeric field from a DBF record
fn get_field_numeric(record: &shapefile::dbase::Record, field_name: &str) -> Option<f64> {
    record.get(field_name).and_then(|v| match v {
        FieldValue::Numeric(Some(n)) => Some(*n),
        FieldValue::Float(Some(f)) => Some(*f as f64),
        _ => None,
    })
}

/// Open and parse a NASR Class_Airspace shapefile from a directory path.
///
/// Expects the directory to contain Class_Airspace.shp and Class_Airspace.dbf.
pub fn parse_shapefile_from_dir(
    dir: &std::path::Path,
) -> Result<Vec<(NewAirspace, serde_json::Value)>> {
    let shp_path = dir.join("Class_Airspace.shp");
    let dbf_path = dir.join("Class_Airspace.dbf");

    if !shp_path.exists() {
        bail!("Class_Airspace.shp not found in {}", dir.display());
    }
    if !dbf_path.exists() {
        bail!("Class_Airspace.dbf not found in {}", dir.display());
    }

    info!("Opening shapefile: {}", shp_path.display());

    let shp_reader = ShapeReader::from_path(&shp_path).context("Failed to open .shp file")?;
    let dbf_reader =
        shapefile::dbase::Reader::from_path(&dbf_path).context("Failed to open .dbf file")?;

    parse_shapefile(shp_reader, dbf_reader)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_local_type() {
        assert_eq!(
            map_local_type_to_airspace_type("CLASS_B"),
            AirspaceType::Ctr
        );
        assert_eq!(
            map_local_type_to_airspace_type("CLASS_C"),
            AirspaceType::Ctr
        );
        assert_eq!(
            map_local_type_to_airspace_type("CLASS_D"),
            AirspaceType::Ctr
        );
        assert_eq!(
            map_local_type_to_airspace_type("CLASS_E5"),
            AirspaceType::Other
        );
        assert_eq!(
            map_local_type_to_airspace_type("UNKNOWN"),
            AirspaceType::Other
        );
    }

    #[test]
    fn test_convert_faa_altitude_normal() {
        let (val, unit, reference) = convert_faa_altitude(Some(3000.0), "FT", "MSL", "");
        assert_eq!(val, Some(3000));
        assert_eq!(unit, Some("FT".to_string()));
        assert_eq!(reference, Some(AltitudeReference::Msl));
    }

    #[test]
    fn test_convert_faa_altitude_flight_level() {
        let (val, unit, reference) = convert_faa_altitude(Some(180.0), "FL", "", "");
        assert_eq!(val, Some(180));
        assert_eq!(unit, Some("FL".to_string()));
        assert_eq!(reference, Some(AltitudeReference::Std));
    }

    #[test]
    fn test_convert_faa_altitude_class_a_boundary() {
        let (val, unit, reference) = convert_faa_altitude(Some(-9998.0), "", "", "AA");
        assert_eq!(val, Some(18000));
        assert_eq!(unit, Some("FT".to_string()));
        assert_eq!(reference, Some(AltitudeReference::Msl));
    }

    #[test]
    fn test_convert_faa_altitude_surface() {
        let (val, unit, reference) = convert_faa_altitude(Some(0.0), "FT", "SFC", "");
        assert_eq!(val, Some(0));
        assert_eq!(unit, Some("FT".to_string()));
        assert_eq!(reference, Some(AltitudeReference::Gnd));
    }

    #[test]
    fn test_convert_faa_altitude_none() {
        let (val, unit, reference) = convert_faa_altitude(None, "", "", "");
        assert_eq!(val, None);
        assert_eq!(unit, None);
        assert_eq!(reference, None);
    }
}
