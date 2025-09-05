use std::str::FromStr;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressType {
    Unknown,
    Icao,
    Flarm,
    OgnTracker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AircraftType {
    Reserved0 = 0x0,
    GliderMotorGlider = 0x1,
    TowTug = 0x2,
    HelicopterGyro = 0x3,
    SkydiverParachute = 0x4,
    DropPlane = 0x5,
    HangGlider = 0x6,
    Paraglider = 0x7,
    RecipEngine = 0x8,
    JetTurboprop = 0x9,
    Unknown = 0xA,
    Balloon = 0xB,
    Airship = 0xC,
    Uav = 0xD,
    ReservedE = 0xE,
    StaticObstacle = 0xF,
}

impl From<u8> for AircraftType {
    fn from(v: u8) -> Self {
        match v & 0x0F {
            0x0 => AircraftType::Reserved0,
            0x1 => AircraftType::GliderMotorGlider,
            0x2 => AircraftType::TowTug,
            0x3 => AircraftType::HelicopterGyro,
            0x4 => AircraftType::SkydiverParachute,
            0x5 => AircraftType::DropPlane,
            0x6 => AircraftType::HangGlider,
            0x7 => AircraftType::Paraglider,
            0x8 => AircraftType::RecipEngine,
            0x9 => AircraftType::JetTurboprop,
            0xA => AircraftType::Unknown,
            0xB => AircraftType::Balloon,
            0xC => AircraftType::Airship,
            0xD => AircraftType::Uav,
            0xE => AircraftType::ReservedE,
            _ => AircraftType::StaticObstacle,
        }
    }
}

impl From<u8> for AddressType {
    fn from(v: u8) -> Self {
        match v & 0x03 {
            0b00 => AddressType::Unknown,
            0b01 => AddressType::Icao,
            0b10 => AddressType::Flarm,
            _ => AddressType::OgnTracker,
        }
    }
}

/// ADS-B emitter category codes as per DO-260B specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdsbEmitterCategory {
    // Category A: Aircraft types
    A0, // No ADS-B emitter category information
    A1, // Light aircraft (< 15,500 lbs)
    A2, // Small aircraft (15,500 to 75,000 lbs)
    A3, // Large aircraft (75,000 to 300,000 lbs)
    A4, // High vortex large aircraft (like B-757)
    A5, // Heavy aircraft (> 300,000 lbs)
    A6, // High performance aircraft
    A7, // Rotorcraft
    
    // Category B: Special aircraft types
    B0, // No ADS-B emitter category information
    B1, // Glider/sailplane
    B2, // Lighter-than-air (airship/balloon)
    B3, // Parachutist/skydiver
    B4, // Ultralight/hang-glider/paraglider
    B6, // Unmanned aerial vehicle
    B7, // Space/trans-atmospheric vehicle
    
    // Category C: Surface vehicles and obstacles
    C0, // No ADS-B emitter category information
    C1, // Surface emergency vehicle
    C2, // Surface service vehicle
    C3, // Point obstacle
    C4, // Cluster obstacle
    C5, // Line obstacle
}

impl AdsbEmitterCategory {
    /// Convert the enum variant to its string representation
    pub fn to_string(&self) -> String {
        match self {
            AdsbEmitterCategory::A0 => "A0".to_string(),
            AdsbEmitterCategory::A1 => "A1".to_string(),
            AdsbEmitterCategory::A2 => "A2".to_string(),
            AdsbEmitterCategory::A3 => "A3".to_string(),
            AdsbEmitterCategory::A4 => "A4".to_string(),
            AdsbEmitterCategory::A5 => "A5".to_string(),
            AdsbEmitterCategory::A6 => "A6".to_string(),
            AdsbEmitterCategory::A7 => "A7".to_string(),
            AdsbEmitterCategory::B0 => "B0".to_string(),
            AdsbEmitterCategory::B1 => "B1".to_string(),
            AdsbEmitterCategory::B2 => "B2".to_string(),
            AdsbEmitterCategory::B3 => "B3".to_string(),
            AdsbEmitterCategory::B4 => "B4".to_string(),
            AdsbEmitterCategory::B6 => "B6".to_string(),
            AdsbEmitterCategory::B7 => "B7".to_string(),
            AdsbEmitterCategory::C0 => "C0".to_string(),
            AdsbEmitterCategory::C1 => "C1".to_string(),
            AdsbEmitterCategory::C2 => "C2".to_string(),
            AdsbEmitterCategory::C3 => "C3".to_string(),
            AdsbEmitterCategory::C4 => "C4".to_string(),
            AdsbEmitterCategory::C5 => "C5".to_string(),
        }
    }
}

impl FromStr for AdsbEmitterCategory {
    type Err = ();
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "A0" => Ok(AdsbEmitterCategory::A0),
            "A1" => Ok(AdsbEmitterCategory::A1),
            "A2" => Ok(AdsbEmitterCategory::A2),
            "A3" => Ok(AdsbEmitterCategory::A3),
            "A4" => Ok(AdsbEmitterCategory::A4),
            "A5" => Ok(AdsbEmitterCategory::A5),
            "A6" => Ok(AdsbEmitterCategory::A6),
            "A7" => Ok(AdsbEmitterCategory::A7),
            "B0" => Ok(AdsbEmitterCategory::B0),
            "B1" => Ok(AdsbEmitterCategory::B1),
            "B2" => Ok(AdsbEmitterCategory::B2),
            "B3" => Ok(AdsbEmitterCategory::B3),
            "B4" => Ok(AdsbEmitterCategory::B4),
            "B6" => Ok(AdsbEmitterCategory::B6),
            "B7" => Ok(AdsbEmitterCategory::B7),
            "C0" => Ok(AdsbEmitterCategory::C0),
            "C1" => Ok(AdsbEmitterCategory::C1),
            "C2" => Ok(AdsbEmitterCategory::C2),
            "C3" => Ok(AdsbEmitterCategory::C3),
            "C4" => Ok(AdsbEmitterCategory::C4),
            "C5" => Ok(AdsbEmitterCategory::C5),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OgnAprsParameters {
    /// Raw two-hex-digit flags (the "XX" after `id`)
    pub id_flags_raw: u8,
    /// 24-bit sender address as parsed from YYYYYY (stored in u32)
    pub address: u32,

    /// Bitfield decode of id_flags_raw (STttttaa).
    pub stealth: bool,
    pub no_tracking: bool,
    pub aircraft_type: AircraftType,
    pub address_type: AddressType,

    /// Optional measurements parsed from the comment
    pub climb_fpm: Option<i32>,            // e.g. -19fpm
    pub turn_rate_rot: Option<f32>,        // e.g. +0.0rot  (rot = half-turns per minute)
    pub snr_db: Option<f32>,               // e.g. 5.5dB
    pub bit_errors_corrected: Option<u32>, // e.g. 3e
    pub freq_offset_khz: Option<f32>,      // e.g. -4.3kHz

    /// APRS precision enhancement (e.g. !W52!)
    pub aprs_pe_lat_digit: Option<u8>,     // '5'
    pub aprs_pe_lon_digit: Option<u8>,     // '2'

    /// Additional APRS fields
    pub flight_number: Option<String>,           // Flight number (e.g. BEL9AD)
    pub emitter_category: Option<AdsbEmitterCategory>, // ADS-B emitter category (e.g. A3)
    pub registration: Option<String>,            // e.g. regOO-SNK  
    pub model: Option<String>,                   // e.g. modelA320
    pub squawk: Option<String>,                  // Squawk code (e.g. Sq1351)
}

#[derive(Debug)]
pub enum ParseOgnError {
    MissingIdField,
    BadIdPrefix,
    BadIdLength,
    BadIdHex,
    BadFlagsHex,
}

impl FromStr for OgnAprsParameters {
    type Err = ParseOgnError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Split by whitespace, but we’ll also scan for the !W..!-block
        let tokens: Vec<&str> = s.split_whitespace().collect();

        // Find the "idXXYYYYYY" token
        let id_tok = tokens.iter().copied().find(|t| t.starts_with("id"))
            .ok_or(ParseOgnError::MissingIdField)?;

        let rest = id_tok.strip_prefix("id").ok_or(ParseOgnError::BadIdPrefix)?;
        // Expect 8 hex chars (2 for flags + 6 for address). Sometimes other OGN formats
        // can be longer (like trackers with more), but the canonical form is 8 after "id".
        if rest.len() < 8 {
            return Err(ParseOgnError::BadIdLength);
        }
        let flags_hex = &rest[..2];
        let addr_hex  = &rest[2..8];

        let id_flags_raw =
            u8::from_str_radix(flags_hex, 16).map_err(|_| ParseOgnError::BadFlagsHex)?;

        let address =
            u32::from_str_radix(addr_hex, 16).map_err(|_| ParseOgnError::BadIdHex)?;

        // Decode STttttaa (MSB -> LSB)
        let stealth     = (id_flags_raw & 0b1000_0000) != 0;
        let no_tracking = (id_flags_raw & 0b0100_0000) != 0;
        let aircraft_type = AircraftType::from((id_flags_raw >> 2) & 0x0F);
        let address_type  = AddressType::from(id_flags_raw & 0x03);

        // Defaults
        let mut climb_fpm: Option<i32> = None;
        let mut turn_rate_rot: Option<f32> = None;
        let mut snr_db: Option<f32> = None;
        let mut bit_errors_corrected: Option<u32> = None;
        let mut freq_offset_khz: Option<f32> = None;
        let mut aprs_pe_lat_digit: Option<u8> = None;
        let mut aprs_pe_lon_digit: Option<u8> = None;
        let mut flight_number: Option<String> = None;
        let mut emitter_category: Option<AdsbEmitterCategory> = None;
        let mut registration: Option<String> = None;
        let mut model: Option<String> = None;
        let mut squawk: Option<String> = None;

        // Also search inline for a "!W..!" block (not necessarily whitespace-separated)
        if let Some(w_start) = s.find("!W")
            && let Some(w_end) = s[w_start + 2..].find('!') {
                let payload = &s[w_start + 2 .. w_start + 2 + w_end];
                let bytes = payload.as_bytes();
                if bytes.len() >= 2 && bytes[0].is_ascii_digit() && bytes[1].is_ascii_digit() {
                    aprs_pe_lat_digit = Some(bytes[0] - b'0');
                    aprs_pe_lon_digit = Some(bytes[1] - b'0');
                }
            }

        // Parse unit-suffixed tokens (case-insensitive for the unit part)
        for tok in tokens {
            let t = tok.trim();
            let tl = t.to_ascii_lowercase();

            // -019fpm
            if tl.ends_with("fpm") {
                if let Some(num) = parse_number_prefix::<i32>(t, 3) {
                    climb_fpm = Some(num);
                }
                continue;
            }

            // +0.0rot
            if tl.ends_with("rot") {
                if let Some(num) = parse_float_prefix::<f32>(t, 3) {
                    turn_rate_rot = Some(num);
                }
                continue;
            }

            // 5.5dB
            if tl.ends_with("db") {
                if let Some(num) = parse_float_prefix::<f32>(t, 2) {
                    snr_db = Some(num);
                }
                continue;
            }

            // 3e  (errors corrected)
            // Require the whole token to be digits followed by 'e' (or 'E')
            if tl.ends_with('e') && tl.chars().take(tl.len().saturating_sub(1)).all(|c| c.is_ascii_digit()) {
                if let Ok(v) = tl[..tl.len() - 1].parse::<u32>() {
                    bit_errors_corrected = Some(v);
                }
                continue;
            }

            // -4.3kHz
            if tl.ends_with("khz") {
                if let Some(num) = parse_float_prefix::<f32>(t, 3) {
                    freq_offset_khz = Some(num);
                }
                continue;
            }

            // Flight number with emitter category: fnA3:BEL9AD or A3:BEL9AD
            // Look for patterns like "fnXX:" or "XX:" where XX is an emitter category
            if let Some(colon_pos) = t.find(':') {
                let prefix = &t[..colon_pos];
                let flight_num = &t[colon_pos + 1..];
                
                // Try to parse with "fn" prefix first
                if prefix.starts_with("fn") && prefix.len() == 4 {
                    let category_str = &prefix[2..];
                    if let Ok(category) = category_str.parse::<AdsbEmitterCategory>() {
                        flight_number = Some(flight_num.to_string());
                        emitter_category = Some(category);
                        continue;
                    }
                }
                
                // Try to parse without "fn" prefix (just the category)
                if prefix.len() == 2 {
                    if let Ok(category) = prefix.parse::<AdsbEmitterCategory>() {
                        flight_number = Some(flight_num.to_string());
                        emitter_category = Some(category);
                        continue;
                    }
                }
            }

            // Registration: regOO-SNK
            if t.starts_with("reg") && t.len() > 3 {
                registration = Some(t[3..].to_string());
                continue;
            }

            // Model: modelA320
            if t.starts_with("model") && t.len() > 5 {
                model = Some(t[5..].to_string());
                continue;
            }
            
            // Squawk: Sq1351 (case insensitive, 4 digits)
            if t.len() == 6 && t.to_ascii_lowercase().starts_with("sq") {
                let digits = &t[2..];
                if digits.chars().all(|c| c.is_ascii_digit()) && digits.len() == 4 {
                    squawk = Some(digits.to_string());
                    continue;
                }
            }
        }

        Ok(OgnAprsParameters {
            id_flags_raw,
            address,
            stealth,
            no_tracking,
            aircraft_type,
            address_type,
            climb_fpm,
            turn_rate_rot,
            snr_db,
            bit_errors_corrected,
            freq_offset_khz,
            aprs_pe_lat_digit,
            aprs_pe_lon_digit,
            flight_number,
            emitter_category,
            registration,
            model,
            squawk,
        })
    }
}

/// Parse the numeric prefix of a token that ends with a fixed unit of length `unit_len`.
/// e.g. t="-019fpm", unit_len=3 -> returns Some(-19)
fn parse_number_prefix<T>(t: &str, unit_len: usize) -> Option<T>
where
    T: std::str::FromStr,
{
    if t.len() < unit_len {
        return None;
    }
    t[..t.len() - unit_len].parse::<T>().ok()
}

fn parse_float_prefix<T>(t: &str, unit_len: usize) -> Option<T>
where
    T: std::str::FromStr,
{
    parse_number_prefix::<T>(t, unit_len)
}

/* ---------------------- Tests ---------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sample_1() {
        let s = "id06DF0A52 -019fpm +0.0rot 5.5dB 3e -4.3kHz !W52!";
        let p: OgnAprsParameters = s.parse().unwrap();

        assert_eq!(p.id_flags_raw, 0x06);
        assert_eq!(p.address, 0xDF0A52);
        assert!(!p.stealth);
        assert!(!p.no_tracking);
        assert_eq!(p.aircraft_type, AircraftType::GliderMotorGlider);
        assert_eq!(p.address_type, AddressType::Flarm);

        assert_eq!(p.climb_fpm, Some(-19));
        assert_eq!(p.turn_rate_rot, Some(0.0));
        assert_eq!(p.snr_db, Some(5.5));
        assert_eq!(p.bit_errors_corrected, Some(3));
        assert_eq!(p.freq_offset_khz, Some(-4.3));
        assert_eq!(p.aprs_pe_lat_digit, Some(5));
        assert_eq!(p.aprs_pe_lon_digit, Some(2));
    }

    #[test]
    fn parses_sample_2() {
        let s = "id0D3E0F90 0fpm +1.2rot 4.0dB 0e -0.1kHz";
        let p: OgnAprsParameters = s.parse().unwrap();

        assert_eq!(p.id_flags_raw, 0x0D);
        assert_eq!(p.address, 0x3E0F90);
        assert!(!p.stealth);
        assert!(!p.no_tracking);
        assert_eq!(p.aircraft_type, AircraftType::HelicopterGyro);
        assert_eq!(p.address_type, AddressType::Icao);

        assert_eq!(p.climb_fpm, Some(0));
        assert_eq!(p.turn_rate_rot, Some(1.2));
        assert_eq!(p.snr_db, Some(4.0));
        assert_eq!(p.bit_errors_corrected, Some(0));
        assert_eq!(p.freq_offset_khz, Some(-0.1));
        assert_eq!(p.aprs_pe_lat_digit, None);
        assert_eq!(p.aprs_pe_lon_digit, None);
        assert_eq!(p.flight_number, None);
        assert_eq!(p.emitter_category, None);
        assert_eq!(p.registration, None);
        assert_eq!(p.model, None);
        assert_eq!(p.squawk, None);
    }

    #[test]
    fn parses_additional_fields() {
        let s = "id06DF0A52 -019fpm +0.0rot 5.5dB 3e -4.3kHz !W52! fnA3:BEL9AD regOO-SNK modelA320";
        let p: OgnAprsParameters = s.parse().unwrap();

        assert_eq!(p.id_flags_raw, 0x06);
        assert_eq!(p.address, 0xDF0A52);
        assert!(!p.stealth);
        assert!(!p.no_tracking);
        assert_eq!(p.aircraft_type, AircraftType::GliderMotorGlider);
        assert_eq!(p.address_type, AddressType::Flarm);

        assert_eq!(p.climb_fpm, Some(-19));
        assert_eq!(p.turn_rate_rot, Some(0.0));
        assert_eq!(p.snr_db, Some(5.5));
        assert_eq!(p.bit_errors_corrected, Some(3));
        assert_eq!(p.freq_offset_khz, Some(-4.3));
        assert_eq!(p.aprs_pe_lat_digit, Some(5));
        assert_eq!(p.aprs_pe_lon_digit, Some(2));
        
        // Test the new fields
        assert_eq!(p.flight_number, Some("BEL9AD".to_string()));
        assert_eq!(p.emitter_category, Some(AdsbEmitterCategory::A3));
        assert_eq!(p.registration, Some("OO-SNK".to_string()));
        assert_eq!(p.model, Some("A320".to_string()));
        assert_eq!(p.squawk, None);
    }

    #[test]
    fn parses_partial_additional_fields() {
        let s = "id06DF0A52 fnA3:KLM123 regN123AB";
        let p: OgnAprsParameters = s.parse().unwrap();

        assert_eq!(p.flight_number, Some("KLM123".to_string()));
        assert_eq!(p.emitter_category, Some(AdsbEmitterCategory::A3));
        assert_eq!(p.registration, Some("N123AB".to_string()));
        assert_eq!(p.model, None);
        assert_eq!(p.squawk, None);
    }

    #[test]
    fn parses_model_only() {
        let s = "id06DF0A52 modelB737";
        let p: OgnAprsParameters = s.parse().unwrap();

        assert_eq!(p.flight_number, None);
        assert_eq!(p.emitter_category, None);
        assert_eq!(p.registration, None);
        assert_eq!(p.model, Some("B737".to_string()));
        assert_eq!(p.squawk, None);
    }
    
    #[test]
    fn parses_squawk_codes() {
        let s1 = "id06DF0A52 Sq1351";
        let p1: OgnAprsParameters = s1.parse().unwrap();
        assert_eq!(p1.squawk, Some("1351".to_string()));
        
        // Test case insensitive
        let s2 = "id06DF0A52 sq7777";
        let p2: OgnAprsParameters = s2.parse().unwrap();
        assert_eq!(p2.squawk, Some("7777".to_string()));
        
        // Test mixed case
        let s3 = "id06DF0A52 Sq0012";
        let p3: OgnAprsParameters = s3.parse().unwrap();
        assert_eq!(p3.squawk, Some("0012".to_string()));
    }
    
    #[test]
    fn parses_emitter_categories_with_flight_numbers() {
        // Test fnA3: format
        let s1 = "id06DF0A52 fnA3:BAW123";
        let p1: OgnAprsParameters = s1.parse().unwrap();
        assert_eq!(p1.flight_number, Some("BAW123".to_string()));
        assert_eq!(p1.emitter_category, Some(AdsbEmitterCategory::A3));
        
        // Test direct A3: format
        let s2 = "id06DF0A52 A3:UAL456";
        let p2: OgnAprsParameters = s2.parse().unwrap();
        assert_eq!(p2.flight_number, Some("UAL456".to_string()));
        assert_eq!(p2.emitter_category, Some(AdsbEmitterCategory::A3));
        
        // Test B1: format (glider)
        let s3 = "id06DF0A52 B1:GLIDER1";
        let p3: OgnAprsParameters = s3.parse().unwrap();
        assert_eq!(p3.flight_number, Some("GLIDER1".to_string()));
        assert_eq!(p3.emitter_category, Some(AdsbEmitterCategory::B1));
        
        // Test C1: format (surface vehicle)
        let s4 = "id06DF0A52 C1:FIRE01";
        let p4: OgnAprsParameters = s4.parse().unwrap();
        assert_eq!(p4.flight_number, Some("FIRE01".to_string()));
        assert_eq!(p4.emitter_category, Some(AdsbEmitterCategory::C1));
    }
    
    #[test]
    fn parses_complete_extended_message() {
        let s = "id06DF0A52 -019fpm +0.0rot 5.5dB 3e -4.3kHz !W52! fnA3:BEL9AD regOO-SNK modelA320 Sq1351";
        let p: OgnAprsParameters = s.parse().unwrap();
        
        // Original fields
        assert_eq!(p.id_flags_raw, 0x06);
        assert_eq!(p.address, 0xDF0A52);
        assert_eq!(p.climb_fpm, Some(-19));
        assert_eq!(p.turn_rate_rot, Some(0.0));
        assert_eq!(p.snr_db, Some(5.5));
        assert_eq!(p.bit_errors_corrected, Some(3));
        assert_eq!(p.freq_offset_khz, Some(-4.3));
        assert_eq!(p.aprs_pe_lat_digit, Some(5));
        assert_eq!(p.aprs_pe_lon_digit, Some(2));
        
        // Extended fields
        assert_eq!(p.flight_number, Some("BEL9AD".to_string()));
        assert_eq!(p.emitter_category, Some(AdsbEmitterCategory::A3));
        assert_eq!(p.registration, Some("OO-SNK".to_string()));
        assert_eq!(p.model, Some("A320".to_string()));
        assert_eq!(p.squawk, Some("1351".to_string()));
    }
    
    #[test]
    fn test_adsb_emitter_category_to_string() {
        assert_eq!(AdsbEmitterCategory::A0.to_string(), "A0");
        assert_eq!(AdsbEmitterCategory::A3.to_string(), "A3");
        assert_eq!(AdsbEmitterCategory::B1.to_string(), "B1");
        assert_eq!(AdsbEmitterCategory::C5.to_string(), "C5");
    }
    
    #[test]
    fn test_adsb_emitter_category_from_str() {
        assert_eq!("A0".parse::<AdsbEmitterCategory>().unwrap(), AdsbEmitterCategory::A0);
        assert_eq!("a3".parse::<AdsbEmitterCategory>().unwrap(), AdsbEmitterCategory::A3);
        assert_eq!("B1".parse::<AdsbEmitterCategory>().unwrap(), AdsbEmitterCategory::B1);
        assert_eq!("c5".parse::<AdsbEmitterCategory>().unwrap(), AdsbEmitterCategory::C5);
        
        // Test invalid category
        assert!("Z9".parse::<AdsbEmitterCategory>().is_err());
        assert!("B5".parse::<AdsbEmitterCategory>().is_err()); // B5 doesn't exist
    }
}
