use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

const DDB_URL: &str = "http://ddb.glidernet.org/download/?j=1";

#[derive(Debug, Clone, PartialEq)]
pub enum DeviceType {
    Flarm,
    Ogn,
    Icao,
    Unknown,
}

impl Serialize for DeviceType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match self {
            DeviceType::Flarm => "F",
            DeviceType::Ogn => "O",
            DeviceType::Icao => "I",
            DeviceType::Unknown => "",
        };
        serializer.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for DeviceType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "F" => Ok(DeviceType::Flarm),
            "O" => Ok(DeviceType::Ogn),
            "I" => Ok(DeviceType::Icao),
            "" => Ok(DeviceType::Unknown),
            _ => Ok(DeviceType::Unknown),
        }
    }
}

impl Default for DeviceType {
    fn default() -> Self {
        DeviceType::Unknown
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub device_type: DeviceType,
    pub device_id: String,
    pub aircraft_model: String,
    pub registration: String,
    #[serde(rename = "cn")]
    pub competition_number: String,
    pub tracked: String,
    pub identified: String,
}

#[derive(Debug, Deserialize)]
struct DeviceResponse {
    devices: Vec<Device>,
}

#[derive(Debug)]
pub struct DeviceDatabase {
    devices: HashMap<String, Device>,
}

impl DeviceDatabase {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
        }
    }

    pub async fn fetch(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let response = reqwest::get(DDB_URL).await?;
        let device_response: DeviceResponse = response.json().await?;

        // Clear existing devices and populate with new data
        self.devices.clear();
        for device in device_response.devices {
            self.devices.insert(device.device_id.clone(), device);
        }

        Ok(())
    }

    pub fn get_device(&self, device_id: &str) -> Option<&Device> {
        self.devices.get(device_id)
    }

    pub fn get_all_devices(&self) -> &HashMap<String, Device> {
        &self.devices
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_device_database_creation() {
        let db = DeviceDatabase::new();
        assert_eq!(db.device_count(), 0);
        assert!(db.get_device("000000").is_none());
    }

    #[test]
    fn test_device_serialization() {
        let device = Device {
            device_type: DeviceType::Flarm,
            device_id: "000000".to_string(),
            aircraft_model: "SZD-41 Jantar Std".to_string(),
            registration: "HA-4403".to_string(),
            competition_number: "J".to_string(),
            tracked: "Y".to_string(),
            identified: "Y".to_string(),
        };

        // Test that the device can be serialized/deserialized
        let json = serde_json::to_string(&device).unwrap();
        let deserialized: Device = serde_json::from_str(&json).unwrap();
        assert_eq!(device, deserialized);
    }

    #[test]
    fn test_device_response_parsing() {
        let json_data = r#"{"devices":[{"device_type":"F","device_id":"000000","aircraft_model":"SZD-41 Jantar Std","registration":"HA-4403","cn":"J","tracked":"Y","identified":"Y"}]}"#;

        let response: DeviceResponse = serde_json::from_str(json_data).unwrap();
        assert_eq!(response.devices.len(), 1);
        assert_eq!(response.devices[0].device_id, "000000");
        assert_eq!(response.devices[0].aircraft_model, "SZD-41 Jantar Std");
        assert_eq!(response.devices[0].device_type, DeviceType::Flarm);
    }

    #[test]
    fn test_device_type_deserialization() {
        // Test Flarm device type
        let flarm_json = r#"{"device_type":"F","device_id":"123456","aircraft_model":"Test","registration":"","cn":"","tracked":"","identified":""}"#;
        let flarm_device: Device = serde_json::from_str(flarm_json).unwrap();
        assert_eq!(flarm_device.device_type, DeviceType::Flarm);

        // Test OGN device type
        let ogn_json = r#"{"device_type":"O","device_id":"123456","aircraft_model":"Test","registration":"","cn":"","tracked":"","identified":""}"#;
        let ogn_device: Device = serde_json::from_str(ogn_json).unwrap();
        assert_eq!(ogn_device.device_type, DeviceType::Ogn);

        // Test ICAO device type
        let icao_json = r#"{"device_type":"I","device_id":"123456","aircraft_model":"Test","registration":"","cn":"","tracked":"","identified":""}"#;
        let icao_device: Device = serde_json::from_str(icao_json).unwrap();
        assert_eq!(icao_device.device_type, DeviceType::Icao);

        // Test empty string (Unknown)
        let unknown_json = r#"{"device_type":"","device_id":"123456","aircraft_model":"Test","registration":"","cn":"","tracked":"","identified":""}"#;
        let unknown_device: Device = serde_json::from_str(unknown_json).unwrap();
        assert_eq!(unknown_device.device_type, DeviceType::Unknown);

        // Test unrecognized value (should default to Unknown)
        let unrecognized_json = r#"{"device_type":"X","device_id":"123456","aircraft_model":"Test","registration":"","cn":"","tracked":"","identified":""}"#;
        let unrecognized_device: Device = serde_json::from_str(unrecognized_json).unwrap();
        assert_eq!(unrecognized_device.device_type, DeviceType::Unknown);
    }
}
