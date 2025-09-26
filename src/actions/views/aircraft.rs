use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::club::AircraftModelView;
use crate::aircraft_registrations::{Aircraft, AirworthinessClass, LightSportType, RegistrantType};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<AircraftModelView>,
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
            model: None, // Will be set when fetching with model data
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
    pub device_type: String,
    pub device_id: u32,
    pub aircraft_model: String,
    pub competition_number: String,
    pub tracked: bool,
    pub identified: bool,
}

impl DeviceView {
    pub fn from_device(device: crate::devices::Device) -> Self {
        Self {
            device_type: device.address_type.to_string(),
            device_id: device.address,
            aircraft_model: device.aircraft_model,
            competition_number: device.competition_number,
            tracked: device.tracked,
            identified: device.identified,
        }
    }
}
