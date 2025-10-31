use anyhow::Result;
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

use crate::email_reporter::EntityMetrics;
use crate::geocoding::Geocoder;
use crate::receiver_repo::ReceiverRepository;

/// Geocode receivers that haven't been geocoded yet
/// Processes ALL receivers where geocoded=false and lat/lng are not null
/// Rate limited to 1 request per second to respect Nominatim usage policy
pub async fn geocode_receivers(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> Result<(usize, usize, usize)> {
    info!("Starting receiver reverse geocoding task...");

    let receiver_repo = ReceiverRepository::new(diesel_pool);
    let geocoder = Geocoder::new();

    // Get ALL receivers that need geocoding
    let receivers = receiver_repo
        .get_receivers_needing_geocoding(i64::MAX)
        .await?;

    if receivers.is_empty() {
        info!("No receivers need geocoding");
        return Ok((0, 0, 0));
    }

    info!("Found {} receivers needing geocoding", receivers.len());

    let mut success_count = 0;
    let mut failure_count = 0;

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
                        }
                    }
                    Err(e) => {
                        error!(
                            "Failed to update receiver {} with geocoded address: {}",
                            receiver.callsign, e
                        );
                        failure_count += 1;
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to reverse geocode receiver {} at ({}, {}): {}",
                    receiver.callsign, lat, lng, e
                );
                failure_count += 1;
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

    Ok((receivers.len(), success_count, failure_count))
}

/// Geocode receivers with metrics for email reporting
pub async fn geocode_receivers_with_metrics(
    diesel_pool: Pool<ConnectionManager<PgConnection>>,
) -> EntityMetrics {
    let start = Instant::now();
    let mut metrics = EntityMetrics::new("Receiver Geocoding");

    match geocode_receivers(diesel_pool).await {
        Ok((total_processed, success, failures)) => {
            metrics.records_loaded = success;
            metrics.records_in_db = Some(total_processed as i64);
            metrics.success = failures == 0; // Only fully successful if no failures
            if failures > 0 {
                metrics.error_message = Some(format!("{} receivers failed to geocode", failures));
            }
        }
        Err(e) => {
            error!("Failed to geocode receivers: {}", e);
            metrics.success = false;
            metrics.error_message = Some(e.to_string());
        }
    }

    metrics.duration_secs = start.elapsed().as_secs_f64();
    metrics
}
