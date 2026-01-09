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

/// Aggregate fixes into H3 coverage hexes for a specific date range
///
/// If start_date or end_date are None, automatically determines the date range:
/// - end_date defaults to yesterday
/// - start_date defaults to 30 days ago (from end_date)
///
/// This ensures all resolutions have coverage for the last 30 days by default.
/// Skips date/resolution combinations that already have coverage data.
pub async fn aggregate_coverage(
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

    let repo = CoverageRepository::new(pool.clone());

    // Find needed coverage dimensions (skip already-computed ones)
    info!("finding needed coverage dimensions");

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
        info!("found dimension: date={} resolution={}", date, resolution);
    }

    // Calculate summary statistics
    let unique_dates: HashSet<NaiveDate> = needed_dimensions.iter().map(|(d, _)| *d).collect();
    let unique_resolutions: HashSet<i16> = needed_dimensions.iter().map(|(_, r)| *r).collect();

    info!(
        "found total dimensions: {} resolutions across {} dates",
        unique_resolutions.len(),
        unique_dates.len()
    );

    if needed_dimensions.is_empty() {
        info!("all coverage dimensions already computed, nothing to do");
        return Ok(());
    }

    // Group dimensions by date for parallel processing
    let mut dates_to_process: Vec<NaiveDate> = unique_dates.into_iter().collect();
    dates_to_process.sort();

    for date in dates_to_process {
        info!("Processing date: {}", date);

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
