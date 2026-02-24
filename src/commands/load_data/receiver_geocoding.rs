use anyhow::Result;
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

use soar::email_reporter::EntityMetrics;
use soar::geocoding::Geocoder;
use soar::receiver_repo::ReceiverRepository;

/// Geocode receivers that haven't been geocoded yet
/// Processes ALL receivers where geocoded=false and lat/lng are not null
/// Rate limited to 1 request per second to respect Nominatim usage policy
pub async fn geocode_receivers(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> Result<(usize, usize, usize, Vec<String>)> {
    info!("Starting receiver reverse geocoding task...");

    let receiver_repo = ReceiverRepository::new(diesel_pool);
    let geocoder = Geocoder::new_batch_geocoding();

    // Get ALL receivers that need geocoding
    let receivers = receiver_repo
        .get_receivers_needing_geocoding(i64::MAX)
        .await?;

    if receivers.is_empty() {
        info!("No receivers need geocoding");
        return Ok((0, 0, 0, Vec::new()));
    }

    info!("Found {} receivers needing geocoding", receivers.len());

    let mut success_count = 0;
    let mut failure_count = 0;
    let mut failed_callsigns = Vec::new();

    for (index, receiver) in receivers.iter().enumerate() {
        let lat = receiver.latitude.unwrap(); // Safe because we filtered for not null
        let lng = receiver.longitude.unwrap();

        info!(
            "Geocoding receiver {}/{}: {} at ({}, {})",
            index + 1,
            receivers.len(),
            receiver.callsign,
            lat,
            lng
        );

        // Reverse geocode the coordinates
        match geocoder.reverse_geocode(lat, lng).await {
            Ok(result) => {
                info!(
                    "Successfully reverse geocoded {}: {}",
                    receiver.callsign, result.display_name
                );

                // Update the receiver with the geocoded address
                match receiver_repo
                    .update_receiver_address(
                        receiver.id,
                        result.street1,
                        result.city,
                        result.state, // This is the "region" field
                        result.country,
                        result.zip_code,
                    )
                    .await
                {
                    Ok(updated) => {
                        if updated {
                            success_count += 1;
                            info!(
                                "Updated receiver {} with geocoded address",
                                receiver.callsign
                            );
                        } else {
                            warn!(
                                "Failed to update receiver {} - not found in database",
                                receiver.callsign
                            );
                            failure_count += 1;
                            failed_callsigns.push(format!("{}|{},{}", receiver.callsign, lat, lng));
                        }
                    }
                    Err(e) => {
                        error!(
                            error = %e,
                            callsign = %receiver.callsign,
                            "Failed to update receiver with geocoded address"
                        );
                        failure_count += 1;
                        failed_callsigns.push(format!("{}|{},{}", receiver.callsign, lat, lng));
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to reverse geocode receiver {} at ({}, {}): {}",
                    receiver.callsign, lat, lng, e
                );
                failure_count += 1;
                failed_callsigns.push(format!("{}|{},{}", receiver.callsign, lat, lng));
            }
        }

        // Rate limiting: Nominatim allows 1 request per second
        if index < receivers.len() - 1 {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    info!(
        "Receiver geocoding complete: {} successful, {} failed out of {} total",
        success_count,
        failure_count,
        receivers.len()
    );

    Ok((
        receivers.len(),
        success_count,
        failure_count,
        failed_callsigns,
    ))
}

/// Geocode receivers with metrics for email reporting
/// Note: Failures to geocode individual receivers are considered normal and don't mark the stage as failed.
/// Failed receivers are tracked and included in the summary email for visibility.
pub async fn geocode_receivers_with_metrics(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> EntityMetrics {
    let start = Instant::now();
    let mut metrics = EntityMetrics::new("Receiver Geocoding");

    let receiver_repo = ReceiverRepository::new(diesel_pool.clone());

    match geocode_receivers(diesel_pool).await {
        Ok((_total_processed, success, failures, failed_callsigns)) => {
            metrics.records_loaded = success;

            // Get the actual count of geocoded receivers in the database (not just those processed this run)
            match receiver_repo.get_geocoded_receiver_count().await {
                Ok(count) => metrics.records_in_db = Some(count),
                Err(e) => {
                    warn!("Failed to get geocoded receiver count: {}", e);
                    metrics.records_in_db = None;
                }
            }

            // Calculate deferred (pending) geocoding counts
            match receiver_repo
                .get_receivers_needing_geocoding(i64::MAX)
                .await
            {
                Ok(pending_receivers) => {
                    let pending_count = pending_receivers.len();
                    if pending_count > 0 {
                        metrics.deferred_count = Some(pending_count);
                        // Get total receivers with valid coordinates for percentage calculation
                        if let Ok(total_with_coords) =
                            receiver_repo.get_receivers_with_coordinates_count().await
                        {
                            metrics.deferred_total = Some(total_with_coords as usize);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to get pending geocoding count: {}", e);
                }
            }

            // Don't mark the stage as failed - geocoding failures are normal
            metrics.success = true;

            // Track failed receivers for the summary email
            if failures > 0 {
                metrics.failed_items = Some(failed_callsigns);
                info!("{} receivers failed to geocode (this is normal)", failures);
            }
        }
        Err(e) => {
            // Only mark as failed if there's a critical error (e.g., database connection issue)
            error!(error = %e, "Failed to geocode receivers");
            metrics.success = false;
            metrics.error_message = Some(e.to_string());
        }
    }

    metrics.duration_secs = start.elapsed().as_secs_f64();
    metrics
}
