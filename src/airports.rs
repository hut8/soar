// https://www.openaip.net/data/exports?page=1&limit=50&sortBy=createdAt&sortDesc=true&contentType=airport&format=ndgeojson&country=US

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirportFeature {
    #[serde(rename = "type")]
    pub feature_type: String,
    pub id: u32,
    pub properties: AirportProperties,
    pub geometry: Geometry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirportProperties {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    #[serde(rename = "icaoCode")]
    pub icao_code: String,
    #[serde(rename = "iataCode")]
    pub iata_code: Option<String>,
    #[serde(rename = "type")]
    pub airport_type: u8,
    #[serde(rename = "trafficType")]
    pub traffic_type: Vec<u8>,
    #[serde(rename = "magneticDeclination")]
    pub magnetic_declination: f64,
    pub country: String,
    pub elevation: Elevation,
    pub ppr: bool,
    pub private: bool,
    #[serde(rename = "skydiveActivity")]
    pub skydive_activity: bool,
    #[serde(rename = "winchOnly")]
    pub winch_only: bool,
    pub frequencies: Vec<Frequency>,
    pub runways: Vec<Runway>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "createdBy")]
    pub created_by: String,
    #[serde(rename = "updatedBy")]
    pub updated_by: String,
    #[serde(rename = "elevationGeoid")]
    pub elevation_geoid: ElevationGeoid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Elevation {
    pub value: u32,
    pub unit: u8,
    #[serde(rename = "referenceDatum")]
    pub reference_datum: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElevationGeoid {
    #[serde(rename = "geoidHeight")]
    pub geoid_height: i32,
    pub hae: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frequency {
    pub value: String,
    pub unit: u8,
    #[serde(rename = "type")]
    pub frequency_type: u8,
    pub name: String,
    pub primary: bool,
    #[serde(rename = "publicUse")]
    pub public_use: bool,
    #[serde(rename = "_id")]
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runway {
    pub designator: String,
    #[serde(rename = "trueHeading")]
    pub true_heading: u16,
    #[serde(rename = "alignedTrueNorth")]
    pub aligned_true_north: bool,
    pub operations: u8,
    #[serde(rename = "mainRunway")]
    pub main_runway: bool,
    #[serde(rename = "turnDirection")]
    pub turn_direction: u8,
    #[serde(rename = "takeOffOnly")]
    pub take_off_only: bool,
    #[serde(rename = "landingOnly")]
    pub landing_only: bool,
    pub surface: Surface,
    pub dimension: Dimension,
    #[serde(rename = "declaredDistance")]
    pub declared_distance: DeclaredDistance,
    #[serde(rename = "pilotCtrlLighting")]
    pub pilot_ctrl_lighting: bool,
    #[serde(rename = "_id")]
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Surface {
    pub composition: Vec<u8>,
    #[serde(rename = "mainComposite")]
    pub main_composite: u8,
    pub condition: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimension {
    pub length: Measurement,
    pub width: Measurement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredDistance {
    pub tora: Measurement,
    pub lda: Measurement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measurement {
    pub value: u32,
    pub unit: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Geometry {
    #[serde(rename = "type")]
    pub geometry_type: String,
    pub coordinates: Vec<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_airport_deserialization() {
        let json_data = r#"
        {
            "type":"Feature",
            "id":4614,
            "properties":{
                "_id":"6261547d0e8346dfd9256ff4",
                "name":"DENVER INTERNATIONAL AIRPORT",
                "icaoCode":"KDEN",
                "type":2,
                "trafficType":[0],
                "magneticDeclination":7,
                "country":"US",
                "elevation":{"value":1655,"unit":0,"referenceDatum":1},
                "ppr":false,
                "private":false,
                "skydiveActivity":false,
                "winchOnly":false,
                "frequencies":[
                    {
                        "value":"119.300",
                        "unit":2,
                        "type":0,
                        "name":"APP (NORTH)",
                        "primary":false,
                        "publicUse":true,
                        "_id":"6261547d0e8346dfd9256ff5"
                    }
                ],
                "runways":[
                    {
                        "designator":"08",
                        "trueHeading":70,
                        "alignedTrueNorth":false,
                        "operations":0,
                        "mainRunway":false,
                        "turnDirection":2,
                        "takeOffOnly":false,
                        "landingOnly":false,
                        "surface":{"composition":[1],"mainComposite":1,"condition":0},
                        "dimension":{"length":{"value":3657,"unit":0},"width":{"value":45,"unit":0}},
                        "declaredDistance":{"tora":{"value":3657,"unit":0},"lda":{"value":3657,"unit":0}},
                        "pilotCtrlLighting":false,
                        "_id":"6261547d0e8346dfd9257009"
                    }
                ],
                "createdAt":"2022-04-21T12:56:29.331Z",
                "updatedAt":"2024-06-21T19:44:51.409Z",
                "createdBy":"AUTO-IMPORTER",
                "updatedBy":"OPONcQnzWGOLiJSceNaf8pvx1fA2",
                "elevationGeoid":{"geoidHeight":-18,"hae":1637},
                "iataCode":"DEN"
            },
            "geometry":{
                "type":"Point",
                "coordinates":[-104.673,39.8617]
            }
        }
        "#;

        let airport: Result<AirportFeature, _> = serde_json::from_str(json_data);
        assert!(airport.is_ok());

        let airport = airport.unwrap();
        assert_eq!(airport.properties.name, "DENVER INTERNATIONAL AIRPORT");
        assert_eq!(airport.properties.icao_code, "KDEN");
        assert_eq!(airport.properties.iata_code, Some("DEN".to_string()));
        assert_eq!(airport.geometry.coordinates, vec![-104.673, 39.8617]);
    }

    #[test]
    fn test_airport_serialization() {
        let airport = AirportFeature {
            feature_type: "Feature".to_string(),
            id: 4614,
            properties: AirportProperties {
                id: "6261547d0e8346dfd9256ff4".to_string(),
                name: "DENVER INTERNATIONAL AIRPORT".to_string(),
                icao_code: "KDEN".to_string(),
                iata_code: Some("DEN".to_string()),
                airport_type: 2,
                traffic_type: vec![0],
                magnetic_declination: 7.0,
                country: "US".to_string(),
                elevation: Elevation {
                    value: 1655,
                    unit: 0,
                    reference_datum: 1,
                },
                ppr: false,
                private: false,
                skydive_activity: false,
                winch_only: false,
                frequencies: vec![],
                runways: vec![],
                created_at: "2022-04-21T12:56:29.331Z".to_string(),
                updated_at: "2024-06-21T19:44:51.409Z".to_string(),
                created_by: "AUTO-IMPORTER".to_string(),
                updated_by: "OPONcQnzWGOLiJSceNaf8pvx1fA2".to_string(),
                elevation_geoid: ElevationGeoid {
                    geoid_height: -18,
                    hae: 1637,
                },
            },
            geometry: Geometry {
                geometry_type: "Point".to_string(),
                coordinates: vec![-104.673, 39.8617],
            },
        };

        let json = serde_json::to_string(&airport);
        assert!(json.is_ok());
    }
}
