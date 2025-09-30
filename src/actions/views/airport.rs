use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};

use crate::airports::Airport;
use crate::runways::Runway;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunwayEnd {
    pub ident: Option<String>,
    pub latitude_deg: Option<f64>,
    pub longitude_deg: Option<f64>,
    pub elevation_ft: Option<i32>,
    pub heading_degt: Option<f64>,
    pub displaced_threshold_ft: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunwayView {
    pub id: i32,
    pub length_ft: Option<i32>,
    pub width_ft: Option<i32>,
    pub surface: Option<String>,
    pub lighted: bool,
    pub closed: bool,
    pub low: RunwayEnd,
    pub high: RunwayEnd,
}

impl From<Runway> for RunwayView {
    fn from(runway: Runway) -> Self {
        Self {
            id: runway.id,
            length_ft: runway.length_ft,
            width_ft: runway.width_ft,
            surface: runway.surface,
            lighted: runway.lighted,
            closed: runway.closed,
            low: RunwayEnd {
                ident: runway.le_ident,
                latitude_deg: runway.le_latitude_deg,
                longitude_deg: runway.le_longitude_deg,
                elevation_ft: runway.le_elevation_ft,
                heading_degt: runway.le_heading_degt,
                displaced_threshold_ft: runway.le_displaced_threshold_ft,
            },
            high: RunwayEnd {
                ident: runway.he_ident,
                latitude_deg: runway.he_latitude_deg,
                longitude_deg: runway.he_longitude_deg,
                elevation_ft: runway.he_elevation_ft,
                heading_degt: runway.he_heading_degt,
                displaced_threshold_ft: runway.he_displaced_threshold_ft,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        view.runways = runways.into_iter().map(RunwayView::from).collect();
        view
    }

    /// Add runways to an existing AirportView
    pub fn add_runways(&mut self, runways: Vec<Runway>) {
        self.runways = runways.into_iter().map(RunwayView::from).collect();
    }
}
