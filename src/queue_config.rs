//! Centralized queue size configuration for all MPSC channels in the application
//!
//! This module defines queue sizes and helper functions for all message-passing channels
//! used throughout the SOAR system. By centralizing these values, we ensure:
//! - Consistent queue sizing across the application
//! - Easy tuning of queue depths for performance optimization
//! - Clear documentation of why each queue has its specific size

// ============================================================================
// NATS JetStream Configuration Constants
// ============================================================================

/// JetStream stream name for raw APRS messages (production)
pub const APRS_RAW_STREAM: &str = "APRS_RAW";

/// JetStream subject for raw APRS messages (production)
pub const APRS_RAW_SUBJECT: &str = "aprs.raw";

/// JetStream consumer name for soar-run (production)
pub const SOAR_RUN_CONSUMER: &str = "soar-run-production";

/// JetStream stream name for raw APRS messages (staging/development)
pub const APRS_RAW_STREAM_STAGING: &str = "STAGING_APRS_RAW";

/// JetStream subject for raw APRS messages (staging/development)
pub const APRS_RAW_SUBJECT_STAGING: &str = "staging.aprs.raw";

/// JetStream consumer name for soar-run (staging/development)
pub const SOAR_RUN_CONSUMER_STAGING: &str = "soar-run-staging";

// ============================================================================
// Queue Size Constants
// ============================================================================

/// Raw APRS message queue from APRS-IS connection to JetStream publisher
/// Medium queue (1000 messages) to buffer during JetStream publish latency
/// At ~500 msg/s, this represents ~2 seconds of buffering
/// Provides headroom for network latency while JetStream provides durability
pub const RAW_MESSAGE_QUEUE_SIZE: usize = 1000;

/// Archive queue for writing raw APRS messages to compressed files
/// Large queue (10,000 messages) since file I/O is slower and batched
/// At ~500 msg/s, this provides ~20 seconds of buffering for archive writes
pub const ARCHIVE_QUEUE_SIZE: usize = 10_000;

/// NATS publish queue for live aircraft position updates
/// Medium queue (1,000 messages) for real-time position broadcasts
/// Balances memory usage with resilience to temporary NATS slowdowns
pub const NATS_PUBLISH_QUEUE_SIZE: usize = 1_000;

/// Aircraft position processing queue (highest volume)
/// Large queue (50,000 messages) to handle bursts of aircraft position updates
/// At ~400 aircraft msgs/s, this provides ~125 seconds of buffering
/// Largest queue because aircraft positions are the most frequent message type
pub const AIRCRAFT_QUEUE_SIZE: usize = 50_000;

/// Receiver status queue (medium volume)
/// Medium queue (10,000 messages) for receiver status updates
/// At ~50 status msgs/s, this provides ~200 seconds of buffering
pub const RECEIVER_STATUS_QUEUE_SIZE: usize = 10_000;

/// Receiver position queue (lower volume)
/// Medium queue (5,000 messages) for receiver position updates
/// At ~10 position msgs/s, this provides ~500 seconds of buffering
pub const RECEIVER_POSITION_QUEUE_SIZE: usize = 5_000;

/// Server status queue (lowest volume)
/// Small queue (1,000 messages) for OGN server status messages
/// At ~1 msg/s, this provides ~1000 seconds of buffering
pub const SERVER_STATUS_QUEUE_SIZE: usize = 1_000;

/// Elevation lookup queue for Google Maps API requests
/// Small queue (1,000 tasks) since elevation lookups are API-rate-limited
/// Prevents excessive queuing of API requests
pub const ELEVATION_QUEUE_SIZE: usize = 1_000;

/// AGL database lookup queue for local terrain database queries
/// Large queue (10,000 tasks) since database lookups are fast but frequent
/// At ~100 lookups/s, this provides ~100 seconds of buffering
pub const AGL_DATABASE_QUEUE_SIZE: usize = 10_000;

/// Calculate the warning threshold for queue depth monitoring
///
/// Returns 80% of queue capacity as the warning threshold. When a queue
/// exceeds this threshold, warnings are logged to indicate potential backpressure.
///
/// # Arguments
/// * `size` - The maximum queue capacity
///
/// # Returns
/// The threshold at which warnings should be emitted (80% of capacity)
///
/// # Examples
/// ```
/// use soar::queue_config::queue_warning_threshold;
///
/// assert_eq!(queue_warning_threshold(100), 80);
/// assert_eq!(queue_warning_threshold(1000), 800);
/// ```
pub const fn queue_warning_threshold(size: usize) -> usize {
    (size * 80) / 100
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_warning_threshold() {
        assert_eq!(queue_warning_threshold(100), 80);
        assert_eq!(queue_warning_threshold(1_000), 800);
        assert_eq!(queue_warning_threshold(10_000), 8_000);
        assert_eq!(queue_warning_threshold(50_000), 40_000);
    }
}
