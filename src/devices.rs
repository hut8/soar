use anyhow::Result;
use bigdecimal::BigDecimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{info, warn};

// Diesel imports
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_derive_enum::DbEnum;

// Import AircraftType and AdsbEmitterCategory for the cached fields
use crate::ogn_aprs_aircraft::{AdsbEmitterCategory, AircraftType};

const DDB_URL_GLIDERNET: &str = "http://ddb.glidernet.org/download/?j=1";
const DDB_URL_GLIDERNET_WORKERS: &str =
    "https://ddb-glidernet-download.davis-chappins.workers.dev/ddb.json";
const DDB_URL_FLARMNET: &str = "https://www.flarmnet.org/files/ddb.json";
const DDB_URL_UNIFIED_FLARMNET: &str = "https://turbo87.github.io/united-flarmnet/united.fln";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceSource {
    Glidernet,
    Flarmnet,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, DbEnum, Serialize, Deserialize)]
#[db_enum(existing_type_path = "crate::schema::sql_types::AddressType")]
pub enum AddressType {
    Flarm,
    Ogn,
    Icao,
    #[default]
    Unknown,
}

impl FromStr for AddressType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "F" => Ok(AddressType::Flarm),
            "O" => Ok(AddressType::Ogn),
            "I" => Ok(AddressType::Icao),
            "" => Ok(AddressType::Unknown),
            _ => Ok(AddressType::Unknown),
        }
    }
}

impl std::fmt::Display for AddressType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AddressType::Flarm => "F",
            AddressType::Ogn => "O",
            AddressType::Icao => "I",
            AddressType::Unknown => "",
        };
        write!(f, "{}", s)
    }
}

// Custom deserializer for AddressType to handle single character strings
fn address_type_from_str<'de, D>(deserializer: D) -> Result<AddressType, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    AddressType::from_str(&s).map_err(|_| serde::de::Error::custom("Invalid address type"))
}

// Custom serializer for AddressType to output single character strings
fn address_type_to_str<S>(address_type: &AddressType, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&address_type.to_string())
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
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Device {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(
        alias = "device_type",
        rename(serialize = "address_type"),
        deserialize_with = "address_type_from_str",
        serialize_with = "address_type_to_str"
    )]
    pub address_type: AddressType,
    #[serde(
        alias = "device_id",
        rename(serialize = "address"),
        deserialize_with = "hex_to_u32",
        serialize_with = "u32_to_hex"
    )]
    pub address: u32,
    pub aircraft_model: String,
    pub registration: String,
    #[serde(rename(deserialize = "cn", serialize = "cn"))]
    pub competition_number: String,
    #[serde(deserialize_with = "string_to_bool", serialize_with = "bool_to_string")]
    pub tracked: bool,
    #[serde(deserialize_with = "string_to_bool", serialize_with = "bool_to_string")]
    pub identified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_mhz: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pilot_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home_base_airport_ident: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aircraft_type_ogn: Option<AircraftType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_fix_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub club_id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icao_model_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adsb_emitter_category: Option<AdsbEmitterCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracker_device_type: Option<String>,
}

// Diesel database model for devices table
#[derive(
    Debug,
    Clone,
    Queryable,
    Selectable,
    Insertable,
    AsChangeset,
    QueryableByName,
    Serialize,
    Deserialize,
)]
#[diesel(table_name = crate::schema::devices)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DeviceModel {
    pub address: i32,
    pub address_type: AddressType,
    pub aircraft_model: String,
    pub registration: String,
    pub competition_number: String,
    pub tracked: bool,
    pub identified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub id: uuid::Uuid,
    pub from_ddb: bool,
    pub frequency_mhz: Option<BigDecimal>,
    pub pilot_name: Option<String>,
    pub home_base_airport_ident: Option<String>,
    pub aircraft_type_ogn: Option<AircraftType>,
    pub last_fix_at: Option<DateTime<Utc>>,
    pub club_id: Option<uuid::Uuid>,
    pub icao_model_code: Option<String>,
    pub adsb_emitter_category: Option<AdsbEmitterCategory>,
    pub tracker_device_type: Option<String>,
}

// For inserting new devices (without timestamps which are set by DB)
#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::devices)]
pub struct NewDevice {
    pub address: i32,
    pub address_type: AddressType,
    pub aircraft_model: String,
    pub registration: String,
    pub competition_number: String,
    pub tracked: bool,
    pub identified: bool,
    pub from_ddb: bool,
    pub frequency_mhz: Option<BigDecimal>,
    pub pilot_name: Option<String>,
    pub home_base_airport_ident: Option<String>,
    pub aircraft_type_ogn: Option<AircraftType>,
    pub last_fix_at: Option<DateTime<Utc>>,
    pub club_id: Option<uuid::Uuid>,
    pub icao_model_code: Option<String>,
    pub adsb_emitter_category: Option<AdsbEmitterCategory>,
    pub tracker_device_type: Option<String>,
}

impl From<Device> for NewDevice {
    fn from(device: Device) -> Self {
        Self {
            address: device.address as i32,
            address_type: device.address_type,
            aircraft_model: device.aircraft_model,
            registration: device.registration,
            competition_number: device.competition_number,
            tracked: device.tracked,
            identified: device.identified,
            from_ddb: true,
            frequency_mhz: device
                .frequency_mhz
                .and_then(|f| f.to_string().parse().ok()),
            pilot_name: device.pilot_name,
            home_base_airport_ident: device.home_base_airport_ident,
            aircraft_type_ogn: device.aircraft_type_ogn,
            last_fix_at: device.last_fix_at,
            club_id: device.club_id,
            icao_model_code: device.icao_model_code,
            adsb_emitter_category: device.adsb_emitter_category,
            tracker_device_type: None, // Not provided by DDB
        }
    }
}

impl From<DeviceModel> for Device {
    fn from(model: DeviceModel) -> Self {
        Self {
            id: Some(model.id),
            address_type: model.address_type,
            address: model.address as u32,
            aircraft_model: model.aircraft_model,
            registration: model.registration,
            competition_number: model.competition_number,
            tracked: model.tracked,
            identified: model.identified,
            frequency_mhz: model
                .frequency_mhz
                .and_then(|bd| bd.to_string().parse().ok()),
            pilot_name: model.pilot_name,
            home_base_airport_ident: model.home_base_airport_ident,
            aircraft_type_ogn: model.aircraft_type_ogn,
            last_fix_at: model.last_fix_at,
            club_id: model.club_id,
            icao_model_code: model.icao_model_code,
            adsb_emitter_category: model.adsb_emitter_category,
            tracker_device_type: model.tracker_device_type,
        }
    }
}

#[derive(Debug, Deserialize)]
struct DeviceResponse {
    devices: Vec<Device>,
}

#[derive(Debug, Clone)]
pub struct DeviceWithSource {
    pub device: Device,
    pub source: DeviceSource,
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
        self.matches_digits_only(suffix)
            || self.matches_digits_plus_one_letter(suffix)
            || self.matches_digits_plus_two_letters(suffix)
    }

    /// Pattern: One to five digits alone (e.g., N1, N12345)
    fn matches_digits_only(&self, suffix: &str) -> bool {
        !suffix.is_empty() && suffix.len() <= 5 && suffix.chars().all(|c| c.is_ascii_digit())
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

#[derive(Debug, Clone)]
pub enum DeviceSearchCriteria {
    Registration(String),
    Address {
        address: u32,
        address_type: AddressType,
    },
}

/// Read and decode unified FlarmNet file from disk
pub fn read_flarmnet_file(path: &str) -> Result<Vec<Device>> {
    info!("Reading unified FlarmNet file from: {}", path);

    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read FlarmNet file {}: {}", path, e))?;

    match flarmnet::xcsoar::decode_file(&content) {
        Ok(decoded) => {
            info!(
                "Successfully decoded unified FlarmNet file with {} total records",
                decoded.records.len()
            );

            let reg_parser = flydent::Parser::new();

            // Convert FlarmNet records to Device structs
            let devices: Vec<Device> = decoded
                .records
                .into_iter()
                .filter_map(|result| {
                    match result {
                        Ok(record) => {
                            // Parse FLARM ID as hex address
                            let address = u32::from_str_radix(&record.flarm_id, 16).ok()?;

                            // Normalize registration using flydent parser
                            let registration =
                                match reg_parser.parse(&record.registration, false, false) {
                                    Some(r) => r.canonical_callsign().to_string(),
                                    None => String::new(),
                                };

                            Some(Device {
                                id: None, // Will be set by database
                                address_type: AddressType::Flarm,
                                address,
                                aircraft_model: record.plane_type,
                                registration,
                                competition_number: record.call_sign,
                                tracked: true,
                                identified: true,
                                frequency_mhz: record.frequency.parse().ok(),
                                pilot_name: if record.pilot_name.is_empty() {
                                    None
                                } else {
                                    Some(record.pilot_name)
                                },
                                home_base_airport_ident: if record.airfield.is_empty() {
                                    None
                                } else {
                                    Some(record.airfield)
                                },
                                aircraft_type_ogn: None, // Not provided in FlarmNet
                                last_fix_at: None,
                                club_id: None,
                                icao_model_code: None,
                                adsb_emitter_category: None,
                                tracker_device_type: None,
                            })
                        }
                        Err(e) => {
                            warn!("Skipping record due to decode error: {}", e);
                            None
                        }
                    }
                })
                .collect();

            info!(
                "Successfully converted {} unified FlarmNet records to devices",
                devices.len()
            );
            Ok(devices)
        }
        Err(e) => {
            warn!("Failed to decode unified FlarmNet file: {}", e);
            Err(anyhow::anyhow!("FlarmNet decode error: {}", e))
        }
    }
}

impl DeviceFetcher {
    pub fn new() -> Self {
        Self
    }

    /// Fetch devices from Glidernet DDB
    async fn fetch_glidernet(&self) -> Result<Vec<Device>> {
        info!("Fetching devices from Glidernet DDB...");

        // Try primary URL first
        match reqwest::get(DDB_URL_GLIDERNET).await {
            Ok(response) if response.status().is_success() => {
                match response.json::<DeviceResponse>().await {
                    Ok(device_response) => {
                        info!(
                            "Successfully fetched {} devices from Glidernet (primary)",
                            device_response.devices.len()
                        );
                        return Ok(device_response.devices);
                    }
                    Err(e) => {
                        warn!("Primary Glidernet URL returned invalid JSON: {}", e);
                    }
                }
            }
            Ok(response) => {
                warn!(
                    "Primary Glidernet URL returned HTTP {}: {}",
                    response.status(),
                    response
                        .status()
                        .canonical_reason()
                        .unwrap_or("Unknown error")
                );
            }
            Err(e) => {
                warn!("Failed to connect to primary Glidernet URL: {}", e);
            }
        }

        // If primary failed, try the backup URL
        info!("Trying backup Glidernet URL...");
        match reqwest::get(DDB_URL_GLIDERNET_WORKERS).await {
            Ok(response) if response.status().is_success() => {
                let device_response: DeviceResponse = response.json().await?;
                info!(
                    "Successfully fetched {} devices from Glidernet (backup)",
                    device_response.devices.len()
                );
                Ok(device_response.devices)
            }
            Ok(response) => Err(anyhow::anyhow!(
                "Backup Glidernet URL returned HTTP {}: {}",
                response.status(),
                response
                    .status()
                    .canonical_reason()
                    .unwrap_or("Unknown error")
            )),
            Err(e) => Err(anyhow::anyhow!(
                "Failed to fetch from both primary and backup Glidernet URLs. Last error: {}",
                e
            )),
        }
    }

    /// Fetch devices from Flarmnet DDB
    async fn fetch_flarmnet(&self) -> Result<Vec<Device>> {
        info!("Fetching devices from Flarmnet DDB...");
        let response = reqwest::get(DDB_URL_FLARMNET).await?;
        let device_response: DeviceResponse = response.json().await?;
        info!(
            "Successfully fetched {} devices from Flarmnet",
            device_response.devices.len()
        );
        Ok(device_response.devices)
    }

    /// Fetch and decode unified FlarmNet file from XCSoar format
    /// This replaces the existing Glidernet/Flarmnet sources
    pub async fn fetch_unified_flarmnet(&self) -> Result<Vec<Device>> {
        info!("Fetching unified FlarmNet from XCSoar...");

        let response = reqwest::get(DDB_URL_UNIFIED_FLARMNET).await?;

        if !response.status().is_success() {
            warn!(
                "Unified FlarmNet returned non-success status: {}",
                response.status()
            );
            return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
        }

        let content = response.text().await?;
        info!("Downloaded unified FlarmNet file, attempting to decode...");

        match flarmnet::xcsoar::decode_file(&content) {
            Ok(decoded) => {
                info!(
                    "Successfully decoded unified FlarmNet file with {} total records",
                    decoded.records.len()
                );

                let reg_parser = flydent::Parser::new();

                // Convert FlarmNet records to Device structs
                let devices: Vec<Device> = decoded
                    .records
                    .into_iter()
                    .filter_map(|result| {
                        match result {
                            Ok(record) => {
                                // Parse FLARM ID as hex address
                                let address = u32::from_str_radix(&record.flarm_id, 16).ok()?;

                                // Normalize registration using flydent parser
                                let registration =
                                    match reg_parser.parse(&record.registration, false, false) {
                                        Some(r) => r.canonical_callsign().to_string(),
                                        None => String::new(),
                                    };

                                Some(Device {
                                    id: None, // Will be set by database
                                    address_type: AddressType::Flarm,
                                    address,
                                    aircraft_model: record.plane_type,
                                    registration,
                                    competition_number: record.call_sign,
                                    tracked: true,
                                    identified: true,
                                    frequency_mhz: record.frequency.parse().ok(),
                                    pilot_name: if record.pilot_name.is_empty() {
                                        None
                                    } else {
                                        Some(record.pilot_name)
                                    },
                                    home_base_airport_ident: if record.airfield.is_empty() {
                                        None
                                    } else {
                                        Some(record.airfield)
                                    },
                                    aircraft_type_ogn: None, // Not provided in FlarmNet
                                    last_fix_at: None,
                                    club_id: None,
                                    icao_model_code: None,
                                    adsb_emitter_category: None,
                                    tracker_device_type: None,
                                })
                            }
                            Err(e) => {
                                warn!("Skipping record due to decode error: {}", e);
                                None
                            }
                        }
                    })
                    .collect();

                info!(
                    "Successfully converted {} unified FlarmNet records to devices",
                    devices.len()
                );
                Ok(devices)
            }
            Err(e) => {
                warn!("Failed to decode unified FlarmNet file: {}", e);
                Err(anyhow::anyhow!("FlarmNet decode error: {}", e))
            }
        }
    }

    /// Fetch all devices from both DDB sources and merge them
    /// In case of conflicts (same device_id), Glidernet takes precedence
    pub async fn fetch_all(&self) -> Result<Vec<Device>> {
        // Fetch from both sources in parallel
        let (glidernet_result, flarmnet_result) =
            tokio::join!(self.fetch_glidernet(), self.fetch_flarmnet());

        let glidernet_devices = match glidernet_result {
            Ok(devices) => devices,
            Err(e) => {
                warn!("Failed to fetch from Glidernet: {}", e);
                Vec::new()
            }
        };

        let flarmnet_devices = match flarmnet_result {
            Ok(devices) => devices,
            Err(e) => {
                warn!("Failed to fetch from Flarmnet: {}", e);
                Vec::new()
            }
        };

        // If we couldn't fetch from either source, return an error
        if glidernet_devices.is_empty() && flarmnet_devices.is_empty() {
            return Err(anyhow::anyhow!(
                "Failed to fetch devices from both Glidernet and Flarmnet"
            ));
        }

        let reg_parser = flydent::Parser::new();

        let device_normalizer = |mut d: Device| {
            let reg = reg_parser.parse(&d.registration, false, false);
            match reg {
                Some(r) => {
                    d.registration = r.canonical_callsign().to_string();
                    d
                }
                None => {
                    d.registration = "".into();
                    d
                }
            }
        };

        // Canonicalize registrations using "flydent" crate
        let flarmnet_devices: Vec<Device> = flarmnet_devices
            .into_iter()
            .map(device_normalizer)
            .collect();
        let glidernet_devices: Vec<Device> = glidernet_devices
            .into_iter()
            .map(device_normalizer)
            .collect();

        // Create a map of device_id -> Device for conflict resolution
        let mut device_map: HashMap<u32, DeviceWithSource> = HashMap::new();

        // Add Flarmnet devices first (lower priority)
        for device in flarmnet_devices {
            device_map.insert(
                device.address,
                DeviceWithSource {
                    device,
                    source: DeviceSource::Flarmnet,
                },
            );
        }

        // Add Glidernet devices (higher priority - will overwrite conflicts)
        let mut conflicts = 0;
        for glidernet_device in glidernet_devices {
            if let Some(flarmnet_device_src) = device_map.get(&glidernet_device.address)
                && flarmnet_device_src.source == DeviceSource::Flarmnet
            {
                let flarmnet_device = flarmnet_device_src.device.clone();
                // Only log a warning if the devices actually have different data
                if glidernet_device != flarmnet_device {
                    let (registration, source) = match (
                        glidernet_device.registration.is_empty(),
                        flarmnet_device.registration.is_empty(),
                    ) {
                        (true, true) => ("".to_string(), DeviceSource::Glidernet),
                        (true, false) => {
                            (flarmnet_device.registration.clone(), DeviceSource::Flarmnet)
                        }
                        (false, _) => (
                            glidernet_device.registration.clone(),
                            DeviceSource::Glidernet,
                        ),
                    };

                    conflicts += 1;
                    let better_label = match source {
                        DeviceSource::Glidernet => "OGN",
                        DeviceSource::Flarmnet => "FLARM",
                    };
                    warn!(
                        "Device conflict for ID {}: using {} data: {} (over {})",
                        glidernet_device.address,
                        better_label,
                        registration,
                        if source == DeviceSource::Glidernet {
                            &flarmnet_device.registration
                        } else {
                            &glidernet_device.registration
                        }
                    );
                    let merged_device = Device {
                        id: None, // Merged devices from external sources don't have our database ID
                        address_type: glidernet_device.address_type,
                        address: glidernet_device.address,
                        aircraft_model: if !glidernet_device.aircraft_model.is_empty() {
                            glidernet_device.aircraft_model.clone()
                        } else {
                            flarmnet_device.aircraft_model.clone()
                        },
                        registration,
                        competition_number: if !glidernet_device.competition_number.is_empty() {
                            glidernet_device.competition_number.clone()
                        } else {
                            flarmnet_device.competition_number.clone()
                        },
                        tracked: glidernet_device.tracked || flarmnet_device.tracked,
                        identified: glidernet_device.identified || flarmnet_device.identified,
                        frequency_mhz: None,
                        pilot_name: None,
                        home_base_airport_ident: None,
                        aircraft_type_ogn: None,
                        last_fix_at: None,
                        club_id: None,
                        icao_model_code: None,
                        adsb_emitter_category: None,
                        tracker_device_type: None,
                    };
                    device_map.insert(
                        glidernet_device.address,
                        DeviceWithSource {
                            device: merged_device,
                            source: DeviceSource::Glidernet, // TODO: Really this is both
                        },
                    );
                }
                // If there's no difference, we keep the Flarmnet device
            } else {
                device_map.insert(
                    glidernet_device.address,
                    DeviceWithSource {
                        device: glidernet_device,
                        source: DeviceSource::Glidernet,
                    },
                );
            }
        }

        let total_devices = device_map.len();
        info!(
            "Merged device databases: {} total devices ({} conflicts resolved in favor of Glidernet)",
            total_devices, conflicts
        );

        // Extract just the devices for return
        Ok(device_map.into_values().map(|dws| dws.device).collect())
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
    fn test_device_source_enum() {
        // Test that DeviceSource enum works correctly
        let glidernet = DeviceSource::Glidernet;
        let flarmnet = DeviceSource::Flarmnet;

        assert_eq!(glidernet, DeviceSource::Glidernet);
        assert_eq!(flarmnet, DeviceSource::Flarmnet);
        assert_ne!(glidernet, flarmnet);
    }

    #[test]
    fn test_device_with_source() {
        let device = create_test_device("N123AB");
        let device_with_source = DeviceWithSource {
            device: device.clone(),
            source: DeviceSource::Glidernet,
        };

        assert_eq!(device_with_source.device, device);
        assert_eq!(device_with_source.source, DeviceSource::Glidernet);
    }

    #[test]
    fn test_device_serialization_with_booleans() {
        let device = Device {
            id: None, // Test device without database ID
            address_type: AddressType::Flarm,
            address: 0x000000,
            aircraft_model: "SZD-41 Jantar Std".to_string(),
            registration: "HA-4403".to_string(),
            competition_number: "J".to_string(),
            tracked: true,
            identified: false,
            frequency_mhz: None,
            pilot_name: None,
            home_base_airport_ident: None,
            aircraft_type_ogn: None,
            last_fix_at: None,
            club_id: None,
            icao_model_code: None,
            adsb_emitter_category: None,
            tracker_device_type: None,
        };

        // Test that the device can be serialized/deserialized
        let json = serde_json::to_string(&device).unwrap();
        let deserialized: Device = serde_json::from_str(&json).unwrap();
        assert_eq!(device, deserialized);

        // Also test that it includes the competition_number field (cn)
        assert!(json.contains("\"cn\":\"J\""));
    }

    #[test]
    fn test_device_response_parsing() {
        let json_data = r#"{"devices":[{"device_type":"F","device_id":"000000","aircraft_model":"SZD-41 Jantar Std","registration":"HA-4403","cn":"J","tracked":"Y","identified":""}]}"#;

        let response: DeviceResponse = serde_json::from_str(json_data).unwrap();
        assert_eq!(response.devices.len(), 1);
        assert_eq!(response.devices[0].address, 0x000000);
        assert_eq!(response.devices[0].aircraft_model, "SZD-41 Jantar Std");
        assert_eq!(response.devices[0].address_type, AddressType::Flarm);
        assert!(response.devices[0].tracked);
        assert!(!response.devices[0].identified);
    }

    #[test]
    fn test_address_type_deserialization() {
        // Test Flarm address type
        let flarm_json = r#"{"device_type":"F","device_id":"123456","aircraft_model":"Test","registration":"","cn":"","tracked":"Y","identified":"Y"}"#;
        let flarm_device: Device = serde_json::from_str(flarm_json).unwrap();
        assert_eq!(flarm_device.address_type, AddressType::Flarm);

        // Test OGN address type
        let ogn_json = r#"{"device_type":"O","device_id":"123456","aircraft_model":"Test","registration":"","cn":"","tracked":"Y","identified":"Y"}"#;
        let ogn_device: Device = serde_json::from_str(ogn_json).unwrap();
        assert_eq!(ogn_device.address_type, AddressType::Ogn);

        // Test ICAO address type
        let icao_json = r#"{"device_type":"I","device_id":"123456","aircraft_model":"Test","registration":"","cn":"","tracked":"Y","identified":"Y"}"#;
        let icao_device: Device = serde_json::from_str(icao_json).unwrap();
        assert_eq!(icao_device.address_type, AddressType::Icao);

        // Test empty string (Unknown)
        let unknown_json = r#"{"device_type":"","device_id":"123456","aircraft_model":"Test","registration":"","cn":"","tracked":"Y","identified":"Y"}"#;
        let unknown_device: Device = serde_json::from_str(unknown_json).unwrap();
        assert_eq!(unknown_device.address_type, AddressType::Unknown);
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
                assert!(
                    matches!(country, RegistrationCountry::UnitedStates),
                    "Expected {} to be US registration",
                    registration
                );
            } else {
                assert!(
                    matches!(country, RegistrationCountry::Other),
                    "Expected {} to be Other registration",
                    registration
                );
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
                assert!(
                    matches!(country, RegistrationCountry::UnitedStates),
                    "Expected {} to be US registration",
                    registration
                );
            } else {
                assert!(
                    matches!(country, RegistrationCountry::Other),
                    "Expected {} to be Other registration",
                    registration
                );
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
                assert!(
                    matches!(country, RegistrationCountry::UnitedStates),
                    "Expected {} to be US registration",
                    registration
                );
            } else {
                assert!(
                    matches!(country, RegistrationCountry::Other),
                    "Expected {} to be Other registration",
                    registration
                );
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
                assert!(
                    matches!(country, RegistrationCountry::UnitedStates),
                    "Expected {} to be US registration",
                    registration
                );
            } else {
                assert!(
                    matches!(country, RegistrationCountry::Other),
                    "Expected {} to be Other registration",
                    registration
                );
            }
        }
    }

    #[test]
    fn test_registration_country_non_us_registrations() {
        // Test non-U.S. registrations
        let test_cases = vec![
            "G-ABCD",  // UK
            "D-ABCD",  // Germany
            "F-ABCD",  // France
            "HA-4403", // Hungary (from existing test)
            "VH-ABC",  // Australia
            "C-GABC",  // Canada
            "JA1234",  // Japan
            "HL1234",  // South Korea
            "B-1234",  // China
            "ABC123",  // No country prefix
            "123456",  // Just numbers
            "",        // Empty string
        ];

        for registration in test_cases {
            let device = create_test_device(registration);
            let country = device.registration_country();
            assert!(
                matches!(country, RegistrationCountry::Other),
                "Expected {} to be Other registration",
                registration
            );
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
                assert!(
                    matches!(country, RegistrationCountry::UnitedStates),
                    "Expected {} to be US registration",
                    registration
                );
            } else {
                assert!(
                    matches!(country, RegistrationCountry::Other),
                    "Expected {} to be Other registration",
                    registration
                );
            }
        }
    }

    // Helper function to create a test device with a specific registration
    fn create_test_device(registration: &str) -> Device {
        Device {
            id: None, // Test devices don't have database IDs
            address_type: AddressType::Flarm,
            address: 0x123456,
            aircraft_model: "Test Aircraft".to_string(),
            registration: registration.to_string(),
            competition_number: "T".to_string(),
            tracked: true,
            identified: true,
            frequency_mhz: None,
            pilot_name: None,
            home_base_airport_ident: None,
            aircraft_type_ogn: None,
            last_fix_at: None,
            club_id: None,
            icao_model_code: None,
            adsb_emitter_category: None,
            tracker_device_type: None,
        }
    }
}
