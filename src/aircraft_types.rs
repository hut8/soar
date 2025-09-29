use anyhow::{Result, anyhow};
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

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
