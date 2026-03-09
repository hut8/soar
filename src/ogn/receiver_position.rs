use crate::flights::haversine_distance;
use crate::ogn::packet_context::PacketContext;
use crate::receiver_repo::ReceiverRepository;
use moka::sync::Cache;
use num_traits::AsPrimitive;
use ogn_parser::{AprsData, AprsPacket};
use std::sync::Arc;
use tracing::{error, trace, warn};

/// Minimum distance in meters for a receiver position change to trigger a warning.
/// Receivers are stationary, so any move beyond this likely indicates a misconfiguration,
/// a different physical receiver reusing the same callsign, or a gateway.
const RECEIVER_MOVE_WARNING_THRESHOLD_M: f64 = 5_000.0;

/// Processor for handling receiver position packets
#[derive(Clone)]
pub struct ReceiverPositionProcessor {
    /// Repository for updating receiver locations
    receiver_repo: ReceiverRepository,
    /// Cache of last-known receiver positions (callsign -> (lat, lon))
    /// Used to detect significant position changes without DB lookups
    position_cache: Arc<Cache<String, (f64, f64)>>,
}

impl ReceiverPositionProcessor {
    /// Create a new ReceiverPositionProcessor
    pub fn new(receiver_repo: ReceiverRepository) -> Self {
        let position_cache = Cache::builder()
            .max_capacity(100_000)
            .time_to_live(std::time::Duration::from_secs(86400))
            .build();

        Self {
            receiver_repo,
            position_cache: Arc::new(position_cache),
        }
    }

    /// Process a receiver position packet and update its location
    /// Note: Receiver is guaranteed to exist in database (created by OgnGenericProcessor)
    #[tracing::instrument(skip(self, packet, _context), fields(callsign = %packet.from))]
    pub async fn process_receiver_position(&self, packet: &AprsPacket, _context: PacketContext) {
        // Extract position data from packet
        if let AprsData::Position(position) = &packet.data {
            let callsign = packet.from.to_string();
            let new_lat: f64 = position.latitude.as_();
            let new_lon: f64 = position.longitude.as_();

            // Check if we've seen this receiver before and warn if it moved significantly
            if let Some((old_lat, old_lon)) = self.position_cache.get(&callsign) {
                let distance_m = haversine_distance(old_lat, old_lon, new_lat, new_lon);
                if distance_m > RECEIVER_MOVE_WARNING_THRESHOLD_M {
                    warn!(
                        "Receiver {} moved {:.1} km: ({:.4}, {:.4}) -> ({:.4}, {:.4})",
                        callsign,
                        distance_m / 1000.0,
                        old_lat,
                        old_lon,
                        new_lat,
                        new_lon,
                    );
                    metrics::counter!("aprs.receiver.position_moved_total").increment(1);
                }
            }

            // Update cache with new position
            self.position_cache
                .insert(callsign.clone(), (new_lat, new_lon));

            // Update receiver position in database
            match self
                .receiver_repo
                .update_receiver_position(&callsign, new_lat, new_lon)
                .await
            {
                Ok(updated) => {
                    if updated {
                        trace!(
                            "Successfully updated position for receiver {}: ({}, {})",
                            callsign, new_lat, new_lon
                        );
                    } else {
                        // This shouldn't happen since OgnGenericProcessor ensures receiver exists
                        warn!(
                            "Receiver {} not found when updating position - this indicates a race condition or database issue",
                            callsign
                        );
                    }
                }
                Err(e) => {
                    error!(callsign = %callsign, error = %e, "Failed to update position for receiver");
                }
            }
        } else {
            warn!(
                "Expected position packet from receiver {} but got different packet type",
                packet.from
            );
        }
    }
}
