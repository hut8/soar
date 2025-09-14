use anyhow::Result;
use diesel::prelude::*;
use diesel::upsert::excluded;
use tracing::info;

use crate::airports::{Airport, AirportModel, NewAirportModel};
use crate::web::PgPool;

#[derive(QueryableByName, Debug)]
struct AirportWithDistance {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    id: i32,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    ident: String,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    airport_type: String,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float8>)]
    latitude_deg: Option<f64>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float8>)]
    longitude_deg: Option<f64>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
    elevation_ft: Option<i32>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    continent: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    iso_country: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    iso_region: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    municipality: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    scheduled_service: bool,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    icao_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    iata_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    gps_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    local_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    home_link: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    wikipedia_link: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    keywords: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float8>)]
    distance_meters: Option<f64>,
}

impl From<AirportWithDistance> for Airport {
    fn from(awd: AirportWithDistance) -> Self {
        Self {
            id: awd.id,
            ident: awd.ident,
            airport_type: awd.airport_type,
            name: awd.name,
            latitude_deg: awd.latitude_deg,
            longitude_deg: awd.longitude_deg,
            elevation_ft: awd.elevation_ft,
            continent: awd.continent,
            iso_country: awd.iso_country,
            iso_region: awd.iso_region,
            municipality: awd.municipality,
            scheduled_service: awd.scheduled_service,
            icao_code: awd.icao_code,
            iata_code: awd.iata_code,
            gps_code: awd.gps_code,
            local_code: awd.local_code,
            home_link: awd.home_link,
            wikipedia_link: awd.wikipedia_link,
            keywords: awd.keywords,
        }
    }
}

pub struct AirportsRepository {
    pool: PgPool,
}

impl AirportsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Upsert airports into the database
    /// This will insert new airports or update existing ones based on the primary key (id)
    pub async fn upsert_airports<I>(&self, airports_list: I) -> Result<usize>
    where
        I: IntoIterator<Item = Airport>,
    {
        use crate::schema::airports::dsl::*;

        let airports_vec: Vec<Airport> = airports_list.into_iter().collect();
        let new_airports: Vec<NewAirportModel> = airports_vec.into_iter().map(|a| a.into()).collect();

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Use Diesel's on_conflict for upserts
            let upserted_count = diesel::insert_into(airports)
                .values(&new_airports)
                .on_conflict(id)
                .do_update()
                .set((
                    ident.eq(excluded(ident)),
                    type_.eq(excluded(type_)),
                    name.eq(excluded(name)),
                    latitude_deg.eq(excluded(latitude_deg)),
                    longitude_deg.eq(excluded(longitude_deg)),
                    elevation_ft.eq(excluded(elevation_ft)),
                    continent.eq(excluded(continent)),
                    iso_country.eq(excluded(iso_country)),
                    iso_region.eq(excluded(iso_region)),
                    municipality.eq(excluded(municipality)),
                    scheduled_service.eq(excluded(scheduled_service)),
                    icao_code.eq(excluded(icao_code)),
                    iata_code.eq(excluded(iata_code)),
                    gps_code.eq(excluded(gps_code)),
                    local_code.eq(excluded(local_code)),
                    home_link.eq(excluded(home_link)),
                    wikipedia_link.eq(excluded(wikipedia_link)),
                    keywords.eq(excluded(keywords)),
                    updated_at.eq(diesel::dsl::now),
                ))
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(upserted_count)
        }).await??;

        info!("Successfully upserted {} airports", result);
        Ok(result)
    }

    /// Get the total count of airports in the database
    pub async fn get_airport_count(&self) -> Result<i64> {
        use crate::schema::airports::dsl::*;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let count = airports.count().get_result::<i64>(&mut conn)?;
            Ok::<i64, anyhow::Error>(count)
        }).await??;

        Ok(result)
    }

    /// Get an airport by its ID
    pub async fn get_airport_by_id(&self, airport_id: i32) -> Result<Option<Airport>> {
        use crate::schema::airports::dsl::*;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let airport_model: Option<AirportModel> = airports
                .filter(id.eq(airport_id))
                .first::<AirportModel>(&mut conn)
                .optional()?;

            Ok::<Option<AirportModel>, anyhow::Error>(airport_model)
        }).await??;

        Ok(result.map(|model| model.into()))
    }

    /// Get an airport by its identifier (ICAO or local code)
    pub async fn get_airport_by_ident(&self, airport_ident: &str) -> Result<Option<Airport>> {
        use crate::schema::airports::dsl::*;

        let airport_ident = airport_ident.to_string();
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let airport_model: Option<AirportModel> = airports
                .filter(ident.eq(&airport_ident))
                .first::<AirportModel>(&mut conn)
                .optional()?;

            Ok::<Option<AirportModel>, anyhow::Error>(airport_model)
        }).await??;

        Ok(result.map(|model| model.into()))
    }

    /// Search airports by name (case-insensitive partial match)
    pub async fn search_by_name(&self, search_name: &str) -> Result<Vec<Airport>> {
        use crate::schema::airports::dsl::*;

        let search_pattern = format!("%{}%", search_name);
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let airport_models: Vec<AirportModel> = airports
                .filter(name.ilike(&search_pattern))
                .order((name, ident))
                .load::<AirportModel>(&mut conn)?;

            Ok::<Vec<AirportModel>, anyhow::Error>(airport_models)
        }).await??;

        Ok(result.into_iter().map(|model| model.into()).collect())
    }

    /// Search airports by country
    pub async fn search_by_country(&self, country_code: &str) -> Result<Vec<Airport>> {
        use crate::schema::airports::dsl::*;

        let country_code = country_code.to_string();
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let airport_models: Vec<AirportModel> = airports
                .filter(iso_country.eq(&country_code))
                .order((name, ident))
                .load::<AirportModel>(&mut conn)?;

            Ok::<Vec<AirportModel>, anyhow::Error>(airport_models)
        }).await??;

        Ok(result.into_iter().map(|model| model.into()).collect())
    }

    /// Search airports by type
    pub async fn search_by_type(&self, type_filter: &str) -> Result<Vec<Airport>> {
        use crate::schema::airports::dsl::*;

        let type_filter = type_filter.to_string();
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let airport_models: Vec<AirportModel> = airports
                .filter(type_.eq(&type_filter))
                .order((name, ident))
                .load::<AirportModel>(&mut conn)?;

            Ok::<Vec<AirportModel>, anyhow::Error>(airport_models)
        }).await??;

        Ok(result.into_iter().map(|model| model.into()).collect())
    }

    /// Find nearest airports to a given point using PostGIS
    /// Returns airports within the specified distance (in meters) ordered by distance
    pub async fn find_nearest_airports(
        &self,
        latitude: f64,
        longitude: f64,
        max_distance_meters: f64,
        limit: i64,
    ) -> Result<Vec<(Airport, f64)>> {
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Use raw SQL for PostGIS functions since Diesel doesn't have native support
            let sql = r#"
                SELECT id, ident, type as airport_type, name, latitude_deg, longitude_deg, elevation_ft,
                       continent, iso_country, iso_region, municipality, scheduled_service,
                       icao_code, iata_code, gps_code, local_code, home_link, wikipedia_link, keywords,
                       ST_Distance(location, ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography) as distance_meters
                FROM airports
                WHERE location IS NOT NULL
                  AND ST_DWithin(location, ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography, $3)
                ORDER BY location <-> ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography
                LIMIT $4
            "#;

            let results: Vec<AirportWithDistance> = diesel::sql_query(sql)
                .bind::<diesel::sql_types::Double, _>(latitude)
                .bind::<diesel::sql_types::Double, _>(longitude)
                .bind::<diesel::sql_types::Double, _>(max_distance_meters)
                .bind::<diesel::sql_types::BigInt, _>(limit)
                .load::<AirportWithDistance>(&mut conn)?;

            Ok::<Vec<AirportWithDistance>, anyhow::Error>(results)
        }).await??;

        let airports_with_distance: Vec<(Airport, f64)> = result
            .into_iter()
            .map(|awd| {
                let distance = awd.distance_meters.unwrap_or(0.0);
                let airport: Airport = awd.into();
                (airport, distance)
            })
            .collect();

        Ok(airports_with_distance)
    }

    /// Get airports with scheduled service only
    pub async fn get_scheduled_service_airports(&self) -> Result<Vec<Airport>> {
        use crate::schema::airports::dsl::*;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let airport_models: Vec<AirportModel> = airports
                .filter(scheduled_service.eq(true))
                .order((name, ident))
                .load::<AirportModel>(&mut conn)?;

            Ok::<Vec<AirportModel>, anyhow::Error>(airport_models)
        }).await??;

        Ok(result.into_iter().map(|model| model.into()).collect())
    }

    /// Fuzzy search airports by name, ICAO, IATA, or ident using trigram similarity
    /// Returns airports ordered by similarity score (best matches first)
    pub async fn fuzzy_search(&self, query: &str, limit: Option<i64>) -> Result<Vec<Airport>> {
        let query_upper = query.to_uppercase();
        let search_limit = limit.unwrap_or(20);

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Use raw SQL for trigram similarity functions
            let sql = r#"
                SELECT id, ident, type as airport_type, name, latitude_deg, longitude_deg, elevation_ft,
                       continent, iso_country, iso_region, municipality, scheduled_service,
                       icao_code, iata_code, gps_code, local_code, home_link, wikipedia_link, keywords,
                       GREATEST(
                           SIMILARITY(UPPER(name), $1),
                           COALESCE(SIMILARITY(UPPER(icao_code), $1), 0),
                           COALESCE(SIMILARITY(UPPER(iata_code), $1), 0),
                           SIMILARITY(UPPER(ident), $1)
                       ) as similarity_score
                FROM airports
                WHERE (
                    SIMILARITY(UPPER(name), $1) > 0.05 OR
                    COALESCE(SIMILARITY(UPPER(icao_code), $1), 0) > 0.05 OR
                    COALESCE(SIMILARITY(UPPER(iata_code), $1), 0) > 0.05 OR
                    SIMILARITY(UPPER(ident), $1) > 0.05
                )
                ORDER BY similarity_score DESC, name
                LIMIT $2
            "#;

            // Create a custom struct for this query result
            #[derive(QueryableByName, Debug)]
            struct AirportWithSimilarity {
                #[diesel(sql_type = diesel::sql_types::Integer)]
                id: i32,
                #[diesel(sql_type = diesel::sql_types::Varchar)]
                ident: String,
                #[diesel(sql_type = diesel::sql_types::Varchar)]
                airport_type: String,
                #[diesel(sql_type = diesel::sql_types::Varchar)]
                name: String,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float8>)]
                latitude_deg: Option<f64>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float8>)]
                longitude_deg: Option<f64>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
                elevation_ft: Option<i32>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
                continent: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
                iso_country: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
                iso_region: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
                municipality: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Bool)]
                scheduled_service: bool,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
                icao_code: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
                iata_code: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
                gps_code: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
                local_code: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                home_link: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                wikipedia_link: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                keywords: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float8>)]
                similarity_score: Option<f64>,
            }

            let results: Vec<AirportWithSimilarity> = diesel::sql_query(sql)
                .bind::<diesel::sql_types::Varchar, _>(&query_upper)
                .bind::<diesel::sql_types::BigInt, _>(search_limit)
                .load::<AirportWithSimilarity>(&mut conn)?;

            let airports: Vec<Airport> = results.into_iter().map(|aws| Airport {
                id: aws.id,
                ident: aws.ident,
                airport_type: aws.airport_type,
                name: aws.name,
                latitude_deg: aws.latitude_deg,
                longitude_deg: aws.longitude_deg,
                elevation_ft: aws.elevation_ft,
                continent: aws.continent,
                iso_country: aws.iso_country,
                iso_region: aws.iso_region,
                municipality: aws.municipality,
                scheduled_service: aws.scheduled_service,
                icao_code: aws.icao_code,
                iata_code: aws.iata_code,
                gps_code: aws.gps_code,
                local_code: aws.local_code,
                home_link: aws.home_link,
                wikipedia_link: aws.wikipedia_link,
                keywords: aws.keywords,
            }).collect();

            Ok::<Vec<Airport>, anyhow::Error>(airports)
        }).await??;

        Ok(result)
    }

    /// Search airports within a radius of a given point using PostGIS
    /// Returns airports within the specified radius (in kilometers)
    pub async fn search_nearby(
        &self,
        latitude: f64,
        longitude: f64,
        radius_km: f64,
        limit: Option<i64>,
    ) -> Result<Vec<Airport>> {
        let search_limit = limit.unwrap_or(20);
        let radius_m = radius_km * 1000.0; // Convert km to meters for PostGIS

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Use raw SQL for PostGIS functions
            let sql = r#"
                SELECT id, ident, type as airport_type, name, latitude_deg, longitude_deg, elevation_ft,
                       continent, iso_country, iso_region, municipality, scheduled_service,
                       icao_code, iata_code, gps_code, local_code, home_link, wikipedia_link, keywords,
                       ST_Distance(location, ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography) as distance_meters
                FROM airports
                WHERE location IS NOT NULL
                AND ST_DWithin(location, ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography, $3)
                ORDER BY distance_meters
                LIMIT $4
            "#;

            let results: Vec<AirportWithDistance> = diesel::sql_query(sql)
                .bind::<diesel::sql_types::Double, _>(latitude)
                .bind::<diesel::sql_types::Double, _>(longitude)
                .bind::<diesel::sql_types::Double, _>(radius_m)
                .bind::<diesel::sql_types::BigInt, _>(search_limit)
                .load::<AirportWithDistance>(&mut conn)?;

            let airports: Vec<Airport> = results.into_iter().map(|awd| awd.into()).collect();

            Ok::<Vec<Airport>, anyhow::Error>(airports)
        }).await??;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use crate::airports::Airport;

    // Note: These tests would require a test database setup
    // For now, they're just structural examples

    fn create_test_airport() -> Airport {
        Airport {
            id: 6523,
            ident: "00A".to_string(),
            airport_type: "heliport".to_string(),
            name: "Total RF Heliport".to_string(),
            latitude_deg: Some(40.070985),
            longitude_deg: Some(-74.933689),
            elevation_ft: Some(11),
            continent: Some("NA".to_string()),
            iso_country: Some("US".to_string()),
            iso_region: Some("US-PA".to_string()),
            municipality: Some("Bensalem".to_string()),
            scheduled_service: false,
            icao_code: None,
            iata_code: None,
            gps_code: Some("K00A".to_string()),
            local_code: Some("00A".to_string()),
            home_link: Some(
                "https://www.penndot.pa.gov/TravelInPA/airports-pa/Pages/Total-RF-Heliport.aspx"
                    .to_string(),
            ),
            wikipedia_link: None,
            keywords: None,
        }
    }

    #[test]
    fn test_airport_creation() {
        let airport = create_test_airport();
        assert_eq!(airport.id, 6523);
        assert_eq!(airport.ident, "00A");
        assert_eq!(airport.airport_type, "heliport");
        assert_eq!(airport.name, "Total RF Heliport");
        assert_eq!(airport.latitude_deg, Some(40.070985));
        assert_eq!(airport.longitude_deg, Some(-74.933689));
        assert_eq!(airport.elevation_ft, Some(11));
        assert_eq!(airport.iso_country, Some("US".to_string()));
        assert!(!airport.scheduled_service);
    }
}
