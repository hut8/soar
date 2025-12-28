use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use diesel::sql_types;
use tracing::{info, warn};
use uuid::Uuid;

use soar::coverage::NewReceiverCoverageH3;
use soar::coverage_repo::CoverageRepository;
use soar::web::PgPool;

/// Aggregate fixes into H3 coverage hexes for a specific date range
///
/// If start_date or end_date are None, automatically determines the date range:
/// - end_date defaults to yesterday
/// - start_date defaults to (last coverage date + 1), or oldest fix date if no coverage exists
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

    // Determine start date (auto-detect if not specified)
    let start_date = match start_date {
        Some(date) => {
            info!("Using specified start date: {}", date);
            date
        }
        None => {
            info!("No start date specified, auto-detecting from database...");
            let last_coverage_date = find_last_coverage_date(pool.clone()).await?;

            match last_coverage_date {
                Some(last_date) => {
                    let next_date = last_date + chrono::Duration::days(1);
                    info!(
                        "Last coverage date: {}, starting from: {}",
                        last_date, next_date
                    );
                    next_date
                }
                None => {
                    info!("No existing coverage found, checking for oldest fix...");
                    let oldest_fix_date = find_oldest_fix_date(pool.clone()).await?;
                    match oldest_fix_date {
                        Some(oldest) => {
                            info!("Oldest fix date: {}, starting from: {}", oldest, oldest);
                            oldest
                        }
                        None => {
                            warn!("No fixes found in database, nothing to aggregate");
                            return Ok(());
                        }
                    }
                }
            }
        }
    };

    // Validate date range
    if start_date > end_date {
        warn!(
            "Start date {} is after end date {}, nothing to aggregate",
            start_date, end_date
        );
        return Ok(());
    }

    info!(
        "Aggregating coverage from {} to {} for resolutions {:?}",
        start_date, end_date, resolutions
    );

    let repo = CoverageRepository::new(pool.clone());

    // Iterate over each day in the range
    let mut current_date = start_date;
    while current_date <= end_date {
        info!("Processing date: {}", current_date);

        // Process all resolutions for this day in parallel
        let mut tasks = Vec::new();
        for resolution in &resolutions {
            let pool_clone = pool.clone();
            let date = current_date;
            let res = *resolution;

            let task = tokio::spawn(async move {
                fetch_and_aggregate_fixes(pool_clone, date, date, res).await
            });
            tasks.push((res, task));
        }

        // Wait for all resolutions to complete and upsert results
        for (resolution, task) in tasks {
            let coverage_data = task.await??;

            if !coverage_data.is_empty() {
                info!(
                    "Upserting {} coverage records for date {} resolution {}",
                    coverage_data.len(),
                    current_date,
                    resolution
                );
                repo.upsert_coverage_batch(coverage_data).await?;
            } else {
                info!(
                    "No fixes found for date {} resolution {}",
                    current_date, resolution
                );
            }
        }

        current_date += chrono::Duration::days(1);
    }

    info!("Coverage aggregation complete");
    Ok(())
}

/// Fetch fixes and aggregate into H3 hexes using PostgreSQL h3 extension
/// This is MUCH faster than the old approach because all H3 conversion and
/// aggregation happens in the database using native h3_lat_lng_to_cell()
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
            #[diesel(sql_type = diesel::sql_types::Integer)]
            fix_count: i32,
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
            "Aggregating fixes from {} to {} using PostgreSQL h3 extension",
            start_datetime, end_datetime
        );
        let start = std::time::Instant::now();

        // Use h3_lat_lng_to_cell() PostgreSQL function for efficient H3 conversion
        // All aggregation happens in the database - WAY faster than fetching + processing in Rust
        let coverage_data: Vec<AggregatedCoverage> = diesel::sql_query(
            r#"
            SELECT
                h3_lat_lng_to_cell(latitude, longitude, $3)::bigint AS h3_index,
                receiver_id,
                DATE(received_at) AS date,
                COUNT(*)::integer AS fix_count,
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
            "Aggregated {} coverage hexes in {:.2}s using PostgreSQL h3 extension",
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

/// Find the most recent date in the receiver_coverage_h3 table
async fn find_last_coverage_date(pool: PgPool) -> Result<Option<NaiveDate>> {
    use soar::schema::receiver_coverage_h3;

    let pool_clone = pool.clone();
    let result = tokio::task::spawn_blocking(move || {
        use diesel::dsl::max;

        let mut conn = pool_clone
            .get()
            .context("Failed to get database connection")?;

        let max_date = receiver_coverage_h3::table
            .select(max(receiver_coverage_h3::date))
            .first::<Option<NaiveDate>>(&mut conn)
            .context("Failed to query max coverage date")?;

        Ok::<Option<NaiveDate>, anyhow::Error>(max_date)
    })
    .await??;

    Ok(result)
}

/// Find the oldest date in the fixes table
async fn find_oldest_fix_date(pool: PgPool) -> Result<Option<NaiveDate>> {
    let pool_clone = pool.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool_clone
            .get()
            .context("Failed to get database connection")?;

        // Query for the minimum timestamp from fixes table
        #[derive(QueryableByName)]
        struct MinTimestamp {
            #[diesel(sql_type = sql_types::Nullable<sql_types::Timestamptz>)]
            min_timestamp: Option<DateTime<Utc>>,
        }

        let query = diesel::sql_query(
            "SELECT MIN(timestamp) as min_timestamp FROM fixes WHERE timestamp IS NOT NULL",
        );

        let result: MinTimestamp = query
            .get_result(&mut conn)
            .context("Failed to query min fix timestamp")?;

        Ok::<Option<NaiveDate>, anyhow::Error>(result.min_timestamp.map(|ts| ts.date_naive()))
    })
    .await??;

    Ok(result)
}
