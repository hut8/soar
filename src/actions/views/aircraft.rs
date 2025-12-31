use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use super::club::AircraftModelView;
use crate::aircraft_registrations::{
    Aircraft as AircraftDomain, AirworthinessClass, LightSportType, RegistrantType,
};
use crate::ogn_aprs_aircraft::AircraftType;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    pub aircraft_id: Option<Uuid>,
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
            aircraft_id: aircraft.aircraft_id,
            model: None,             // Will be set when fetching with model data
            aircraft_type_ogn: None, // Will be set when fetching with device data
        }
    }
}

impl From<crate::aircraft_registrations::AircraftRegistrationModel> for AircraftRegistrationView {
    fn from(model: crate::aircraft_registrations::AircraftRegistrationModel) -> Self {
        Self {
            registration_number: model.registration_number,
            serial_number: model.serial_number,
            manufacturer_code: Some(model.manufacturer_code),
            model_code: Some(model.model_code),
            series_code: Some(model.series_code),
            engine_manufacturer_code: model.engine_manufacturer_code,
            engine_model_code: model.engine_model_code,
            year_manufactured: model.year_mfr.and_then(|y| u16::try_from(y).ok()),
            registrant_type: model.registrant_type_code,
            registrant_name: model.registrant_name,
            aircraft_type: model.aircraft_type.map(|at| at.to_string()),
            engine_type: model.type_engine_code,
            status_code: model.status_code,
            transponder_code: model.transponder_code.and_then(|t| u32::try_from(t).ok()),
            airworthiness_class: model.airworthiness_class,
            airworthiness_date: model.airworthiness_date,
            certificate_issue_date: model.certificate_issue_date,
            expiration_date: model.expiration_date,
            club_id: model.club_id,
            home_base_airport_id: model
                .home_base_airport_id
                .map(|id| Uuid::from_u128(id as u128)),
            kit_manufacturer_name: model.kit_mfr_name,
            kit_model_name: model.kit_model_name,
            other_names: vec![], // Not available in model, would need separate query
            light_sport_type: model.light_sport_type,
            aircraft_id: model.aircraft_id,
            model: None,
            aircraft_type_ogn: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AircraftWithDeviceView {
    #[serde(flatten)]
    pub aircraft: AircraftRegistrationView,
    pub device: Option<AircraftView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct AircraftView {
    pub id: uuid::Uuid,

    pub address_type: String,
    pub address: String,
    pub aircraft_model: String,
    pub registration: Option<String>,
    pub competition_number: String,
    pub tracked: bool,
    pub identified: bool,
    pub club_id: Option<Uuid>,
    pub created_at: String,
    pub updated_at: String,
    pub from_ogn_ddb: bool,
    pub from_adsbx_ddb: bool,
    pub frequency_mhz: Option<f64>,
    pub pilot_name: Option<String>,
    pub home_base_airport_ident: Option<String>,
    pub aircraft_type_ogn: Option<crate::ogn_aprs_aircraft::AircraftType>,
    pub last_fix_at: Option<String>,
    pub tracker_device_type: Option<String>,
    pub icao_model_code: Option<String>,
    pub country_code: Option<String>,
    /// Latest fix latitude (for quick map linking)
    pub latest_latitude: Option<f64>,
    /// Latest fix longitude (for quick map linking)
    pub latest_longitude: Option<f64>,
    /// Active flight ID if device is currently on an active flight
    pub active_flight_id: Option<Uuid>,
    /// Current fix (latest position data for this aircraft)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_fix: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixes: Option<Vec<crate::fixes::Fix>>,
}

impl AircraftView {
    pub fn from_device(device: crate::aircraft::Aircraft) -> Self {
        // Format address as 6-digit hex
        let address_hex = format!("{:06X}", device.address);

        // Address type as single character for backward compatibility
        let address_type = match device.address_type {
            crate::aircraft::AddressType::Ogn => "O",
            crate::aircraft::AddressType::Flarm => "F",
            crate::aircraft::AddressType::Icao => "I",
            crate::aircraft::AddressType::Unknown => "",
        }
        .to_string();

        Self {
            id: device
                .id
                .expect("Aircraft must have an ID to create AircraftView"),
            address_type,
            address: address_hex,
            aircraft_model: device.aircraft_model,
            registration: device.registration,
            competition_number: device.competition_number,
            tracked: device.tracked,
            identified: device.identified,
            club_id: device.club_id,
            created_at: device
                .created_at
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
            updated_at: device
                .updated_at
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
            from_ogn_ddb: device.from_ogn_ddb.unwrap_or(false),
            from_adsbx_ddb: device.from_adsbx_ddb.unwrap_or(false),
            frequency_mhz: device.frequency_mhz,
            pilot_name: device.pilot_name,
            home_base_airport_ident: device.home_base_airport_ident,
            aircraft_type_ogn: device.aircraft_type_ogn,
            last_fix_at: device.last_fix_at.map(|dt| dt.to_rfc3339()),
            tracker_device_type: device.tracker_device_type,
            icao_model_code: device.icao_model_code,
            country_code: device.country_code,
            latest_latitude: None,
            latest_longitude: None,
            active_flight_id: None,
            current_fix: None,
            fixes: None,
        }
    }

    pub fn from_device_model(device_model: crate::aircraft::AircraftModel) -> Self {
        // Format address as 6-digit hex
        let address_hex = format!("{:06X}", device_model.address as u32);

        // Address type as single character for backward compatibility
        let address_type = match device_model.address_type {
            crate::aircraft::AddressType::Ogn => "O",
            crate::aircraft::AddressType::Flarm => "F",
            crate::aircraft::AddressType::Icao => "I",
            crate::aircraft::AddressType::Unknown => "",
        }
        .to_string();

        Self {
            id: device_model.id,
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
            from_ogn_ddb: device_model.from_ogn_ddb,
            from_adsbx_ddb: device_model.from_adsbx_ddb,
            frequency_mhz: device_model
                .frequency_mhz
                .and_then(|bd| bd.to_string().parse().ok()),
            pilot_name: device_model.pilot_name,
            home_base_airport_ident: device_model.home_base_airport_ident,
            aircraft_type_ogn: device_model.aircraft_type_ogn,
            last_fix_at: device_model.last_fix_at.map(|dt| dt.to_rfc3339()),
            tracker_device_type: device_model.tracker_device_type,
            icao_model_code: device_model.icao_model_code,
            country_code: device_model.country_code,
            latest_latitude: device_model.latitude,
            latest_longitude: device_model.longitude,
            active_flight_id: None,
            current_fix: device_model.current_fix,
            fixes: None,
        }
    }
}

impl From<crate::aircraft::Aircraft> for AircraftView {
    fn from(device: crate::aircraft::Aircraft) -> Self {
        Self::from_device(device)
    }
}

impl From<crate::aircraft::AircraftModel> for AircraftView {
    fn from(device_model: crate::aircraft::AircraftModel) -> Self {
        Self::from_device_model(device_model)
    }
}

/// Complete aircraft information with device and latest fix
/// Registration and model data are fetched separately when needed (e.g., when viewing details)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct Aircraft {
    #[serde(flatten)]
    pub device: AircraftView,
}

/// Bounds of a cluster of aircraft
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ClusterBounds {
    pub north: f64,
    pub south: f64,
    pub east: f64,
    pub west: f64,
}

/// A cluster of aircraft grouped by spatial proximity
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct AircraftCluster {
    pub id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub count: i64,
    pub bounds: ClusterBounds,
}

/// Discriminated union for either an individual aircraft or a cluster
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AircraftOrCluster {
    Aircraft { data: Box<Aircraft> },
    Cluster { data: AircraftCluster },
}

/// Response from aircraft search endpoint
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct AircraftSearchResponse {
    pub items: Vec<AircraftOrCluster>,
    pub total: i64,
    pub clustered: bool,
}
