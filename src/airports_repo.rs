use anyhow::Result;
use num_traits::{FromPrimitive, ToPrimitive};
use sqlx::PgPool;
use sqlx::types::BigDecimal;
use tracing::{info, warn};

use crate::airports::Airport;

/// Helper function to convert Option<f64> to Option<BigDecimal>
fn f64_to_bigdecimal(value: Option<f64>) -> Option<BigDecimal> {
    value.and_then(BigDecimal::from_f64)
}

/// Helper function to convert Option<BigDecimal> to Option<f64>
fn bigdecimal_to_f64(value: Option<BigDecimal>) -> Option<f64> {
    value.and_then(|v| v.to_f64())
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
    pub async fn upsert_airports<I>(&self, airports: I) -> Result<usize>
    where
        I: IntoIterator<Item = Airport>,
    {
        let airports_vec: Vec<Airport> = airports.into_iter().collect();
        let mut upserted_count = 0;
        let mut failed_count = 0;

        for airport in airports_vec {
            // Process each airport in its own transaction to avoid transaction abort issues
            let mut transaction = self.pool.begin().await?;

            // Use ON CONFLICT to handle upserts
            let result = sqlx::query!(
                r#"
                INSERT INTO airports (
                    id,
                    ident,
                    type,
                    name,
                    latitude_deg,
                    longitude_deg,
                    elevation_ft,
                    continent,
                    iso_country,
                    iso_region,
                    municipality,
                    scheduled_service,
                    icao_code,
                    iata_code,
                    gps_code,
                    local_code,
                    home_link,
                    wikipedia_link,
                    keywords
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19
                )
                ON CONFLICT (id)
                DO UPDATE SET
                    ident = EXCLUDED.ident,
                    type = EXCLUDED.type,
                    name = EXCLUDED.name,
                    latitude_deg = EXCLUDED.latitude_deg,
                    longitude_deg = EXCLUDED.longitude_deg,
                    elevation_ft = EXCLUDED.elevation_ft,
                    continent = EXCLUDED.continent,
                    iso_country = EXCLUDED.iso_country,
                    iso_region = EXCLUDED.iso_region,
                    municipality = EXCLUDED.municipality,
                    scheduled_service = EXCLUDED.scheduled_service,
                    icao_code = EXCLUDED.icao_code,
                    iata_code = EXCLUDED.iata_code,
                    gps_code = EXCLUDED.gps_code,
                    local_code = EXCLUDED.local_code,
                    home_link = EXCLUDED.home_link,
                    wikipedia_link = EXCLUDED.wikipedia_link,
                    keywords = EXCLUDED.keywords,
                    updated_at = NOW()
                "#,
                airport.id,
                airport.ident,
                airport.airport_type,
                airport.name,
                f64_to_bigdecimal(airport.latitude_deg),
                f64_to_bigdecimal(airport.longitude_deg),
                airport.elevation_ft,
                airport.continent,
                airport.iso_country,
                airport.iso_region,
                airport.municipality,
                airport.scheduled_service,
                airport.icao_code,
                airport.iata_code,
                airport.gps_code,
                airport.local_code,
                airport.home_link,
                airport.wikipedia_link,
                airport.keywords
            )
            .execute(&mut *transaction)
            .await;

            match result {
                Ok(_) => {
                    // Commit the transaction for this airport
                    match transaction.commit().await {
                        Ok(_) => {
                            upserted_count += 1;
                        }
                        Err(e) => {
                            warn!(
                                "Failed to commit transaction for airport {}: {}",
                                airport.ident, e
                            );
                            failed_count += 1;
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to upsert airport {}: {}\nAirport data: {:#?}",
                        airport.ident, e, airport
                    );
                    transaction.rollback().await?;
                    failed_count += 1;
                }
            }
        }

        if failed_count > 0 {
            warn!("Failed to upsert {} airports", failed_count);
        }
        info!("Successfully upserted {} airports", upserted_count);

        Ok(upserted_count)
    }

    /// Get the total count of airports in the database
    pub async fn get_airport_count(&self) -> Result<i64> {
        let result = sqlx::query!("SELECT COUNT(*) as count FROM airports")
            .fetch_one(&self.pool)
            .await?;

        Ok(result.count.unwrap_or(0))
    }

    /// Get an airport by its ID
    pub async fn get_airport_by_id(&self, id: i32) -> Result<Option<Airport>> {
        let result = sqlx::query!(
            r#"
            SELECT id, ident, type, name, latitude_deg, longitude_deg, elevation_ft,
                   continent, iso_country, iso_region, municipality, scheduled_service,
                   icao_code, iata_code, gps_code, local_code, home_link, wikipedia_link, keywords
            FROM airports
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            Ok(Some(Airport {
                id: row.id,
                ident: row.ident,
                airport_type: row.r#type,
                name: row.name,
                latitude_deg: bigdecimal_to_f64(row.latitude_deg),
                longitude_deg: bigdecimal_to_f64(row.longitude_deg),
                elevation_ft: row.elevation_ft,
                continent: row.continent,
                iso_country: row.iso_country,
                iso_region: row.iso_region,
                municipality: row.municipality,
                scheduled_service: row.scheduled_service,
                icao_code: row.icao_code,
                iata_code: row.iata_code,
                gps_code: row.gps_code,
                local_code: row.local_code,
                home_link: row.home_link,
                wikipedia_link: row.wikipedia_link,
                keywords: row.keywords,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get an airport by its identifier (ICAO or local code)
    pub async fn get_airport_by_ident(&self, ident: &str) -> Result<Option<Airport>> {
        let result = sqlx::query!(
            r#"
            SELECT id, ident, type, name, latitude_deg, longitude_deg, elevation_ft,
                   continent, iso_country, iso_region, municipality, scheduled_service,
                   icao_code, iata_code, gps_code, local_code, home_link, wikipedia_link, keywords
            FROM airports
            WHERE ident = $1
            "#,
            ident
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            Ok(Some(Airport {
                id: row.id,
                ident: row.ident,
                airport_type: row.r#type,
                name: row.name,
                latitude_deg: bigdecimal_to_f64(row.latitude_deg),
                longitude_deg: bigdecimal_to_f64(row.longitude_deg),
                elevation_ft: row.elevation_ft,
                continent: row.continent,
                iso_country: row.iso_country,
                iso_region: row.iso_region,
                municipality: row.municipality,
                scheduled_service: row.scheduled_service,
                icao_code: row.icao_code,
                iata_code: row.iata_code,
                gps_code: row.gps_code,
                local_code: row.local_code,
                home_link: row.home_link,
                wikipedia_link: row.wikipedia_link,
                keywords: row.keywords,
            }))
        } else {
            Ok(None)
        }
    }

    /// Search airports by name (case-insensitive partial match)
    pub async fn search_by_name(&self, name: &str) -> Result<Vec<Airport>> {
        let results = sqlx::query!(
            r#"
            SELECT id, ident, type, name, latitude_deg, longitude_deg, elevation_ft,
                   continent, iso_country, iso_region, municipality, scheduled_service,
                   icao_code, iata_code, gps_code, local_code, home_link, wikipedia_link, keywords
            FROM airports
            WHERE name ILIKE $1
            ORDER BY name, ident
            "#,
            format!("%{}%", name)
        )
        .fetch_all(&self.pool)
        .await?;

        let mut airports = Vec::new();
        for row in results {
            airports.push(Airport {
                id: row.id,
                ident: row.ident,
                airport_type: row.r#type,
                name: row.name,
                latitude_deg: bigdecimal_to_f64(row.latitude_deg),
                longitude_deg: bigdecimal_to_f64(row.longitude_deg),
                elevation_ft: row.elevation_ft,
                continent: row.continent,
                iso_country: row.iso_country,
                iso_region: row.iso_region,
                municipality: row.municipality,
                scheduled_service: row.scheduled_service,
                icao_code: row.icao_code,
                iata_code: row.iata_code,
                gps_code: row.gps_code,
                local_code: row.local_code,
                home_link: row.home_link,
                wikipedia_link: row.wikipedia_link,
                keywords: row.keywords,
            });
        }

        Ok(airports)
    }

    /// Search airports by country
    pub async fn search_by_country(&self, iso_country: &str) -> Result<Vec<Airport>> {
        let results = sqlx::query!(
            r#"
            SELECT id, ident, type, name, latitude_deg, longitude_deg, elevation_ft,
                   continent, iso_country, iso_region, municipality, scheduled_service,
                   icao_code, iata_code, gps_code, local_code, home_link, wikipedia_link, keywords
            FROM airports
            WHERE iso_country = $1
            ORDER BY name, ident
            "#,
            iso_country
        )
        .fetch_all(&self.pool)
        .await?;

        let mut airports = Vec::new();
        for row in results {
            airports.push(Airport {
                id: row.id,
                ident: row.ident,
                airport_type: row.r#type,
                name: row.name,
                latitude_deg: bigdecimal_to_f64(row.latitude_deg),
                longitude_deg: bigdecimal_to_f64(row.longitude_deg),
                elevation_ft: row.elevation_ft,
                continent: row.continent,
                iso_country: row.iso_country,
                iso_region: row.iso_region,
                municipality: row.municipality,
                scheduled_service: row.scheduled_service,
                icao_code: row.icao_code,
                iata_code: row.iata_code,
                gps_code: row.gps_code,
                local_code: row.local_code,
                home_link: row.home_link,
                wikipedia_link: row.wikipedia_link,
                keywords: row.keywords,
            });
        }

        Ok(airports)
    }

    /// Search airports by type
    pub async fn search_by_type(&self, airport_type: &str) -> Result<Vec<Airport>> {
        let results = sqlx::query!(
            r#"
            SELECT id, ident, type, name, latitude_deg, longitude_deg, elevation_ft,
                   continent, iso_country, iso_region, municipality, scheduled_service,
                   icao_code, iata_code, gps_code, local_code, home_link, wikipedia_link, keywords
            FROM airports
            WHERE type = $1
            ORDER BY name, ident
            "#,
            airport_type
        )
        .fetch_all(&self.pool)
        .await?;

        let mut airports = Vec::new();
        for row in results {
            airports.push(Airport {
                id: row.id,
                ident: row.ident,
                airport_type: row.r#type,
                name: row.name,
                latitude_deg: bigdecimal_to_f64(row.latitude_deg),
                longitude_deg: bigdecimal_to_f64(row.longitude_deg),
                elevation_ft: row.elevation_ft,
                continent: row.continent,
                iso_country: row.iso_country,
                iso_region: row.iso_region,
                municipality: row.municipality,
                scheduled_service: row.scheduled_service,
                icao_code: row.icao_code,
                iata_code: row.iata_code,
                gps_code: row.gps_code,
                local_code: row.local_code,
                home_link: row.home_link,
                wikipedia_link: row.wikipedia_link,
                keywords: row.keywords,
            });
        }

        Ok(airports)
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
        let results = sqlx::query!(
            r#"
            SELECT id, ident, type, name, latitude_deg, longitude_deg, elevation_ft,
                   continent, iso_country, iso_region, municipality, scheduled_service,
                   icao_code, iata_code, gps_code, local_code, home_link, wikipedia_link, keywords,
                   ST_Distance(location, ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography) as distance_meters
            FROM airports
            WHERE location IS NOT NULL
              AND ST_DWithin(location, ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography, $3)
            ORDER BY location <-> ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography
            LIMIT $4
            "#,
            latitude,
            longitude,
            max_distance_meters,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        let mut airports_with_distance = Vec::new();
        for row in results {
            let airport = Airport {
                id: row.id,
                ident: row.ident,
                airport_type: row.r#type,
                name: row.name,
                latitude_deg: bigdecimal_to_f64(row.latitude_deg),
                longitude_deg: bigdecimal_to_f64(row.longitude_deg),
                elevation_ft: row.elevation_ft,
                continent: row.continent,
                iso_country: row.iso_country,
                iso_region: row.iso_region,
                municipality: row.municipality,
                scheduled_service: row.scheduled_service,
                icao_code: row.icao_code,
                iata_code: row.iata_code,
                gps_code: row.gps_code,
                local_code: row.local_code,
                home_link: row.home_link,
                wikipedia_link: row.wikipedia_link,
                keywords: row.keywords,
            };
            let distance = row.distance_meters.unwrap_or(0.0);
            airports_with_distance.push((airport, distance));
        }

        Ok(airports_with_distance)
    }

    /// Get airports with scheduled service only
    pub async fn get_scheduled_service_airports(&self) -> Result<Vec<Airport>> {
        let results = sqlx::query!(
            r#"
            SELECT id, ident, type, name, latitude_deg, longitude_deg, elevation_ft,
                   continent, iso_country, iso_region, municipality, scheduled_service,
                   icao_code, iata_code, gps_code, local_code, home_link, wikipedia_link, keywords
            FROM airports
            WHERE scheduled_service = true
            ORDER BY name, ident
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut airports = Vec::new();
        for row in results {
            airports.push(Airport {
                id: row.id,
                ident: row.ident,
                airport_type: row.r#type,
                name: row.name,
                latitude_deg: bigdecimal_to_f64(row.latitude_deg),
                longitude_deg: bigdecimal_to_f64(row.longitude_deg),
                elevation_ft: row.elevation_ft,
                continent: row.continent,
                iso_country: row.iso_country,
                iso_region: row.iso_region,
                municipality: row.municipality,
                scheduled_service: row.scheduled_service,
                icao_code: row.icao_code,
                iata_code: row.iata_code,
                gps_code: row.gps_code,
                local_code: row.local_code,
                home_link: row.home_link,
                wikipedia_link: row.wikipedia_link,
                keywords: row.keywords,
            });
        }

        Ok(airports)
    }

    /// Fuzzy search airports by name, ICAO, IATA, or ident using trigram similarity
    /// Returns airports ordered by similarity score (best matches first)
    pub async fn fuzzy_search(&self, query: &str, limit: Option<i64>) -> Result<Vec<Airport>> {
        let limit = limit.unwrap_or(20);
        let query_upper = query.to_uppercase();
        
        let results = sqlx::query!(
            r#"
            SELECT id, ident, type, name, latitude_deg, longitude_deg, elevation_ft,
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
            "#,
            query_upper,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        let mut airports = Vec::new();
        for row in results {
            airports.push(Airport {
                id: row.id,
                ident: row.ident,
                airport_type: row.r#type,
                name: row.name,
                latitude_deg: bigdecimal_to_f64(row.latitude_deg),
                longitude_deg: bigdecimal_to_f64(row.longitude_deg),
                elevation_ft: row.elevation_ft,
                continent: row.continent,
                iso_country: row.iso_country,
                iso_region: row.iso_region,
                municipality: row.municipality,
                scheduled_service: row.scheduled_service,
                icao_code: row.icao_code,
                iata_code: row.iata_code,
                gps_code: row.gps_code,
                local_code: row.local_code,
                home_link: row.home_link,
                wikipedia_link: row.wikipedia_link,
                keywords: row.keywords,
            });
        }

        Ok(airports)
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
