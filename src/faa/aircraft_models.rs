use anyhow::{anyhow, Context, Result};
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::str::FromStr;

/// Convenience: inclusive 1-based positions from the spec → 0-based Rust slice range
fn fw(s: &str, start_1: usize, end_1: usize) -> &str {
    let start = start_1.saturating_sub(1);
    let end = end_1.min(s.len());
    if start >= end || end > s.len() {
        ""
    } else {
        &s[start..end]
    }
}

fn to_opt_string(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() { None } else { Some(t.to_string()) }
}

fn to_string_trim(s: &str) -> String {
    s.trim().to_string()
}

fn to_opt_u16(s: &str) -> Option<u16> {
    let t = s.trim();
    if t.is_empty() { return None; }
    t.parse::<u16>().ok()
}

/// Type of Aircraft (position 61)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AircraftType {
    Glider,
    Balloon,
    BlimpDirigible,
    FixedWingSingleEngine,
    FixedWingMultiEngine,
    Rotorcraft,
    WeightShiftControl,
    PoweredParachute,
    Gyroplane,
    HybridLift,
    Other,
}

impl FromStr for AircraftType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim() {
            "1" => Ok(AircraftType::Glider),
            "2" => Ok(AircraftType::Balloon),
            "3" => Ok(AircraftType::BlimpDirigible),
            "4" => Ok(AircraftType::FixedWingSingleEngine),
            "5" => Ok(AircraftType::FixedWingMultiEngine),
            "6" => Ok(AircraftType::Rotorcraft),
            "7" => Ok(AircraftType::WeightShiftControl),
            "8" => Ok(AircraftType::PoweredParachute),
            "9" => Ok(AircraftType::Gyroplane),
            "H" => Ok(AircraftType::HybridLift),
            "O" => Ok(AircraftType::Other),
            _ => Err(anyhow!("Invalid aircraft type code: {}", s)),
        }
    }
}

impl fmt::Display for AircraftType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            AircraftType::Glider => "1",
            AircraftType::Balloon => "2",
            AircraftType::BlimpDirigible => "3",
            AircraftType::FixedWingSingleEngine => "4",
            AircraftType::FixedWingMultiEngine => "5",
            AircraftType::Rotorcraft => "6",
            AircraftType::WeightShiftControl => "7",
            AircraftType::PoweredParachute => "8",
            AircraftType::Gyroplane => "9",
            AircraftType::HybridLift => "H",
            AircraftType::Other => "O",
        };
        write!(f, "{}", code)
    }
}

/// Type of Engine (positions 63–64)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineType {
    None,
    Reciprocating,
    TurboProp,
    TurboShaft,
    TurboJet,
    TurboFan,
    Ramjet,
    TwoCycle,
    FourCycle,
    Unknown,
    Electric,
    Rotary,
}

impl FromStr for EngineType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let trimmed = s.trim();
        let code = trimmed.parse::<u8>()
            .with_context(|| format!("Failed to parse engine type code as number: {}", trimmed))?;

        match code {
            0 => Ok(EngineType::None),
            1 => Ok(EngineType::Reciprocating),
            2 => Ok(EngineType::TurboProp),
            3 => Ok(EngineType::TurboShaft),
            4 => Ok(EngineType::TurboJet),
            5 => Ok(EngineType::TurboFan),
            6 => Ok(EngineType::Ramjet),
            7 => Ok(EngineType::TwoCycle),
            8 => Ok(EngineType::FourCycle),
            9 => Ok(EngineType::Unknown),
            10 => Ok(EngineType::Electric),
            11 => Ok(EngineType::Rotary),
            _ => Err(anyhow!("Invalid engine type code: {}", code)),
        }
    }
}

impl fmt::Display for EngineType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            EngineType::None => "0",
            EngineType::Reciprocating => "1",
            EngineType::TurboProp => "2",
            EngineType::TurboShaft => "3",
            EngineType::TurboJet => "4",
            EngineType::TurboFan => "5",
            EngineType::Ramjet => "6",
            EngineType::TwoCycle => "7",
            EngineType::FourCycle => "8",
            EngineType::Unknown => "9",
            EngineType::Electric => "10",
            EngineType::Rotary => "11",
        };
        write!(f, "{}", code)
    }
}

/// Aircraft Category Code (position 66)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AircraftCategory {
    Land,
    Sea,
    Amphibian,
}

impl FromStr for AircraftCategory {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim() {
            "1" => Ok(AircraftCategory::Land),
            "2" => Ok(AircraftCategory::Sea),
            "3" => Ok(AircraftCategory::Amphibian),
            _ => Err(anyhow!("Invalid aircraft category code: {}", s)),
        }
    }
}

impl fmt::Display for AircraftCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            AircraftCategory::Land => "1",
            AircraftCategory::Sea => "2",
            AircraftCategory::Amphibian => "3",
        };
        write!(f, "{}", code)
    }
}

/// Builder Certification Code (position 68)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuilderCertification {
    TypeCertificated,
    NotTypeCertificated,
    LightSport,
}

impl FromStr for BuilderCertification {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim() {
            "0" => Ok(BuilderCertification::TypeCertificated),
            "1" => Ok(BuilderCertification::NotTypeCertificated),
            "2" => Ok(BuilderCertification::LightSport),
            _ => Err(anyhow!("Invalid builder certification code: {}", s)),
        }
    }
}

impl fmt::Display for BuilderCertification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            BuilderCertification::TypeCertificated => "0",
            BuilderCertification::NotTypeCertificated => "1",
            BuilderCertification::LightSport => "2",
        };
        write!(f, "{}", code)
    }
}

/// Aircraft Weight Class (positions 77–83)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeightClass {
    UpTo12499,
    From12500To19999,
    From20000AndOver,
    UavUpTo55,
}

impl FromStr for WeightClass {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let trimmed = s.trim();
        let code = trimmed.strip_prefix("CLASS ").unwrap_or(trimmed);

        match code {
            "1" => Ok(WeightClass::UpTo12499),
            "2" => Ok(WeightClass::From12500To19999),
            "3" => Ok(WeightClass::From20000AndOver),
            "4" => Ok(WeightClass::UavUpTo55),
            _ => Err(anyhow!("Invalid weight class code: {}", s)),
        }
    }
}

impl fmt::Display for WeightClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            WeightClass::UpTo12499 => "CLASS 1",
            WeightClass::From12500To19999 => "CLASS 2",
            WeightClass::From20000AndOver => "CLASS 3",
            WeightClass::UavUpTo55 => "CLASS 4",
        };
        write!(f, "{}", code)
    }
}

/// Aircraft Model record from FAA Aircraft Reference File
#[derive(Debug, Clone)]
pub struct AircraftModel {
    pub manufacturer_code: String,           // positions 1–3
    pub model_code: String,                  // positions 4–5
    pub series_code: String,                 // positions 6–7
    pub manufacturer_name: String,           // positions 9–38
    pub model_name: String,                  // positions 40–59
    pub aircraft_type: Option<AircraftType>, // position 61
    pub engine_type: Option<EngineType>,     // positions 63–64
    pub aircraft_category: Option<AircraftCategory>, // position 66
    pub builder_certification: Option<BuilderCertification>, // position 68
    pub number_of_engines: Option<u16>,      // positions 70–71
    pub number_of_seats: Option<u16>,        // positions 73–75
    pub weight_class: Option<WeightClass>,   // positions 77–83
    pub cruising_speed: Option<u16>,         // positions 85–88
    pub type_certificate_data_sheet: Option<String>, // positions 90–105
    pub type_certificate_data_holder: Option<String>, // positions 107–157
}

impl AircraftModel {
    pub fn from_fixed_width_line(line: &str) -> Result<Self> {
        // Expect at least the last position we touch (156, allowing for shorter lines)
        if line.len() < 156 {
            return Err(anyhow!("Line too short: expected at least 156 chars, got {}", line.len()));
        }

        let manufacturer_code = to_string_trim(fw(line, 1, 3));
        if manufacturer_code.is_empty() {
            return Err(anyhow!("Missing manufacturer code at positions 1–3"));
        }

        let model_code = to_string_trim(fw(line, 4, 5));
        let series_code = to_string_trim(fw(line, 6, 7));
        let manufacturer_name = to_string_trim(fw(line, 9, 38));
        let model_name = to_string_trim(fw(line, 40, 59));

        let aircraft_type = to_opt_string(fw(line, 61, 61))
            .map(|s| AircraftType::from_str(&s))
            .transpose()?;

        let engine_type = to_opt_string(fw(line, 63, 64))
            .map(|s| EngineType::from_str(&s))
            .transpose()?;

        let aircraft_category = to_opt_string(fw(line, 66, 66))
            .map(|s| AircraftCategory::from_str(&s))
            .transpose()?;

        let builder_certification = to_opt_string(fw(line, 68, 68))
            .map(|s| BuilderCertification::from_str(&s))
            .transpose()?;

        let number_of_engines = to_opt_u16(fw(line, 70, 71));
        let number_of_seats = to_opt_u16(fw(line, 73, 75));

        let weight_class = to_opt_string(fw(line, 77, 83))
            .map(|s| WeightClass::from_str(&s))
            .transpose()?;

        let cruising_speed = to_opt_u16(fw(line, 85, 88));
        let type_certificate_data_sheet = to_opt_string(fw(line, 90, 105));
        let type_certificate_data_holder = to_opt_string(fw(line, 107, 157));

        Ok(AircraftModel {
            manufacturer_code,
            model_code,
            series_code,
            manufacturer_name,
            model_name,
            aircraft_type,
            engine_type,
            aircraft_category,
            builder_certification,
            number_of_engines,
            number_of_seats,
            weight_class,
            cruising_speed,
            type_certificate_data_sheet,
            type_certificate_data_holder,
        })
    }
}

/// Read a fixed-width FAA Aircraft Reference file and parse all rows.
/// Skips blank lines. Returns an error on the first malformed (too-short) line.
pub fn read_aircraft_models_file<P: AsRef<Path>>(path: P) -> Result<Vec<AircraftModel>> {
    let f = File::open(path.as_ref())
        .with_context(|| format!("Opening {:?}", path.as_ref()))?;
    let reader = BufReader::new(f);
    let mut out = Vec::new();

    for (lineno, line) in reader.lines().enumerate().skip(1) {
        let line = line.with_context(|| format!("Reading line {}", lineno + 1))?;
        let trimmed = line.trim_end_matches(&['\r', '\n'][..]);

        if trimmed.trim().is_empty() {
            continue;
        }
        let rec = AircraftModel::from_fixed_width_line(trimmed)
            .with_context(|| format!("Parsing line {}", lineno + 1))?;
        out.push(rec);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aircraft_type_enum() {
        assert_eq!(AircraftType::from_str("1").unwrap(), AircraftType::Glider);
        assert_eq!(AircraftType::from_str("4").unwrap(), AircraftType::FixedWingSingleEngine);
        assert_eq!(AircraftType::from_str("H").unwrap(), AircraftType::HybridLift);
        assert_eq!(AircraftType::from_str("O").unwrap(), AircraftType::Other);

        assert_eq!(AircraftType::Glider.to_string(), "1");
        assert_eq!(AircraftType::HybridLift.to_string(), "H");
        assert_eq!(AircraftType::Other.to_string(), "O");

        assert!(AircraftType::from_str("X").is_err());
    }

    #[test]
    fn test_engine_type_enum() {
        assert_eq!(EngineType::from_str("0").unwrap(), EngineType::None);
        assert_eq!(EngineType::from_str("1").unwrap(), EngineType::Reciprocating);
        assert_eq!(EngineType::from_str("10").unwrap(), EngineType::Electric);
        assert_eq!(EngineType::from_str("11").unwrap(), EngineType::Rotary);

        // Test leading zeros are handled correctly
        assert_eq!(EngineType::from_str("01").unwrap(), EngineType::Reciprocating);
        assert_eq!(EngineType::from_str("02").unwrap(), EngineType::TurboProp);
        assert_eq!(EngineType::from_str("00").unwrap(), EngineType::None);

        assert_eq!(EngineType::None.to_string(), "0");
        assert_eq!(EngineType::Electric.to_string(), "10");
        assert_eq!(EngineType::Rotary.to_string(), "11");

        assert!(EngineType::from_str("12").is_err());
    }

    #[test]
    fn test_aircraft_category_enum() {
        assert_eq!(AircraftCategory::from_str("1").unwrap(), AircraftCategory::Land);
        assert_eq!(AircraftCategory::from_str("2").unwrap(), AircraftCategory::Sea);
        assert_eq!(AircraftCategory::from_str("3").unwrap(), AircraftCategory::Amphibian);

        assert_eq!(AircraftCategory::Land.to_string(), "1");
        assert_eq!(AircraftCategory::Sea.to_string(), "2");
        assert_eq!(AircraftCategory::Amphibian.to_string(), "3");

        assert!(AircraftCategory::from_str("4").is_err());
    }

    #[test]
    fn test_builder_certification_enum() {
        assert_eq!(BuilderCertification::from_str("0").unwrap(), BuilderCertification::TypeCertificated);
        assert_eq!(BuilderCertification::from_str("1").unwrap(), BuilderCertification::NotTypeCertificated);
        assert_eq!(BuilderCertification::from_str("2").unwrap(), BuilderCertification::LightSport);

        assert_eq!(BuilderCertification::TypeCertificated.to_string(), "0");
        assert_eq!(BuilderCertification::NotTypeCertificated.to_string(), "1");
        assert_eq!(BuilderCertification::LightSport.to_string(), "2");

        assert!(BuilderCertification::from_str("3").is_err());
    }

    #[test]
    fn test_weight_class_enum() {
        // Test parsing without "CLASS " prefix
        assert_eq!(WeightClass::from_str("1").unwrap(), WeightClass::UpTo12499);
        assert_eq!(WeightClass::from_str("2").unwrap(), WeightClass::From12500To19999);
        assert_eq!(WeightClass::from_str("3").unwrap(), WeightClass::From20000AndOver);
        assert_eq!(WeightClass::from_str("4").unwrap(), WeightClass::UavUpTo55);

        // Test parsing with "CLASS " prefix
        assert_eq!(WeightClass::from_str("CLASS 1").unwrap(), WeightClass::UpTo12499);
        assert_eq!(WeightClass::from_str("CLASS 2").unwrap(), WeightClass::From12500To19999);
        assert_eq!(WeightClass::from_str("CLASS 3").unwrap(), WeightClass::From20000AndOver);
        assert_eq!(WeightClass::from_str("CLASS 4").unwrap(), WeightClass::UavUpTo55);

        // Test Display format (should include "CLASS " prefix)
        assert_eq!(WeightClass::UpTo12499.to_string(), "CLASS 1");
        assert_eq!(WeightClass::From12500To19999.to_string(), "CLASS 2");
        assert_eq!(WeightClass::From20000AndOver.to_string(), "CLASS 3");
        assert_eq!(WeightClass::UavUpTo55.to_string(), "CLASS 4");

        // Test invalid codes
        assert!(WeightClass::from_str("5").is_err());
        assert!(WeightClass::from_str("CLASS 5").is_err());
    }

    #[test]
    fn test_aircraft_model_parsing() {
        // Create a test line with the expected fixed-width format
        // Build the line character by character to ensure exact positioning
        let mut test_line = String::with_capacity(200);

        // Positions 1-3: Manufacturer Code
        test_line.push_str("ABC");
        // Positions 4-5: Model Code
        test_line.push_str("12");
        // Positions 6-7: Series Code
        test_line.push_str("34");
        // Position 8: Space
        test_line.push(' ');
        // Positions 9-38: Manufacturer Name (30 chars)
        test_line.push_str(&format!("{:<30}", "TEST MANUFACTURER"));
        // Position 39: Space
        test_line.push(' ');
        // Positions 40-59: Model Name (20 chars)
        test_line.push_str(&format!("{:<20}", "TEST MODEL"));
        // Position 60: Space
        test_line.push(' ');
        // Position 61: Aircraft Type
        test_line.push('4');
        // Position 62: Space
        test_line.push(' ');
        // Positions 63-64: Engine Type
        test_line.push('1');
        test_line.push(' '); // pad to 2 chars
        // Position 65: Space
        test_line.push(' ');
        // Position 66: Aircraft Category
        test_line.push('1');
        // Position 67: Space
        test_line.push(' ');
        // Position 68: Builder Certification
        test_line.push('0');
        // Position 69: Space
        test_line.push(' ');
        // Positions 70-71: Number of Engines
        test_line.push_str(" 1");
        // Position 72: Space
        test_line.push(' ');
        // Positions 73-75: Number of Seats
        test_line.push_str("  4");
        // Position 76: Space
        test_line.push(' ');
        // Positions 77-83: Weight Class (7 chars)
        test_line.push_str("1      ");
        // Position 84: Space
        test_line.push(' ');
        // Positions 85-88: Cruising Speed
        test_line.push_str(" 120");
        // Position 89: Space
        test_line.push(' ');
        // Positions 90-105: Type Certificate Data Sheet (16 chars)
        test_line.push_str(&format!("{:<16}", "A23CE"));
        // Position 106: Space
        test_line.push(' ');
        // Positions 107-157: Type Certificate Data Holder (51 chars)
        test_line.push_str(&format!("{:<51}", "TEST CERTIFICATE HOLDER"));

        let model = AircraftModel::from_fixed_width_line(&test_line).unwrap();

        assert_eq!(model.manufacturer_code, "ABC");
        assert_eq!(model.model_code, "12");
        assert_eq!(model.series_code, "34");
        assert_eq!(model.manufacturer_name, "TEST MANUFACTURER");
        assert_eq!(model.model_name, "TEST MODEL");
        assert_eq!(model.aircraft_type, Some(AircraftType::FixedWingSingleEngine));
        assert_eq!(model.engine_type, Some(EngineType::Reciprocating));
        assert_eq!(model.aircraft_category, Some(AircraftCategory::Land));
        assert_eq!(model.builder_certification, Some(BuilderCertification::TypeCertificated));
        assert_eq!(model.number_of_engines, Some(1));
        assert_eq!(model.number_of_seats, Some(4));
        assert_eq!(model.weight_class, Some(WeightClass::UpTo12499));
        assert_eq!(model.cruising_speed, Some(120));
        assert_eq!(model.type_certificate_data_sheet, Some("A23CE".to_string()));
        assert_eq!(model.type_certificate_data_holder, Some("TEST CERTIFICATE HOLDER".to_string()));
    }
}
