use chrono::DateTime;
use soar::fix_processor::FixProcessor;
use soar::ogn::{
    OgnGenericProcessor, ReceiverPositionProcessor, ReceiverStatusProcessor, ServerStatusProcessor,
};

/// Process a received APRS message: parse, archive, and route inline.
///
/// Wraps the shared `soar::ogn::process_ogn_message` with production metrics
/// (lag, parse counts, latency histogram, per-type counters).
// Note: Intentionally NOT using #[tracing::instrument] here - it causes trace accumulation
// in Tempo because spawned tasks inherit trace context and all messages end up in one huge trace.
pub(crate) async fn process_aprs_message(
    received_at: DateTime<chrono::Utc>,
    message: &str,
    generic_processor: &OgnGenericProcessor,
    fix_processor: &FixProcessor,
    receiver_status_processor: &ReceiverStatusProcessor,
    receiver_position_processor: &ReceiverPositionProcessor,
    server_status_processor: &ServerStatusProcessor,
) {
    let start_time = std::time::Instant::now();

    metrics::counter!("aprs.process_aprs_message.called_total").increment(1);

    // Calculate and record lag (difference between now and packet timestamp)
    let now = chrono::Utc::now();
    let lag_seconds = (now - received_at).num_milliseconds() as f64 / 1000.0;
    metrics::gauge!("aprs.lag_seconds").set(lag_seconds);

    // Delegate to the shared OGN processing logic
    let processed = soar::ogn::process_ogn_message(
        received_at,
        message,
        generic_processor,
        fix_processor,
        receiver_status_processor,
        receiver_position_processor,
        server_status_processor,
    )
    .await;

    if processed {
        metrics::counter!("aprs.messages.processed_total").increment(1);
    } else {
        metrics::counter!("aprs.parse.failed_total").increment(1);
    }

    // Record processing latency
    let elapsed_micros = start_time.elapsed().as_micros() as f64 / 1000.0;
    metrics::histogram!("aprs.message_processing_latency_ms").record(elapsed_micros);
}
