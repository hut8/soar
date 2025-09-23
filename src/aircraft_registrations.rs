use anyhow::{Context, Result, anyhow};
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use uuid::Uuid;

// Import Point from clubs module
use crate::locations::Point;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, DbEnum)]
#[db_enum(existing_type_path = "crate::schema::sql_types::AirworthinessClass")]
pub enum AirworthinessClass {
    Standard,
    Limited,
    Restricted,
    Experimental,
    Provisional,
    Multiple,
    Primary,
    #[serde(rename = "Special Flight Permit")]
    SpecialFlightPermit,
    #[serde(rename = "Light Sport")]
    LightSport,
}

impl From<&str> for AirworthinessClass {
    fn from(code: &str) -> Self {
        match code {
            "1" => AirworthinessClass::Standard,
            "2" => AirworthinessClass::Limited,
            "3" => AirworthinessClass::Restricted,
            "4" => AirworthinessClass::Experimental,
            "5" => AirworthinessClass::Provisional,
            "6" => AirworthinessClass::Multiple,
            "7" => AirworthinessClass::Primary,
            "8" => AirworthinessClass::SpecialFlightPermit,
            "9" => AirworthinessClass::LightSport,
            _ => AirworthinessClass::Standard, // Default fallback
        }
    }
}

impl From<Option<String>> for AirworthinessClass {
    fn from(code: Option<String>) -> Self {
        match code {
            Some(ref s) => AirworthinessClass::from(s.as_str()),
            None => AirworthinessClass::Standard, // Default fallback
        }
    }
}

impl AirworthinessClass {
    /// Get the legacy string code for this airworthiness class
    pub fn code(&self) -> &'static str {
        match self {
            AirworthinessClass::Standard => "1",
            AirworthinessClass::Limited => "2",
            AirworthinessClass::Restricted => "3",
            AirworthinessClass::Experimental => "4",
            AirworthinessClass::Provisional => "5",
            AirworthinessClass::Multiple => "6",
            AirworthinessClass::Primary => "7",
            AirworthinessClass::SpecialFlightPermit => "8",
            AirworthinessClass::LightSport => "9",
        }
    }
}

impl std::fmt::Display for AirworthinessClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AirworthinessClass::Standard => "standard",
            AirworthinessClass::Limited => "limited",
            AirworthinessClass::Restricted => "restricted",
            AirworthinessClass::Experimental => "experimental",
            AirworthinessClass::Provisional => "provisional",
            AirworthinessClass::Multiple => "multiple",
            AirworthinessClass::Primary => "primary",
            AirworthinessClass::SpecialFlightPermit => "special_flight_permit",
            AirworthinessClass::LightSport => "light_sport",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrantType {
    Individual,
    Partnership,
    Corporation,
    CoOwned,
    Government,
    Llc,
    NonCitizenCorporation,
    NonCitizenCoOwned,
    Unknown,
}

impl From<&str> for RegistrantType {
    fn from(code: &str) -> Self {
        match code {
            "1" => RegistrantType::Individual,
            "2" => RegistrantType::Partnership,
            "3" => RegistrantType::Corporation,
            "4" => RegistrantType::CoOwned,
            "5" => RegistrantType::Government,
            "7" => RegistrantType::Llc,
            "8" => RegistrantType::NonCitizenCorporation,
            "9" => RegistrantType::NonCitizenCoOwned,
            _ => RegistrantType::Unknown,
        }
    }
}

impl From<Option<String>> for RegistrantType {
    fn from(code: Option<String>) -> Self {
        match code {
            Some(ref s) => RegistrantType::from(s.as_str()),
            None => RegistrantType::Unknown,
        }
    }
}

/// Convenience: inclusive 1-based positions from the spec → 0-based Rust slice range
fn fw(s: &str, start_1: usize, end_1: usize) -> &str {
    let start = start_1.saturating_sub(1);
    let end = end_1.min(s.len());
    if start >= end || end > s.len() {
        ""
    } else {
        &s[start..end]
    }
}

fn to_opt_string(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn to_opt_string_no_zero(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() || t == "0" {
        None
    } else {
        Some(t.to_string())
    }
}

fn to_string_trim(s: &str) -> String {
    s.trim().to_string()
}

fn to_opt_date(yyyymmdd: &str) -> Option<NaiveDate> {
    let t = yyyymmdd.trim();
    if t.len() != 8 || !t.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let y = &t[0..4].parse::<i32>().ok()?;
    let m = &t[4..6].parse::<u32>().ok()?;
    let d = &t[6..8].parse::<u32>().ok()?;
    NaiveDate::from_ymd_opt(*y, *m, *d)
}

fn to_opt_u32(s: &str) -> Option<u32> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    t.parse::<u32>().ok()
}

fn format_zip_code(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }

    // If it's 9 digits, insert a dash after the first 5
    if t.len() == 9 && t.chars().all(|c| c.is_ascii_digit()) {
        Some(format!("{}-{}", &t[0..5], &t[5..9]))
    } else {
        // Return as-is for 5-digit zips or other formats
        Some(t.to_string())
    }
}

fn to_opt_u32_nonzero(s: &str) -> Option<u32> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    match t.parse::<u32>().ok() {
        Some(0) => None, // Convert zero to None
        Some(n) => Some(n),
        None => None,
    }
}

fn yn_to_bool(s: &str) -> Option<bool> {
    match s.trim() {
        "Y" | "y" => Some(true),
        "N" | "n" => Some(false),
        "" => None,
        _ => None,
    }
}

/// FAA stores Mode S in octal (257–264) and hex (602–611). We store a single number:
/// - Prefer HEX if present (common 24-bit ICAO code)
/// - Else try OCTAL
fn parse_transponder_number(line: &str) -> Option<u32> {
    let hex_raw = fw(line, 602, 611).trim(); // 10 chars (but often 6 hex significant)
    if !hex_raw.is_empty() {
        // Strip non-hex padding and parse
        let hex = hex_raw.trim();
        // Some files right-align; just remove spaces and parse as hex
        if let Ok(v) = u32::from_str_radix(hex, 16) {
            return Some(v);
        }
    }
    let oct_raw = fw(line, 257, 264).trim();
    if !oct_raw.is_empty() && oct_raw.chars().all(|c| ('0'..='7').contains(&c)) {
        // Convert octal to decimal
        if let Ok(v) = u32::from_str_radix(oct_raw, 8) {
            return Some(v);
        }
    }
    None
}

/// Named Approved Operation flags (first 8 pages only).
/// Keep the raw 9-char string for audit, and set flags via mapping by class/slots.
/// You can tweak the mapping tables below to match your ingestion guide precisely.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApprovedOps {
    // Restricted (examples)
    pub restricted_other: bool,
    pub restricted_ag_pest_control: bool,
    pub restricted_aerial_surveying: bool,
    pub restricted_aerial_advertising: bool,
    pub restricted_forest: bool,
    pub restricted_patrolling: bool,
    pub restricted_weather_control: bool,
    pub restricted_carriage_of_cargo: bool,

    // Experimental (examples)
    pub exp_show_compliance: bool,
    pub exp_research_development: bool,
    pub exp_amateur_built: bool,
    pub exp_exhibition: bool,
    pub exp_racing: bool,
    pub exp_crew_training: bool,
    pub exp_market_survey: bool,
    pub exp_operating_kit_built: bool,
    pub exp_lsa_reg_prior_2008: bool,       // 8A (legacy)
    pub exp_lsa_operating_kit_built: bool,  // 8B
    pub exp_lsa_prev_21_190: bool,          // 8C
    pub exp_uas_research_development: bool, // 9A
    pub exp_uas_market_survey: bool,        // 9B
    pub exp_uas_crew_training: bool,        // 9C
    pub exp_uas_exhibition: bool,           // 9D
    pub exp_uas_compliance_with_cfr: bool,  // 9E

    // Special Flight Permit
    pub sfp_ferry_for_repairs_alterations_storage: bool,
    pub sfp_evacuate_impending_danger: bool,
    pub sfp_excess_of_max_certificated: bool,
    pub sfp_delivery_or_export: bool,
    pub sfp_production_flight_testing: bool,
    pub sfp_customer_demo: bool,
}

/// Minimal, adjustable mapping from the 9-char ops string to flags.
/// This *must* be aligned with your specific FAA slot key for the file vintage you ingest.
/// As a safe default, we only set a few widely used Experimental flags by digit presence.
fn parse_approved_ops(airworthiness_class_code: &str, raw_239_247: &str) -> ApprovedOps {
    let mut ops = ApprovedOps::default();
    let raw = raw_239_247.trim();

    // If Experimental (code '4'), interpret common digits:
    // 1=show compliance, 2=R&D, 3=amateur built, 4=exhibition, 5=racing,
    // 6=crew training, 7=market survey, 8=kit-built (legacy), 9=UAS bucket (sub-letters A..E)
    if airworthiness_class_code == "4" {
        for ch in raw.chars() {
            match ch {
                '1' => ops.exp_show_compliance = true,
                '2' => ops.exp_research_development = true,
                '3' => ops.exp_amateur_built = true,
                '4' => ops.exp_exhibition = true,
                '5' => ops.exp_racing = true,
                '6' => ops.exp_crew_training = true,
                '7' => ops.exp_market_survey = true,
                '8' => ops.exp_operating_kit_built = true, // generic legacy bucket
                '9' => {
                    // UAS sub-bucket; the actual sub-letter may be in the same field in some vintages.
                    // If your source emits '9A'...'9E' explicitly, extend parsing here.
                }
                // Occasionally letters present for subcategories (8A/8B/8C, 9A..9E)
                'A' => {
                    ops.exp_lsa_reg_prior_2008 = true;
                    ops.exp_uas_research_development = true;
                }
                'B' => {
                    ops.exp_lsa_operating_kit_built = true;
                    ops.exp_uas_market_survey = true;
                }
                'C' => {
                    ops.exp_lsa_prev_21_190 = true;
                    ops.exp_uas_crew_training = true;
                }
                'D' => ops.exp_uas_exhibition = true,
                'E' => ops.exp_uas_compliance_with_cfr = true,
                _ => {}
            }
        }
    }

    // If Restricted (code '3'), you may map by position index (239..247).
    // Leave default false unless you have an authoritative slot meaning table.
    // If Special Flight Permit (code '8'), similarly map as needed.
    ops
}

// Diesel database models for aircraft_registrations table
#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::aircraft_registrations)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AircraftRegistrationModel {
    pub registration_number: String,
    pub serial_number: String,
    pub manufacturer_code: Option<String>,
    pub model_code: Option<String>,
    pub series_code: Option<String>,
    pub engine_manufacturer_code: Option<String>,
    pub engine_model_code: Option<String>,
    pub year_mfr: Option<i32>,
    pub type_registration_code: Option<String>,
    pub registrant_name: Option<String>,
    pub location_id: Option<Uuid>,
    pub last_action_date: Option<NaiveDate>,
    pub certificate_issue_date: Option<NaiveDate>,
    pub airworthiness_class: Option<AirworthinessClass>,
    pub approved_operations_raw: Option<String>,
    pub op_restricted_other: bool,
    pub op_restricted_ag_pest_control: bool,
    pub op_restricted_aerial_surveying: bool,
    pub op_restricted_aerial_advertising: bool,
    pub op_restricted_forest: bool,
    pub op_restricted_patrolling: bool,
    pub op_restricted_weather_control: bool,
    pub op_restricted_carriage_of_cargo: bool,
    pub op_experimental_show_compliance: bool,
    pub op_experimental_research_development: bool,
    pub op_experimental_amateur_built: bool,
    pub op_experimental_exhibition: bool,
    pub op_experimental_racing: bool,
    pub op_experimental_crew_training: bool,
    pub op_experimental_market_survey: bool,
    pub op_experimental_operating_kit_built: bool,
    pub op_experimental_light_sport_reg_prior_2008: bool,
    pub op_experimental_light_sport_operating_kit_built: bool,
    pub op_experimental_light_sport_prev_21_190: bool,
    pub op_experimental_uas_research_development: bool,
    pub op_experimental_uas_market_survey: bool,
    pub op_experimental_uas_crew_training: bool,
    pub op_experimental_uas_exhibition: bool,
    pub op_experimental_uas_compliance_with_cfr: bool,
    pub op_sfp_ferry_for_repairs_alterations_storage: bool,
    pub op_sfp_evacuate_impending_danger: bool,
    pub op_sfp_excess_of_max_certificated: bool,
    pub op_sfp_delivery_or_export: bool,
    pub op_sfp_production_flight_testing: bool,
    pub op_sfp_customer_demo: bool,
    pub type_aircraft_code: Option<String>,
    pub type_engine_code: Option<i16>,
    pub status_code: Option<String>,
    pub transponder_code: Option<i64>,
    pub fractional_owner: Option<bool>,
    pub airworthiness_date: Option<NaiveDate>,
    pub expiration_date: Option<NaiveDate>,
    pub unique_id: Option<String>,
    pub kit_mfr_name: Option<String>,
    pub kit_model_name: Option<String>,
    pub club_id: Option<Uuid>,
    pub device_id: Option<Uuid>,
}

// Insertable model for new aircraft registrations (without generated fields)
#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::aircraft_registrations, check_for_backend(diesel::pg::Pg))]
pub struct NewAircraftRegistration {
    pub registration_number: String,
    pub serial_number: String,
    pub manufacturer_code: Option<String>,
    pub model_code: Option<String>,
    pub series_code: Option<String>,
    pub engine_manufacturer_code: Option<String>,
    pub engine_model_code: Option<String>,
    pub year_mfr: Option<i32>,
    pub type_registration_code: Option<String>,
    pub registrant_name: Option<String>,
    pub location_id: Option<Uuid>,
    pub last_action_date: Option<NaiveDate>,
    pub certificate_issue_date: Option<NaiveDate>,
    pub airworthiness_class: Option<AirworthinessClass>,
    pub approved_operations_raw: Option<String>,
    pub op_restricted_other: bool,
    pub op_restricted_ag_pest_control: bool,
    pub op_restricted_aerial_surveying: bool,
    pub op_restricted_aerial_advertising: bool,
    pub op_restricted_forest: bool,
    pub op_restricted_patrolling: bool,
    pub op_restricted_weather_control: bool,
    pub op_restricted_carriage_of_cargo: bool,
    pub op_experimental_show_compliance: bool,
    pub op_experimental_research_development: bool,
    pub op_experimental_amateur_built: bool,
    pub op_experimental_exhibition: bool,
    pub op_experimental_racing: bool,
    pub op_experimental_crew_training: bool,
    pub op_experimental_market_survey: bool,
    pub op_experimental_operating_kit_built: bool,
    pub op_experimental_light_sport_reg_prior_2008: bool,
    pub op_experimental_light_sport_operating_kit_built: bool,
    pub op_experimental_light_sport_prev_21_190: bool,
    pub op_experimental_uas_research_development: bool,
    pub op_experimental_uas_market_survey: bool,
    pub op_experimental_uas_crew_training: bool,
    pub op_experimental_uas_exhibition: bool,
    pub op_experimental_uas_compliance_with_cfr: bool,
    pub op_sfp_ferry_for_repairs_alterations_storage: bool,
    pub op_sfp_evacuate_impending_danger: bool,
    pub op_sfp_excess_of_max_certificated: bool,
    pub op_sfp_delivery_or_export: bool,
    pub op_sfp_production_flight_testing: bool,
    pub op_sfp_customer_demo: bool,
    pub type_aircraft_code: Option<String>,
    pub type_engine_code: Option<i16>,
    pub status_code: Option<String>,
    pub transponder_code: Option<i64>,
    pub fractional_owner: Option<bool>,
    pub airworthiness_date: Option<NaiveDate>,
    pub expiration_date: Option<NaiveDate>,
    pub unique_id: Option<String>,
    pub kit_mfr_name: Option<String>,
    pub kit_model_name: Option<String>,
    pub club_id: Option<Uuid>,
    pub device_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aircraft {
    pub n_number: String,                         // 1–5
    pub serial_number: String,                    // 7–36
    pub manufacturer_code: Option<String>,        // 38–40
    pub model_code: Option<String>,               // 41–42
    pub series_code: Option<String>,              // 43–44
    pub engine_manufacturer_code: Option<String>, // 46–48
    pub engine_model_code: Option<String>,        // 49–50
    pub year_mfr: Option<u16>,                    // 52–55

    // Registrant / address
    pub type_registration_code: Option<String>, // 57
    pub registrant_name: Option<String>,        // 59–108
    #[serde(skip_serializing)]
    pub street1: Option<String>, // 110–142 (legacy, kept for parsing)
    #[serde(skip_serializing)]
    pub street2: Option<String>, // 144–176 (legacy, kept for parsing)
    #[serde(skip_serializing)]
    pub city: Option<String>, // 178–195 (legacy, kept for parsing)
    #[serde(skip_serializing)]
    pub state: Option<String>, // 197–198 (legacy, kept for parsing)
    #[serde(skip_serializing)]
    pub zip_code: Option<String>, // 200–209 (legacy, kept for parsing)
    #[serde(skip_serializing)]
    pub region_code: Option<String>, // 211 (legacy, kept for parsing)
    #[serde(skip_serializing)]
    pub county_mail_code: Option<String>, // 213–215 (legacy, kept for parsing)
    #[serde(skip_serializing)]
    pub country_mail_code: Option<String>, // 217–218 (legacy, kept for parsing)

    // Location normalization
    pub location_id: Option<Uuid>, // Foreign key to locations table

    // Dates
    pub last_action_date: Option<NaiveDate>,       // 220–227
    pub certificate_issue_date: Option<NaiveDate>, // 229–236

    // Airworthiness & ops
    pub airworthiness_class: Option<AirworthinessClass>, // 238
    pub approved_operations_raw: Option<String>,         // 239–247
    pub approved_ops: ApprovedOps,                       // mapped flags (best effort)

    pub type_aircraft_code: Option<String>, // 249
    pub type_engine_code: Option<i16>,      // 251–252
    pub status_code: Option<String>,        // 254–255

    // Mode S transponder as a single number
    pub transponder_code: Option<u32>, // from 602–611 (hex) or 257–264 (octal)

    pub fractional_owner: Option<bool>,        // 266
    pub airworthiness_date: Option<NaiveDate>, // 268–275

    // Other Names (up to 5)
    pub other_names: Vec<String>, // 277–326, 328–377, 379–428, 430–479, 481–530

    // Registration expiration
    pub expiration_date: Option<NaiveDate>, // 532–539

    // FAA unique ID
    pub unique_id: Option<String>, // 541–548

    // Amateur/kit
    pub kit_mfr_name: Option<String>,   // 550–579
    pub kit_model_name: Option<String>, // 581–600

    // New fields for location and airport relationships
    pub home_base_airport_id: Option<Uuid>, // Foreign key to airports table
    #[serde(skip_serializing)]
    pub registered_location: Option<Point>, // WGS84 point of registration address (legacy, now in locations table)

    // Device relationship
    pub device_id: Option<Uuid>, // Foreign key to devices table
}

impl Aircraft {
    /// Returns the registrant type based on the type_registration_code
    pub fn registrant_type(&self) -> RegistrantType {
        RegistrantType::from(self.type_registration_code.clone())
    }

    /// Get a complete address string for geocoding
    pub fn address_string(&self) -> Option<String> {
        let mut parts = Vec::new();

        if let Some(street1) = &self.street1
            && !street1.trim().is_empty()
        {
            parts.push(street1.trim().to_string());
        }

        if let Some(street2) = &self.street2
            && !street2.trim().is_empty()
        {
            parts.push(street2.trim().to_string());
        }

        if let Some(city) = &self.city
            && !city.trim().is_empty()
        {
            parts.push(city.trim().to_string());
        }

        if let Some(state) = &self.state
            && !state.trim().is_empty()
        {
            parts.push(state.trim().to_string());
        }

        if let Some(zip) = &self.zip_code
            && !zip.trim().is_empty()
        {
            parts.push(zip.trim().to_string());
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(", "))
        }
    }

    /// Returns a normalized club name if the aircraft appears to be registered to a gliding club
    pub fn club_name(&self) -> Option<String> {
        // Check if registrant type is one of the eligible types
        let registrant_type = self.registrant_type();
        match registrant_type {
            RegistrantType::Corporation
            | RegistrantType::CoOwned
            | RegistrantType::Llc
            | RegistrantType::Partnership
            | RegistrantType::Unknown => {}
            _ => return None,
        }

        // Get the registrant name
        let name = self.registrant_name.as_ref()?;
        let name_upper = name.to_uppercase();

        // Check if it looks like a gliding club (contains "SOAR" or "CLUB")
        if !name_upper.contains("SOAR") && !name_upper.contains("CLUB") {
            return None;
        }

        // Normalize the name
        let mut normalized = name_upper.clone();

        // Replace ASSOCIATES and ASSOC with ASSOCIATION (but not if already ASSOCIATION)
        normalized = normalized.replace("ASSOCIATES", "ASSOCIATION");
        // Only replace ASSOC if it's not already part of ASSOCIATION
        if !normalized.contains("ASSOCIATION") {
            normalized = normalized.replace("ASSOC", "ASSOCIATION");
        } else {
            // Replace standalone ASSOC that's not part of ASSOCIATION
            normalized = normalized.replace(" ASSOC ", " ASSOCIATION ");
            if normalized.ends_with(" ASSOC") {
                normalized = normalized.replace(" ASSOC", " ASSOCIATION");
            }
            if normalized.starts_with("ASSOC ") {
                normalized = normalized.replace("ASSOC ", "ASSOCIATION ");
            }
        }

        // Remove common business suffixes
        let suffixes_to_remove = [
            " LLC",
            " CO",
            " INC",
            " CORP",
            " CORPORATION",
            " LTD",
            " LIMITED",
            " LP",
            " LLP",
            " PLLC",
            " PC",
            " PA",
            " COMPANY",
            " INCORPORATED",
            " PARTNERSHIP",
        ];

        for suffix in &suffixes_to_remove {
            if normalized.ends_with(suffix) {
                normalized = normalized[..normalized.len() - suffix.len()].to_string();
                break; // Only remove one suffix
            }
        }

        // Trim any trailing whitespace
        normalized = normalized.trim().to_string();

        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    }

    pub fn from_fixed_width_line(line: &str) -> Result<Self> {
        // Expect at least the last position we touch. Many files are 611/612 chars.
        if line.len() < 611 {
            return Err(anyhow!(
                "Line too short: expected ~611 chars, got {}",
                line.len()
            ));
        }

        let n_number_raw = to_string_trim(fw(line, 1, 5));
        if n_number_raw.is_empty() {
            return Err(anyhow!("Missing N-number at positions 1–5"));
        }

        let n_number = if n_number_raw.starts_with('N') {
            n_number_raw
        } else {
            format!("N{}", n_number_raw)
        };

        let serial_number = to_string_trim(fw(line, 7, 36));

        let manufacturer_code = to_opt_string(fw(line, 38, 40));
        let model_code = to_opt_string(fw(line, 41, 42));
        let series_code = to_opt_string(fw(line, 43, 44));
        let engine_manufacturer_code = to_opt_string(fw(line, 46, 48));
        let engine_model_code = to_opt_string(fw(line, 49, 50));
        let year_mfr = to_opt_u32_nonzero(fw(line, 52, 55)).map(|v| v as u16);

        let type_registration_code = to_opt_string(fw(line, 57, 57));
        let registrant_name = to_opt_string(fw(line, 59, 108));
        let street1 = to_opt_string(fw(line, 110, 142));
        let street2 = to_opt_string(fw(line, 144, 176));
        let city = to_opt_string(fw(line, 178, 195));
        let state = to_opt_string(fw(line, 197, 198));
        let zip_code = format_zip_code(fw(line, 200, 209));
        let region_code = to_opt_string(fw(line, 211, 211));
        let county_mail_code = to_opt_string(fw(line, 213, 215));
        let country_mail_code = to_opt_string(fw(line, 217, 218));

        let last_action_date = to_opt_date(fw(line, 220, 227));
        let certificate_issue_date = to_opt_date(fw(line, 229, 236));

        let airworthiness_class_code = to_opt_string_no_zero(fw(line, 238, 238));
        let airworthiness_class = airworthiness_class_code
            .as_ref()
            .map(|code| AirworthinessClass::from(code.as_str()));
        let approved_operations_raw = to_opt_string_no_zero(fw(line, 239, 247));

        let approved_ops = if let (Some(class_code), Some(raw)) =
            (&airworthiness_class_code, &approved_operations_raw)
        {
            parse_approved_ops(class_code.as_str(), raw.as_str())
        } else {
            ApprovedOps::default()
        };

        let type_aircraft_code = to_opt_string(fw(line, 249, 249));
        let type_engine_code = to_opt_string(fw(line, 251, 252)).and_then(|s| s.parse().ok());
        let status_code = to_opt_string(fw(line, 254, 255));

        let transponder_code = parse_transponder_number(line);

        let fractional_owner = yn_to_bool(fw(line, 266, 266));
        let airworthiness_date = to_opt_date(fw(line, 268, 275));

        let other1 = to_opt_string(fw(line, 277, 326));
        let other2 = to_opt_string(fw(line, 328, 377));
        let other3 = to_opt_string(fw(line, 379, 428));
        let other4 = to_opt_string(fw(line, 430, 479));
        let other5 = to_opt_string(fw(line, 481, 530));
        let other_names = [other1, other2, other3, other4, other5]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        let expiration_date = to_opt_date(fw(line, 532, 539));
        let unique_id = to_opt_string(fw(line, 541, 548));
        let kit_mfr_name = to_opt_string(fw(line, 550, 579));
        let kit_model_name = to_opt_string(fw(line, 581, 600));

        Ok(Aircraft {
            n_number,
            serial_number,
            manufacturer_code,
            model_code,
            series_code,
            engine_manufacturer_code,
            engine_model_code,
            year_mfr,

            type_registration_code,
            registrant_name,
            street1,
            street2,
            city,
            state,
            zip_code,
            region_code,
            county_mail_code,
            country_mail_code,

            last_action_date,
            certificate_issue_date,

            airworthiness_class,
            approved_operations_raw,
            approved_ops,

            type_aircraft_code,
            type_engine_code,
            status_code,

            transponder_code,

            fractional_owner,
            airworthiness_date,

            other_names,

            expiration_date,

            unique_id,

            kit_mfr_name,
            kit_model_name,

            // Initialize new fields as None/empty
            home_base_airport_id: None,
            location_id: None,
            registered_location: None,
            device_id: None,
        })
    }
}

/// Read a fixed-width FAA Aircraft Master file (first 8 pages spec) and parse all rows.
/// Skips blank lines. Returns an error on the first malformed (too-short) line.
pub fn read_aircraft_file<P: AsRef<Path>>(path: P) -> Result<Vec<Aircraft>> {
    let f = File::open(path.as_ref()).with_context(|| format!("Opening {:?}", path.as_ref()))?;
    let reader = BufReader::new(f);
    let mut out = Vec::new();

    for (lineno, line) in reader.lines().enumerate().skip(1) {
        let line = line.with_context(|| format!("Reading line {}", lineno + 1))?;
        let trimmed = line.trim_end_matches(&['\r', '\n'][..]);

        if trimmed.trim().is_empty() {
            continue;
        }
        let rec = Aircraft::from_fixed_width_line(trimmed)
            .with_context(|| format!("Parsing line {}", lineno + 1))?;
        out.push(rec);
    }

    Ok(out)
}

/// Parse a CSV date in YYYYMMDD format
fn parse_csv_date(date_str: &str) -> Option<NaiveDate> {
    let trimmed = date_str.trim();
    if trimmed.is_empty() {
        return None;
    }
    to_opt_date(trimmed)
}

/// Parse Mode S code from hex string (CSV format)
fn parse_csv_mode_s(hex_str: &str) -> Option<u32> {
    let trimmed = hex_str.trim();
    if trimmed.is_empty() {
        return None;
    }
    u32::from_str_radix(trimmed, 16).ok()
}

impl Aircraft {
    /// Parse an Aircraft from a CSV line with the FAA registration format
    /// Expected CSV columns (0-based indices):
    /// 0: N-NUMBER, 1: SERIAL NUMBER, 2: MFR MDL CODE, 3: ENG MFR MDL, 4: YEAR MFR,
    /// 5: TYPE REGISTRANT, 6: NAME, 7: STREET, 8: STREET2, 9: CITY, 10: STATE, 11: ZIP CODE,
    /// 12: REGION, 13: COUNTY, 14: COUNTRY, 15: LAST ACTION DATE, 16: CERT ISSUE DATE,
    /// 17: CERTIFICATION, 18: TYPE AIRCRAFT, 19: TYPE ENGINE, 20: STATUS CODE,
    /// 21: MODE S CODE, 22: FRACT OWNER, 23: AIR WORTH DATE, 24-28: OTHER NAMES(1-5),
    /// 29: EXPIRATION DATE, 30: UNIQUE ID, 31: KIT MFR, 32: KIT MODEL, 33: MODE S CODE HEX
    pub fn from_faa_csv_line(line: &str) -> Result<Self> {
        let fields: Vec<&str> = line.split(',').collect();

        if fields.len() < 34 {
            return Err(anyhow!(
                "CSV line has insufficient fields: expected at least 34, got {}",
                fields.len()
            ));
        }

        let n_number = to_string_trim(fields[0]);
        if n_number.is_empty() {
            return Err(anyhow!("Missing N-number in CSV"));
        }
        let n_number = if !n_number.starts_with("N") {
            format!("N{}", n_number)
        } else {
            n_number
        };

        let serial_number = to_string_trim(fields[1]);

        // Split the old mfr_mdl_code field into three parts
        let mfr_mdl_code = to_opt_string(fields[2]);
        let (manufacturer_code, model_code, series_code) = if let Some(ref code) = mfr_mdl_code {
            let code_chars: Vec<char> = code.chars().collect();
            let manufacturer_code = if code_chars.len() >= 3 {
                Some(code_chars.iter().take(3).collect::<String>())
            } else {
                None
            };
            let model_code = if code_chars.len() >= 5 {
                Some(code_chars.iter().skip(3).take(2).collect::<String>())
            } else {
                None
            };
            let series_code = if code_chars.len() >= 7 {
                Some(code_chars.iter().skip(5).take(2).collect::<String>())
            } else {
                None
            };
            (manufacturer_code, model_code, series_code)
        } else {
            (None, None, None)
        };

        let (engine_manufacturer_code, engine_model_code) =
            if let Some(engine_code) = to_opt_string(fields[3]) {
                let code_chars: Vec<char> = engine_code.chars().collect();
                let engine_manufacturer_code = if code_chars.len() >= 3 {
                    Some(code_chars.iter().take(3).collect::<String>())
                } else {
                    None
                };
                let engine_model_code = if code_chars.len() >= 5 {
                    Some(code_chars.iter().skip(3).take(2).collect::<String>())
                } else {
                    None
                };
                (engine_manufacturer_code, engine_model_code)
            } else {
                (None, None)
            };
        let year_mfr = to_opt_u32_nonzero(fields[4]).map(|v| v as u16);

        let type_registration_code = to_opt_string(fields[5]);
        let registrant_name = to_opt_string(fields[6]);
        let street1 = to_opt_string(fields[7]);
        let street2 = to_opt_string(fields[8]);
        let city = to_opt_string(fields[9]);
        let state = to_opt_string(fields[10]);
        let zip_code = format_zip_code(fields[11]);
        let region_code = to_opt_string(fields[12]);
        let county_mail_code = to_opt_string(fields[13]);
        let country_mail_code = to_opt_string(fields[14]);

        let last_action_date = parse_csv_date(fields[15]);
        let certificate_issue_date = parse_csv_date(fields[16]);

        let airworthiness_class_code = to_opt_string(fields[17]);
        let airworthiness_class = airworthiness_class_code
            .as_ref()
            .map(|code| AirworthinessClass::from(code.as_str()));
        let type_aircraft_code = to_opt_string(fields[18]);
        let type_engine_code = to_opt_string(fields[19]).and_then(|s| s.parse().ok());
        let status_code = to_opt_string(fields[20]);

        // Try MODE S CODE HEX first (field 33), then MODE S CODE (field 21)
        let transponder_code = parse_csv_mode_s(fields[33]).or_else(|| to_opt_u32(fields[21]));

        let fractional_owner = yn_to_bool(fields[22]);
        let airworthiness_date = parse_csv_date(fields[23]);

        // Other names (fields 24-28)
        let other_names = (24..=28)
            .filter_map(|i| fields.get(i).and_then(|s| to_opt_string(s)))
            .collect::<Vec<_>>();

        let expiration_date = parse_csv_date(fields[29]);
        let unique_id = to_opt_string(fields[30]);
        let kit_mfr_name = to_opt_string(fields[31]);
        let kit_model_name = to_opt_string(fields[32]);

        // For CSV, we don't have the detailed approved operations parsing
        let approved_operations_raw = None;
        let approved_ops = ApprovedOps::default();

        Ok(Aircraft {
            n_number,
            serial_number,
            manufacturer_code,
            model_code,
            series_code,
            engine_manufacturer_code,
            engine_model_code,
            year_mfr,

            type_registration_code,
            registrant_name,
            street1,
            street2,
            city,
            state,
            zip_code,
            region_code,
            county_mail_code,
            country_mail_code,

            last_action_date,
            certificate_issue_date,

            airworthiness_class,
            approved_operations_raw,
            approved_ops,

            type_aircraft_code,
            type_engine_code,
            status_code,

            transponder_code,

            fractional_owner,
            airworthiness_date,

            other_names,

            expiration_date,

            unique_id,

            kit_mfr_name,
            kit_model_name,

            // Initialize new fields as None/empty
            home_base_airport_id: None,
            location_id: None,
            registered_location: None,
            device_id: None,
        })
    }
}

/// Read a CSV FAA Aircraft registration file and parse all rows.
/// Automatically skips the first line (header) and any blank lines.
/// Returns an error on the first malformed line.
pub fn read_aircraft_csv_file<P: AsRef<Path>>(path: P) -> Result<Vec<Aircraft>> {
    let f = File::open(path.as_ref()).with_context(|| format!("Opening {:?}", path.as_ref()))?;
    let reader = BufReader::new(f);
    let mut out = Vec::new();
    let mut is_first_line = true;

    for (lineno, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("Reading line {}", lineno + 1))?;
        let trimmed = line.trim_end_matches(&['\r', '\n'][..]);

        // Skip header line (first line)
        if is_first_line {
            is_first_line = false;
            continue;
        }

        // Skip blank lines
        if trimmed.trim().is_empty() {
            continue;
        }

        let rec = Aircraft::from_faa_csv_line(trimmed)
            .with_context(|| format!("Parsing CSV line {}", lineno + 1))?;
        out.push(rec);
    }

    Ok(out)
}

/// Updated read_aircraft_file that also skips the first line for any CSV files
/// Detects format based on file extension or content
pub fn read_aircraft_file_with_header_skip<P: AsRef<Path>>(path: P) -> Result<Vec<Aircraft>> {
    let path_ref = path.as_ref();
    let extension = path_ref
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    if extension.eq_ignore_ascii_case("csv") {
        return read_aircraft_csv_file(path);
    }

    // For fixed-width files, also skip first line if it looks like a header
    let f = File::open(path_ref).with_context(|| format!("Opening {path_ref:?}"))?;
    let reader = BufReader::new(f);
    let mut out = Vec::new();
    let mut is_first_line = true;

    for (lineno, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("Reading line {}", lineno + 1))?;
        let trimmed = line.trim_end_matches(&['\r', '\n'][..]);

        if trimmed.trim().is_empty() {
            continue;
        }

        // Skip first line if it looks like a header (contains common header keywords)
        if is_first_line {
            is_first_line = false;
            let line_upper = trimmed.to_uppercase();
            if line_upper.contains("N-NUMBER")
                || line_upper.contains("SERIAL")
                || line_upper.contains("REGISTRANT")
            {
                continue;
            }
        }

        let rec = Aircraft::from_fixed_width_line(trimmed)
            .with_context(|| format!("Parsing line {}", lineno + 1))?;
        out.push(rec);
    }

    Ok(out)
}

// Conversion traits between API model and Diesel models
impl From<Aircraft> for NewAircraftRegistration {
    fn from(aircraft: Aircraft) -> Self {
        // Create ApprovedOps struct from the aircraft's approved_ops
        let ops = &aircraft.approved_ops;

        NewAircraftRegistration {
            registration_number: aircraft.n_number,
            serial_number: aircraft.serial_number,
            manufacturer_code: aircraft.manufacturer_code,
            model_code: aircraft.model_code,
            series_code: aircraft.series_code,
            engine_manufacturer_code: aircraft.engine_manufacturer_code,
            engine_model_code: aircraft.engine_model_code,
            year_mfr: aircraft.year_mfr.map(|y| y as i32),
            type_registration_code: aircraft.type_registration_code,
            registrant_name: aircraft.registrant_name,
            location_id: aircraft.location_id,
            last_action_date: aircraft.last_action_date,
            certificate_issue_date: aircraft.certificate_issue_date,
            airworthiness_class: aircraft.airworthiness_class,
            approved_operations_raw: aircraft.approved_operations_raw,
            op_restricted_other: ops.restricted_other,
            op_restricted_ag_pest_control: ops.restricted_ag_pest_control,
            op_restricted_aerial_surveying: ops.restricted_aerial_surveying,
            op_restricted_aerial_advertising: ops.restricted_aerial_advertising,
            op_restricted_forest: ops.restricted_forest,
            op_restricted_patrolling: ops.restricted_patrolling,
            op_restricted_weather_control: ops.restricted_weather_control,
            op_restricted_carriage_of_cargo: ops.restricted_carriage_of_cargo,
            op_experimental_show_compliance: ops.exp_show_compliance,
            op_experimental_research_development: ops.exp_research_development,
            op_experimental_amateur_built: ops.exp_amateur_built,
            op_experimental_exhibition: ops.exp_exhibition,
            op_experimental_racing: ops.exp_racing,
            op_experimental_crew_training: ops.exp_crew_training,
            op_experimental_market_survey: ops.exp_market_survey,
            op_experimental_operating_kit_built: ops.exp_operating_kit_built,
            op_experimental_light_sport_reg_prior_2008: ops.exp_lsa_reg_prior_2008,
            op_experimental_light_sport_operating_kit_built: ops.exp_lsa_operating_kit_built,
            op_experimental_light_sport_prev_21_190: ops.exp_lsa_prev_21_190,
            op_experimental_uas_research_development: ops.exp_uas_research_development,
            op_experimental_uas_market_survey: ops.exp_uas_market_survey,
            op_experimental_uas_crew_training: ops.exp_uas_crew_training,
            op_experimental_uas_exhibition: ops.exp_uas_exhibition,
            op_experimental_uas_compliance_with_cfr: ops.exp_uas_compliance_with_cfr,
            op_sfp_ferry_for_repairs_alterations_storage: ops
                .sfp_ferry_for_repairs_alterations_storage,
            op_sfp_evacuate_impending_danger: ops.sfp_evacuate_impending_danger,
            op_sfp_excess_of_max_certificated: ops.sfp_excess_of_max_certificated,
            op_sfp_delivery_or_export: ops.sfp_delivery_or_export,
            op_sfp_production_flight_testing: ops.sfp_production_flight_testing,
            op_sfp_customer_demo: ops.sfp_customer_demo,
            type_aircraft_code: aircraft.type_aircraft_code,
            type_engine_code: aircraft.type_engine_code,
            status_code: aircraft.status_code,
            transponder_code: aircraft.transponder_code.map(|t| t as i64),
            fractional_owner: aircraft.fractional_owner,
            airworthiness_date: aircraft.airworthiness_date,
            expiration_date: aircraft.expiration_date,
            unique_id: aircraft.unique_id,
            kit_mfr_name: aircraft.kit_mfr_name,
            kit_model_name: aircraft.kit_model_name,
            club_id: None, // Will be set by repository logic
            device_id: aircraft.device_id,
        }
    }
}

impl From<AircraftRegistrationModel> for Aircraft {
    fn from(model: AircraftRegistrationModel) -> Self {
        // Convert the boolean flags back to ApprovedOps struct
        let approved_ops = ApprovedOps {
            restricted_other: model.op_restricted_other,
            restricted_ag_pest_control: model.op_restricted_ag_pest_control,
            restricted_aerial_surveying: model.op_restricted_aerial_surveying,
            restricted_aerial_advertising: model.op_restricted_aerial_advertising,
            restricted_forest: model.op_restricted_forest,
            restricted_patrolling: model.op_restricted_patrolling,
            restricted_weather_control: model.op_restricted_weather_control,
            restricted_carriage_of_cargo: model.op_restricted_carriage_of_cargo,
            exp_show_compliance: model.op_experimental_show_compliance,
            exp_research_development: model.op_experimental_research_development,
            exp_amateur_built: model.op_experimental_amateur_built,
            exp_exhibition: model.op_experimental_exhibition,
            exp_racing: model.op_experimental_racing,
            exp_crew_training: model.op_experimental_crew_training,
            exp_market_survey: model.op_experimental_market_survey,
            exp_operating_kit_built: model.op_experimental_operating_kit_built,
            exp_lsa_reg_prior_2008: model.op_experimental_light_sport_reg_prior_2008,
            exp_lsa_operating_kit_built: model.op_experimental_light_sport_operating_kit_built,
            exp_lsa_prev_21_190: model.op_experimental_light_sport_prev_21_190,
            exp_uas_research_development: model.op_experimental_uas_research_development,
            exp_uas_market_survey: model.op_experimental_uas_market_survey,
            exp_uas_crew_training: model.op_experimental_uas_crew_training,
            exp_uas_exhibition: model.op_experimental_uas_exhibition,
            exp_uas_compliance_with_cfr: model.op_experimental_uas_compliance_with_cfr,
            sfp_ferry_for_repairs_alterations_storage: model
                .op_sfp_ferry_for_repairs_alterations_storage,
            sfp_evacuate_impending_danger: model.op_sfp_evacuate_impending_danger,
            sfp_excess_of_max_certificated: model.op_sfp_excess_of_max_certificated,
            sfp_delivery_or_export: model.op_sfp_delivery_or_export,
            sfp_production_flight_testing: model.op_sfp_production_flight_testing,
            sfp_customer_demo: model.op_sfp_customer_demo,
        };

        Aircraft {
            n_number: model.registration_number,
            serial_number: model.serial_number,
            manufacturer_code: model.manufacturer_code,
            model_code: model.model_code,
            series_code: model.series_code,
            engine_manufacturer_code: model.engine_manufacturer_code,
            engine_model_code: model.engine_model_code,
            year_mfr: model.year_mfr.map(|y| y as u16),
            type_registration_code: model.type_registration_code,
            registrant_name: model.registrant_name,
            // Legacy fields (not stored in database anymore, set to None)
            street1: None,
            street2: None,
            city: None,
            state: None,
            zip_code: None,
            region_code: None,
            county_mail_code: None,
            country_mail_code: None,
            location_id: model.location_id,
            last_action_date: model.last_action_date,
            certificate_issue_date: model.certificate_issue_date,
            airworthiness_class: model.airworthiness_class,
            approved_operations_raw: model.approved_operations_raw,
            approved_ops,
            type_aircraft_code: model.type_aircraft_code,
            type_engine_code: model.type_engine_code,
            status_code: model.status_code,
            transponder_code: model.transponder_code.map(|t| t as u32),
            fractional_owner: model.fractional_owner,
            airworthiness_date: model.airworthiness_date,
            other_names: Vec::new(), // Would need separate query to fetch
            expiration_date: model.expiration_date,
            unique_id: model.unique_id,
            kit_mfr_name: model.kit_mfr_name,
            kit_model_name: model.kit_model_name,
            home_base_airport_id: None, // Would need to derive from location or separate field
            registered_location: None,  // Legacy field, now in locations table
            device_id: model.device_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_parsing_with_valid_registrations() {
        let csv_path = "tests/fixtures/faa/registrations-valid.csv";
        let aircraft = read_aircraft_csv_file(csv_path).expect("Failed to read CSV file");

        assert!(!aircraft.is_empty(), "Should parse at least one aircraft");

        // Test first aircraft (152AS -> N152AS)
        let first = &aircraft[0];
        assert_eq!(first.n_number, "N152AS");
        assert_eq!(first.serial_number, "3545");
        assert_eq!(first.manufacturer_code, Some("166".to_string()));
        assert_eq!(first.model_code, Some("02".to_string()));
        assert_eq!(first.series_code, Some("25".to_string()));
        assert_eq!(first.year_mfr, Some(1980));
        assert_eq!(first.type_registration_code, Some("3".to_string()));
        assert_eq!(
            first.registrant_name,
            Some("ADIRONDACK SOARING ASSOCIATION INC".to_string())
        );
        assert_eq!(first.city, Some("BALLSTON SPA".to_string()));
        assert_eq!(first.state, Some("NY".to_string()));
        assert_eq!(first.zip_code, Some("12020-2816".to_string())); // Test zip code formatting
        assert_eq!(first.status_code, Some("V".to_string()));

        // Test Mode S code parsing (hex)
        assert_eq!(first.transponder_code, Some(0xA0D1AE));

        // Test second aircraft (9845L)
        let second = &aircraft[1];
        assert_eq!(second.n_number, "N9845L");
        assert_eq!(second.serial_number, "17276634");
        assert_eq!(second.year_mfr, Some(1986));
        assert_eq!(
            second.registrant_name,
            Some("GALAXY AVIATION LLC".to_string())
        );
        assert_eq!(second.city, Some("NANTUCKET".to_string()));
        assert_eq!(second.state, Some("MA".to_string()));
        assert_eq!(second.transponder_code, Some(0xADBD12));

        // Test third aircraft (360EF)
        let third = &aircraft[2];
        assert_eq!(third.n_number, "N360EF");
        assert_eq!(third.serial_number, "3060");
        assert_eq!(third.year_mfr, Some(1995));
        assert_eq!(
            third.registrant_name,
            Some("US AIRFORCE SPECIAL OPERATIONS COMMAND".to_string())
        );
        assert_eq!(third.city, Some("HURLBURT FIELD".to_string()));
        assert_eq!(third.state, Some("FL".to_string()));
        assert_eq!(third.transponder_code, Some(0xA40CB6));

        // Test fourth aircraft (8437D)
        let fourth = &aircraft[3];
        assert_eq!(fourth.n_number, "N8437D");
        assert_eq!(fourth.serial_number, "22-5692");
        assert_eq!(fourth.year_mfr, Some(1957));
        assert_eq!(fourth.registrant_name, Some("CLARK DONALD S".to_string()));
        assert_eq!(fourth.city, Some("ATLANTIC BEACH".to_string()));
        assert_eq!(fourth.state, Some("FL".to_string()));
        assert_eq!(fourth.transponder_code, Some(0xAB8E4F));
    }

    #[test]
    fn test_header_skipping() {
        let csv_path = "tests/fixtures/faa/registrations-valid.csv";
        let aircraft = read_aircraft_csv_file(csv_path).expect("Failed to read CSV file");

        // Ensure we don't parse the header as an aircraft
        // The first aircraft should be N152AS (152AS with N prefix), not the header line
        assert_eq!(aircraft[0].n_number, "N152AS");

        // Verify we have exactly 4 aircraft records (not 5 with header)
        assert_eq!(aircraft.len(), 4);
    }

    #[test]
    fn test_mode_s_hex_parsing() {
        // Test the Mode S hex parsing specifically
        assert_eq!(parse_csv_mode_s("A0D1AE"), Some(0xA0D1AE));
        assert_eq!(parse_csv_mode_s("ADBD12"), Some(0xADBD12));
        assert_eq!(parse_csv_mode_s("A40CB6"), Some(0xA40CB6));
        assert_eq!(parse_csv_mode_s("AB8E4F"), Some(0xAB8E4F));
        assert_eq!(parse_csv_mode_s(""), None);
        assert_eq!(parse_csv_mode_s("   "), None);
    }

    #[test]
    fn test_year_mfr_zero_handling() {
        // Test that zero year_mfr values are converted to None
        assert_eq!(to_opt_u32_nonzero("0"), None);
        assert_eq!(to_opt_u32_nonzero("1980"), Some(1980));
        assert_eq!(to_opt_u32_nonzero(""), None);
        assert_eq!(to_opt_u32_nonzero("   "), None);
        assert_eq!(to_opt_u32_nonzero("2023"), Some(2023));
    }

    #[test]
    fn test_approved_operations_zero_handling() {
        // Test that "0" values for approved_operations_raw are converted to None
        assert_eq!(to_opt_string_no_zero("0"), None);
        assert_eq!(to_opt_string_no_zero("123"), Some("123".to_string()));
        assert_eq!(to_opt_string_no_zero(""), None);
        assert_eq!(to_opt_string_no_zero("   "), None);
        assert_eq!(to_opt_string_no_zero("  0  "), None);
        assert_eq!(to_opt_string_no_zero("12345"), Some("12345".to_string()));
    }

    #[test]
    fn test_zip_code_formatting() {
        // Test 9-digit zip codes get formatted with dash
        assert_eq!(format_zip_code("123456789"), Some("12345-6789".to_string()));
        assert_eq!(format_zip_code("987654321"), Some("98765-4321".to_string()));

        // Test 5-digit zip codes remain unchanged
        assert_eq!(format_zip_code("12345"), Some("12345".to_string()));
        assert_eq!(format_zip_code("90210"), Some("90210".to_string()));

        // Test empty strings return None
        assert_eq!(format_zip_code(""), None);
        assert_eq!(format_zip_code("   "), None);

        // Test whitespace trimming
        assert_eq!(format_zip_code("  12345  "), Some("12345".to_string()));
        assert_eq!(
            format_zip_code("  123456789  "),
            Some("12345-6789".to_string())
        );

        // Test non-numeric 9-character strings remain unchanged
        assert_eq!(format_zip_code("12345abcd"), Some("12345abcd".to_string()));
        assert_eq!(format_zip_code("abcd56789"), Some("abcd56789".to_string()));

        // Test other lengths remain unchanged
        assert_eq!(format_zip_code("1234"), Some("1234".to_string()));
        assert_eq!(
            format_zip_code("1234567890"),
            Some("1234567890".to_string())
        );
    }

    #[test]
    fn test_airworthiness_class_code_zero_handling() {
        // Test that "0" values for airworthiness_class_code are converted to None
        assert_eq!(to_opt_string_no_zero("0"), None);
        assert_eq!(to_opt_string_no_zero("1"), Some("1".to_string()));
        assert_eq!(to_opt_string_no_zero("4"), Some("4".to_string()));
        assert_eq!(to_opt_string_no_zero(""), None);
        assert_eq!(to_opt_string_no_zero("   "), None);
        assert_eq!(to_opt_string_no_zero("  0  "), None);
    }

    #[test]
    fn test_registrant_type_enum() {
        // Test RegistrantType enum conversion from string codes
        assert_eq!(RegistrantType::from("1"), RegistrantType::Individual);
        assert_eq!(RegistrantType::from("2"), RegistrantType::Partnership);
        assert_eq!(RegistrantType::from("3"), RegistrantType::Corporation);
        assert_eq!(RegistrantType::from("4"), RegistrantType::CoOwned);
        assert_eq!(RegistrantType::from("5"), RegistrantType::Government);
        assert_eq!(RegistrantType::from("7"), RegistrantType::Llc);
        assert_eq!(
            RegistrantType::from("8"),
            RegistrantType::NonCitizenCorporation
        );
        assert_eq!(RegistrantType::from("9"), RegistrantType::NonCitizenCoOwned);
        assert_eq!(RegistrantType::from("6"), RegistrantType::Unknown); // Invalid code
        assert_eq!(RegistrantType::from(""), RegistrantType::Unknown);
        assert_eq!(RegistrantType::from("X"), RegistrantType::Unknown);

        // Test conversion from Option<String>
        assert_eq!(
            RegistrantType::from(Some("3".to_string())),
            RegistrantType::Corporation
        );
        assert_eq!(RegistrantType::from(None), RegistrantType::Unknown);
    }

    #[test]
    fn test_aircraft_registrant_type_method() {
        let csv_path = "tests/fixtures/faa/registrations-valid.csv";
        let aircraft = read_aircraft_csv_file(csv_path).expect("Failed to read CSV file");

        // Test first aircraft (152AS) - should be Corporation (code "3")
        let first = &aircraft[0];
        assert_eq!(first.registrant_type(), RegistrantType::Corporation);

        // Test that the method works with the actual data
        assert_eq!(first.type_registration_code, Some("3".to_string()));
    }

    #[test]
    fn test_club_name_method() {
        // Test with actual data from CSV - first aircraft is "ADIRONDACK SOARING ASSOCIATION INC"
        let csv_path = "tests/fixtures/faa/registrations-valid.csv";
        let aircraft = read_aircraft_csv_file(csv_path).expect("Failed to read CSV file");

        let first = &aircraft[0];
        assert_eq!(
            first.registrant_name,
            Some("ADIRONDACK SOARING ASSOCIATION INC".to_string())
        );
        assert_eq!(first.registrant_type(), RegistrantType::Corporation);

        // Should return normalized club name (contains "SOAR" and is Corporation)
        let club_name = first.club_name();
        assert_eq!(
            club_name,
            Some("ADIRONDACK SOARING ASSOCIATION".to_string())
        );

        // Test with manual Aircraft instances
        let mut test_aircraft = Aircraft {
            n_number: "TEST1".to_string(),
            serial_number: "12345".to_string(),
            type_registration_code: Some("3".to_string()), // Corporation
            registrant_name: Some("MOUNTAIN SOARING CLUB INC".to_string()),
            manufacturer_code: None,
            model_code: None,
            series_code: None,
            engine_manufacturer_code: None,
            engine_model_code: None,
            year_mfr: None,
            street1: None,
            street2: None,
            city: None,
            state: None,
            zip_code: None,
            region_code: None,
            county_mail_code: None,
            country_mail_code: None,
            last_action_date: None,
            certificate_issue_date: None,
            airworthiness_class: None,
            approved_operations_raw: None,
            approved_ops: ApprovedOps::default(),
            type_aircraft_code: None,
            type_engine_code: None,
            status_code: None,
            transponder_code: None,
            fractional_owner: None,
            airworthiness_date: None,
            other_names: Vec::new(),
            expiration_date: None,
            unique_id: None,
            kit_mfr_name: None,
            kit_model_name: None,
            home_base_airport_id: None,
            registered_location: None,
            location_id: None,
            device_id: None,
        };

        // Test club with "SOAR" in name
        assert_eq!(
            test_aircraft.club_name(),
            Some("MOUNTAIN SOARING CLUB".to_string())
        );

        // Test club with "CLUB" in name
        test_aircraft.registrant_name = Some("VALLEY GLIDING CLUB LLC".to_string());
        assert_eq!(
            test_aircraft.club_name(),
            Some("VALLEY GLIDING CLUB".to_string())
        );

        // Test ASSOCIATES -> ASSOCIATION replacement
        test_aircraft.registrant_name = Some("RIDGE SOARING ASSOCIATES CO".to_string());
        assert_eq!(
            test_aircraft.club_name(),
            Some("RIDGE SOARING ASSOCIATION".to_string())
        );

        // Test ASSOC -> ASSOCIATION replacement
        test_aircraft.registrant_name = Some("THERMAL SOARING ASSOC INC".to_string());
        assert_eq!(
            test_aircraft.club_name(),
            Some("THERMAL SOARING ASSOCIATION".to_string())
        );

        // Test with Individual registrant type (should return None)
        test_aircraft.type_registration_code = Some("1".to_string()); // Individual
        test_aircraft.registrant_name = Some("JOHN DOE SOARING CLUB".to_string());
        assert_eq!(test_aircraft.club_name(), None);

        // Test with Government registrant type (should return None)
        test_aircraft.type_registration_code = Some("5".to_string()); // Government
        assert_eq!(test_aircraft.club_name(), None);

        // Test with eligible type but no "SOAR" or "CLUB" in name
        test_aircraft.type_registration_code = Some("3".to_string()); // Corporation
        test_aircraft.registrant_name = Some("AVIATION SERVICES INC".to_string());
        assert_eq!(test_aircraft.club_name(), None);

        // Test with LLC registrant type
        test_aircraft.type_registration_code = Some("7".to_string()); // LLC
        test_aircraft.registrant_name = Some("DESERT SOARING LLC".to_string());
        assert_eq!(
            test_aircraft.club_name(),
            Some("DESERT SOARING".to_string())
        );

        // Test with Partnership registrant type
        test_aircraft.type_registration_code = Some("2".to_string()); // Partnership
        test_aircraft.registrant_name = Some("COASTAL CLUB PARTNERSHIP".to_string());
        assert_eq!(test_aircraft.club_name(), Some("COASTAL CLUB".to_string()));

        // Test with CoOwned registrant type
        test_aircraft.type_registration_code = Some("4".to_string()); // CoOwned
        test_aircraft.registrant_name = Some("ALPINE SOARING CO-OWNED".to_string());
        assert_eq!(
            test_aircraft.club_name(),
            Some("ALPINE SOARING CO-OWNED".to_string())
        );

        // Test with Unknown registrant type
        test_aircraft.type_registration_code = Some("X".to_string()); // Unknown
        test_aircraft.registrant_name = Some("MYSTERY SOARING CLUB".to_string());
        assert_eq!(
            test_aircraft.club_name(),
            Some("MYSTERY SOARING CLUB".to_string())
        );

        // Test with no registrant name
        test_aircraft.type_registration_code = Some("3".to_string()); // Corporation
        test_aircraft.registrant_name = None;
        assert_eq!(test_aircraft.club_name(), None);

        // Test multiple suffix removal (should only remove one)
        test_aircraft.registrant_name = Some("PEAK SOARING CORPORATION INC".to_string());
        assert_eq!(
            test_aircraft.club_name(),
            Some("PEAK SOARING CORPORATION".to_string())
        );
    }
}
