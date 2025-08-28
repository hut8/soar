use std::str::FromStr;

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

        // Also search inline for a "!W..!" block (not necessarily whitespace-separated)
        if let Some(w_start) = s.find("!W") {
            if let Some(w_end) = s[w_start + 2..].find('!') {
                let payload = &s[w_start + 2 .. w_start + 2 + w_end];
                let bytes = payload.as_bytes();
                if bytes.len() >= 2 && bytes[0].is_ascii_digit() && bytes[1].is_ascii_digit() {
                    aprs_pe_lat_digit = Some(bytes[0] - b'0');
                    aprs_pe_lon_digit = Some(bytes[1] - b'0');
                }
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
    }
}
