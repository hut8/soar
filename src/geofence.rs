//! Geofence models and types
//!
//! Geofences are "upside-down birthday cake" shaped boundaries with multiple altitude layers,
//! similar to Class B airspace. Each layer has its own radius from the center point.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// A single altitude layer with its radius
/// Altitudes are MSL (Mean Sea Level) in feet
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct GeofenceLayer {
    /// Floor altitude in feet MSL
    pub floor_ft: i32,
    /// Ceiling altitude in feet MSL
    pub ceiling_ft: i32,
    /// Radius from center in nautical miles
    pub radius_nm: f64,
}

impl GeofenceLayer {
    /// Create a new layer
    pub fn new(floor_ft: i32, ceiling_ft: i32, radius_nm: f64) -> Self {
        Self {
            floor_ft,
            ceiling_ft,
            radius_nm,
        }
    }

    /// Check if an altitude (MSL feet) is within this layer's altitude range
    pub fn contains_altitude(&self, altitude_ft: i32) -> bool {
        altitude_ft >= self.floor_ft && altitude_ft <= self.ceiling_ft
    }

    /// Get the radius in meters
    pub fn radius_meters(&self) -> f64 {
        self.radius_nm * 1852.0
    }
}

/// Geofence data for API responses
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct Geofence {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub center_latitude: f64,
    pub center_longitude: f64,
    pub max_radius_meters: f64,
    pub layers: Vec<GeofenceLayer>,
    pub owner_user_id: Uuid,
    pub club_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a new geofence
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct CreateGeofenceRequest {
    pub name: String,
    pub description: Option<String>,
    pub center_latitude: f64,
    pub center_longitude: f64,
    pub layers: Vec<GeofenceLayer>,
    pub club_id: Option<Uuid>,
}

impl CreateGeofenceRequest {
    /// Validate the request
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Name is required".to_string());
        }
        if self.name.len() > 255 {
            return Err("Name must be 255 characters or less".to_string());
        }
        if self.layers.is_empty() {
            return Err("At least one layer is required".to_string());
        }
        for (i, layer) in self.layers.iter().enumerate() {
            if layer.ceiling_ft <= layer.floor_ft {
                return Err(format!(
                    "Layer {}: ceiling must be greater than floor",
                    i + 1
                ));
            }
            if layer.radius_nm <= 0.0 {
                return Err(format!("Layer {}: radius must be positive", i + 1));
            }
        }
        if self.center_latitude < -90.0 || self.center_latitude > 90.0 {
            return Err("Latitude must be between -90 and 90".to_string());
        }
        if self.center_longitude < -180.0 || self.center_longitude > 180.0 {
            return Err("Longitude must be between -180 and 180".to_string());
        }
        Ok(())
    }

    /// Calculate the maximum radius across all layers (in meters)
    pub fn max_radius_meters(&self) -> f64 {
        self.layers
            .iter()
            .map(|l| l.radius_meters())
            .fold(0.0_f64, f64::max)
    }
}

/// Request to update a geofence
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct UpdateGeofenceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub center_latitude: Option<f64>,
    pub center_longitude: Option<f64>,
    pub layers: Option<Vec<GeofenceLayer>>,
}

impl UpdateGeofenceRequest {
    /// Validate the request
    pub fn validate(&self) -> Result<(), String> {
        if let Some(name) = &self.name {
            if name.is_empty() {
                return Err("Name cannot be empty".to_string());
            }
            if name.len() > 255 {
                return Err("Name must be 255 characters or less".to_string());
            }
        }
        if let Some(layers) = &self.layers {
            if layers.is_empty() {
                return Err("At least one layer is required".to_string());
            }
            for (i, layer) in layers.iter().enumerate() {
                if layer.ceiling_ft <= layer.floor_ft {
                    return Err(format!(
                        "Layer {}: ceiling must be greater than floor",
                        i + 1
                    ));
                }
                if layer.radius_nm <= 0.0 {
                    return Err(format!("Layer {}: radius must be positive", i + 1));
                }
            }
        }
        if let Some(lat) = self.center_latitude
            && !(-90.0..=90.0).contains(&lat)
        {
            return Err("Latitude must be between -90 and 90".to_string());
        }
        if let Some(lon) = self.center_longitude
            && !(-180.0..=180.0).contains(&lon)
        {
            return Err("Longitude must be between -180 and 180".to_string());
        }
        Ok(())
    }

    /// Calculate the maximum radius across all layers (in meters)
    pub fn max_radius_meters(&self) -> Option<f64> {
        self.layers.as_ref().map(|layers| {
            layers
                .iter()
                .map(|l| l.radius_meters())
                .fold(0.0_f64, f64::max)
        })
    }
}

/// Geofence subscriber entry
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct GeofenceSubscriber {
    pub geofence_id: Uuid,
    pub user_id: Uuid,
    pub send_email: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to subscribe to a geofence
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeToGeofenceRequest {
    #[serde(default = "default_true")]
    pub send_email: bool,
}

fn default_true() -> bool {
    true
}

/// Aircraft-geofence link
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct AircraftGeofence {
    pub aircraft_id: Uuid,
    pub geofence_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Request to link aircraft to geofence
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkAircraftRequest {
    pub aircraft_id: Uuid,
}

/// Geofence exit event - recorded when an aircraft exits a geofence boundary
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct GeofenceExitEvent {
    pub id: Uuid,
    pub geofence_id: Uuid,
    pub flight_id: Uuid,
    pub aircraft_id: Uuid,
    pub exit_time: DateTime<Utc>,
    pub exit_latitude: f64,
    pub exit_longitude: f64,
    pub exit_altitude_msl_ft: Option<i32>,
    pub exit_layer: GeofenceLayer,
    pub email_notifications_sent: i32,
    pub created_at: DateTime<Utc>,
}

/// Geofence with linked aircraft count and subscriber count
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct GeofenceWithCounts {
    #[serde(flatten)]
    pub geofence: Geofence,
    pub aircraft_count: i64,
    pub subscriber_count: i64,
}

/// List response for geofences
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct GeofenceListResponse {
    pub geofences: Vec<GeofenceWithCounts>,
}

/// Detail response for a single geofence
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct GeofenceDetailResponse {
    pub geofence: Geofence,
    pub aircraft_count: i64,
    pub subscriber_count: i64,
}

/// Exit events response
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct GeofenceExitEventsResponse {
    pub events: Vec<GeofenceExitEvent>,
}
