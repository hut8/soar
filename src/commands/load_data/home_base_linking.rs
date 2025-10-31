use anyhow::Result;
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use std::time::Instant;
use tracing::{error, info, warn};

use soar::airports_repo::AirportsRepository;
use soar::clubs_repo::ClubsRepository;
use soar::email_reporter::EntityMetrics;

/// Link soaring clubs to their nearest suitable airports as home bases
pub async fn link_home_bases(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> Result<(usize, usize)> {
    info!("Starting home base linking for soaring clubs...");

    // Create repositories
    let clubs_repo = ClubsRepository::new(diesel_pool.clone());
    let airports_repo = AirportsRepository::new(diesel_pool);

    // Get soaring clubs without home base airport IDs
    let clubs = clubs_repo.get_soaring_clubs_without_home_base().await?;
    let club_count = clubs.len();

    if club_count == 0 {
        info!("No soaring clubs need home base linking");
        return Ok((0, 0));
    }

    info!(
        "Found {} soaring clubs that need home base linking",
        club_count
    );

    let mut linked_count = 0;
    let mut failed_count = 0;
    let max_distance_miles = 10.0;
    let max_distance_meters = max_distance_miles * 1609.34; // Convert miles to meters
    let allowed_types = ["large_airport", "medium_airport", "small_airport"];

    for club in clubs {
        if let Some(location) = club.base_location {
            info!(
                "Processing club: {} at ({}, {})",
                club.name, location.latitude, location.longitude
            );

            // Find nearest airports within 10 miles
            match airports_repo
                .find_nearest_airports(
                    location.latitude,
                    location.longitude,
                    max_distance_meters,
                    50, // limit to 50 results to check
                )
                .await
            {
                Ok(nearby_airports) => {
                    // Filter by allowed airport types
                    let suitable_airports: Vec<_> = nearby_airports
                        .into_iter()
                        .filter(|(airport, _distance)| {
                            allowed_types.contains(&airport.airport_type.as_str())
                        })
                        .collect();

                    if let Some((nearest_airport, distance)) = suitable_airports.first() {
                        // Create Google Maps link showing the direct line between club and airport
                        let maps_link = if let (Some(airport_lat), Some(airport_lng)) = (
                            &nearest_airport.latitude_deg,
                            &nearest_airport.longitude_deg,
                        ) {
                            format!(
                                "https://www.google.com/maps/dir/{},{}/{},{}",
                                location.latitude, location.longitude, airport_lat, airport_lng
                            )
                        } else {
                            "No coordinates available".to_string()
                        };

                        info!(
                            "Found suitable airport: {} ({}) at {:.2} miles from {} - Map: {}",
                            nearest_airport.name,
                            nearest_airport.ident,
                            distance / 1609.34, // Convert meters to miles
                            club.name,
                            maps_link
                        );

                        // Update the club's home base airport ID
                        match clubs_repo
                            .update_home_base_airport(club.id, nearest_airport.id)
                            .await
                        {
                            Ok(updated) => {
                                if updated {
                                    info!(
                                        "Successfully linked {} to airport {} ({}) - Map: {}",
                                        club.name,
                                        nearest_airport.name,
                                        nearest_airport.ident,
                                        maps_link
                                    );
                                    linked_count += 1;
                                } else {
                                    warn!("Failed to update club {} - not found", club.name);
                                    failed_count += 1;
                                }
                            }
                            Err(e) => {
                                error!("Failed to update club {} home base: {}", club.name, e);
                                failed_count += 1;
                            }
                        }
                    } else {
                        info!(
                            "No suitable airports found within {} miles of {}",
                            max_distance_miles, club.name
                        );
                        failed_count += 1;
                    }
                }
                Err(e) => {
                    error!("Failed to find airports near club {}: {}", club.name, e);
                    failed_count += 1;
                }
            }
        } else {
            warn!("Skipping club {} - no base location available", club.name);
            failed_count += 1;
        }
    }

    info!(
        "Home base linking completed: {} successfully linked, {} failed",
        linked_count, failed_count
    );

    Ok((linked_count, failed_count))
}

pub async fn link_home_bases_with_metrics(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> EntityMetrics {
    let start = Instant::now();
    let mut metrics = EntityMetrics::new("Club airport linking");

    match link_home_bases(diesel_pool.clone()).await {
        Ok((linked, failed)) => {
            metrics.records_loaded = linked;

            // Get total count of clubs with home_base_airport_id set
            match get_clubs_with_home_base_count(diesel_pool).await {
                Ok(total) => {
                    metrics.records_in_db = Some(total);
                }
                Err(e) => {
                    info!("Failed to get clubs home base count: {}", e);
                    metrics.records_in_db = None;
                }
            }
            metrics.success = true;

            if failed > 0 {
                info!(
                    "Home base linking completed with {} failures out of {} total clubs",
                    failed,
                    linked + failed
                );
            }
        }
        Err(e) => {
            error!("Failed to link home bases: {}", e);
            metrics.success = false;
            metrics.error_message = Some(e.to_string());
        }
    }

    metrics.duration_secs = start.elapsed().as_secs_f64();
    metrics
}

/// Get count of clubs with home_base_airport_id set
async fn get_clubs_with_home_base_count(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> Result<i64> {
    use diesel::dsl::count_star;
    use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
    use soar::schema::clubs::dsl::*;

    tokio::task::spawn_blocking(move || {
        let mut conn = diesel_pool.get()?;

        let count = clubs
            .filter(home_base_airport_id.is_not_null())
            .select(count_star())
            .first::<i64>(&mut conn)?;

        Ok(count)
    })
    .await?
}
