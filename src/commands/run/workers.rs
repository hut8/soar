use chrono::DateTime;
use ogn_parser::AprsPacket;
use soar::adsb_accumulator::AdsbAccumulator;
use soar::aircraft_repo::AircraftRepository;
use soar::fix_processor::FixProcessor;
use soar::packet_processors::{
    AircraftPositionProcessor, PacketContext, PacketRouter, ReceiverPositionProcessor,
    ReceiverStatusProcessor, ServerStatusProcessor,
};
use soar::raw_messages_repo::RawMessagesRepository;
use std::sync::Arc;
use tracing::info;

/// Spawn intake queue processor for OGN/APRS messages.
/// This task reads raw OGN/APRS messages from the intake queue and processes them.
/// Separating socket consumption from processing allows graceful shutdown.
pub(crate) fn spawn_ogn_intake_worker(
    ogn_intake_rx: flume::Receiver<(DateTime<chrono::Utc>, String)>,
    packet_router: PacketRouter,
) {
    tokio::spawn(async move {
        info!("Intake queue processor started");
        let mut messages_processed = 0u64;
        while let Ok((received_at, message)) = ogn_intake_rx.recv_async().await {
            // Note: No tracing spans here - they cause trace accumulation in Tempo
            // Use metrics only for observability in the hot path
            metrics::gauge!("worker.active", "type" => "intake").increment(1.0);
            super::aprs::process_aprs_message(received_at, &message, &packet_router).await;
            messages_processed += 1;
            metrics::counter!("aprs.intake.processed_total").increment(1);
            metrics::gauge!("worker.active", "type" => "intake").decrement(1.0);

            // Update intake queue depth metric
            metrics::gauge!("aprs.intake_queue.depth").set(ogn_intake_rx.len() as f64);
        }
        info!(
            "Intake queue processor stopped after processing {} messages",
            messages_processed
        );
    });
    info!("Spawned intake queue processor");
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
    let num_beast_workers = 200;
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

                // Update SBS intake queue depth metric (sample from each worker)
                metrics::gauge!("sbs.intake_queue.depth").set(sbs_intake_rx.len() as f64);
            }
            info!("SBS intake queue worker {} stopped", worker_id);
        });
    }
    info!("Spawned {} SBS intake queue workers", num_sbs_workers);
}

/// Spawn dedicated worker pools for each APRS processor type.
/// Each pool has a different number of workers based on traffic volume and processing cost.
pub(crate) fn spawn_aprs_processor_workers(
    aircraft: (
        flume::Receiver<(AprsPacket, PacketContext)>,
        AircraftPositionProcessor,
    ),
    receiver_status: (
        flume::Receiver<(AprsPacket, PacketContext)>,
        ReceiverStatusProcessor,
    ),
    receiver_position: (
        flume::Receiver<(AprsPacket, PacketContext)>,
        ReceiverPositionProcessor,
    ),
    server_status: (
        flume::Receiver<(String, chrono::DateTime<chrono::Utc>)>,
        ServerStatusProcessor,
    ),
) {
    let (aircraft_rx, aircraft_position_processor) = aircraft;
    let (receiver_status_rx, receiver_status_processor) = receiver_status;
    let (receiver_position_rx, receiver_position_processor) = receiver_position;
    let (server_status_rx, server_status_processor) = server_status;
    // Aircraft position workers (80 workers - heaviest processing due to FixProcessor + flight tracking)
    // Increased from 20 to 80 because aircraft queue was constantly full at 1,000 capacity
    // Most APRS messages (~80-90%) are aircraft positions, so this queue needs the most workers
    let num_aircraft_workers = 80;
    info!(
        "Spawning {} aircraft position workers",
        num_aircraft_workers
    );
    for _worker_id in 0..num_aircraft_workers {
        let worker_rx = aircraft_rx.clone();
        let processor = aircraft_position_processor.clone();
        tokio::spawn(async move {
            while let Ok((packet, context)) = worker_rx.recv_async().await {
                // Note: No tracing spans here - they cause trace accumulation in Tempo
                metrics::gauge!("worker.active", "type" => "aircraft").increment(1.0);
                let start = std::time::Instant::now();
                processor.process_aircraft_position(&packet, context).await;
                let duration = start.elapsed();
                metrics::histogram!("aprs.aircraft.duration_ms")
                    .record(duration.as_millis() as f64);
                metrics::counter!("aprs.aircraft.processed_total").increment(1);
                metrics::counter!("aprs.messages.processed.aircraft_total").increment(1);
                metrics::counter!("aprs.messages.processed.total_total").increment(1);
                metrics::gauge!("worker.active", "type" => "aircraft").decrement(1.0);
            }
        });
    }

    // Receiver status workers (6 workers - medium processing)
    let num_receiver_status_workers = 6;
    info!(
        "Spawning {} receiver status workers",
        num_receiver_status_workers
    );
    for _worker_id in 0..num_receiver_status_workers {
        let worker_rx = receiver_status_rx.clone();
        let processor = receiver_status_processor.clone();
        tokio::spawn(async move {
            while let Ok((packet, context)) = worker_rx.recv_async().await {
                // Note: No tracing spans here - they cause trace accumulation in Tempo
                metrics::gauge!("worker.active", "type" => "receiver_status").increment(1.0);
                let start = std::time::Instant::now();
                processor.process_status_packet(&packet, context).await;
                let duration = start.elapsed();
                metrics::histogram!("aprs.receiver_status.duration_ms")
                    .record(duration.as_millis() as f64);
                metrics::counter!("aprs.receiver_status.processed_total").increment(1);
                metrics::counter!("aprs.messages.processed.receiver_status_total").increment(1);
                metrics::counter!("aprs.messages.processed.total_total").increment(1);
                metrics::gauge!("worker.active", "type" => "receiver_status").decrement(1.0);
            }
        });
    }

    // Receiver position workers (4 workers - light processing)
    let num_receiver_position_workers = 4;
    info!(
        "Spawning {} receiver position workers",
        num_receiver_position_workers
    );
    for _worker_id in 0..num_receiver_position_workers {
        let worker_rx = receiver_position_rx.clone();
        let processor = receiver_position_processor.clone();
        tokio::spawn(async move {
            while let Ok((packet, context)) = worker_rx.recv_async().await {
                // Note: No tracing spans here - they cause trace accumulation in Tempo
                metrics::gauge!("worker.active", "type" => "receiver_position").increment(1.0);
                let start = std::time::Instant::now();
                processor.process_receiver_position(&packet, context).await;
                let duration = start.elapsed();
                metrics::histogram!("aprs.receiver_position.duration_ms")
                    .record(duration.as_millis() as f64);
                metrics::counter!("aprs.receiver_position.processed_total").increment(1);
                metrics::counter!("aprs.messages.processed.receiver_position_total").increment(1);
                metrics::counter!("aprs.messages.processed.total_total").increment(1);
                metrics::gauge!("worker.active", "type" => "receiver_position").decrement(1.0);
            }
        });
    }

    // Server status workers (2 workers - very light processing)
    info!("Spawning 2 server status workers");
    for _worker_id in 0..2 {
        let worker_rx = server_status_rx.clone();
        let processor = server_status_processor.clone();
        tokio::spawn(async move {
            while let Ok((message, received_at)) = worker_rx.recv_async().await {
                // Note: No tracing spans here - they cause trace accumulation in Tempo
                metrics::gauge!("worker.active", "type" => "server_status").increment(1.0);
                let start = std::time::Instant::now();
                processor
                    .process_server_message(&message, received_at)
                    .await;
                let duration = start.elapsed();
                metrics::histogram!("aprs.server_status.duration_ms")
                    .record(duration.as_millis() as f64);
                metrics::counter!("aprs.server_status.processed_total").increment(1);
                metrics::counter!("aprs.messages.processed.server_total").increment(1);
                metrics::counter!("aprs.messages.processed.total_total").increment(1);
                metrics::gauge!("worker.active", "type" => "server_status").decrement(1.0);
            }
        });
    }
}
