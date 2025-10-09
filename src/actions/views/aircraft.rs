use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::club::AircraftModelView;
use crate::aircraft_registrations::{Aircraft, AirworthinessClass, LightSportType, RegistrantType};
use crate::ogn_aprs_aircraft::AircraftType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AircraftView {
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

impl From<Aircraft> for AircraftView {
    fn from(aircraft: Aircraft) -> Self {
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
    pub aircraft: AircraftView,
    pub device: Option<DeviceView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,

    /// Formatted device address with prefix (e.g., "OGN-8B570F", "FLARM-123456", "ICAO-ABCDEF")
    pub device_address: String,

    pub address_type: String,
    pub address: String,
    pub aircraft_model: String,
    pub registration: String,
    pub competition_number: String,
    pub tracked: bool,
    pub identified: bool,
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
            id: device.id,
            device_address,
            address_type,
            address: address_hex,
            aircraft_model: device.aircraft_model,
            registration: device.registration,
            competition_number: device.competition_number,
            tracked: device.tracked,
            identified: device.identified,
        }
    }
}

impl From<crate::devices::Device> for DeviceView {
    fn from(device: crate::devices::Device) -> Self {
        Self::from_device(device)
    }
}
