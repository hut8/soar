use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::club::AircraftModelView;
use crate::aircraft_registrations::{
    Aircraft as AircraftDomain, AirworthinessClass, LightSportType, RegistrantType,
};
use crate::ogn_aprs_aircraft::AircraftType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AircraftRegistrationView {
    pub registration_number: String,
    pub serial_number: String,
    pub manufacturer_code: Option<String>,
    pub model_code: Option<String>,
    pub series_code: Option<String>,
    pub engine_manufacturer_code: Option<String>,
    pub engine_model_code: Option<String>,
    pub year_manufactured: Option<u16>,
    pub registrant_type: Option<RegistrantType>,
    pub registrant_name: Option<String>,
    pub aircraft_type: Option<String>,
    pub engine_type: Option<i16>,
    pub status_code: Option<String>,
    pub transponder_code: Option<u32>,
    pub airworthiness_class: Option<AirworthinessClass>,
    pub airworthiness_date: Option<NaiveDate>,
    pub certificate_issue_date: Option<NaiveDate>,
    pub expiration_date: Option<NaiveDate>,
    pub club_id: Option<Uuid>,
    pub home_base_airport_id: Option<Uuid>,
    pub kit_manufacturer_name: Option<String>,
    pub kit_model_name: Option<String>,
    pub other_names: Vec<String>,
    pub light_sport_type: Option<LightSportType>,
    pub device_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<AircraftModelView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aircraft_type_ogn: Option<AircraftType>,
}

impl From<AircraftDomain> for AircraftRegistrationView {
    fn from(aircraft: AircraftDomain) -> Self {
        Self {
            registration_number: aircraft.n_number,
            serial_number: aircraft.serial_number,
            manufacturer_code: aircraft.manufacturer_code,
            model_code: aircraft.model_code,
            series_code: aircraft.series_code,
            engine_manufacturer_code: aircraft.engine_manufacturer_code,
            engine_model_code: aircraft.engine_model_code,
            year_manufactured: aircraft.year_mfr,
            registrant_type: aircraft.registrant_type_code,
            registrant_name: aircraft.registrant_name,
            aircraft_type: aircraft.aircraft_type.map(|at| at.to_string()),
            engine_type: aircraft.type_engine_code,
            status_code: aircraft.status_code,
            transponder_code: aircraft.transponder_code,
            airworthiness_class: aircraft.airworthiness_class,
            airworthiness_date: aircraft.airworthiness_date,
            certificate_issue_date: aircraft.certificate_issue_date,
            expiration_date: aircraft.expiration_date,
            club_id: None, // Will need to be set from the database
            home_base_airport_id: aircraft.home_base_airport_id,
            kit_manufacturer_name: aircraft.kit_mfr_name,
            kit_model_name: aircraft.kit_model_name,
            other_names: aircraft.other_names,
            light_sport_type: aircraft.light_sport_type,
            device_id: aircraft.device_id,
            model: None,             // Will be set when fetching with model data
            aircraft_type_ogn: None, // Will be set when fetching with device data
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AircraftWithDeviceView {
    #[serde(flatten)]
    pub aircraft: AircraftRegistrationView,
    pub device: Option<DeviceView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceView {
    pub id: uuid::Uuid,

    /// Formatted device address with prefix (e.g., "OGN-8B570F", "FLARM-123456", "ICAO-ABCDEF")
    pub device_address: String,

    pub address_type: String,
    pub address: String,
    pub aircraft_model: String,
    pub registration: String,
    pub competition_number: String,
    pub tracked: bool,
    pub identified: bool,
    pub club_id: Option<Uuid>,
    pub created_at: String,
    pub updated_at: String,
    pub from_ddb: bool,
    pub frequency_mhz: Option<f64>,
    pub pilot_name: Option<String>,
    pub home_base_airport_ident: Option<String>,
    pub aircraft_type_ogn: Option<crate::ogn_aprs_aircraft::AircraftType>,
    pub last_fix_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixes: Option<Vec<crate::fixes::Fix>>,
}

impl DeviceView {
    pub fn from_device(device: crate::devices::Device) -> Self {
        // Format address as 6-digit hex
        let address_hex = format!("{:06X}", device.address);

        // Create prefix based on address type
        let prefix = match device.address_type {
            crate::devices::AddressType::Ogn => "OGN",
            crate::devices::AddressType::Flarm => "FLARM",
            crate::devices::AddressType::Icao => "ICAO",
            crate::devices::AddressType::Unknown => "UNKNOWN",
        };

        // Combine prefix and address
        let device_address = format!("{}-{}", prefix, address_hex);

        // Address type as single character for backward compatibility
        let address_type = match device.address_type {
            crate::devices::AddressType::Ogn => "O",
            crate::devices::AddressType::Flarm => "F",
            crate::devices::AddressType::Icao => "I",
            crate::devices::AddressType::Unknown => "",
        }
        .to_string();

        Self {
            id: device
                .id
                .expect("Device must have an ID to create DeviceView"),
            device_address,
            address_type,
            address: address_hex,
            aircraft_model: device.aircraft_model,
            registration: device.registration,
            competition_number: device.competition_number,
            tracked: device.tracked,
            identified: device.identified,
            club_id: device.club_id,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            from_ddb: false,
            frequency_mhz: device.frequency_mhz,
            pilot_name: device.pilot_name,
            home_base_airport_ident: device.home_base_airport_ident,
            aircraft_type_ogn: device.aircraft_type_ogn,
            last_fix_at: device.last_fix_at.map(|dt| dt.to_rfc3339()),
            fixes: None,
        }
    }

    pub fn from_device_model(device_model: crate::devices::DeviceModel) -> Self {
        // Format address as 6-digit hex
        let address_hex = format!("{:06X}", device_model.address as u32);

        // Create prefix based on address type
        let prefix = match device_model.address_type {
            crate::devices::AddressType::Ogn => "OGN",
            crate::devices::AddressType::Flarm => "FLARM",
            crate::devices::AddressType::Icao => "ICAO",
            crate::devices::AddressType::Unknown => "UNKNOWN",
        };

        // Combine prefix and address
        let device_address = format!("{}-{}", prefix, address_hex);

        // Address type as single character for backward compatibility
        let address_type = match device_model.address_type {
            crate::devices::AddressType::Ogn => "O",
            crate::devices::AddressType::Flarm => "F",
            crate::devices::AddressType::Icao => "I",
            crate::devices::AddressType::Unknown => "",
        }
        .to_string();

        Self {
            id: device_model.id,
            device_address,
            address_type,
            address: address_hex,
            aircraft_model: device_model.aircraft_model,
            registration: device_model.registration,
            competition_number: device_model.competition_number,
            tracked: device_model.tracked,
            identified: device_model.identified,
            club_id: device_model.club_id,
            created_at: device_model.created_at.to_rfc3339(),
            updated_at: device_model.updated_at.to_rfc3339(),
            from_ddb: device_model.from_ddb,
            frequency_mhz: device_model
                .frequency_mhz
                .and_then(|bd| bd.to_string().parse().ok()),
            pilot_name: device_model.pilot_name,
            home_base_airport_ident: device_model.home_base_airport_ident,
            aircraft_type_ogn: device_model.aircraft_type_ogn,
            last_fix_at: device_model.last_fix_at.map(|dt| dt.to_rfc3339()),
            fixes: None,
        }
    }
}

impl From<crate::devices::Device> for DeviceView {
    fn from(device: crate::devices::Device) -> Self {
        Self::from_device(device)
    }
}

impl From<crate::devices::DeviceModel> for DeviceView {
    fn from(device_model: crate::devices::DeviceModel) -> Self {
        Self::from_device_model(device_model)
    }
}

/// Complete aircraft information with device, registration, model, and recent fixes
/// This is the enriched view used when returning devices with full aircraft data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aircraft {
    #[serde(flatten)]
    pub device: DeviceView,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aircraft_registration: Option<crate::aircraft_registrations::AircraftRegistrationModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aircraft_model: Option<crate::faa::aircraft_models::AircraftModel>,
}
