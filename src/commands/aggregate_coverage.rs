use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use diesel::sql_types;
use h3o::{LatLng, Resolution};
use rayon::prelude::*;
use std::collections::HashMap;
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

/// Fetch fixes and aggregate into H3 hexes using SQL
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

        // Query fixes and group by receiver, date
        // We'll convert lat/lng to H3 in Rust (can't do in SQL without extension)
        #[derive(QueryableByName)]
        struct FixRow {
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            receiver_id: Uuid,
            #[diesel(sql_type = diesel::sql_types::Date)]
            date: NaiveDate,
            #[diesel(sql_type = diesel::sql_types::Double)]
            latitude: f64,
            #[diesel(sql_type = diesel::sql_types::Double)]
            longitude: f64,
            #[diesel(sql_type = diesel::sql_types::Nullable<sql_types::Integer>)]
            altitude_msl_feet: Option<i32>,
            #[diesel(sql_type = diesel::sql_types::Timestamptz)]
            timestamp: DateTime<Utc>,
        }

        info!("Fetching fixes from {} to {}", start_datetime, end_datetime);
        let fetch_start = std::time::Instant::now();

        let fixes: Vec<FixRow> = diesel::sql_query(
            r#"
            SELECT
                receiver_id,
                DATE(received_at) as date,
                latitude,
                longitude,
                altitude_msl_feet,
                timestamp
            FROM fixes
            WHERE received_at >= $1 AND received_at < $2
            ORDER BY receiver_id, date, timestamp
            "#,
        )
        .bind::<sql_types::Timestamptz, _>(start_datetime)
        .bind::<sql_types::Timestamptz, _>(end_datetime)
        .load(&mut conn)
        .context("Failed to fetch fixes for coverage aggregation")?;

        let fetch_duration = fetch_start.elapsed();
        info!(
            "Fetched {} fixes in {:.2}s, converting to H3...",
            fixes.len(),
            fetch_duration.as_secs_f64()
        );

        if fixes.is_empty() {
            warn!("No fixes found in the specified date range");
            return Ok(Vec::new());
        }

        // Group by (h3_index, receiver_id, date) and aggregate using rayon for parallelization
        let res = Resolution::try_from(resolution as u8)
            .context(format!("Invalid H3 resolution: {}", resolution))?;

        let h3_start = std::time::Instant::now();

        // Use rayon to parallelize H3 conversion and aggregation
        let coverage_map: HashMap<(i64, Uuid, NaiveDate), CoverageAggregate> = fixes
            .par_iter()
            .fold(HashMap::new, |mut map, fix| {
                // Convert lat/lng to H3 (skip invalid coordinates)
                if let Ok(latlng) = LatLng::new(fix.latitude, fix.longitude) {
                    let h3_index: u64 = latlng.to_cell(res).into();
                    let h3_i64 = h3_index as i64;
                    let key = (h3_i64, fix.receiver_id, fix.date);

                    map.entry(key)
                        .and_modify(|agg: &mut CoverageAggregate| {
                            agg.fix_count += 1;
                            agg.first_seen_at = agg.first_seen_at.min(fix.timestamp);
                            agg.last_seen_at = agg.last_seen_at.max(fix.timestamp);

                            if let Some(alt) = fix.altitude_msl_feet {
                                agg.min_altitude =
                                    Some(agg.min_altitude.map_or(alt, |min: i32| min.min(alt)));
                                agg.max_altitude =
                                    Some(agg.max_altitude.map_or(alt, |max: i32| max.max(alt)));
                                agg.altitude_sum += alt as i64;
                                agg.altitude_count += 1;
                            }
                        })
                        .or_insert_with(|| CoverageAggregate {
                            fix_count: 1,
                            first_seen_at: fix.timestamp,
                            last_seen_at: fix.timestamp,
                            min_altitude: fix.altitude_msl_feet,
                            max_altitude: fix.altitude_msl_feet,
                            altitude_sum: fix.altitude_msl_feet.unwrap_or(0) as i64,
                            altitude_count: if fix.altitude_msl_feet.is_some() {
                                1
                            } else {
                                0
                            },
                        });
                }
                map
            })
            .reduce(
                HashMap::new,
                |mut map1: HashMap<(i64, Uuid, NaiveDate), CoverageAggregate>,
                 map2: HashMap<(i64, Uuid, NaiveDate), CoverageAggregate>| {
                    // Merge two hashmaps by combining aggregates for matching keys
                    for (key, agg2) in map2 {
                        map1.entry(key)
                            .and_modify(|agg1: &mut CoverageAggregate| {
                                agg1.fix_count += agg2.fix_count;
                                agg1.first_seen_at = agg1.first_seen_at.min(agg2.first_seen_at);
                                agg1.last_seen_at = agg1.last_seen_at.max(agg2.last_seen_at);
                                agg1.min_altitude = match (agg1.min_altitude, agg2.min_altitude) {
                                    (Some(a), Some(b)) => Some(a.min(b)),
                                    (Some(a), None) => Some(a),
                                    (None, Some(b)) => Some(b),
                                    (None, None) => None,
                                };
                                agg1.max_altitude = match (agg1.max_altitude, agg2.max_altitude) {
                                    (Some(a), Some(b)) => Some(a.max(b)),
                                    (Some(a), None) => Some(a),
                                    (None, Some(b)) => Some(b),
                                    (None, None) => None,
                                };
                                agg1.altitude_sum += agg2.altitude_sum;
                                agg1.altitude_count += agg2.altitude_count;
                            })
                            .or_insert(agg2);
                    }
                    map1
                },
            );

        let h3_duration = h3_start.elapsed();
        info!(
            "Converted {} unique hexes in {:.2}s ({:.0} fixes/sec)",
            coverage_map.len(),
            h3_duration.as_secs_f64(),
            fixes.len() as f64 / h3_duration.as_secs_f64()
        );

        // Convert to NewReceiverCoverageH3
        let coverage_data: Vec<NewReceiverCoverageH3> = coverage_map
            .into_iter()
            .map(|((h3_index, receiver_id, date), agg)| {
                let avg_altitude = if agg.altitude_count > 0 {
                    Some((agg.altitude_sum / agg.altitude_count as i64) as i32)
                } else {
                    None
                };

                NewReceiverCoverageH3 {
                    h3_index,
                    resolution,
                    receiver_id,
                    date,
                    fix_count: agg.fix_count,
                    first_seen_at: agg.first_seen_at,
                    last_seen_at: agg.last_seen_at,
                    min_altitude_msl_feet: agg.min_altitude,
                    max_altitude_msl_feet: agg.max_altitude,
                    avg_altitude_msl_feet: avg_altitude,
                }
            })
            .collect();

        Ok(coverage_data)
    })
    .await?
}

struct CoverageAggregate {
    fix_count: i32,
    first_seen_at: DateTime<Utc>,
    last_seen_at: DateTime<Utc>,
    min_altitude: Option<i32>,
    max_altitude: Option<i32>,
    altitude_sum: i64,
    altitude_count: i32,
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
