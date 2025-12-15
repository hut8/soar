use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::airspaces;

/// ICAO Airspace Classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, DbEnum)]
#[db_enum(existing_type_path = "crate::schema::sql_types::AirspaceClass")]
pub enum AirspaceClass {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    #[serde(rename = "SUA")]
    Sua, // Special Use Airspace
}

/// OpenAIP Airspace Types (37 types)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, DbEnum)]
#[db_enum(existing_type_path = "crate::schema::sql_types::AirspaceType")]
pub enum AirspaceType {
    Restricted,
    Danger,
    Prohibited,
    #[serde(rename = "CTR")]
    Ctr,
    #[serde(rename = "TMZ")]
    Tmz,
    #[serde(rename = "RMZ")]
    Rmz,
    #[serde(rename = "TMA")]
    Tma,
    #[serde(rename = "ATZ")]
    Atz,
    #[serde(rename = "MATZ")]
    Matz,
    Airway,
    #[serde(rename = "MTR")]
    Mtr,
    AlertArea,
    WarningArea,
    ProtectedArea,
    #[serde(rename = "HTZ")]
    Htz,
    GliderProhibited,
    GliderSector,
    NoGliders,
    WaveWindow,
    Other,
    #[serde(rename = "FIR")]
    Fir,
    #[serde(rename = "UIR")]
    Uir,
    #[serde(rename = "ADIZ")]
    Adiz,
    #[serde(rename = "ATZ_P")]
    AtzP,
    #[serde(rename = "ATZ_MBZ")]
    AtzMbz,
    #[serde(rename = "TFR")]
    Tfr,
    #[serde(rename = "TRA")]
    Tra,
    #[serde(rename = "TSA")]
    Tsa,
    #[serde(rename = "FIS")]
    Fis,
    #[serde(rename = "UAS")]
    Uas,
    #[serde(rename = "RFFS")]
    Rffs,
    Sport,
    DropZone,
    Gliding,
    MilitaryOps,
    NotAssigned,
}

/// Altitude Reference System
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, DbEnum)]
#[db_enum(existing_type_path = "crate::schema::sql_types::AltitudeReference")]
pub enum AltitudeReference {
    #[serde(rename = "MSL")]
    Msl, // Mean Sea Level
    #[serde(rename = "AGL")]
    Agl, // Above Ground Level
    #[serde(rename = "STD")]
    Std, // Standard (Flight Level)
    #[serde(rename = "GND")]
    Gnd, // Ground
    #[serde(rename = "UNL")]
    Unl, // Unlimited
}

/// Airspace model for database queries (excluding geometry fields)
#[derive(Debug, Clone, Queryable, Selectable, Serialize)]
#[diesel(table_name = airspaces)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AirspaceModel {
    pub id: Uuid,
    pub openaip_id: i32,
    pub name: String,
    pub airspace_class: Option<AirspaceClass>,
    pub airspace_type: AirspaceType,
    pub country_code: Option<String>,
    pub lower_value: Option<i32>,
    pub lower_unit: Option<String>,
    pub lower_reference: Option<AltitudeReference>,
    pub upper_value: Option<i32>,
    pub upper_unit: Option<String>,
    pub upper_reference: Option<AltitudeReference>,
    // Note: geometry fields excluded from default queries (use raw queries for geometry)
    #[serde(skip)]
    pub remarks: Option<String>,
    pub activity_type: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub openaip_updated_at: Option<DateTime<Utc>>,
}

/// Insertable airspace for upserts (geometry handled via raw SQL)
#[derive(Debug, Clone)]
pub struct NewAirspace {
    pub openaip_id: i32,
    pub name: String,
    pub airspace_class: Option<AirspaceClass>,
    pub airspace_type: AirspaceType,
    pub country_code: Option<String>,
    pub lower_value: Option<i32>,
    pub lower_unit: Option<String>,
    pub lower_reference: Option<AltitudeReference>,
    pub upper_value: Option<i32>,
    pub upper_unit: Option<String>,
    pub upper_reference: Option<AltitudeReference>,
    pub remarks: Option<String>,
    pub activity_type: Option<String>,
    pub openaip_updated_at: Option<DateTime<Utc>>,
}

/// GeoJSON Feature for API responses
#[derive(Debug, Serialize)]
pub struct AirspaceGeoJson {
    #[serde(rename = "type")]
    pub feature_type: String, // Always "Feature"
    pub geometry: serde_json::Value, // GeoJSON geometry
    pub properties: AirspaceProperties,
}

#[derive(Debug, Serialize)]
pub struct AirspaceProperties {
    pub id: Uuid,
    pub openaip_id: i32,
    pub name: String,
    pub airspace_class: Option<AirspaceClass>,
    pub airspace_type: AirspaceType,
    pub country_code: Option<String>,
    pub lower_limit: String, // Formatted like "500 FT MSL" or "FL095"
    pub upper_limit: String,
    pub remarks: Option<String>,
    pub activity_type: Option<String>,
}

impl AirspaceGeoJson {
    /// Format altitude limit for display
    pub fn format_altitude(
        value: Option<i32>,
        unit: Option<&str>,
        reference: Option<AltitudeReference>,
    ) -> String {
        match (value, unit, reference) {
            (Some(v), Some("FL"), _) => format!("FL{:03}", v),
            (Some(v), Some(u), Some(r)) => format!("{} {} {:?}", v, u, r),
            (Some(v), Some(u), None) => format!("{} {}", v, u),
            (None, _, Some(AltitudeReference::Gnd)) => "GND".to_string(),
            (None, _, Some(AltitudeReference::Unl)) => "UNL".to_string(),
            _ => "Unknown".to_string(),
        }
    }
}
