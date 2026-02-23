use chrono::DateTime;
use soar::adsb_accumulator::AdsbAccumulator;
use soar::aircraft_repo::AircraftRepository;
use soar::fix_processor::FixProcessor;
use soar::ogn::{
    OgnGenericProcessor, ReceiverPositionProcessor, ReceiverStatusProcessor, ServerStatusProcessor,
};
use soar::raw_messages_repo::RawMessagesRepository;
use std::sync::Arc;
use tracing::info;

/// Spawn OGN intake queue workers for parallel processing of OGN/APRS messages.
/// Each worker parses, archives, and processes messages inline — no intermediate queues.
/// Modeled after Beast intake workers for consistent architecture.
pub(crate) fn spawn_ogn_intake_workers(
    ogn_intake_rx: flume::Receiver<(DateTime<chrono::Utc>, String)>,
    generic_processor: &OgnGenericProcessor,
    fix_processor: &FixProcessor,
    receiver_status_processor: &ReceiverStatusProcessor,
    receiver_position_processor: &ReceiverPositionProcessor,
    server_status_processor: &ServerStatusProcessor,
    num_workers: usize,
) {
    info!("Spawning {} OGN intake queue workers", num_workers);

    for worker_id in 0..num_workers {
        let worker_rx = ogn_intake_rx.clone();
        let worker_generic = generic_processor.clone();
        let worker_fix = fix_processor.clone();
        let worker_receiver_status = receiver_status_processor.clone();
        let worker_receiver_position = receiver_position_processor.clone();
        let worker_server_status = server_status_processor.clone();

        tokio::spawn(async move {
            while let Ok((received_at, message)) = worker_rx.recv_async().await {
                // Note: No tracing spans here - they cause trace accumulation in Tempo
                metrics::gauge!("worker.active", "type" => "ogn_intake").increment(1.0);
                super::aprs::process_aprs_message(
                    received_at,
                    &message,
                    &worker_generic,
                    &worker_fix,
                    &worker_receiver_status,
                    &worker_receiver_position,
                    &worker_server_status,
                )
                .await;
                metrics::counter!("aprs.intake.processed_total").increment(1);
                metrics::gauge!("worker.active", "type" => "ogn_intake").decrement(1.0);

                // Update intake queue depth metric (sample from a single worker to reduce contention)
                if worker_id == 0 {
                    metrics::gauge!("aprs.intake_queue.depth").set(worker_rx.len() as f64);
                }
            }
            info!("OGN intake queue worker {} stopped", worker_id);
        });
    }
    info!("Spawned {} OGN intake queue workers", num_workers);
}

/// Spawn Beast intake queue workers for parallel processing of Beast (binary) messages.
/// Beast message processing involves database operations (aircraft lookup, raw message storage)
/// and state accumulation, so we need multiple workers to handle high traffic volumes.
/// Using 200 workers: ADS-B traffic is ~30,000 msg/sec vs OGN's ~300 msg/sec (100x more)
/// With 200 workers at ~150 msg/sec per worker, we can handle up to 30k msg/sec
pub(crate) fn spawn_beast_intake_workers(
    beast_intake_rx: flume::Receiver<(DateTime<chrono::Utc>, Vec<u8>)>,
    aircraft_repo: &AircraftRepository,
    beast_repo: &RawMessagesRepository,
    fix_processor: &FixProcessor,
    accumulator: &Arc<AdsbAccumulator>,
) {
    let num_beast_workers = 50;
    info!("Spawning {} Beast intake queue workers", num_beast_workers);

    for worker_id in 0..num_beast_workers {
        let beast_aircraft_repo = aircraft_repo.clone();
        let beast_repo_clone = beast_repo.clone();
        let beast_fix_processor = fix_processor.clone();
        let beast_accumulator = accumulator.clone();
        let beast_intake_rx = beast_intake_rx.clone();

        tokio::spawn(async move {
            while let Ok((received_at, raw_frame)) = beast_intake_rx.recv_async().await {
                // Note: No tracing spans here - they cause trace accumulation in Tempo
                metrics::gauge!("worker.active", "type" => "beast_intake").increment(1.0);
                let start_time = std::time::Instant::now();
                super::beast::process_beast_message(
                    received_at,
                    &raw_frame,
                    &beast_aircraft_repo,
                    &beast_repo_clone,
                    &beast_fix_processor,
                    &beast_accumulator,
                )
                .await;

                let duration = start_time.elapsed();
                metrics::histogram!("beast.run.process_message_duration_ms")
                    .record(duration.as_millis() as f64);
                metrics::counter!("beast.run.intake.processed_total").increment(1);
                metrics::gauge!("worker.active", "type" => "beast_intake").decrement(1.0);

                // Update Beast intake queue depth metric (sample from each worker)
                metrics::gauge!("beast.intake_queue.depth").set(beast_intake_rx.len() as f64);
            }
            info!("Beast intake queue worker {} stopped", worker_id);
        });
    }
    info!("Spawned {} Beast intake queue workers", num_beast_workers);
}

/// Spawn SBS intake queue workers for parallel processing of SBS (BaseStation) CSV messages.
/// SBS typically has lower traffic than Beast, so fewer workers are needed.
/// SBS shares the same accumulator with Beast for consistent state tracking.
pub(crate) fn spawn_sbs_intake_workers(
    sbs_intake_rx: flume::Receiver<(DateTime<chrono::Utc>, Vec<u8>)>,
    aircraft_repo: &AircraftRepository,
    sbs_repo: &RawMessagesRepository,
    fix_processor: &FixProcessor,
    accumulator: &Arc<AdsbAccumulator>,
) {
    let num_sbs_workers = 50;
    info!("Spawning {} SBS intake queue workers", num_sbs_workers);

    for worker_id in 0..num_sbs_workers {
        let sbs_aircraft_repo = aircraft_repo.clone();
        let sbs_repo = sbs_repo.clone();
        let sbs_fix_processor = fix_processor.clone();
        let sbs_accumulator = accumulator.clone();
        let sbs_intake_rx = sbs_intake_rx.clone();

        tokio::spawn(async move {
            while let Ok((received_at, csv_bytes)) = sbs_intake_rx.recv_async().await {
                // Note: No tracing spans here - they cause trace accumulation in Tempo
                metrics::gauge!("worker.active", "type" => "sbs_intake").increment(1.0);
                let start_time = std::time::Instant::now();
                super::sbs::process_sbs_message(
                    received_at,
                    &csv_bytes,
                    &sbs_aircraft_repo,
                    &sbs_repo,
                    &sbs_fix_processor,
                    &sbs_accumulator,
                )
                .await;

                let duration = start_time.elapsed();
                metrics::histogram!("sbs.run.process_message_duration_ms")
                    .record(duration.as_millis() as f64);
                metrics::counter!("sbs.run.intake.processed_total").increment(1);
                metrics::gauge!("worker.active", "type" => "sbs_intake").decrement(1.0);

                // Update SBS intake queue depth metric (sample from each worker)
                metrics::gauge!("sbs.intake_queue.depth").set(sbs_intake_rx.len() as f64);
            }
            info!("SBS intake queue worker {} stopped", worker_id);
        });
    }
    info!("Spawned {} SBS intake queue workers", num_sbs_workers);
}
