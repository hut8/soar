use serde_derive::Deserialize;
use serde_derive::Serialize;
use std::collections::HashMap;
use reqwest;

#[cfg(test)]
use serde_json;

const DDB_URL: &str = "http://ddb.glidernet.org/download/?j=1";

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub device_type: String,
    pub device_id: String,
    pub aircraft_model: String,
    pub registration: String,
    pub cn: String,
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
            device_type: "F".to_string(),
            device_id: "000000".to_string(),
            aircraft_model: "SZD-41 Jantar Std".to_string(),
            registration: "HA-4403".to_string(),
            cn: "J".to_string(),
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
    }
}
