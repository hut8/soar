use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

/// Diesel model for receiver_coverage_h3 table
#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::receiver_coverage_h3)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ReceiverCoverageH3 {
    pub h3_index: i64,
    pub resolution: i16,
    pub receiver_id: Uuid,
    pub date: NaiveDate,
    pub fix_count: i32,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub min_altitude_msl_feet: Option<i32>,
    pub max_altitude_msl_feet: Option<i32>,
    pub avg_altitude_msl_feet: Option<i32>,
    pub updated_at: DateTime<Utc>,
}

/// Insertable model for coverage upserts
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = crate::schema::receiver_coverage_h3)]
pub struct NewReceiverCoverageH3 {
    pub h3_index: i64,
    pub resolution: i16,
    pub receiver_id: Uuid,
    pub date: NaiveDate,
    pub fix_count: i32,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub min_altitude_msl_feet: Option<i32>,
    pub max_altitude_msl_feet: Option<i32>,
    pub avg_altitude_msl_feet: Option<i32>,
}

/// GeoJSON Feature for H3 hex (API response)
#[derive(Serialize, Debug, Clone)]
pub struct CoverageHexFeature {
    #[serde(rename = "type")]
    pub feature_type: String, // Always "Feature"
    pub geometry: serde_json::Value, // H3 hex polygon as GeoJSON
    pub properties: CoverageHexProperties,
}

/// Properties for coverage hex feature
#[derive(Serialize, Debug, Clone)]
pub struct CoverageHexProperties {
    pub h3_index: String, // H3 index as hex string for frontend
    pub resolution: i16,
    pub receiver_id: Uuid,
    pub fix_count: i32,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub min_altitude_msl_feet: Option<i32>,
    pub max_altitude_msl_feet: Option<i32>,
    pub avg_altitude_msl_feet: Option<i32>,
    pub coverage_hours: f64, // Hours between first and last seen
}

impl CoverageHexFeature {
    /// Convert H3 index to GeoJSON polygon
    pub fn from_coverage(coverage: ReceiverCoverageH3) -> anyhow::Result<Self> {
        use h3o::CellIndex;

        // Convert BIGINT to H3 index
        let h3_index = CellIndex::try_from(coverage.h3_index as u64)?;

        // Get hex boundary as lat/lng coordinates
        let boundary = h3_index.boundary();

        // Convert to GeoJSON polygon
        // Note: GeoJSON uses [lng, lat] order, not [lat, lng]
        let mut coords: Vec<[f64; 2]> = boundary
            .iter()
            .map(|latlng| [latlng.lng(), latlng.lat()])
            .collect();

        // Close the polygon by adding the first point again
        if let Some(first) = coords.first().copied() {
            coords.push(first);
        }

        let geometry = serde_json::json!({
            "type": "Polygon",
            "coordinates": [coords]
        });

        let coverage_hours =
            (coverage.last_seen_at - coverage.first_seen_at).num_seconds() as f64 / 3600.0;

        Ok(Self {
            feature_type: "Feature".to_string(),
            geometry,
            properties: CoverageHexProperties {
                h3_index: h3_index.to_string(), // Hex string representation
                resolution: coverage.resolution,
                receiver_id: coverage.receiver_id,
                fix_count: coverage.fix_count,
                first_seen_at: coverage.first_seen_at,
                last_seen_at: coverage.last_seen_at,
                min_altitude_msl_feet: coverage.min_altitude_msl_feet,
                max_altitude_msl_feet: coverage.max_altitude_msl_feet,
                avg_altitude_msl_feet: coverage.avg_altitude_msl_feet,
                coverage_hours,
            },
        })
    }
}
