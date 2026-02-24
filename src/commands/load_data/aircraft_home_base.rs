use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use std::time::Instant;
use tracing::{error, info};

use soar::email_reporter::EntityMetrics;

type PgPool = Pool<ConnectionManager<PgConnection>>;

/// Calculate aircraft home bases using a two-step approach:
/// 1. Copy home base from club if aircraft belongs to a club with a home base
/// 2. Calculate from flight statistics for remaining aircraft (minimum 3 flights)
pub async fn calculate_aircraft_home_bases_with_metrics(pool: PgPool) -> EntityMetrics {
    let start = Instant::now();
    let mut metrics = EntityMetrics::new("Calculate aircraft home bases");

    info!("Calculating aircraft home bases...");

    match calculate_aircraft_home_bases(&pool).await {
        Ok((club_copied, flight_calculated)) => {
            info!(
                "Successfully set home bases: {} from clubs, {} from flight statistics",
                club_copied, flight_calculated
            );
            metrics.records_loaded = club_copied + flight_calculated;

            // Get total count of aircraft with home_base_airport_ident set
            match get_aircraft_with_home_base_count(&pool).await {
                Ok(total) => {
                    metrics.records_in_db = Some(total);
                }
                Err(e) => {
                    info!("Failed to get aircraft home base count: {}", e);
                    metrics.records_in_db = None;
                }
            }
            metrics.success = true;
        }
        Err(e) => {
            error!(error = %e, "Failed to calculate aircraft home bases");
            metrics.success = false;
            metrics.error_message = Some(e.to_string());
        }
    }

    metrics.duration_secs = start.elapsed().as_secs_f64();
    metrics
}

/// Calculate aircraft home bases using two steps:
/// Returns (club_copied_count, flight_calculated_count)
async fn calculate_aircraft_home_bases(pool: &PgPool) -> Result<(usize, usize)> {
    let pool_clone = pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool_clone.get()?;

        // Step 1: Copy home base from club
        info!("Step 1: Copying home base airport from clubs to aircraft...");
        let club_count = diesel::sql_query(
            r#"
            UPDATE aircraft
            SET home_base_airport_ident = (
                SELECT airports.ident
                FROM clubs
                JOIN airports ON clubs.home_base_airport_id = airports.id
                WHERE clubs.id = aircraft.club_id
            )
            WHERE aircraft.club_id IS NOT NULL
              AND aircraft.home_base_airport_ident IS NULL
              AND EXISTS (
                  SELECT 1 FROM clubs
                  WHERE clubs.id = aircraft.club_id
                    AND clubs.home_base_airport_id IS NOT NULL
              )
            "#,
        )
        .execute(&mut conn)?;

        info!("Copied home base from clubs for {} aircraft", club_count);

        // Step 2: Calculate from flight statistics
        info!("Step 2: Calculating home base from flight statistics (minimum 3 flights)...");
        let flight_count = diesel::sql_query(
            r#"
            WITH airport_frequency AS (
                -- Count both departures and arrivals for each aircraft/airport pair
                SELECT
                    aircraft_id,
                    airport_id,
                    COUNT(*) as occurrence_count
                FROM (
                    SELECT aircraft_id, departure_airport_id as airport_id
                    FROM flights
                    WHERE aircraft_id IS NOT NULL
                      AND departure_airport_id IS NOT NULL

                    UNION ALL

                    SELECT aircraft_id, arrival_airport_id as airport_id
                    FROM flights
                    WHERE aircraft_id IS NOT NULL
                      AND arrival_airport_id IS NOT NULL
                ) combined_airports
                GROUP BY aircraft_id, airport_id
            ),
            aircraft_flight_counts AS (
                -- Filter to aircraft with at least 3 flights
                SELECT
                    aircraft_id,
                    COUNT(*) as flight_count
                FROM flights
                WHERE aircraft_id IS NOT NULL
                GROUP BY aircraft_id
                HAVING COUNT(*) >= 3
            ),
            most_frequent_airport AS (
                -- Pick the most frequent airport per aircraft
                SELECT DISTINCT ON (af.aircraft_id)
                    af.aircraft_id,
                    a.ident as airport_ident,
                    af.occurrence_count
                FROM airport_frequency af
                JOIN aircraft_flight_counts afc ON af.aircraft_id = afc.aircraft_id
                JOIN airports a ON a.id = af.airport_id
                JOIN aircraft ac ON ac.id = af.aircraft_id
                WHERE ac.home_base_airport_ident IS NULL  -- Only aircraft without home base
                ORDER BY af.aircraft_id, af.occurrence_count DESC, a.ident  -- Tie-breaker: alphabetical
            )
            UPDATE aircraft
            SET home_base_airport_ident = mfa.airport_ident
            FROM most_frequent_airport mfa
            WHERE aircraft.id = mfa.aircraft_id
            "#,
        )
        .execute(&mut conn)?;

        info!(
            "Calculated home base from flight statistics for {} aircraft",
            flight_count
        );

        Ok((club_count, flight_count))
    })
    .await?
}

/// Get count of aircraft with home_base_airport_ident set
async fn get_aircraft_with_home_base_count(pool: &PgPool) -> Result<i64> {
    use diesel::dsl::count_star;
    use soar::schema::aircraft::dsl::*;

    let pool = pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        let count = aircraft
            .filter(home_base_airport_ident.is_not_null())
            .select(count_star())
            .first::<i64>(&mut conn)?;

        Ok(count)
    })
    .await?
}
