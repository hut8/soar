//! IGC file format generation for flight exports
//!
//! This module generates IGC (International Gliding Commission) files from flight data.
//! IGC is a standard format for recording glider flights, commonly used for flight
//! verification and analysis in the soaring community.
//!
//! Reference: https://xp-soaring.github.io/igc_file_format/igc_format_2008.html

use crate::aircraft::Aircraft;
use crate::fixes::Fix;
use crate::flights::Flight;
use chrono::{DateTime, Datelike, Timelike, Utc};

/// Generate an IGC file from flight data and fixes
///
/// Returns the IGC content as a string with CRLF line endings.
/// The generated file is "unofficial" (manufacturer code XXX) since SOAR
/// is not a certified flight recorder.
pub fn generate_igc(flight: &Flight, fixes: &[Fix], aircraft: Option<&Aircraft>) -> String {
    let mut igc = String::new();

    // A record - Manufacturer and logger ID
    // XXX = unofficial/uncertified logger
    igc.push_str("AXXXSOAR\r\n");

    // Get flight date from takeoff time or first fix
    let flight_date = flight
        .takeoff_time
        .or_else(|| fixes.first().map(|f| f.timestamp))
        .unwrap_or_else(Utc::now);

    // H records - Header information
    write_header_records(&mut igc, flight, &flight_date, aircraft);

    // I record - B record extension definitions
    // We don't include extensions to keep it simple
    // Format: I NN [SS FF CCC]...
    // For now, just basic B records without extensions
    igc.push_str("I00\r\n");

    // B records - Position fixes
    for fix in fixes {
        write_b_record(&mut igc, fix);
    }

    igc
}

/// Write header (H) records to the IGC file
fn write_header_records(
    igc: &mut String,
    flight: &Flight,
    flight_date: &DateTime<Utc>,
    aircraft: Option<&Aircraft>,
) {
    // HFDTE - Flight date (DDMMYY)
    igc.push_str(&format!(
        "HFDTE{:02}{:02}{:02}\r\n",
        flight_date.day(),
        flight_date.month(),
        flight_date.year() % 100
    ));

    // HFFXA - Fix accuracy (meters) - we use 035 as a reasonable default for APRS/ADS-B
    igc.push_str("HFFXA035\r\n");

    // HFPLT - Pilot in charge
    let pilot_name = aircraft
        .and_then(|a| a.pilot_name.as_deref().or(a.owner_operator.as_deref()))
        .unwrap_or("Unknown");
    igc.push_str(&format!("HFPLTPILOTINCHARGE:{}\r\n", pilot_name));

    // HFGTY - Glider type
    let glider_type = aircraft
        .map(|a| a.aircraft_model.as_str())
        .unwrap_or("Unknown");
    igc.push_str(&format!("HFGTYGLIDERTYPE:{}\r\n", glider_type));

    // HFGID - Glider ID (registration or device address)
    let glider_id = aircraft
        .and_then(|a| a.registration.as_deref())
        .unwrap_or(&flight.device_address);
    igc.push_str(&format!("HFGIDGLIDERID:{}\r\n", glider_id));

    // HFDTM - GPS Datum (100 = WGS-1984)
    igc.push_str("HFDTM100GPSDATUM:WGS-1984\r\n");

    // HFFTY - Flight recorder type
    igc.push_str("HFFTYFRTYPE:SOAR Flight Tracker\r\n");

    // HFGPS - GPS info (generic since we aggregate multiple sources)
    igc.push_str("HFGPSGPS:Multiple Sources,12ch,18000m\r\n");
}

/// Write a B (fix) record to the IGC file
///
/// Format: B HHMMSS DDMMmmmN DDDMMmmmE V PPPPP GGGGG
/// - Time: HHMMSS UTC
/// - Latitude: DDMMmmmN/S (degrees, minutes with 3 decimal places)
/// - Longitude: DDDMMmmmE/W (degrees, minutes with 3 decimal places)
/// - Validity: A (3D fix) or V (2D/no GPS)
/// - Pressure altitude: 5 digits in meters
/// - GPS altitude: 5 digits in meters
fn write_b_record(igc: &mut String, fix: &Fix) {
    // Time (HHMMSS)
    let time = format!(
        "{:02}{:02}{:02}",
        fix.timestamp.hour(),
        fix.timestamp.minute(),
        fix.timestamp.second()
    );

    // Latitude (DDMMmmmN/S)
    let lat = format_latitude(fix.latitude);

    // Longitude (DDDMMmmmE/W)
    let lon = format_longitude(fix.longitude);

    // Validity - A for 3D fix (we assume valid since we have altitude)
    let validity = if fix.altitude_msl_feet.is_some() {
        'A'
    } else {
        'V'
    };

    // Pressure altitude (meters) - we use GPS altitude as approximation
    // IGC spec requires 5 digits with leading zeros
    let pressure_alt = fix
        .altitude_msl_feet
        .map(|ft| (ft as f64 * 0.3048).round() as i32)
        .unwrap_or(0)
        .max(0);

    // GPS altitude (meters)
    let gps_alt = fix
        .altitude_msl_feet
        .map(|ft| (ft as f64 * 0.3048).round() as i32)
        .unwrap_or(0)
        .max(0);

    igc.push_str(&format!(
        "B{}{}{}{}{:05}{:05}\r\n",
        time, lat, lon, validity, pressure_alt, gps_alt
    ));
}

/// Format latitude as DDMMmmmN/S
/// Example: 54.11868 -> 5407121N
fn format_latitude(lat: f64) -> String {
    let abs_lat = lat.abs();
    let degrees = abs_lat.trunc() as u32;
    let minutes = (abs_lat - degrees as f64) * 60.0;
    let minutes_int = minutes.trunc() as u32;
    let minutes_frac = ((minutes - minutes_int as f64) * 1000.0).round() as u32;

    let hemisphere = if lat >= 0.0 { 'N' } else { 'S' };

    format!(
        "{:02}{:02}{:03}{}",
        degrees, minutes_int, minutes_frac, hemisphere
    )
}

/// Format longitude as DDDMMmmmE/W
/// Example: -2.82237 -> 00249342W
fn format_longitude(lon: f64) -> String {
    let abs_lon = lon.abs();
    let degrees = abs_lon.trunc() as u32;
    let minutes = (abs_lon - degrees as f64) * 60.0;
    let minutes_int = minutes.trunc() as u32;
    let minutes_frac = ((minutes - minutes_int as f64) * 1000.0).round() as u32;

    let hemisphere = if lon >= 0.0 { 'E' } else { 'W' };

    format!(
        "{:03}{:02}{:03}{}",
        degrees, minutes_int, minutes_frac, hemisphere
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_latitude_north() {
        // 54.11868 degrees = 54° 07.121' N
        let result = format_latitude(54.11868);
        assert_eq!(result, "5407121N");
    }

    #[test]
    fn test_format_latitude_south() {
        // -33.5 degrees = 33° 30.000' S
        let result = format_latitude(-33.5);
        assert_eq!(result, "3330000S");
    }

    #[test]
    fn test_format_longitude_west() {
        // -2.82237 degrees = 2° 49.342' W
        let result = format_longitude(-2.82237);
        assert_eq!(result, "00249342W");
    }

    #[test]
    fn test_format_longitude_east() {
        // 10.25 degrees = 10° 15.000' E
        let result = format_longitude(10.25);
        assert_eq!(result, "01015000E");
    }

    #[test]
    fn test_format_latitude_zero() {
        let result = format_latitude(0.0);
        assert_eq!(result, "0000000N");
    }

    #[test]
    fn test_format_longitude_zero() {
        let result = format_longitude(0.0);
        assert_eq!(result, "00000000E");
    }
}
