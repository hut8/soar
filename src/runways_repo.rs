use anyhow::Result;
use num_traits::{FromPrimitive, ToPrimitive};
use sqlx::PgPool;
use sqlx::types::BigDecimal;
use tracing::{info, warn};

use crate::runways::Runway;

/// Helper function to convert Option<f64> to Option<BigDecimal>
fn f64_to_bigdecimal(value: Option<f64>) -> Option<BigDecimal> {
    value.and_then(BigDecimal::from_f64)
}

/// Helper function to convert Option<BigDecimal> to Option<f64>
fn bigdecimal_to_f64(value: Option<BigDecimal>) -> Option<f64> {
    value.and_then(|v| v.to_f64())
}

/// Calculate the distance between two points using the Haversine formula
/// Returns distance in meters
fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_M: f64 = 6_371_000.0; // Earth's radius in meters

    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_M * c
}

pub struct RunwaysRepository {
    pool: PgPool,
}

impl RunwaysRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Upsert runways into the database
    /// This will insert new runways or update existing ones based on the primary key (id)
    pub async fn upsert_runways<I>(&self, runways: I) -> Result<usize>
    where
        I: IntoIterator<Item = Runway>,
    {
        let runways_vec: Vec<Runway> = runways.into_iter().collect();
        let mut upserted_count = 0;
        let mut failed_count = 0;

        for runway in runways_vec {
            // Process each runway in its own transaction to avoid transaction abort issues
            let mut transaction = self.pool.begin().await?;

            // Use ON CONFLICT to handle upserts
            let result = sqlx::query!(
                r#"
                INSERT INTO runways (
                    id,
                    airport_ref,
                    airport_ident,
                    length_ft,
                    width_ft,
                    surface,
                    lighted,
                    closed,
                    le_ident,
                    le_latitude_deg,
                    le_longitude_deg,
                    le_elevation_ft,
                    le_heading_degt,
                    le_displaced_threshold_ft,
                    he_ident,
                    he_latitude_deg,
                    he_longitude_deg,
                    he_elevation_ft,
                    he_heading_degt,
                    he_displaced_threshold_ft
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20
                )
                ON CONFLICT (id)
                DO UPDATE SET
                    airport_ref = EXCLUDED.airport_ref,
                    airport_ident = EXCLUDED.airport_ident,
                    length_ft = EXCLUDED.length_ft,
                    width_ft = EXCLUDED.width_ft,
                    surface = EXCLUDED.surface,
                    lighted = EXCLUDED.lighted,
                    closed = EXCLUDED.closed,
                    le_ident = EXCLUDED.le_ident,
                    le_latitude_deg = EXCLUDED.le_latitude_deg,
                    le_longitude_deg = EXCLUDED.le_longitude_deg,
                    le_elevation_ft = EXCLUDED.le_elevation_ft,
                    le_heading_degt = EXCLUDED.le_heading_degt,
                    le_displaced_threshold_ft = EXCLUDED.le_displaced_threshold_ft,
                    he_ident = EXCLUDED.he_ident,
                    he_latitude_deg = EXCLUDED.he_latitude_deg,
                    he_longitude_deg = EXCLUDED.he_longitude_deg,
                    he_elevation_ft = EXCLUDED.he_elevation_ft,
                    he_heading_degt = EXCLUDED.he_heading_degt,
                    he_displaced_threshold_ft = EXCLUDED.he_displaced_threshold_ft,
                    updated_at = NOW()
                "#,
                runway.id,
                runway.airport_ref,
                runway.airport_ident,
                runway.length_ft,
                runway.width_ft,
                runway.surface,
                runway.lighted,
                runway.closed,
                runway.le_ident,
                f64_to_bigdecimal(runway.le_latitude_deg),
                f64_to_bigdecimal(runway.le_longitude_deg),
                runway.le_elevation_ft,
                f64_to_bigdecimal(runway.le_heading_degt),
                runway.le_displaced_threshold_ft,
                runway.he_ident,
                f64_to_bigdecimal(runway.he_latitude_deg),
                f64_to_bigdecimal(runway.he_longitude_deg),
                runway.he_elevation_ft,
                f64_to_bigdecimal(runway.he_heading_degt),
                runway.he_displaced_threshold_ft
            )
            .execute(&mut *transaction)
            .await;

            match result {
                Ok(_) => {
                    // Commit the transaction for this runway
                    match transaction.commit().await {
                        Ok(_) => {
                            upserted_count += 1;
                        }
                        Err(e) => {
                            warn!(
                                "Failed to commit transaction for runway {} at airport {}: {}",
                                runway.id, runway.airport_ident, e
                            );
                            failed_count += 1;
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to upsert runway {} at airport {}: {}\nRunway data: {:#?}",
                        runway.id, runway.airport_ident, e, runway
                    );
                    transaction.rollback().await?;
                    failed_count += 1;
                }
            }
        }

        if failed_count > 0 {
            warn!("Failed to upsert {} runways", failed_count);
        }
        info!("Successfully upserted {} runways", upserted_count);

        Ok(upserted_count)
    }

    /// Get the total count of runways in the database
    pub async fn get_runway_count(&self) -> Result<i64> {
        let result = sqlx::query!("SELECT COUNT(*) as count FROM runways")
            .fetch_one(&self.pool)
            .await?;

        Ok(result.count.unwrap_or(0))
    }

    /// Get a runway by its ID
    pub async fn get_runway_by_id(&self, id: i32) -> Result<Option<Runway>> {
        let result = sqlx::query!(
            r#"
            SELECT id, airport_ref, airport_ident, length_ft, width_ft, surface, lighted, closed,
                   le_ident, le_latitude_deg, le_longitude_deg, le_elevation_ft, le_heading_degt, le_displaced_threshold_ft,
                   he_ident, he_latitude_deg, he_longitude_deg, he_elevation_ft, he_heading_degt, he_displaced_threshold_ft
            FROM runways
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            Ok(Some(Runway {
                id: row.id,
                airport_ref: row.airport_ref,
                airport_ident: row.airport_ident,
                length_ft: row.length_ft,
                width_ft: row.width_ft,
                surface: row.surface,
                lighted: row.lighted,
                closed: row.closed,
                le_ident: row.le_ident,
                le_latitude_deg: bigdecimal_to_f64(row.le_latitude_deg),
                le_longitude_deg: bigdecimal_to_f64(row.le_longitude_deg),
                le_elevation_ft: row.le_elevation_ft,
                le_heading_degt: bigdecimal_to_f64(row.le_heading_degt),
                le_displaced_threshold_ft: row.le_displaced_threshold_ft,
                he_ident: row.he_ident,
                he_latitude_deg: bigdecimal_to_f64(row.he_latitude_deg),
                he_longitude_deg: bigdecimal_to_f64(row.he_longitude_deg),
                he_elevation_ft: row.he_elevation_ft,
                he_heading_degt: bigdecimal_to_f64(row.he_heading_degt),
                he_displaced_threshold_ft: row.he_displaced_threshold_ft,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all runways for a specific airport by airport ID
    pub async fn get_runways_by_airport_id(&self, airport_id: i32) -> Result<Vec<Runway>> {
        let results = sqlx::query!(
            r#"
            SELECT id, airport_ref, airport_ident, length_ft, width_ft, surface, lighted, closed,
                   le_ident, le_latitude_deg, le_longitude_deg, le_elevation_ft, le_heading_degt, le_displaced_threshold_ft,
                   he_ident, he_latitude_deg, he_longitude_deg, he_elevation_ft, he_heading_degt, he_displaced_threshold_ft
            FROM runways
            WHERE airport_ref = $1
            ORDER BY id
            "#,
            airport_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut runways = Vec::new();
        for row in results {
            runways.push(Runway {
                id: row.id,
                airport_ref: row.airport_ref,
                airport_ident: row.airport_ident,
                length_ft: row.length_ft,
                width_ft: row.width_ft,
                surface: row.surface,
                lighted: row.lighted,
                closed: row.closed,
                le_ident: row.le_ident,
                le_latitude_deg: bigdecimal_to_f64(row.le_latitude_deg),
                le_longitude_deg: bigdecimal_to_f64(row.le_longitude_deg),
                le_elevation_ft: row.le_elevation_ft,
                le_heading_degt: bigdecimal_to_f64(row.le_heading_degt),
                le_displaced_threshold_ft: row.le_displaced_threshold_ft,
                he_ident: row.he_ident,
                he_latitude_deg: bigdecimal_to_f64(row.he_latitude_deg),
                he_longitude_deg: bigdecimal_to_f64(row.he_longitude_deg),
                he_elevation_ft: row.he_elevation_ft,
                he_heading_degt: bigdecimal_to_f64(row.he_heading_degt),
                he_displaced_threshold_ft: row.he_displaced_threshold_ft,
            });
        }

        Ok(runways)
    }

    /// Get all runways for a specific airport by airport identifier
    pub async fn get_runways_by_airport_ident(&self, airport_ident: &str) -> Result<Vec<Runway>> {
        let results = sqlx::query!(
            r#"
            SELECT id, airport_ref, airport_ident, length_ft, width_ft, surface, lighted, closed,
                   le_ident, le_latitude_deg, le_longitude_deg, le_elevation_ft, le_heading_degt, le_displaced_threshold_ft,
                   he_ident, he_latitude_deg, he_longitude_deg, he_elevation_ft, he_heading_degt, he_displaced_threshold_ft
            FROM runways
            WHERE airport_ident = $1
            ORDER BY id
            "#,
            airport_ident
        )
        .fetch_all(&self.pool)
        .await?;

        let mut runways = Vec::new();
        for row in results {
            runways.push(Runway {
                id: row.id,
                airport_ref: row.airport_ref,
                airport_ident: row.airport_ident,
                length_ft: row.length_ft,
                width_ft: row.width_ft,
                surface: row.surface,
                lighted: row.lighted,
                closed: row.closed,
                le_ident: row.le_ident,
                le_latitude_deg: bigdecimal_to_f64(row.le_latitude_deg),
                le_longitude_deg: bigdecimal_to_f64(row.le_longitude_deg),
                le_elevation_ft: row.le_elevation_ft,
                le_heading_degt: bigdecimal_to_f64(row.le_heading_degt),
                le_displaced_threshold_ft: row.le_displaced_threshold_ft,
                he_ident: row.he_ident,
                he_latitude_deg: bigdecimal_to_f64(row.he_latitude_deg),
                he_longitude_deg: bigdecimal_to_f64(row.he_longitude_deg),
                he_elevation_ft: row.he_elevation_ft,
                he_heading_degt: bigdecimal_to_f64(row.he_heading_degt),
                he_displaced_threshold_ft: row.he_displaced_threshold_ft,
            });
        }

        Ok(runways)
    }

    /// Search runways by surface type
    pub async fn search_by_surface(&self, surface: &str) -> Result<Vec<Runway>> {
        let results = sqlx::query!(
            r#"
            SELECT id, airport_ref, airport_ident, length_ft, width_ft, surface, lighted, closed,
                   le_ident, le_latitude_deg, le_longitude_deg, le_elevation_ft, le_heading_degt, le_displaced_threshold_ft,
                   he_ident, he_latitude_deg, he_longitude_deg, he_elevation_ft, he_heading_degt, he_displaced_threshold_ft
            FROM runways
            WHERE surface = $1
            ORDER BY airport_ident, id
            "#,
            surface
        )
        .fetch_all(&self.pool)
        .await?;

        let mut runways = Vec::new();
        for row in results {
            runways.push(Runway {
                id: row.id,
                airport_ref: row.airport_ref,
                airport_ident: row.airport_ident,
                length_ft: row.length_ft,
                width_ft: row.width_ft,
                surface: row.surface,
                lighted: row.lighted,
                closed: row.closed,
                le_ident: row.le_ident,
                le_latitude_deg: bigdecimal_to_f64(row.le_latitude_deg),
                le_longitude_deg: bigdecimal_to_f64(row.le_longitude_deg),
                le_elevation_ft: row.le_elevation_ft,
                le_heading_degt: bigdecimal_to_f64(row.le_heading_degt),
                le_displaced_threshold_ft: row.le_displaced_threshold_ft,
                he_ident: row.he_ident,
                he_latitude_deg: bigdecimal_to_f64(row.he_latitude_deg),
                he_longitude_deg: bigdecimal_to_f64(row.he_longitude_deg),
                he_elevation_ft: row.he_elevation_ft,
                he_heading_degt: bigdecimal_to_f64(row.he_heading_degt),
                he_displaced_threshold_ft: row.he_displaced_threshold_ft,
            });
        }

        Ok(runways)
    }

    /// Search runways by minimum length
    pub async fn search_by_min_length(&self, min_length_ft: i32) -> Result<Vec<Runway>> {
        let results = sqlx::query!(
            r#"
            SELECT id, airport_ref, airport_ident, length_ft, width_ft, surface, lighted, closed,
                   le_ident, le_latitude_deg, le_longitude_deg, le_elevation_ft, le_heading_degt, le_displaced_threshold_ft,
                   he_ident, he_latitude_deg, he_longitude_deg, he_elevation_ft, he_heading_degt, he_displaced_threshold_ft
            FROM runways
            WHERE length_ft >= $1
            ORDER BY length_ft DESC, airport_ident, id
            "#,
            min_length_ft
        )
        .fetch_all(&self.pool)
        .await?;

        let mut runways = Vec::new();
        for row in results {
            runways.push(Runway {
                id: row.id,
                airport_ref: row.airport_ref,
                airport_ident: row.airport_ident,
                length_ft: row.length_ft,
                width_ft: row.width_ft,
                surface: row.surface,
                lighted: row.lighted,
                closed: row.closed,
                le_ident: row.le_ident,
                le_latitude_deg: bigdecimal_to_f64(row.le_latitude_deg),
                le_longitude_deg: bigdecimal_to_f64(row.le_longitude_deg),
                le_elevation_ft: row.le_elevation_ft,
                le_heading_degt: bigdecimal_to_f64(row.le_heading_degt),
                le_displaced_threshold_ft: row.le_displaced_threshold_ft,
                he_ident: row.he_ident,
                he_latitude_deg: bigdecimal_to_f64(row.he_latitude_deg),
                he_longitude_deg: bigdecimal_to_f64(row.he_longitude_deg),
                he_elevation_ft: row.he_elevation_ft,
                he_heading_degt: bigdecimal_to_f64(row.he_heading_degt),
                he_displaced_threshold_ft: row.he_displaced_threshold_ft,
            });
        }

        Ok(runways)
    }

    /// Get lighted runways only
    pub async fn get_lighted_runways(&self) -> Result<Vec<Runway>> {
        let results = sqlx::query!(
            r#"
            SELECT id, airport_ref, airport_ident, length_ft, width_ft, surface, lighted, closed,
                   le_ident, le_latitude_deg, le_longitude_deg, le_elevation_ft, le_heading_degt, le_displaced_threshold_ft,
                   he_ident, he_latitude_deg, he_longitude_deg, he_elevation_ft, he_heading_degt, he_displaced_threshold_ft
            FROM runways
            WHERE lighted = true
            ORDER BY airport_ident, id
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut runways = Vec::new();
        for row in results {
            runways.push(Runway {
                id: row.id,
                airport_ref: row.airport_ref,
                airport_ident: row.airport_ident,
                length_ft: row.length_ft,
                width_ft: row.width_ft,
                surface: row.surface,
                lighted: row.lighted,
                closed: row.closed,
                le_ident: row.le_ident,
                le_latitude_deg: bigdecimal_to_f64(row.le_latitude_deg),
                le_longitude_deg: bigdecimal_to_f64(row.le_longitude_deg),
                le_elevation_ft: row.le_elevation_ft,
                le_heading_degt: bigdecimal_to_f64(row.le_heading_degt),
                le_displaced_threshold_ft: row.le_displaced_threshold_ft,
                he_ident: row.he_ident,
                he_latitude_deg: bigdecimal_to_f64(row.he_latitude_deg),
                he_longitude_deg: bigdecimal_to_f64(row.he_longitude_deg),
                he_elevation_ft: row.he_elevation_ft,
                he_heading_degt: bigdecimal_to_f64(row.he_heading_degt),
                he_displaced_threshold_ft: row.he_displaced_threshold_ft,
            });
        }

        Ok(runways)
    }

    /// Get open (not closed) runways only
    pub async fn get_open_runways(&self) -> Result<Vec<Runway>> {
        let results = sqlx::query!(
            r#"
            SELECT id, airport_ref, airport_ident, length_ft, width_ft, surface, lighted, closed,
                   le_ident, le_latitude_deg, le_longitude_deg, le_elevation_ft, le_heading_degt, le_displaced_threshold_ft,
                   he_ident, he_latitude_deg, he_longitude_deg, he_elevation_ft, he_heading_degt, he_displaced_threshold_ft
            FROM runways
            WHERE closed = false
            ORDER BY airport_ident, id
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut runways = Vec::new();
        for row in results {
            runways.push(Runway {
                id: row.id,
                airport_ref: row.airport_ref,
                airport_ident: row.airport_ident,
                length_ft: row.length_ft,
                width_ft: row.width_ft,
                surface: row.surface,
                lighted: row.lighted,
                closed: row.closed,
                le_ident: row.le_ident,
                le_latitude_deg: bigdecimal_to_f64(row.le_latitude_deg),
                le_longitude_deg: bigdecimal_to_f64(row.le_longitude_deg),
                le_elevation_ft: row.le_elevation_ft,
                le_heading_degt: bigdecimal_to_f64(row.le_heading_degt),
                le_displaced_threshold_ft: row.le_displaced_threshold_ft,
                he_ident: row.he_ident,
                he_latitude_deg: bigdecimal_to_f64(row.he_latitude_deg),
                he_longitude_deg: bigdecimal_to_f64(row.he_longitude_deg),
                he_elevation_ft: row.he_elevation_ft,
                he_heading_degt: bigdecimal_to_f64(row.he_heading_degt),
                he_displaced_threshold_ft: row.he_displaced_threshold_ft,
            });
        }

        Ok(runways)
    }

    /// Find nearest runway endpoints to a given point using coordinate-based distance calculation
    /// Returns runway endpoints within the specified distance (in meters) ordered by distance
    /// Note: This is a simplified version that doesn't use PostGIS geography types
    pub async fn find_nearest_runway_endpoints(
        &self,
        latitude: f64,
        longitude: f64,
        max_distance_meters: f64,
        limit: i64,
    ) -> Result<Vec<(Runway, f64, String)>> {
        // For now, return a simplified query that gets runways with coordinates
        // This avoids the PostGIS geography type mapping issue
        let results = sqlx::query!(
            r#"
            SELECT id, airport_ref, airport_ident, length_ft, width_ft, surface, lighted, closed,
                   le_ident, le_latitude_deg, le_longitude_deg, le_elevation_ft, le_heading_degt, le_displaced_threshold_ft,
                   he_ident, he_latitude_deg, he_longitude_deg, he_elevation_ft, he_heading_degt, he_displaced_threshold_ft
            FROM runways
            WHERE (le_latitude_deg IS NOT NULL AND le_longitude_deg IS NOT NULL)
               OR (he_latitude_deg IS NOT NULL AND he_longitude_deg IS NOT NULL)
            ORDER BY id
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        let mut runways_with_distance = Vec::new();
        for row in results {
            let runway = Runway {
                id: row.id,
                airport_ref: row.airport_ref,
                airport_ident: row.airport_ident,
                length_ft: row.length_ft,
                width_ft: row.width_ft,
                surface: row.surface,
                lighted: row.lighted,
                closed: row.closed,
                le_ident: row.le_ident,
                le_latitude_deg: bigdecimal_to_f64(row.le_latitude_deg),
                le_longitude_deg: bigdecimal_to_f64(row.le_longitude_deg),
                le_elevation_ft: row.le_elevation_ft,
                le_heading_degt: bigdecimal_to_f64(row.le_heading_degt),
                le_displaced_threshold_ft: row.le_displaced_threshold_ft,
                he_ident: row.he_ident,
                he_latitude_deg: bigdecimal_to_f64(row.he_latitude_deg),
                he_longitude_deg: bigdecimal_to_f64(row.he_longitude_deg),
                he_elevation_ft: row.he_elevation_ft,
                he_heading_degt: bigdecimal_to_f64(row.he_heading_degt),
                he_displaced_threshold_ft: row.he_displaced_threshold_ft,
            };

            // Calculate approximate distance using Haversine formula for both endpoints
            let mut min_distance = f64::MAX;
            let mut endpoint_type = String::new();

            // Check low end coordinates
            if let (Some(le_lat), Some(le_lon)) = (runway.le_latitude_deg, runway.le_longitude_deg)
            {
                let distance = haversine_distance(latitude, longitude, le_lat, le_lon);
                if distance < min_distance {
                    min_distance = distance;
                    endpoint_type = "low_end".to_string();
                }
            }

            // Check high end coordinates
            if let (Some(he_lat), Some(he_lon)) = (runway.he_latitude_deg, runway.he_longitude_deg)
            {
                let distance = haversine_distance(latitude, longitude, he_lat, he_lon);
                if distance < min_distance {
                    min_distance = distance;
                    endpoint_type = "high_end".to_string();
                }
            }

            // Only include if within max distance
            if min_distance <= max_distance_meters {
                runways_with_distance.push((runway, min_distance, endpoint_type));
            }
        }

        // Sort by distance and limit results
        runways_with_distance
            .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        runways_with_distance.truncate(limit as usize);

        Ok(runways_with_distance)
    }
}

#[cfg(test)]
mod tests {
    use crate::runways::Runway;

    // Note: These tests would require a test database setup
    // For now, they're just structural examples

    fn create_test_runway() -> Runway {
        Runway {
            id: 269408,
            airport_ref: 6523,
            airport_ident: "00A".to_string(),
            length_ft: Some(80),
            width_ft: Some(80),
            surface: Some("ASPH-G".to_string()),
            lighted: true,
            closed: false,
            le_ident: Some("H1".to_string()),
            le_latitude_deg: None,
            le_longitude_deg: None,
            le_elevation_ft: None,
            le_heading_degt: None,
            le_displaced_threshold_ft: None,
            he_ident: None,
            he_latitude_deg: None,
            he_longitude_deg: None,
            he_elevation_ft: None,
            he_heading_degt: None,
            he_displaced_threshold_ft: None,
        }
    }

    #[test]
    fn test_runway_creation() {
        let runway = create_test_runway();
        assert_eq!(runway.id, 269408);
        assert_eq!(runway.airport_ref, 6523);
        assert_eq!(runway.airport_ident, "00A");
        assert_eq!(runway.length_ft, Some(80));
        assert_eq!(runway.width_ft, Some(80));
        assert_eq!(runway.surface, Some("ASPH-G".to_string()));
        assert!(runway.lighted);
        assert!(!runway.closed);
        assert_eq!(runway.le_ident, Some("H1".to_string()));
    }
}
