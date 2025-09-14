use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddressType {
    Unknown,
    Icao,
    Flarm,
    OgnTracker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AircraftType {
    Reserved0,
    GliderMotorGlider,
    TowTug,
    HelicopterGyro,
    SkydiverParachute,
    DropPlane,
    HangGlider,
    Paraglider,
    RecipEngine,
    JetTurboprop,
    Unknown,
    Balloon,
    Airship,
    Uav,
    ReservedE,
    StaticObstacle,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, DbEnum)]
#[db_enum(existing_type_path = "crate::schema::sql_types::AdsbEmitterCategory")]
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

impl fmt::Display for AdsbEmitterCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AdsbEmitterCategory::A0 => "A0",
            AdsbEmitterCategory::A1 => "A1",
            AdsbEmitterCategory::A2 => "A2",
            AdsbEmitterCategory::A3 => "A3",
            AdsbEmitterCategory::A4 => "A4",
            AdsbEmitterCategory::A5 => "A5",
            AdsbEmitterCategory::A6 => "A6",
            AdsbEmitterCategory::A7 => "A7",
            AdsbEmitterCategory::B0 => "B0",
            AdsbEmitterCategory::B1 => "B1",
            AdsbEmitterCategory::B2 => "B2",
            AdsbEmitterCategory::B3 => "B3",
            AdsbEmitterCategory::B4 => "B4",
            AdsbEmitterCategory::B6 => "B6",
            AdsbEmitterCategory::B7 => "B7",
            AdsbEmitterCategory::C0 => "C0",
            AdsbEmitterCategory::C1 => "C1",
            AdsbEmitterCategory::C2 => "C2",
            AdsbEmitterCategory::C3 => "C3",
            AdsbEmitterCategory::C4 => "C4",
            AdsbEmitterCategory::C5 => "C5",
        };
        write!(f, "{}", s)
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

#[derive(Debug)]
pub enum ParseOgnError {
    MissingIdField,
    BadIdPrefix,
    BadIdLength,
    BadIdHex,
    BadFlagsHex,
}

/* ---------------------- Tests ---------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_adsb_emitter_category_to_string() {
        assert_eq!(AdsbEmitterCategory::A0.to_string(), "A0");
        assert_eq!(AdsbEmitterCategory::A3.to_string(), "A3");
        assert_eq!(AdsbEmitterCategory::B1.to_string(), "B1");
        assert_eq!(AdsbEmitterCategory::C5.to_string(), "C5");
    }

    #[test]
    fn test_adsb_emitter_category_from_str() {
        assert_eq!(
            "A0".parse::<AdsbEmitterCategory>().unwrap(),
            AdsbEmitterCategory::A0
        );
        assert_eq!(
            "a3".parse::<AdsbEmitterCategory>().unwrap(),
            AdsbEmitterCategory::A3
        );
        assert_eq!(
            "B1".parse::<AdsbEmitterCategory>().unwrap(),
            AdsbEmitterCategory::B1
        );
        assert_eq!(
            "c5".parse::<AdsbEmitterCategory>().unwrap(),
            AdsbEmitterCategory::C5
        );

        // Test invalid category
        assert!("Z9".parse::<AdsbEmitterCategory>().is_err());
        assert!("B5".parse::<AdsbEmitterCategory>().is_err()); // B5 doesn't exist
    }
}
