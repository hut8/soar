use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use diesel::sql_types;
use std::collections::HashSet;
use tracing::{info, warn};
use uuid::Uuid;

use soar::coverage::NewReceiverCoverageH3;
use soar::coverage_repo::CoverageRepository;
use soar::web::PgPool;

/// Run all scheduled aggregation tasks
///
/// This is the main entry point for daily aggregation jobs.
/// Runs both coverage hex aggregation and flight analytics aggregation.
///
/// If start_date or end_date are None, automatically determines the date range:
/// - end_date defaults to yesterday
/// - start_date defaults to 30 days ago (from end_date)
pub async fn run_aggregates(
    pool: PgPool,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    resolutions: Vec<i16>,
) -> Result<()> {
    // Determine end date (default to yesterday)
    let end_date = end_date.unwrap_or_else(|| {
        let yesterday = Utc::now().date_naive() - chrono::Duration::days(1);
        info!(
            "No end date specified, defaulting to yesterday: {}",
            yesterday
        );
        yesterday
    });

    // Determine start date (default to 30 days ago if not specified)
    let start_date = start_date.unwrap_or_else(|| {
        let thirty_days_ago = end_date - chrono::Duration::days(30);
        info!(
            "No start date specified, defaulting to 30 days ago: {}",
            thirty_days_ago
        );
        thirty_days_ago
    });

    // Validate date range
    if start_date > end_date {
        warn!(
            "Start date {} is after end date {}, nothing to aggregate",
            start_date, end_date
        );
        return Ok(());
    }

    info!(
        "Running aggregates for date range {} to {}",
        start_date, end_date
    );

    // Run coverage hex aggregation
    aggregate_coverage_hexes(pool.clone(), start_date, end_date, resolutions).await?;

    // Run flight analytics aggregation
    aggregate_flight_analytics(pool.clone(), start_date, end_date).await?;

    info!("All aggregation tasks complete");
    Ok(())
}

/// Aggregate fixes into H3 coverage hexes for a specific date range
///
/// Skips date/resolution combinations that already have coverage data.
async fn aggregate_coverage_hexes(
    pool: PgPool,
    start_date: NaiveDate,
    end_date: NaiveDate,
    resolutions: Vec<i16>,
) -> Result<()> {
    let repo = CoverageRepository::new(pool.clone());

    // Find needed coverage dimensions (skip already-computed ones)
    info!("Finding needed coverage dimensions");

    let existing_dimensions = repo
        .get_existing_coverage_dimensions(start_date, end_date, &resolutions)
        .await?;

    // Build list of needed dimensions
    let mut needed_dimensions: Vec<(NaiveDate, i16)> = Vec::new();
    let mut current_date = start_date;
    while current_date <= end_date {
        for &resolution in &resolutions {
            if !existing_dimensions.contains(&(current_date, resolution)) {
                needed_dimensions.push((current_date, resolution));
            }
        }
        current_date += chrono::Duration::days(1);
    }

    // Log each needed dimension
    for (date, resolution) in &needed_dimensions {
        info!("Found dimension: date={} resolution={}", date, resolution);
    }

    // Calculate summary statistics
    let unique_dates: HashSet<NaiveDate> = needed_dimensions.iter().map(|(d, _)| *d).collect();
    let unique_resolutions: HashSet<i16> = needed_dimensions.iter().map(|(_, r)| *r).collect();

    info!(
        "Found total dimensions: {} resolutions across {} dates",
        unique_resolutions.len(),
        unique_dates.len()
    );

    if needed_dimensions.is_empty() {
        info!("All coverage dimensions already computed, nothing to do");
        return Ok(());
    }

    // Group dimensions by date for parallel processing
    let mut dates_to_process: Vec<NaiveDate> = unique_dates.into_iter().collect();
    dates_to_process.sort();

    for date in dates_to_process {
        info!("processing coverage for date: {}", date);

        // Get resolutions needed for this date
        let resolutions_for_date: Vec<i16> = needed_dimensions
            .iter()
            .filter(|(d, _)| *d == date)
            .map(|(_, r)| *r)
            .collect();

        // Process all resolutions for this day in parallel
        let mut tasks = Vec::new();
        for resolution in resolutions_for_date {
            let pool_clone = pool.clone();

            let task = tokio::spawn(async move {
                fetch_and_aggregate_fixes(pool_clone, date, date, resolution).await
            });
            tasks.push((resolution, task));
        }

        // Wait for all resolutions to complete and upsert results
        for (resolution, task) in tasks {
            let coverage_data = task.await??;

            if !coverage_data.is_empty() {
                info!(
                    "Upserting {} coverage records for date {} resolution {}",
                    coverage_data.len(),
                    date,
                    resolution
                );
                repo.upsert_coverage_batch(coverage_data).await?;
            } else {
                info!("No fixes found for date {} resolution {}", date, resolution);
            }
        }
    }

    info!("Coverage aggregation complete");
    Ok(())
}

/// Fetch fixes and aggregate into H3 hexes using PostgreSQL h3 extension
/// This is MUCH faster than the old approach because all H3 conversion and
/// aggregation happens in the database using native h3_latlng_to_cell()
async fn fetch_and_aggregate_fixes(
    pool: PgPool,
    start_date: NaiveDate,
    end_date: NaiveDate,
    resolution: i16,
) -> Result<Vec<NewReceiverCoverageH3>> {
    let pool_clone = pool.clone();
    let start_datetime = start_date
        .and_hms_opt(0, 0, 0)
        .context("Invalid start date")?
        .and_utc();
    let end_datetime = end_date
        .and_hms_opt(23, 59, 59)
        .context("Invalid end date")?
        .and_utc();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool_clone.get()?;

        #[derive(QueryableByName)]
        struct AggregatedCoverage {
            #[diesel(sql_type = diesel::sql_types::BigInt)]
            h3_index: i64,
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            receiver_id: Uuid,
            #[diesel(sql_type = diesel::sql_types::Date)]
            date: NaiveDate,
            #[diesel(sql_type = diesel::sql_types::BigInt)]
            fix_count: i64,
            #[diesel(sql_type = diesel::sql_types::Timestamptz)]
            first_seen_at: DateTime<Utc>,
            #[diesel(sql_type = diesel::sql_types::Timestamptz)]
            last_seen_at: DateTime<Utc>,
            #[diesel(sql_type = diesel::sql_types::Nullable<sql_types::Integer>)]
            min_altitude_msl_feet: Option<i32>,
            #[diesel(sql_type = diesel::sql_types::Nullable<sql_types::Integer>)]
            max_altitude_msl_feet: Option<i32>,
            #[diesel(sql_type = diesel::sql_types::Nullable<sql_types::Integer>)]
            avg_altitude_msl_feet: Option<i32>,
        }

        info!(
            "Aggregating fixes from {} to {}",
            start_datetime, end_datetime
        );
        let start = std::time::Instant::now();

        // Use h3_latlng_to_cell() PostgreSQL function for efficient H3 conversion
        // All aggregation happens in the database - WAY faster than fetching + processing in Rust
        // Note: ST_MakePoint takes (longitude, latitude) in PostGIS (x, y order)
        let coverage_data: Vec<AggregatedCoverage> = diesel::sql_query(
            r#"
            SELECT
                h3_latlng_to_cell(ST_MakePoint(longitude, latitude)::geography, $3)::bigint AS h3_index,
                receiver_id,
                DATE(received_at) AS date,
                COUNT(*)::bigint AS fix_count,
                MIN(timestamp) AS first_seen_at,
                MAX(timestamp) AS last_seen_at,
                MIN(altitude_msl_feet) AS min_altitude_msl_feet,
                MAX(altitude_msl_feet) AS max_altitude_msl_feet,
                AVG(altitude_msl_feet)::integer AS avg_altitude_msl_feet
            FROM fixes
            WHERE received_at >= $1 AND received_at < $2
              AND latitude IS NOT NULL
              AND longitude IS NOT NULL
            GROUP BY h3_index, receiver_id, date
            ORDER BY h3_index, receiver_id, date
            "#,
        )
        .bind::<sql_types::Timestamptz, _>(start_datetime)
        .bind::<sql_types::Timestamptz, _>(end_datetime)
        .bind::<sql_types::SmallInt, _>(resolution)
        .load(&mut conn)
        .context("Failed to aggregate fixes for coverage")?;

        let duration = start.elapsed();
        info!(
            "Aggregated {} coverage hexes in {:.2}s",
            coverage_data.len(),
            duration.as_secs_f64()
        );

        if coverage_data.is_empty() {
            warn!("No fixes found in the specified date range");
            return Ok(Vec::new());
        }

        // Convert to NewReceiverCoverageH3
        let results: Vec<NewReceiverCoverageH3> = coverage_data
            .into_iter()
            .map(|row| NewReceiverCoverageH3 {
                h3_index: row.h3_index,
                resolution,
                receiver_id: row.receiver_id,
                date: row.date,
                fix_count: row.fix_count,
                first_seen_at: row.first_seen_at,
                last_seen_at: row.last_seen_at,
                min_altitude_msl_feet: row.min_altitude_msl_feet,
                max_altitude_msl_feet: row.max_altitude_msl_feet,
                avg_altitude_msl_feet: row.avg_altitude_msl_feet,
            })
            .collect();

        Ok(results)
    })
    .await?
}

// ============================================================================
// Flight Analytics Aggregation
// ============================================================================

/// Aggregate flight analytics for the specified date range
///
/// Populates the following tables:
/// - flight_analytics_daily: Daily flight counts and stats
/// - flight_analytics_hourly: Hourly flight counts
/// - flight_duration_buckets: Distribution of flight durations
/// - aircraft_analytics: Per-aircraft lifetime stats with z-scores
/// - club_analytics_daily: Per-club daily stats
/// - airport_analytics_daily: Per-airport departure/arrival counts
async fn aggregate_flight_analytics(
    pool: PgPool,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<()> {
    info!(
        "Aggregating flight analytics from {} to {}",
        start_date, end_date
    );

    let start = std::time::Instant::now();

    // Run aggregations in sequence to avoid overwhelming the database
    aggregate_daily_flights(pool.clone(), start_date, end_date).await?;
    aggregate_hourly_flights(pool.clone(), start_date, end_date).await?;
    aggregate_duration_buckets(pool.clone()).await?;
    aggregate_aircraft_analytics(pool.clone()).await?;
    aggregate_club_analytics(pool.clone(), start_date, end_date).await?;
    aggregate_airport_analytics(pool.clone(), start_date, end_date).await?;

    let duration = start.elapsed();
    info!(
        "Flight analytics aggregation complete in {:.2}s",
        duration.as_secs_f64()
    );

    Ok(())
}

/// Aggregate daily flight statistics
async fn aggregate_daily_flights(
    pool: PgPool,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<()> {
    info!("Aggregating daily flight statistics");

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        diesel::sql_query(
            r#"
            INSERT INTO flight_analytics_daily (
                date,
                flight_count,
                total_duration_seconds,
                avg_duration_seconds,
                total_distance_meters,
                tow_flight_count,
                cross_country_count,
                updated_at
            )
            SELECT
                DATE(takeoff_time) as date,
                COUNT(*) as flight_count,
                COALESCE(SUM(EXTRACT(EPOCH FROM (landing_time - takeoff_time))::bigint), 0) as total_duration_seconds,
                COALESCE(AVG(EXTRACT(EPOCH FROM (landing_time - takeoff_time)))::integer, 0) as avg_duration_seconds,
                COALESCE(SUM(total_distance_meters), 0) as total_distance_meters,
                COUNT(*) FILTER (WHERE towed_by_aircraft_id IS NOT NULL) as tow_flight_count,
                COUNT(*) FILTER (WHERE departure_airport_id IS DISTINCT FROM arrival_airport_id AND arrival_airport_id IS NOT NULL) as cross_country_count,
                NOW()
            FROM flights
            WHERE takeoff_time IS NOT NULL
              AND DATE(takeoff_time) >= $1
              AND DATE(takeoff_time) <= $2
            GROUP BY DATE(takeoff_time)
            ON CONFLICT (date) DO UPDATE SET
                flight_count = EXCLUDED.flight_count,
                total_duration_seconds = EXCLUDED.total_duration_seconds,
                avg_duration_seconds = EXCLUDED.avg_duration_seconds,
                total_distance_meters = EXCLUDED.total_distance_meters,
                tow_flight_count = EXCLUDED.tow_flight_count,
                cross_country_count = EXCLUDED.cross_country_count,
                updated_at = NOW()
            "#,
        )
        .bind::<sql_types::Date, _>(start_date)
        .bind::<sql_types::Date, _>(end_date)
        .execute(&mut conn)
        .context("Failed to aggregate daily flights")?;

        info!("Daily flight statistics aggregated");
        Ok(())
    })
    .await?
}

/// Aggregate hourly flight statistics
async fn aggregate_hourly_flights(
    pool: PgPool,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<()> {
    info!("Aggregating hourly flight statistics");

    let start_datetime = start_date
        .and_hms_opt(0, 0, 0)
        .context("Invalid start date")?
        .and_utc();
    let end_datetime = end_date
        .and_hms_opt(23, 59, 59)
        .context("Invalid end date")?
        .and_utc();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        diesel::sql_query(
            r#"
            INSERT INTO flight_analytics_hourly (
                hour,
                flight_count,
                active_devices,
                active_clubs,
                updated_at
            )
            SELECT
                date_trunc('hour', takeoff_time) as hour,
                COUNT(*) as flight_count,
                COUNT(DISTINCT aircraft_id) as active_devices,
                COUNT(DISTINCT club_id) as active_clubs,
                NOW()
            FROM flights
            WHERE takeoff_time IS NOT NULL
              AND takeoff_time >= $1
              AND takeoff_time <= $2
            GROUP BY date_trunc('hour', takeoff_time)
            ON CONFLICT (hour) DO UPDATE SET
                flight_count = EXCLUDED.flight_count,
                active_devices = EXCLUDED.active_devices,
                active_clubs = EXCLUDED.active_clubs,
                updated_at = NOW()
            "#,
        )
        .bind::<sql_types::Timestamptz, _>(start_datetime)
        .bind::<sql_types::Timestamptz, _>(end_datetime)
        .execute(&mut conn)
        .context("Failed to aggregate hourly flights")?;

        info!("Hourly flight statistics aggregated");
        Ok(())
    })
    .await?
}

/// Aggregate flight duration buckets (full recompute)
async fn aggregate_duration_buckets(pool: PgPool) -> Result<()> {
    info!("Aggregating flight duration buckets");

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        // Reset all bucket counts to 0
        diesel::sql_query("UPDATE flight_duration_buckets SET flight_count = 0, updated_at = NOW()")
            .execute(&mut conn)
            .context("Failed to reset duration buckets")?;

        // Update each bucket with counts from flights table
        // Uses the get_duration_bucket() function defined in the database
        diesel::sql_query(
            r#"
            UPDATE flight_duration_buckets fdb
            SET
                flight_count = counts.cnt,
                updated_at = NOW()
            FROM (
                SELECT
                    get_duration_bucket(EXTRACT(EPOCH FROM (landing_time - takeoff_time))::integer) as bucket_name,
                    COUNT(*) as cnt
                FROM flights
                WHERE takeoff_time IS NOT NULL
                  AND landing_time IS NOT NULL
                GROUP BY bucket_name
            ) counts
            WHERE fdb.bucket_name = counts.bucket_name
            "#,
        )
        .execute(&mut conn)
        .context("Failed to aggregate duration buckets")?;

        info!("Flight duration buckets aggregated");
        Ok(())
    })
    .await?
}

/// Aggregate per-aircraft analytics (full recompute with z-scores)
async fn aggregate_aircraft_analytics(pool: PgPool) -> Result<()> {
    info!("Aggregating aircraft analytics");

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        // First, compute basic stats for each aircraft
        diesel::sql_query(
            r#"
            INSERT INTO aircraft_analytics (
                aircraft_id,
                registration,
                aircraft_model,
                flight_count_total,
                flight_count_30d,
                flight_count_7d,
                last_flight_at,
                avg_flight_duration_seconds,
                total_distance_meters,
                updated_at
            )
            SELECT
                a.id as aircraft_id,
                a.registration,
                a.aircraft_model,
                COUNT(f.id) as flight_count_total,
                COUNT(f.id) FILTER (WHERE f.takeoff_time >= CURRENT_DATE - 30) as flight_count_30d,
                COUNT(f.id) FILTER (WHERE f.takeoff_time >= CURRENT_DATE - 7) as flight_count_7d,
                MAX(f.takeoff_time) as last_flight_at,
                COALESCE(AVG(EXTRACT(EPOCH FROM (f.landing_time - f.takeoff_time)))::integer, 0) as avg_flight_duration_seconds,
                COALESCE(SUM(f.total_distance_meters), 0) as total_distance_meters,
                NOW()
            FROM aircraft a
            LEFT JOIN flights f ON f.aircraft_id = a.id AND f.takeoff_time IS NOT NULL
            GROUP BY a.id, a.registration, a.aircraft_model
            ON CONFLICT (aircraft_id) DO UPDATE SET
                registration = EXCLUDED.registration,
                aircraft_model = EXCLUDED.aircraft_model,
                flight_count_total = EXCLUDED.flight_count_total,
                flight_count_30d = EXCLUDED.flight_count_30d,
                flight_count_7d = EXCLUDED.flight_count_7d,
                last_flight_at = EXCLUDED.last_flight_at,
                avg_flight_duration_seconds = EXCLUDED.avg_flight_duration_seconds,
                total_distance_meters = EXCLUDED.total_distance_meters,
                updated_at = NOW()
            "#,
        )
        .execute(&mut conn)
        .context("Failed to aggregate aircraft analytics")?;

        // Then compute z-scores for flight_count_30d
        diesel::sql_query(
            r#"
            WITH stats AS (
                SELECT
                    AVG(flight_count_30d)::float as mean,
                    STDDEV(flight_count_30d)::float as stddev
                FROM aircraft_analytics
                WHERE flight_count_30d > 0
            )
            UPDATE aircraft_analytics
            SET z_score_30d = CASE
                WHEN stats.stddev > 0 THEN
                    ROUND(((flight_count_30d - stats.mean) / stats.stddev)::numeric, 2)
                ELSE 0
            END
            FROM stats
            "#,
        )
        .execute(&mut conn)
        .context("Failed to compute z-scores")?;

        info!("Aircraft analytics aggregated with z-scores");
        Ok(())
    })
    .await?
}

/// Aggregate per-club daily analytics
async fn aggregate_club_analytics(
    pool: PgPool,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<()> {
    info!("Aggregating club analytics");

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        diesel::sql_query(
            r#"
            INSERT INTO club_analytics_daily (
                club_id,
                date,
                club_name,
                flight_count,
                active_devices,
                total_airtime_seconds,
                tow_count,
                updated_at
            )
            SELECT
                f.club_id,
                DATE(f.takeoff_time) as date,
                c.name as club_name,
                COUNT(*) as flight_count,
                COUNT(DISTINCT f.aircraft_id) as active_devices,
                COALESCE(SUM(EXTRACT(EPOCH FROM (f.landing_time - f.takeoff_time))::bigint), 0) as total_airtime_seconds,
                COUNT(*) FILTER (WHERE f.towed_by_aircraft_id IS NOT NULL) as tow_count,
                NOW()
            FROM flights f
            JOIN clubs c ON c.id = f.club_id
            WHERE f.club_id IS NOT NULL
              AND f.takeoff_time IS NOT NULL
              AND DATE(f.takeoff_time) >= $1
              AND DATE(f.takeoff_time) <= $2
            GROUP BY f.club_id, DATE(f.takeoff_time), c.name
            ON CONFLICT (club_id, date) DO UPDATE SET
                club_name = EXCLUDED.club_name,
                flight_count = EXCLUDED.flight_count,
                active_devices = EXCLUDED.active_devices,
                total_airtime_seconds = EXCLUDED.total_airtime_seconds,
                tow_count = EXCLUDED.tow_count,
                updated_at = NOW()
            "#,
        )
        .bind::<sql_types::Date, _>(start_date)
        .bind::<sql_types::Date, _>(end_date)
        .execute(&mut conn)
        .context("Failed to aggregate club analytics")?;

        info!("Club analytics aggregated");
        Ok(())
    })
    .await?
}

/// Aggregate per-airport daily analytics
async fn aggregate_airport_analytics(
    pool: PgPool,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<()> {
    info!("Aggregating airport analytics");

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;

        // Aggregate departures
        diesel::sql_query(
            r#"
            INSERT INTO airport_analytics_daily (
                airport_id,
                date,
                airport_ident,
                airport_name,
                departure_count,
                arrival_count,
                updated_at
            )
            SELECT
                f.departure_airport_id as airport_id,
                DATE(f.takeoff_time) as date,
                a.ident as airport_ident,
                a.name as airport_name,
                COUNT(*) as departure_count,
                0 as arrival_count,
                NOW()
            FROM flights f
            JOIN airports a ON a.id = f.departure_airport_id
            WHERE f.departure_airport_id IS NOT NULL
              AND f.takeoff_time IS NOT NULL
              AND DATE(f.takeoff_time) >= $1
              AND DATE(f.takeoff_time) <= $2
            GROUP BY f.departure_airport_id, DATE(f.takeoff_time), a.ident, a.name
            ON CONFLICT (airport_id, date) DO UPDATE SET
                airport_ident = EXCLUDED.airport_ident,
                airport_name = EXCLUDED.airport_name,
                departure_count = EXCLUDED.departure_count,
                updated_at = NOW()
            "#,
        )
        .bind::<sql_types::Date, _>(start_date)
        .bind::<sql_types::Date, _>(end_date)
        .execute(&mut conn)
        .context("Failed to aggregate airport departures")?;

        // Aggregate arrivals
        diesel::sql_query(
            r#"
            INSERT INTO airport_analytics_daily (
                airport_id,
                date,
                airport_ident,
                airport_name,
                departure_count,
                arrival_count,
                updated_at
            )
            SELECT
                f.arrival_airport_id as airport_id,
                DATE(f.landing_time) as date,
                a.ident as airport_ident,
                a.name as airport_name,
                0 as departure_count,
                COUNT(*) as arrival_count,
                NOW()
            FROM flights f
            JOIN airports a ON a.id = f.arrival_airport_id
            WHERE f.arrival_airport_id IS NOT NULL
              AND f.landing_time IS NOT NULL
              AND DATE(f.landing_time) >= $1
              AND DATE(f.landing_time) <= $2
            GROUP BY f.arrival_airport_id, DATE(f.landing_time), a.ident, a.name
            ON CONFLICT (airport_id, date) DO UPDATE SET
                airport_ident = EXCLUDED.airport_ident,
                airport_name = EXCLUDED.airport_name,
                arrival_count = EXCLUDED.arrival_count,
                updated_at = NOW()
            "#,
        )
        .bind::<sql_types::Date, _>(start_date)
        .bind::<sql_types::Date, _>(end_date)
        .execute(&mut conn)
        .context("Failed to aggregate airport arrivals")?;

        info!("Airport analytics aggregated");
        Ok(())
    })
    .await?
}
