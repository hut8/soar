use anyhow::Result;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

const DDB_URL: &str = "http://ddb.glidernet.org/download/?j=1";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DeviceType {
    Flarm,
    Ogn,
    Icao,
    #[default]
    Unknown,
}

impl FromStr for DeviceType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "F" => Ok(DeviceType::Flarm),
            "O" => Ok(DeviceType::Ogn),
            "I" => Ok(DeviceType::Icao),
            "" => Ok(DeviceType::Unknown),
            _ => Ok(DeviceType::Unknown),
        }
    }
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DeviceType::Flarm => "F",
            DeviceType::Ogn => "O",
            DeviceType::Icao => "I",
            DeviceType::Unknown => "",
        };
        write!(f, "{}", s)
    }
}

impl Serialize for DeviceType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for DeviceType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(DeviceType::from_str(&s).unwrap_or(DeviceType::Unknown))
    }
}

// Custom deserializer for string to boolean conversion
fn string_to_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(s.eq_ignore_ascii_case("Y"))
}

// Custom serializer for boolean to string conversion
fn bool_to_string<S>(value: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = if *value { "Y" } else { "" };
    serializer.serialize_str(s)
}

fn hex_to_u32<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    u32::from_str_radix(s, 16).map_err(serde::de::Error::custom)
}

fn u32_to_hex<S>(x: &u32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&format!("{:06X}", x))
}

pub enum RegistrationCountry {
    UnitedStates,
    Other
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub device_type: DeviceType,
    #[serde(deserialize_with = "hex_to_u32", serialize_with = "u32_to_hex")]
    pub device_id: u32,
    pub aircraft_model: String,
    pub registration: String,
    #[serde(rename = "cn")]
    pub competition_number: String,
    #[serde(deserialize_with = "string_to_bool", serialize_with = "bool_to_string")]
    pub tracked: bool,
    #[serde(deserialize_with = "string_to_bool", serialize_with = "bool_to_string")]
    pub identified: bool,
}

#[derive(Debug, Deserialize)]
struct DeviceResponse {
    devices: Vec<Device>,
}

/// Fetcher for devices from the DDB (Device Database)
/// This struct is responsible only for fetching and parsing device data
#[derive(Debug, Default)]
pub struct DeviceFetcher;

impl Device {
    /// Determines the registration country based on the registration format
    /// Returns UnitedStates if the registration follows U.S. N-number format, otherwise Other
    pub fn registration_country(&self) -> RegistrationCountry {
        if self.is_us_n_number(&self.registration) {
            RegistrationCountry::UnitedStates
        } else {
            RegistrationCountry::Other
        }
    }

    /// Checks if a registration follows U.S. N-number format
    /// Restrictions: suffix letters I and O are not allowed
    /// Valid formats:
    /// - One to five digits alone (e.g., N1, N12345)
    /// - One to four digits plus one suffix letter (e.g., N1A, N123Z)
    /// - One to three digits plus two suffix letters (e.g., N12AB)
    fn is_us_n_number(&self, registration: &str) -> bool {
        // Must start with 'N' (case insensitive)
        if !registration.to_uppercase().starts_with('N') {
            return false;
        }

        // Remove the 'N' prefix
        let suffix = &registration[1..];

        // Must have at least 1 character after 'N'
        if suffix.is_empty() {
            return false;
        }

        // Check each valid pattern
        self.matches_digits_only(suffix) ||
        self.matches_digits_plus_one_letter(suffix) ||
        self.matches_digits_plus_two_letters(suffix)
    }

    /// Pattern: One to five digits alone (e.g., N1, N12345)
    fn matches_digits_only(&self, suffix: &str) -> bool {
        suffix.len() >= 1 && suffix.len() <= 5 && suffix.chars().all(|c| c.is_ascii_digit())
    }

    /// Pattern: One to four digits plus one suffix letter (e.g., N1A, N123Z)
    fn matches_digits_plus_one_letter(&self, suffix: &str) -> bool {
        if suffix.len() < 2 || suffix.len() > 5 {
            return false;
        }

        let (digits, letters) = suffix.split_at(suffix.len() - 1);

        // Check digits part (1-4 digits)
        if digits.is_empty() || digits.len() > 4 || !digits.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }

        // Check letter part (exactly 1 letter, not I or O)
        if letters.len() != 1 {
            return false;
        }

        let letter = letters.chars().next().unwrap().to_ascii_uppercase();
        letter.is_ascii_alphabetic() && letter != 'I' && letter != 'O'
    }

    /// Pattern: One to three digits plus two suffix letters (e.g., N12AB)
    fn matches_digits_plus_two_letters(&self, suffix: &str) -> bool {
        if suffix.len() < 3 || suffix.len() > 5 {
            return false;
        }

        let (digits, letters) = suffix.split_at(suffix.len() - 2);

        // Check digits part (1-3 digits)
        if digits.is_empty() || digits.len() > 3 || !digits.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }

        // Check letters part (exactly 2 letters, neither I nor O)
        if letters.len() != 2 {
            return false;
        }

        letters.chars().all(|c| {
            let upper_c = c.to_ascii_uppercase();
            upper_c.is_ascii_alphabetic() && upper_c != 'I' && upper_c != 'O'
        })
    }
}

impl DeviceFetcher {
    pub fn new() -> Self {
        Self
    }

    /// Fetch all devices from the DDB API
    pub async fn fetch_all(&self) -> Result<Vec<Device>> {
        let response = reqwest::get(DDB_URL).await?;
        let device_response: DeviceResponse = response.json().await?;
        Ok(device_response.devices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_device_fetcher_creation() {
        let fetcher = DeviceFetcher::new();
        // Just test that it can be created - actual fetch requires network
        assert_eq!(std::mem::size_of_val(&fetcher), 0); // Zero-sized struct
    }

    #[test]
    fn test_device_serialization_with_booleans() {
        let device = Device {
            device_type: DeviceType::Flarm,
            device_id: 0x000000,
            aircraft_model: "SZD-41 Jantar Std".to_string(),
            registration: "HA-4403".to_string(),
            competition_number: "J".to_string(),
            tracked: true,
            identified: false,
        };

        // Test that the device can be serialized/deserialized
        let json = serde_json::to_string(&device).unwrap();
        let deserialized: Device = serde_json::from_str(&json).unwrap();
        assert_eq!(device, deserialized);
    }

    #[test]
    fn test_device_response_parsing() {
        let json_data = r#"{"devices":[{"device_type":"F","device_id":"000000","aircraft_model":"SZD-41 Jantar Std","registration":"HA-4403","cn":"J","tracked":"Y","identified":""}]}"#;

        let response: DeviceResponse = serde_json::from_str(json_data).unwrap();
        assert_eq!(response.devices.len(), 1);
        assert_eq!(response.devices[0].device_id, 0x000000);
        assert_eq!(response.devices[0].aircraft_model, "SZD-41 Jantar Std");
        assert_eq!(response.devices[0].device_type, DeviceType::Flarm);
        assert!(response.devices[0].tracked);
        assert!(!response.devices[0].identified);
    }

    #[test]
    fn test_device_type_deserialization() {
        // Test Flarm device type
        let flarm_json = r#"{"device_type":"F","device_id":"123456","aircraft_model":"Test","registration":"","cn":"","tracked":"Y","identified":"Y"}"#;
        let flarm_device: Device = serde_json::from_str(flarm_json).unwrap();
        assert_eq!(flarm_device.device_type, DeviceType::Flarm);

        // Test OGN device type
        let ogn_json = r#"{"device_type":"O","device_id":"123456","aircraft_model":"Test","registration":"","cn":"","tracked":"Y","identified":"Y"}"#;
        let ogn_device: Device = serde_json::from_str(ogn_json).unwrap();
        assert_eq!(ogn_device.device_type, DeviceType::Ogn);

        // Test ICAO device type
        let icao_json = r#"{"device_type":"I","device_id":"123456","aircraft_model":"Test","registration":"","cn":"","tracked":"Y","identified":"Y"}"#;
        let icao_device: Device = serde_json::from_str(icao_json).unwrap();
        assert_eq!(icao_device.device_type, DeviceType::Icao);

        // Test empty string (Unknown)
        let unknown_json = r#"{"device_type":"","device_id":"123456","aircraft_model":"Test","registration":"","cn":"","tracked":"Y","identified":"Y"}"#;
        let unknown_device: Device = serde_json::from_str(unknown_json).unwrap();
        assert_eq!(unknown_device.device_type, DeviceType::Unknown);
    }

    #[test]
    fn test_boolean_string_conversion() {
        // Test tracked and identified field conversions
        let json_data = r#"{"device_type":"F","device_id":"123456","aircraft_model":"Test","registration":"","cn":"","tracked":"Y","identified":""}"#;
        let device: Device = serde_json::from_str(json_data).unwrap();
        assert!(device.tracked);
        assert!(!device.identified);

        // Test case insensitive
        let json_data2 = r#"{"device_type":"F","device_id":"123456","aircraft_model":"Test","registration":"","cn":"","tracked":"y","identified":"N"}"#;
        let device2: Device = serde_json::from_str(json_data2).unwrap();
        assert!(device2.tracked);
        assert!(!device2.identified);
    }

    #[test]
    fn test_registration_country_us_n_numbers() {
        // Test valid U.S. N-numbers - digits only (1-5 digits)
        let test_cases = vec![
            ("N1", true),
            ("N12", true),
            ("N123", true),
            ("N1234", true),
            ("N12345", true),
        ];

        for (registration, expected_us) in test_cases {
            let device = create_test_device(registration);
            let country = device.registration_country();
            if expected_us {
                assert!(matches!(country, RegistrationCountry::UnitedStates),
                    "Expected {} to be US registration", registration);
            } else {
                assert!(matches!(country, RegistrationCountry::Other),
                    "Expected {} to be Other registration", registration);
            }
        }
    }

    #[test]
    fn test_registration_country_us_n_numbers_with_one_letter() {
        // Test valid U.S. N-numbers - digits plus one letter (1-4 digits + 1 letter)
        let test_cases = vec![
            ("N1A", true),
            ("N12B", true),
            ("N123C", true),
            ("N1234D", true),
            ("N1Z", true),
            ("N123H", true),
        ];

        for (registration, expected_us) in test_cases {
            let device = create_test_device(registration);
            let country = device.registration_country();
            if expected_us {
                assert!(matches!(country, RegistrationCountry::UnitedStates),
                    "Expected {} to be US registration", registration);
            } else {
                assert!(matches!(country, RegistrationCountry::Other),
                    "Expected {} to be Other registration", registration);
            }
        }
    }

    #[test]
    fn test_registration_country_us_n_numbers_with_two_letters() {
        // Test valid U.S. N-numbers - digits plus two letters (1-3 digits + 2 letters)
        let test_cases = vec![
            ("N1AB", true),
            ("N12CD", true),
            ("N123EF", true),
            ("N1ZZ", true),
            ("N99XY", true),
        ];

        for (registration, expected_us) in test_cases {
            let device = create_test_device(registration);
            let country = device.registration_country();
            if expected_us {
                assert!(matches!(country, RegistrationCountry::UnitedStates),
                    "Expected {} to be US registration", registration);
            } else {
                assert!(matches!(country, RegistrationCountry::Other),
                    "Expected {} to be Other registration", registration);
            }
        }
    }

    #[test]
    fn test_registration_country_invalid_us_n_numbers() {
        // Test invalid U.S. N-numbers
        let test_cases = vec![
            // Too many digits for digits-only pattern
            ("N123456", false),
            // Invalid letters I and O
            ("N1I", false),
            ("N1O", false),
            ("N12I", false),
            ("N12O", false),
            ("N1AI", false),
            ("N1AO", false),
            ("N1IA", false),
            ("N1OA", false),
            // Too many digits with letters
            ("N12345A", false),
            ("N1234AB", false),
            // Empty after N
            ("N", false),
            // No digits before letters
            ("NAB", false),
            ("NA", false),
            // Mixed invalid patterns
            ("N12ABC", false),
        ];

        for (registration, expected_us) in test_cases {
            let device = create_test_device(registration);
            let country = device.registration_country();
            if expected_us {
                assert!(matches!(country, RegistrationCountry::UnitedStates),
                    "Expected {} to be US registration", registration);
            } else {
                assert!(matches!(country, RegistrationCountry::Other),
                    "Expected {} to be Other registration", registration);
            }
        }
    }

    #[test]
    fn test_registration_country_non_us_registrations() {
        // Test non-U.S. registrations
        let test_cases = vec![
            "G-ABCD",    // UK
            "D-ABCD",    // Germany
            "F-ABCD",    // France
            "HA-4403",   // Hungary (from existing test)
            "VH-ABC",    // Australia
            "C-GABC",    // Canada
            "JA1234",    // Japan
            "HL1234",    // South Korea
            "B-1234",    // China
            "ABC123",    // No country prefix
            "123456",    // Just numbers
            "",          // Empty string
        ];

        for registration in test_cases {
            let device = create_test_device(registration);
            let country = device.registration_country();
            assert!(matches!(country, RegistrationCountry::Other),
                "Expected {} to be Other registration", registration);
        }
    }

    #[test]
    fn test_registration_country_case_insensitive() {
        // Test that N-number detection is case insensitive
        let test_cases = vec![
            ("n123", true),
            ("n123a", true),
            ("n12ab", true),
            ("N123", true),
            ("N123A", true),
            ("N12AB", true),
        ];

        for (registration, expected_us) in test_cases {
            let device = create_test_device(registration);
            let country = device.registration_country();
            if expected_us {
                assert!(matches!(country, RegistrationCountry::UnitedStates),
                    "Expected {} to be US registration", registration);
            } else {
                assert!(matches!(country, RegistrationCountry::Other),
                    "Expected {} to be Other registration", registration);
            }
        }
    }

    // Helper function to create a test device with a specific registration
    fn create_test_device(registration: &str) -> Device {
        Device {
            device_type: DeviceType::Flarm,
            device_id: 0x123456,
            aircraft_model: "Test Aircraft".to_string(),
            registration: registration.to_string(),
            competition_number: "T".to_string(),
            tracked: true,
            identified: true,
        }
    }
}
