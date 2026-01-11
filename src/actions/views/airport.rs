use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

use crate::airports::Airport;
use crate::runways::Runway;

/// Calculate a point at a given distance and bearing from a reference point
/// distance_meters: distance in meters
/// bearing_degrees: bearing in degrees (0 = north, 90 = east)
fn calculate_endpoint(
    lat: f64,
    lon: f64,
    distance_meters: f64,
    bearing_degrees: f64,
) -> (f64, f64) {
    let earth_radius_meters = 6371000.0; // Earth's radius in meters
    let bearing_rad = bearing_degrees * PI / 180.0;
    let lat_rad = lat * PI / 180.0;
    let lon_rad = lon * PI / 180.0;

    let angular_distance = distance_meters / earth_radius_meters;

    let new_lat_rad = (lat_rad.sin() * angular_distance.cos()
        + lat_rad.cos() * angular_distance.sin() * bearing_rad.cos())
    .asin();

    let new_lon_rad = lon_rad
        + (bearing_rad.sin() * angular_distance.sin() * lat_rad.cos())
            .atan2(angular_distance.cos() - lat_rad.sin() * new_lat_rad.sin());

    let new_lat = new_lat_rad * 180.0 / PI;
    let new_lon = new_lon_rad * 180.0 / PI;

    (new_lat, new_lon)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunwayEnd {
    pub ident: Option<String>,
    pub latitude_deg: Option<f64>,
    pub longitude_deg: Option<f64>,
    pub elevation_ft: Option<i32>,
    pub heading_degt: Option<f64>,
    pub displaced_threshold_ft: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunwayView {
    pub id: i32,
    pub airport_ident: String,
    pub length_ft: Option<i32>,
    pub width_ft: Option<i32>,
    pub surface: Option<String>,
    pub lighted: bool,
    pub closed: bool,
    pub low: RunwayEnd,
    pub high: RunwayEnd,
    /// Polygon representing the runway rectangle as [lat, lon] coordinates
    /// Array of 4 corner points: [low-left, low-right, high-right, high-left]
    pub polyline: Vec<[f64; 2]>,
}

impl RunwayView {
    /// Create a RunwayView from a Runway
    /// Polyline will only be calculated if runway has endpoint coordinates and width
    pub fn from_runway(runway: Runway) -> Self {
        let low = RunwayEnd {
            ident: runway.le_ident.clone(),
            latitude_deg: runway.le_latitude_deg,
            longitude_deg: runway.le_longitude_deg,
            elevation_ft: runway.le_elevation_ft,
            heading_degt: runway.le_heading_degt,
            displaced_threshold_ft: runway.le_displaced_threshold_ft,
        };

        let high = RunwayEnd {
            ident: runway.he_ident.clone(),
            latitude_deg: runway.he_latitude_deg,
            longitude_deg: runway.he_longitude_deg,
            elevation_ft: runway.he_elevation_ft,
            heading_degt: runway.he_heading_degt,
            displaced_threshold_ft: runway.he_displaced_threshold_ft,
        };

        // Calculate polyline
        let polyline = Self::calculate_polyline(&runway, &low, &high);

        Self {
            id: runway.id,
            airport_ident: runway.airport_ident,
            length_ft: runway.length_ft,
            width_ft: runway.width_ft,
            surface: runway.surface,
            lighted: runway.lighted,
            closed: runway.closed,
            low,
            high,
            polyline,
        }
    }

    /// Calculate the polygon rectangle for a runway
    /// Returns 4 corners: [low-left, low-right, high-right, high-left]
    /// Only creates polyline if runway has actual endpoint coordinates
    fn calculate_polyline(runway: &Runway, low: &RunwayEnd, high: &RunwayEnd) -> Vec<[f64; 2]> {
        let width_ft = match runway.width_ft {
            Some(w) if w > 0 => w,
            _ => return vec![], // Can't create polygon without width
        };

        let width_meters = width_ft as f64 * 0.3048; // Convert feet to meters
        let half_width = width_meters / 2.0;

        // Only create polyline if we have actual runway endpoint coordinates
        let (low_lat, low_lon, high_lat, high_lon) =
            if let (Some(ll), Some(ln), Some(hl), Some(hn)) = (
                low.latitude_deg,
                low.longitude_deg,
                high.latitude_deg,
                high.longitude_deg,
            ) {
                (ll, ln, hl, hn)
            } else {
                return vec![]; // Can't calculate without actual endpoint coordinates
            };

        // Calculate bearing from low to high
        let bearing = Self::calculate_bearing(low_lat, low_lon, high_lat, high_lon);

        // Perpendicular bearing (90 degrees to the right)
        let perp_bearing = (bearing + 90.0) % 360.0;

        // Calculate the 4 corners
        let low_left = calculate_endpoint(low_lat, low_lon, half_width, perp_bearing + 180.0);
        let low_right = calculate_endpoint(low_lat, low_lon, half_width, perp_bearing);
        let high_right = calculate_endpoint(high_lat, high_lon, half_width, perp_bearing);
        let high_left = calculate_endpoint(high_lat, high_lon, half_width, perp_bearing + 180.0);

        vec![
            [low_left.0, low_left.1],
            [low_right.0, low_right.1],
            [high_right.0, high_right.1],
            [high_left.0, high_left.1],
        ]
    }

    /// Calculate bearing from point 1 to point 2 in degrees
    fn calculate_bearing(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
        let lat1_rad = lat1 * PI / 180.0;
        let lat2_rad = lat2 * PI / 180.0;
        let dlon = (lon2 - lon1) * PI / 180.0;

        let y = dlon.sin() * lat2_rad.cos();
        let x = lat1_rad.cos() * lat2_rad.sin() - lat1_rad.sin() * lat2_rad.cos() * dlon.cos();

        let bearing_rad = y.atan2(x);

        (bearing_rad * 180.0 / PI + 360.0) % 360.0
    }
}

impl From<Runway> for RunwayView {
    fn from(runway: Runway) -> Self {
        Self::from_runway(runway)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AirportView {
    pub id: i32,
    pub ident: String,
    pub airport_type: String,
    pub name: String,
    pub latitude_deg: Option<BigDecimal>,
    pub longitude_deg: Option<BigDecimal>,
    pub elevation_ft: Option<i32>,
    pub continent: Option<String>,
    pub iso_country: Option<String>,
    pub iso_region: Option<String>,
    pub municipality: Option<String>,
    pub scheduled_service: bool,
    pub icao_code: Option<String>,
    pub iata_code: Option<String>,
    pub gps_code: Option<String>,
    pub local_code: Option<String>,
    pub home_link: Option<String>,
    pub wikipedia_link: Option<String>,
    pub keywords: Option<String>,
    pub runways: Vec<RunwayView>,
}

impl From<Airport> for AirportView {
    fn from(airport: Airport) -> Self {
        Self {
            id: airport.id,
            ident: airport.ident,
            airport_type: airport.airport_type,
            name: airport.name,
            latitude_deg: airport.latitude_deg,
            longitude_deg: airport.longitude_deg,
            elevation_ft: airport.elevation_ft,
            continent: airport.continent,
            iso_country: airport.iso_country,
            iso_region: airport.iso_region,
            municipality: airport.municipality,
            scheduled_service: airport.scheduled_service,
            icao_code: airport.icao_code,
            iata_code: airport.iata_code,
            gps_code: airport.gps_code,
            local_code: airport.local_code,
            home_link: airport.home_link,
            wikipedia_link: airport.wikipedia_link,
            keywords: airport.keywords,
            runways: Vec::new(), // Will be populated separately
        }
    }
}

impl AirportView {
    /// Create an AirportView with runways populated
    pub fn with_runways(airport: Airport, runways: Vec<Runway>) -> Self {
        let mut view = AirportView::from(airport);

        view.runways = runways.into_iter().map(RunwayView::from_runway).collect();
        view
    }

    /// Add runways to an existing AirportView
    pub fn add_runways(&mut self, runways: Vec<Runway>) {
        self.runways = runways.into_iter().map(RunwayView::from_runway).collect();
    }
}
