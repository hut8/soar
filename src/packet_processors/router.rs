use super::generic::{GenericProcessor, PacketContext};
use ogn_parser::{AprsData, AprsPacket, PositionSourceType};
use tracing::{Instrument, debug, trace, warn};

/// Task representing a message to be processed by PacketRouter workers
enum MessageTask {
    /// APRS packet to process
    Packet {
        packet: Box<AprsPacket>,
        raw_message: String,
        received_at: chrono::DateTime<chrono::Utc>,
    },
    /// Server message to process
    ServerMessage {
        raw_message: String,
        received_at: chrono::DateTime<chrono::Utc>,
    },
}

/// PacketRouter routes packets to appropriate specialized processor queues
/// Contains an internal worker pool that processes messages in parallel
#[derive(Clone)]
pub struct PacketRouter {
    /// Generic processor for archiving, receiver identification, and APRS message insertion
    generic_processor: GenericProcessor,
    /// Internal queue for message tasks (1000 capacity)
    internal_queue_tx: flume::Sender<MessageTask>,
    /// Internal queue receiver (used once during start())
    #[allow(clippy::type_complexity)]
    internal_queue_rx: Option<flume::Receiver<MessageTask>>,
    /// Optional channel sender for aircraft position packets
    aircraft_position_tx: Option<flume::Sender<(AprsPacket, PacketContext)>>,
    /// Optional channel sender for receiver status packets
    receiver_status_tx: Option<flume::Sender<(AprsPacket, PacketContext)>>,
    /// Optional channel sender for receiver position packets
    receiver_position_tx: Option<flume::Sender<(AprsPacket, PacketContext)>>,
    /// Optional channel sender for server status messages (message, received_at timestamp)
    server_status_tx: Option<flume::Sender<(String, chrono::DateTime<chrono::Utc>)>>,
}

impl PacketRouter {
    /// Create a new PacketRouter with a generic processor
    ///
    /// Creates an internal queue of 1000 message tasks.
    /// Workers are spawned when start() is called after configuration.
    pub fn new(generic_processor: GenericProcessor) -> Self {
        const INTERNAL_QUEUE_SIZE: usize = 1_000;

        let (internal_queue_tx, internal_queue_rx) =
            flume::bounded::<MessageTask>(INTERNAL_QUEUE_SIZE);

        Self {
            generic_processor,
            internal_queue_tx,
            internal_queue_rx: Some(internal_queue_rx),
            aircraft_position_tx: None,
            receiver_status_tx: None,
            receiver_position_tx: None,
            server_status_tx: None,
        }
    }

    /// Start the worker pool after all queues have been configured
    ///
    /// This MUST be called after all with_*_queue() methods to ensure
    /// workers have access to the configured queues.
    pub fn start(mut self, num_workers: usize) -> Self {
        // Take the receiver (can only be done once)
        let internal_queue_rx = self
            .internal_queue_rx
            .take()
            .expect("start() can only be called once");

        // Spawn workers with the fully configured router
        self.spawn_workers(internal_queue_rx, num_workers);

        self
    }

    /// Spawn worker pool to process messages from internal queue
    fn spawn_workers(&self, internal_queue_rx: flume::Receiver<MessageTask>, num_workers: usize) {
        for worker_id in 0..num_workers {
            let rx = internal_queue_rx.clone();
            let router = self.clone();

            tokio::spawn(
                async move {
                    tracing::info!("PacketRouter worker {} started", worker_id);
                    while let Ok(task) = rx.recv_async().await {
                        match task {
                            MessageTask::Packet {
                                packet,
                                raw_message,
                                received_at,
                            } => {
                                router
                                    .process_packet_internal(*packet, &raw_message, received_at)
                                    .await;
                            }
                            MessageTask::ServerMessage {
                                raw_message,
                                received_at,
                            } => {
                                router
                                    .process_server_message_internal(&raw_message, received_at)
                                    .await;
                            }
                        }

                        // Update internal queue depth metric
                        metrics::gauge!("aprs.router.internal_queue_depth").set(rx.len() as f64);
                    }
                    tracing::info!("PacketRouter worker {} stopped", worker_id);
                }
                .instrument(tracing::info_span!("router_worker", worker_id)),
            );
        }

        tracing::info!("Spawned {} PacketRouter workers", num_workers);
    }

    /// Add aircraft position queue sender
    pub fn with_aircraft_position_queue(
        mut self,
        sender: flume::Sender<(AprsPacket, PacketContext)>,
    ) -> Self {
        self.aircraft_position_tx = Some(sender);
        self
    }

    /// Add receiver status queue sender
    pub fn with_receiver_status_queue(
        mut self,
        sender: flume::Sender<(AprsPacket, PacketContext)>,
    ) -> Self {
        self.receiver_status_tx = Some(sender);
        self
    }

    /// Add receiver position queue sender
    pub fn with_receiver_position_queue(
        mut self,
        sender: flume::Sender<(AprsPacket, PacketContext)>,
    ) -> Self {
        self.receiver_position_tx = Some(sender);
        self
    }

    /// Add server status queue sender
    pub fn with_server_status_queue(
        mut self,
        sender: flume::Sender<(String, chrono::DateTime<chrono::Utc>)>,
    ) -> Self {
        self.server_status_tx = Some(sender);
        self
    }
}

impl PacketRouter {
    /// Enqueue a server message for processing (blocking)
    ///
    /// Messages are placed in internal queue and processed by worker pool.
    /// This method will block until space is available in the queue.
    pub async fn process_server_message(
        &self,
        raw_message: &str,
        received_at: chrono::DateTime<chrono::Utc>,
    ) {
        let task = MessageTask::ServerMessage {
            raw_message: raw_message.to_string(),
            received_at,
        };

        // Block until space is available - never drop messages
        if let Err(e) = self.internal_queue_tx.send_async(task).await {
            warn!(
                "PacketRouter internal queue disconnected, cannot send server message: {}",
                e
            );
            metrics::counter!("aprs.router.internal_queue_disconnected_total").increment(1);
        }
    }

    /// Internal worker method to process a server message
    async fn process_server_message_internal(
        &self,
        raw_message: &str,
        received_at: chrono::DateTime<chrono::Utc>,
    ) {
        // Step 1: Archive via GenericProcessor
        self.generic_processor
            .process_server_message(raw_message)
            .await;

        // Step 2: Route to server status queue if configured
        if let Some(tx) = &self.server_status_tx {
            let message_with_timestamp = (raw_message.to_string(), received_at);
            if let Err(e) = tx.send_async(message_with_timestamp).await {
                warn!(
                    "Server status queue CLOSED - cannot route server message: {}",
                    e
                );
                metrics::counter!("aprs.server_status_queue.closed_total").increment(1);
            } else {
                trace!("Routed server message to queue");
            }
        } else {
            trace!("No server status queue configured, server message archived only");
        }
    }

    /// Enqueue an APRS packet for processing (blocking)
    ///
    /// Packets are placed in internal queue and processed by worker pool.
    /// This method will block until space is available in the queue.
    pub async fn process_packet(
        &self,
        packet: AprsPacket,
        raw_message: &str,
        received_at: chrono::DateTime<chrono::Utc>,
    ) {
        let task = MessageTask::Packet {
            packet: Box::new(packet),
            raw_message: raw_message.to_string(),
            received_at,
        };

        // Block until space is available - never drop packets
        if let Err(e) = self.internal_queue_tx.send_async(task).await {
            warn!(
                "PacketRouter internal queue disconnected, cannot send packet: {}",
                e
            );
            metrics::counter!("aprs.router.internal_queue_disconnected_total").increment(1);
        }
    }

    /// Internal worker method to process an APRS packet
    ///
    /// 1. GenericProcessor archives and inserts to database
    /// 2. Route to appropriate queue based on packet type
    async fn process_packet_internal(
        &self,
        packet: AprsPacket,
        raw_message: &str,
        received_at: chrono::DateTime<chrono::Utc>,
    ) {
        metrics::counter!("aprs.router.process_packet_internal.called_total").increment(1);

        // Step 1: Generic processing - archives, identifies receiver, inserts APRS message
        let context = match self
            .generic_processor
            .process_packet(&packet, raw_message, received_at)
            .await
        {
            Some(ctx) => {
                metrics::counter!("aprs.router.generic_processor.success_total").increment(1);
                ctx
            }
            None => {
                warn!(
                    "Generic processing failed for packet from {}, skipping routing",
                    packet.from
                );
                metrics::counter!("aprs.router.generic_processor.failed_total").increment(1);
                return;
            }
        };

        // Step 2: Route to appropriate queue based on packet type
        let packet_from = packet.from.clone();
        let position_source = packet.position_source_type();

        match &packet.data {
            AprsData::Position(_) => {
                metrics::counter!("aprs.router.packet_type.position_total").increment(1);
                // Route based on position source type
                match position_source {
                    PositionSourceType::Aircraft => {
                        metrics::counter!("aprs.router.position_source.aircraft_total")
                            .increment(1);
                        // Route to aircraft position queue
                        if let Some(tx) = &self.aircraft_position_tx {
                            if let Err(e) = tx.send_async((packet, context)).await {
                                warn!(
                                    "Aircraft position queue CLOSED - cannot route packet from {}: {}",
                                    packet_from, e
                                );
                                metrics::counter!("aprs.aircraft_queue.closed_total").increment(1);
                            } else {
                                trace!("Routed aircraft position to queue");
                                metrics::counter!("aprs.router.routed.aircraft_total").increment(1);
                            }
                        } else {
                            trace!("No aircraft position queue configured, packet archived only");
                            metrics::counter!("aprs.router.no_queue.aircraft_total").increment(1);
                        }
                    }
                    PositionSourceType::Receiver => {
                        // Route to receiver position queue
                        if let Some(tx) = &self.receiver_position_tx {
                            if let Err(e) = tx.send_async((packet, context)).await {
                                warn!(
                                    "Receiver position queue CLOSED - cannot route packet from {}: {}",
                                    packet_from, e
                                );
                                metrics::counter!("aprs.receiver_position_queue.closed_total")
                                    .increment(1);
                            } else {
                                trace!("Routed receiver position to queue");
                            }
                        } else {
                            trace!("No receiver position queue configured, packet archived only");
                        }
                    }
                    PositionSourceType::WeatherStation => {
                        trace!(
                            "Position from weather station {} - archived only",
                            packet_from
                        );
                    }
                    source_type => {
                        trace!(
                            "Position from unknown source type {:?} from {} - archived only",
                            source_type, packet_from
                        );
                    }
                }
            }
            AprsData::Status(_) => {
                metrics::counter!("aprs.router.packet_type.status_total").increment(1);
                // Route to receiver status queue
                trace!(
                    "Received status packet from {} (source type: {:?})",
                    packet_from, position_source
                );
                if let Some(tx) = &self.receiver_status_tx {
                    if let Err(e) = tx.send_async((packet, context)).await {
                        warn!(
                            "Receiver status queue CLOSED - cannot route packet from {}: {}",
                            packet_from, e
                        );
                        metrics::counter!("aprs.receiver_status_queue.closed_total").increment(1);
                    } else {
                        trace!("Routed receiver status to queue");
                    }
                } else {
                    trace!("No receiver status queue configured, packet archived only");
                }
            }
            _ => {
                debug!(
                    "Received packet of type {:?}, no specific handler - archived only",
                    std::mem::discriminant(&packet.data)
                );
            }
        }
    }
}
