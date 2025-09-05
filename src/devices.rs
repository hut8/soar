use anyhow::Result;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

const DDB_URL: &str = "http://ddb.glidernet.org/download/?j=1";

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Default)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub device_type: DeviceType,
    pub device_id: String,
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

impl DeviceFetcher {
    pub fn new() -> Self {
        Self::default()
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
            device_id: "000000".to_string(),
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
        assert_eq!(response.devices[0].device_id, "000000");
        assert_eq!(response.devices[0].aircraft_model, "SZD-41 Jantar Std");
        assert_eq!(response.devices[0].device_type, DeviceType::Flarm);
        assert_eq!(response.devices[0].tracked, true);
        assert_eq!(response.devices[0].identified, false);
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
        assert_eq!(device.tracked, true);
        assert_eq!(device.identified, false);

        // Test case insensitive
        let json_data2 = r#"{"device_type":"F","device_id":"123456","aircraft_model":"Test","registration":"","cn":"","tracked":"y","identified":"N"}"#;
        let device2: Device = serde_json::from_str(json_data2).unwrap();
        assert_eq!(device2.tracked, true);
        assert_eq!(device2.identified, false);
    }
}