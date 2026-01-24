use anyhow::{Result, anyhow};
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use ts_rs::TS;

/// FAA Aircraft Type enum based on MASTER.TXT Type Aircraft Code
/// Maps the FAA codes (1-9, H, O) to descriptive aircraft types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, DbEnum)]
#[db_enum(existing_type_path = "crate::schema::sql_types::AircraftType")]
pub enum AircraftType {
    Glider,                // Code: 1
    Balloon,               // Code: 2
    BlimpDirigible,        // Code: 3
    FixedWingSingleEngine, // Code: 4
    FixedWingMultiEngine,  // Code: 5
    Rotorcraft,            // Code: 6
    WeightShiftControl,    // Code: 7
    PoweredParachute,      // Code: 8
    Gyroplane,             // Code: 9
    HybridLift,            // Code: H
    Other,                 // Code: O
}

impl FromStr for AircraftType {
    type Err = anyhow::Error;

    /// Parse FAA aircraft type code from MASTER.TXT
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
        let name = match self {
            AircraftType::Glider => "Glider",
            AircraftType::Balloon => "Balloon",
            AircraftType::BlimpDirigible => "Blimp/Dirigible",
            AircraftType::FixedWingSingleEngine => "Fixed Wing Single Engine",
            AircraftType::FixedWingMultiEngine => "Fixed Wing Multi Engine",
            AircraftType::Rotorcraft => "Rotorcraft",
            AircraftType::WeightShiftControl => "Weight-shift-control",
            AircraftType::PoweredParachute => "Powered Parachute",
            AircraftType::Gyroplane => "Gyroplane",
            AircraftType::HybridLift => "Hybrid Lift",
            AircraftType::Other => "Other",
        };
        write!(f, "{}", name)
    }
}

/// Get the FAA code for an aircraft type (for persistence in MASTER.TXT format)
impl AircraftType {
    pub fn to_faa_code(&self) -> &'static str {
        match self {
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
        }
    }
}

/// Aircraft Category - unified classification combining ICAO Doc 8643 categories with OGN types
///
/// ICAO standard values: Landplane, Seaplane, Amphibian, Gyroplane, Helicopter, Tiltrotor
/// Extended values for non-standard types: Balloon, Drone, PoweredParachute, Rotorcraft, Vtol, Electric
/// OGN-specific values: Glider, TowTug, Paraglider, HangGlider, Airship, SkydiverParachute, StaticObstacle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, DbEnum, TS)]
#[db_enum(existing_type_path = "crate::schema::sql_types::AircraftCategory")]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
pub enum AircraftCategory {
    // ICAO Doc 8643 categories
    Landplane,        // L (ICAO Doc 8643)
    Helicopter,       // H (ICAO Doc 8643)
    Balloon,          // B (extended)
    Amphibian,        // A (ICAO Doc 8643)
    Gyroplane,        // G (ICAO Doc 8643 - gyrocopter)
    Drone,            // D (extended)
    PoweredParachute, // P (extended)
    Rotorcraft,       // R (extended)
    Seaplane,         // S (ICAO Doc 8643)
    Tiltrotor,        // T (ICAO Doc 8643)
    Vtol,             // V (extended)
    Electric,         // E (extended)
    // OGN-specific categories (from aircraft_type_ogn)
    Glider,            // OGN type 0x1
    TowTug,            // OGN type 0x2
    Paraglider,        // OGN type 0x7
    HangGlider,        // OGN type 0x6
    Airship,           // OGN type 0xC
    SkydiverParachute, // OGN type 0x4
    StaticObstacle,    // OGN type 0xF
    Unknown,           // - (unknown/unspecified)
}

impl AircraftCategory {
    /// Parse from ICAO description first character (ICAO Doc 8643)
    pub fn from_short_type_char(c: char) -> Option<Self> {
        match c {
            'L' => Some(AircraftCategory::Landplane),
            'H' => Some(AircraftCategory::Helicopter),
            'B' => Some(AircraftCategory::Balloon),
            'A' => Some(AircraftCategory::Amphibian),
            'G' => Some(AircraftCategory::Gyroplane),
            'D' => Some(AircraftCategory::Drone),
            'P' => Some(AircraftCategory::PoweredParachute),
            'R' => Some(AircraftCategory::Rotorcraft),
            'S' => Some(AircraftCategory::Seaplane),
            'T' => Some(AircraftCategory::Tiltrotor),
            'V' => Some(AircraftCategory::Vtol),
            'E' => Some(AircraftCategory::Electric),
            '-' => Some(AircraftCategory::Unknown),
            _ => None,
        }
    }

    /// Decode OGN aircraft type byte to AircraftCategory
    ///
    /// OGN/APRS packets encode aircraft type in a 4-bit field (0x0-0xF).
    /// This maps those values to the unified AircraftCategory enum.
    pub fn from_ogn_byte(v: u8) -> Self {
        match v & 0x0F {
            0x0 => AircraftCategory::Unknown, // Reserved (treated as Unknown)
            0x1 => AircraftCategory::Glider,  // Glider/sailplane
            0x2 => AircraftCategory::TowTug,  // Tow plane
            0x3 => AircraftCategory::Rotorcraft, // Helicopter/gyrocopter
            0x4 => AircraftCategory::SkydiverParachute, // Skydiver/parachute
            0x5 => AircraftCategory::Landplane, // Drop plane (fixed-wing)
            0x6 => AircraftCategory::HangGlider, // Hang glider
            0x7 => AircraftCategory::Paraglider, // Paraglider
            0x8 => AircraftCategory::Landplane, // Reciprocating engine aircraft
            0x9 => AircraftCategory::Landplane, // Jet/turboprop aircraft
            0xA => AircraftCategory::Unknown, // Unknown
            0xB => AircraftCategory::Balloon, // Balloon
            0xC => AircraftCategory::Airship, // Airship
            0xD => AircraftCategory::Drone,   // UAV/drone
            0xE => AircraftCategory::Unknown, // Reserved
            _ => AircraftCategory::StaticObstacle, // Static obstacle (0xF)
        }
    }
}

impl FromStr for AircraftCategory {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "landplane" => Ok(AircraftCategory::Landplane),
            "helicopter" => Ok(AircraftCategory::Helicopter),
            "balloon" => Ok(AircraftCategory::Balloon),
            "amphibian" => Ok(AircraftCategory::Amphibian),
            "gyroplane" => Ok(AircraftCategory::Gyroplane),
            "drone" => Ok(AircraftCategory::Drone),
            "powered_parachute" | "poweredparachute" => Ok(AircraftCategory::PoweredParachute),
            "rotorcraft" => Ok(AircraftCategory::Rotorcraft),
            "seaplane" => Ok(AircraftCategory::Seaplane),
            "tiltrotor" => Ok(AircraftCategory::Tiltrotor),
            "vtol" => Ok(AircraftCategory::Vtol),
            "electric" => Ok(AircraftCategory::Electric),
            "glider" => Ok(AircraftCategory::Glider),
            "tow_tug" | "towtug" => Ok(AircraftCategory::TowTug),
            "paraglider" => Ok(AircraftCategory::Paraglider),
            "hang_glider" | "hangglider" => Ok(AircraftCategory::HangGlider),
            "airship" => Ok(AircraftCategory::Airship),
            "skydiver_parachute" | "skydiverparachute" => Ok(AircraftCategory::SkydiverParachute),
            "static_obstacle" | "staticobstacle" => Ok(AircraftCategory::StaticObstacle),
            "unknown" => Ok(AircraftCategory::Unknown),
            _ => Err(anyhow!("Invalid aircraft category: {}", s)),
        }
    }
}

/// ICAO Engine Type (3rd character of ICAO description from ICAO Doc 8643)
/// ICAO standard values: Piston, Jet, Turbine (turboprop/turboshaft), Electric, Rocket
/// Extended values: Special, None (for gliders/balloons)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, DbEnum)]
#[db_enum(existing_type_path = "crate::schema::sql_types::EngineType")]
pub enum EngineType {
    Piston,   // P (ICAO Doc 8643)
    Jet,      // J (ICAO Doc 8643)
    Turbine,  // T (ICAO Doc 8643 - turboprop/turboshaft)
    Electric, // E (ICAO Doc 8643)
    Rocket,   // R (ICAO Doc 8643)
    Special,  // S (extended)
    None,     // - (extended - no engine: glider, balloon, ground vehicle)
    Unknown,  // unknown/unspecified
}

impl EngineType {
    /// Parse from ICAO description third character (ICAO Doc 8643)
    pub fn from_short_type_char(c: char) -> Option<Self> {
        match c {
            'P' => Some(EngineType::Piston),
            'J' => Some(EngineType::Jet),
            'T' => Some(EngineType::Turbine),
            'E' => Some(EngineType::Electric),
            'R' => Some(EngineType::Rocket),
            'S' => Some(EngineType::Special),
            '-' => Some(EngineType::None),
            _ => None,
        }
    }
}

/// Wing type for ICAO aircraft type classification
/// Data source: https://www.kaggle.com/datasets/colmog/aircraft-and-aircraft-manufacturers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, DbEnum)]
#[db_enum(existing_type_path = "crate::schema::sql_types::WingType")]
#[serde(rename_all = "snake_case")]
pub enum WingType {
    FixedWing,
    RotaryWing,
}

impl FromStr for WingType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim() {
            "Fixed Wing" => Ok(WingType::FixedWing),
            "Rotary Wing" => Ok(WingType::RotaryWing),
            _ => Err(anyhow!("Invalid wing type: {}", s)),
        }
    }
}

impl fmt::Display for WingType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            WingType::FixedWing => "Fixed Wing",
            WingType::RotaryWing => "Rotary Wing",
        };
        write!(f, "{}", name)
    }
}

/// Aircraft category for ICAO type classification (Airplane vs Helicopter)
/// Data source: https://www.kaggle.com/datasets/colmog/aircraft-and-aircraft-manufacturers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, DbEnum)]
#[db_enum(existing_type_path = "crate::schema::sql_types::IcaoAircraftCategory")]
#[serde(rename_all = "snake_case")]
pub enum IcaoAircraftCategory {
    Airplane,
    Helicopter,
}

impl FromStr for IcaoAircraftCategory {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim() {
            "Airplane" => Ok(IcaoAircraftCategory::Airplane),
            "Helicopter" => Ok(IcaoAircraftCategory::Helicopter),
            _ => Err(anyhow!("Invalid aircraft category: {}", s)),
        }
    }
}

impl fmt::Display for IcaoAircraftCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            IcaoAircraftCategory::Airplane => "Airplane",
            IcaoAircraftCategory::Helicopter => "Helicopter",
        };
        write!(f, "{}", name)
    }
}
