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
/// Medium queue (1,000 messages) - balances throughput with crash loss
/// JetStream provides durable queuing upstream of this queue
/// Increased from 100 to 1,000 to improve throughput for high-volume processing
pub const AIRCRAFT_QUEUE_SIZE: usize = 1_000;

/// Receiver status queue (medium volume)
/// Small queue (100 messages) - minimizes message loss during crashes
/// JetStream handles durability, this is just for worker coordination
/// Reduced from 10K to limit crash loss
pub const RECEIVER_STATUS_QUEUE_SIZE: usize = 100;

/// Receiver position queue (lower volume)
/// Small queue (100 messages) - minimizes message loss during crashes
/// JetStream handles durability, this is just for worker coordination
/// Reduced from 5K to limit crash loss
pub const RECEIVER_POSITION_QUEUE_SIZE: usize = 100;

/// Server status queue (lowest volume)
/// Small queue (100 messages) - minimizes message loss during crashes
/// JetStream handles durability, this is just for worker coordination
/// Reduced from 1K to limit crash loss
pub const SERVER_STATUS_QUEUE_SIZE: usize = 100;

/// Elevation lookup queue for Google Maps API requests
/// Very large queue (100,000 tasks) to prevent blocking fix processing
/// Changed from 50K to 100K and switched to blocking send to never drop elevation tasks
/// Uses blocking send - applies backpressure if queue fills but never drops messages
pub const ELEVATION_QUEUE_SIZE: usize = 100_000;

/// AGL database lookup queue for batch writes to database
/// Small queue (100 tasks) - minimizes message loss during crashes
/// Reduced from 10K since AGL is optional data (can be recalculated)
pub const AGL_DATABASE_QUEUE_SIZE: usize = 100;

/// JetStream intake queue for buffering raw APRS messages from JetStream
/// Medium queue (1,000 messages) to buffer between JetStream consumer and processing
/// Allows graceful shutdown by stopping JetStream reads and draining this queue
/// At ~30 msg/s current rate, this provides ~33 seconds of buffering
pub const JETSTREAM_INTAKE_QUEUE_SIZE: usize = 1_000;

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
