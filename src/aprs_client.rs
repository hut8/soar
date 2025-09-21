use crate::Fix;
use crate::receiver_status_repo::ReceiverStatusRepository;
use crate::receiver_statuses::NewReceiverStatus;
use anyhow::Result;

/// Result type for connection attempts
enum ConnectionResult {
    /// Connection was successful and ran until completion/disconnection
    Success,
    /// Connection failed immediately (couldn't establish connection)
    ConnectionFailed(anyhow::Error),
    /// Connection was established but failed during operation
    OperationFailed(anyhow::Error),
}
use ogn_parser::{AprsData, AprsPacket, PositionSourceType};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::trace;
use tracing::{debug, error, info, warn};

/// Trait for processing APRS packets
/// Implementors can define custom logic for handling received APRS packets
pub trait PacketProcessor: Send + Sync {
    /// Process a received APRS packet
    ///
    /// # Arguments
    /// * `packet` - The parsed APRS packet
    fn process_packet(&self, _packet: AprsPacket) {
        // Default implementation does nothing
    }

    /// Process a raw APRS message (optional - for logging, archiving, etc.)
    ///
    /// # Arguments
    /// * `raw_message` - The raw APRS message string received from the server
    fn process_raw_message(&self, _raw_message: &str) {
        // Default implementation does nothing
    }
}

/// Trait for processing position fixes extracted from APRS messages
pub trait FixProcessor: Send + Sync {
    /// Process a position fix
    ///
    /// # Arguments
    /// * `fix` - The position fix extracted from an APRS packet
    /// * `raw_message` - The raw APRS message that generated this fix
    fn process_fix(&self, fix: Fix, raw_message: &str);
}

/// Trait for processing APRS position messages
pub trait PositionProcessor: Send + Sync {
    /// Process an APRS position message
    ///
    /// # Arguments
    /// * `packet` - The complete APRS packet containing position data
    /// * `raw_message` - The raw APRS message string
    fn process_position(&self, packet: &AprsPacket, raw_message: &str);
}

/// Trait for processing APRS status messages
pub trait StatusProcessor: Send + Sync {
    /// Process an APRS status message
    ///
    /// # Arguments
    /// * `packet` - The complete APRS packet containing status data
    /// * `raw_message` - The raw APRS message string
    fn process_status(&self, packet: &AprsPacket, raw_message: &str);
}

/// Unified processor struct that contains all processor types
/// This simplifies APRS client construction by grouping all processors together
#[derive(Clone)]
pub struct AprsProcessors {
    /// Processor for general APRS packets and raw message handling
    pub packet_processor: Arc<dyn PacketProcessor>,
    /// Optional processor for position fixes (backward compatibility)
    pub fix_processor: Option<Arc<dyn FixProcessor>>,
    /// Optional processor for position messages
    pub position_processor: Option<Arc<dyn PositionProcessor>>,
    /// Optional processor for status messages
    pub status_processor: Option<Arc<dyn StatusProcessor>>,
}

impl AprsProcessors {
    /// Create a new AprsProcessors with just a packet processor
    pub fn new(packet_processor: Arc<dyn PacketProcessor>) -> Self {
        Self {
            packet_processor,
            fix_processor: None,
            position_processor: None,
            status_processor: None,
        }
    }

    /// Create a new AprsProcessors with a PacketRouter (recommended approach)
    /// This is the modern way to configure APRS processing
    pub fn with_packet_router(router: PacketRouter) -> Self {
        Self::new(Arc::new(router))
    }

    /// Create a new AprsProcessors with packet and fix processors
    pub fn with_fix_processor(
        packet_processor: Arc<dyn PacketProcessor>,
        fix_processor: Arc<dyn FixProcessor>,
    ) -> Self {
        Self {
            packet_processor,
            fix_processor: Some(fix_processor),
            position_processor: None,
            status_processor: None,
        }
    }

    /// Create a new AprsProcessors with packet, position, and status processors
    pub fn with_processors(
        packet_processor: Arc<dyn PacketProcessor>,
        position_processor: Option<Arc<dyn PositionProcessor>>,
        status_processor: Option<Arc<dyn StatusProcessor>>,
    ) -> Self {
        Self {
            packet_processor,
            fix_processor: None,
            position_processor,
            status_processor,
        }
    }

    /// Create a new AprsProcessors with all processor types
    pub fn with_all_processors(
        packet_processor: Arc<dyn PacketProcessor>,
        fix_processor: Option<Arc<dyn FixProcessor>>,
        position_processor: Option<Arc<dyn PositionProcessor>>,
        status_processor: Option<Arc<dyn StatusProcessor>>,
    ) -> Self {
        Self {
            packet_processor,
            fix_processor,
            position_processor,
            status_processor,
        }
    }

    /// Add a fix processor to existing processors
    pub fn add_fix_processor(mut self, fix_processor: Arc<dyn FixProcessor>) -> Self {
        self.fix_processor = Some(fix_processor);
        self
    }

    /// Add a position processor to existing processors
    pub fn add_position_processor(
        mut self,
        position_processor: Arc<dyn PositionProcessor>,
    ) -> Self {
        self.position_processor = Some(position_processor);
        self
    }

    /// Add a status processor to existing processors
    pub fn add_status_processor(mut self, status_processor: Arc<dyn StatusProcessor>) -> Self {
        self.status_processor = Some(status_processor);
        self
    }
}

/// Type alias for boxed packet processor trait objects
pub type BoxedPacketProcessor = Box<dyn PacketProcessor>;

/// Configuration for the APRS client
#[derive(Debug, Clone)]
pub struct AprsClientConfig {
    /// APRS server hostname
    pub server: String,
    /// APRS server port
    pub port: u16,
    /// Maximum number of connection retry attempts
    pub max_retries: u32,
    /// Callsign for authentication
    pub callsign: String,
    /// Password for authentication (optional)
    pub password: Option<String>,
    /// APRS filter string (optional)
    pub filter: Option<String>,
    /// Delay between reconnection attempts in seconds
    pub retry_delay_seconds: u64,
    /// Base directory for message archive (optional)
    pub archive_base_dir: Option<String>,
    /// Path to CSV log file for unparsed APRS fragments (optional)
    pub unparsed_log_path: Option<String>,
}

impl Default for AprsClientConfig {
    fn default() -> Self {
        Self {
            server: "aprs.glidernet.org".to_string(),
            port: 14580,
            max_retries: 5,
            callsign: "N0CALL".to_string(),
            password: None,
            filter: None,
            retry_delay_seconds: 5,
            archive_base_dir: None,
            unparsed_log_path: None,
        }
    }
}

/// APRS client that connects to an APRS-IS server via TCP
pub struct AprsClient {
    config: AprsClientConfig,
    processors: AprsProcessors,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl AprsClient {
    /// Create a new APRS client with unified processors
    /// This is the recommended constructor that takes an AprsProcessors struct
    pub fn new(config: AprsClientConfig, processors: AprsProcessors) -> Self {
        Self {
            config,
            processors,
            shutdown_tx: None,
        }
    }

    /// Create a new APRS client with a PacketRouter (modern recommended approach)
    /// This is the simplest way to create an APRS client with the new architecture
    pub fn with_packet_router(config: AprsClientConfig, router: PacketRouter) -> Self {
        Self::new(config, AprsProcessors::with_packet_router(router))
    }

    /// Create a new APRS client with just a packet processor (backward compatibility)
    pub fn new_with_packet_processor(
        config: AprsClientConfig,
        packet_processor: Arc<dyn PacketProcessor>,
    ) -> Self {
        Self {
            config,
            processors: AprsProcessors::new(packet_processor),
            shutdown_tx: None,
        }
    }

    /// Create a new APRS client with both packet and fix processors (backward compatibility)
    pub fn new_with_fix_processor(
        config: AprsClientConfig,
        packet_processor: Arc<dyn PacketProcessor>,
        fix_processor: Arc<dyn FixProcessor>,
    ) -> Self {
        Self {
            config,
            processors: AprsProcessors::with_fix_processor(packet_processor, fix_processor),
            shutdown_tx: None,
        }
    }

    /// Create a new APRS client with position and status processors (backward compatibility)
    pub fn new_with_processors(
        config: AprsClientConfig,
        packet_processor: Arc<dyn PacketProcessor>,
        position_processor: Option<Arc<dyn PositionProcessor>>,
        status_processor: Option<Arc<dyn StatusProcessor>>,
    ) -> Self {
        Self {
            config,
            processors: AprsProcessors::with_processors(
                packet_processor,
                position_processor,
                status_processor,
            ),
            shutdown_tx: None,
        }
    }

    /// Create a new APRS client with all processor types (backward compatibility)
    pub fn new_with_all_processors(
        config: AprsClientConfig,
        packet_processor: Arc<dyn PacketProcessor>,
        fix_processor: Option<Arc<dyn FixProcessor>>,
        position_processor: Option<Arc<dyn PositionProcessor>>,
        status_processor: Option<Arc<dyn StatusProcessor>>,
    ) -> Self {
        Self {
            config,
            processors: AprsProcessors::with_all_processors(
                packet_processor,
                fix_processor,
                position_processor,
                status_processor,
            ),
            shutdown_tx: None,
        }
    }

    /// Start the APRS client
    /// This will connect to the server and begin processing messages
    pub async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let config = self.config.clone();
        let processors = self.processors.clone();

        tokio::spawn(async move {
            let mut retry_count = 0;

            loop {
                // Check if shutdown was requested
                if shutdown_rx.try_recv().is_ok() {
                    info!("Shutdown requested, stopping APRS client");
                    break;
                }

                match Self::connect_and_run(&config, processors.clone()).await {
                    ConnectionResult::Success => {
                        info!("APRS client connection ended normally");
                        retry_count = 0; // Reset retry count on successful connection
                    }
                    ConnectionResult::ConnectionFailed(e) => {
                        error!("APRS client connection failed: {}", e);
                        retry_count += 1;

                        if retry_count >= config.max_retries {
                            error!(
                                "Maximum retry attempts ({}) reached, stopping client",
                                config.max_retries
                            );
                            break;
                        }

                        warn!(
                            "Retrying connection in {} seconds (attempt {}/{})",
                            config.retry_delay_seconds, retry_count, config.max_retries
                        );
                        sleep(Duration::from_secs(config.retry_delay_seconds)).await;
                    }
                    ConnectionResult::OperationFailed(e) => {
                        error!(
                            "APRS client operation failed after successful connection: {}",
                            e
                        );
                        // Reset retry count since connection was initially successful
                        retry_count = 0;

                        warn!(
                            "Retrying connection in {} seconds (connection was successful)",
                            config.retry_delay_seconds
                        );
                        sleep(Duration::from_secs(config.retry_delay_seconds)).await;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the APRS client
    pub async fn stop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(()).await;
        }
    }

    /// Connect to the APRS server and run the message processing loop
    async fn connect_and_run(
        config: &AprsClientConfig,
        processors: AprsProcessors,
    ) -> ConnectionResult {
        info!(
            "Connecting to APRS server {}:{}",
            config.server, config.port
        );

        // Connect to the APRS server
        let stream = match TcpStream::connect(format!("{}:{}", config.server, config.port)).await {
            Ok(stream) => {
                info!("Connected to APRS server");
                stream
            }
            Err(e) => {
                return ConnectionResult::ConnectionFailed(e.into());
            }
        };

        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);

        // Send login command
        let login_cmd = Self::build_login_command(config);
        info!("Sending login command: {}", login_cmd.trim());
        if let Err(e) = writer.write_all(login_cmd.as_bytes()).await {
            return ConnectionResult::OperationFailed(e.into());
        }
        if let Err(e) = writer.flush().await {
            return ConnectionResult::OperationFailed(e.into());
        }
        info!("Login command sent successfully");

        // Read and process messages
        let mut line = String::new();
        let mut first_message = true;
        loop {
            line.clear();
            match buf_reader.read_line(&mut line).await {
                Ok(0) => {
                    warn!("Connection closed by server");
                    break;
                }
                Ok(_) => {
                    let trimmed_line = line.trim();
                    if !trimmed_line.is_empty() {
                        if first_message {
                            info!("First message from server: {}", trimmed_line);
                            first_message = false;
                        } else {
                            debug!("Received: {}", trimmed_line);
                        }
                        // Skip server messages (lines starting with #)
                        if !trimmed_line.starts_with('#') {
                            Self::process_message(trimmed_line, &processors, config).await;
                        } else {
                            info!("Server message: {}", trimmed_line);
                        }
                    }
                }
                Err(e) => {
                    return ConnectionResult::OperationFailed(e.into());
                }
            }
        }

        ConnectionResult::Success
    }

    /// Build the login command for APRS-IS authentication
    fn build_login_command(config: &AprsClientConfig) -> String {
        let mut login_cmd = format!("user {} pass ", config.callsign);

        // Add password or use -1 for read-only access
        match &config.password {
            Some(pass) => login_cmd.push_str(pass),
            None => login_cmd.push_str("-1"),
        }

        // Add version info
        login_cmd.push_str(" vers soar-aprs-client 1.0");

        // Add filter if specified
        if let Some(filter) = &config.filter {
            login_cmd.push_str(" filter ");
            login_cmd.push_str(filter);
            info!("Using APRS filter: {}", filter);
        }

        login_cmd.push_str("\r\n");
        login_cmd
    }

    /// Process a received APRS message
    /// The AprsClient is now focused only on packet dispatching
    async fn process_message(
        message: &str,
        processors: &AprsProcessors,
        config: &AprsClientConfig,
    ) {
        // Always call process_raw_message first (for logging/archiving)
        processors.packet_processor.process_raw_message(message);

        // Try to parse the message using ogn-parser
        match ogn_parser::parse(message) {
            Ok(parsed) => {
                // Log unparsed fragments if present and configured
                Self::log_unparsed_fragments_if_configured(&parsed, message, config).await;

                // Dispatch the packet to the packet processor - this is the primary responsibility
                processors.packet_processor.process_packet(parsed.clone());

                // Backward compatibility: still support the old processor interfaces
                Self::handle_backward_compatibility(&parsed, message, processors).await;
            }
            Err(e) => {
                error!("Failed to parse APRS message '{message}': {e}");
            }
        }
    }

    /// Handle unparsed fragment logging if configured
    async fn log_unparsed_fragments_if_configured(
        packet: &AprsPacket,
        message: &str,
        config: &AprsClientConfig,
    ) {
        if let Some(log_path) = &config.unparsed_log_path {
            match &packet.data {
                AprsData::Position(pos) => {
                    if let Some(unparsed) = &pos.comment.unparsed {
                        error!("Unparsed position fragment: {unparsed} from message: {message}");
                        if let Err(e) =
                            Self::log_unparsed_to_csv(log_path, "position", unparsed, message).await
                        {
                            warn!("Failed to write to unparsed log: {}", e);
                        }
                    }
                }
                AprsData::Status(status) => {
                    if let Some(unparsed) = &status.comment.unparsed {
                        error!("Unparsed status fragment: {unparsed} from message: {message}");
                        if let Err(e) =
                            Self::log_unparsed_to_csv(log_path, "status", unparsed, message).await
                        {
                            warn!("Failed to write to unparsed log: {}", e);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Handle backward compatibility with old processor interfaces
    async fn handle_backward_compatibility(
        packet: &AprsPacket,
        message: &str,
        processors: &AprsProcessors,
    ) {
        match &packet.data {
            AprsData::Position(_) => {
                // Process with position processor if available (backward compatibility)
                if let Some(pos_proc) = &processors.position_processor {
                    pos_proc.process_position(packet, message);
                }

                // Process with fix processor if available (backward compatibility)
                // Only process aircraft position sources for fixes
                if let Some(fix_proc) = &processors.fix_processor {
                    if packet.position_source_type() == PositionSourceType::Aircraft {
                        match Fix::from_aprs_packet(packet.clone()) {
                            Ok(Some(fix)) => {
                                fix_proc.process_fix(fix, message);
                            }
                            Ok(None) => {
                                trace!("No position fix in APRS position packet");
                            }
                            Err(e) => {
                                debug!("Failed to extract fix from APRS position packet: {}", e);
                            }
                        }
                    } else {
                        trace!(
                            "Skipping fix processing for non-aircraft position source: {:?}",
                            packet.position_source_type()
                        );
                    }
                }
            }
            AprsData::Status(_) => {
                // Process with status processor if available (backward compatibility)
                if let Some(status_proc) = &processors.status_processor {
                    status_proc.process_status(packet, message);
                }
            }
            _ => {
                trace!("Received non-position/non-status message, only packet processor called");
            }
        }
    }

    /// Log unparsed APRS fragments to CSV file
    async fn log_unparsed_to_csv(
        log_path: &str,
        fragment_type: &str,
        unparsed_fragment: &str,
        whole_message: &str,
    ) -> Result<()> {
        // Escape CSV fields by wrapping in quotes and escaping internal quotes
        let escaped_fragment = unparsed_fragment.replace('"', "\"\"");
        let escaped_message = whole_message.replace('"', "\"\"");

        let csv_line = format!(
            "{},\"{}\",\"{}\"\n",
            fragment_type, escaped_fragment, escaped_message
        );

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .await?;

        file.write_all(csv_line.as_bytes()).await?;
        file.flush().await?;

        Ok(())
    }
}

/// Builder pattern for creating APRS client configurations
pub struct AprsClientConfigBuilder {
    config: AprsClientConfig,
}

impl AprsClientConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: AprsClientConfig::default(),
        }
    }

    pub fn server<S: Into<String>>(mut self, server: S) -> Self {
        self.config.server = server.into();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.config.max_retries = max_retries;
        self
    }

    pub fn callsign<S: Into<String>>(mut self, callsign: S) -> Self {
        self.config.callsign = callsign.into();
        self
    }

    pub fn password<S: Into<String>>(mut self, password: Option<S>) -> Self {
        self.config.password = password.map(|p| p.into());
        self
    }

    pub fn filter<S: Into<String>>(mut self, filter: Option<S>) -> Self {
        self.config.filter = filter.map(|f| f.into());
        self
    }

    pub fn retry_delay_seconds(mut self, seconds: u64) -> Self {
        self.config.retry_delay_seconds = seconds;
        self
    }

    pub fn archive_base_dir<S: Into<String>>(mut self, archive_base_dir: Option<S>) -> Self {
        self.config.archive_base_dir = archive_base_dir.map(|d| d.into());
        self
    }

    pub fn unparsed_log_path<S: Into<String>>(mut self, unparsed_log_path: Option<S>) -> Self {
        self.config.unparsed_log_path = unparsed_log_path.map(|p| p.into());
        self
    }

    pub fn build(self) -> AprsClientConfig {
        self.config
    }
}

impl Default for AprsClientConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// PacketRouter implements PacketProcessor and routes packets to appropriate specialized processors
/// This is the main router that the AprsClient should use
pub struct PacketRouter {
    /// Optional base directory for message archival
    archive_base_dir: Option<String>,
    /// Position packet processor for handling position data
    position_processor: Option<PositionPacketProcessor>,
    /// Receiver status processor for handling status data from receivers
    receiver_status_processor: Option<ReceiverStatusProcessor>,
}

impl PacketRouter {
    /// Create a new PacketRouter with optional archival
    pub fn new(archive_base_dir: Option<String>) -> Self {
        Self {
            archive_base_dir,
            position_processor: None,
            receiver_status_processor: None,
        }
    }

    /// Add a position processor to the router
    pub fn with_position_processor(mut self, processor: PositionPacketProcessor) -> Self {
        self.position_processor = Some(processor);
        self
    }

    /// Add a receiver status processor to the router
    pub fn with_receiver_status_processor(mut self, processor: ReceiverStatusProcessor) -> Self {
        self.receiver_status_processor = Some(processor);
        self
    }
}

impl PacketProcessor for PacketRouter {
    fn process_raw_message(&self, raw_message: &str) {
        // Handle archival if configured
        if let Some(base_dir) = &self.archive_base_dir {
            // TODO: Implement daily log file archival
            // For now, just trace log it
            trace!("Would archive to {}: {}", base_dir, raw_message);
        }
    }

    fn process_packet(&self, packet: AprsPacket) {
        match &packet.data {
            AprsData::Position(_) => {
                if let Some(pos_proc) = &self.position_processor {
                    pos_proc.process_position_packet(&packet);
                } else {
                    trace!("No position processor configured, skipping position packet");
                }
            }
            AprsData::Status(_) => {
                if let Some(status_proc) = &self.receiver_status_processor {
                    status_proc.process_status_packet(&packet);
                } else {
                    trace!("No receiver status processor configured, skipping status packet");
                }
            }
            _ => {
                debug!(
                    "Received packet of type {:?}, no specific handler",
                    std::mem::discriminant(&packet.data)
                );
            }
        }
    }
}

/// Processor for handling position packets from various sources
pub struct PositionPacketProcessor {
    /// Aircraft position processor for handling aircraft-specific logic
    aircraft_processor: Option<AircraftPositionProcessor>,
}

impl Default for PositionPacketProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl PositionPacketProcessor {
    /// Create a new PositionPacketProcessor
    pub fn new() -> Self {
        Self {
            aircraft_processor: None,
        }
    }

    /// Add an aircraft position processor
    pub fn with_aircraft_processor(mut self, processor: AircraftPositionProcessor) -> Self {
        self.aircraft_processor = Some(processor);
        self
    }

    /// Process a position packet, routing based on source type
    pub fn process_position_packet(&self, packet: &AprsPacket) {
        match packet.position_source_type() {
            PositionSourceType::Aircraft => {
                if let Some(aircraft_proc) = &self.aircraft_processor {
                    aircraft_proc.process_aircraft_position(packet);
                } else {
                    trace!("No aircraft processor configured, skipping aircraft position");
                }
            }
            source_type => {
                trace!(
                    "Position from non-aircraft source {:?} - not implemented yet",
                    source_type
                );
            }
        }
    }
}

/// Processor for handling aircraft position packets
pub struct AircraftPositionProcessor {
    /// Fix processor for database storage
    fix_processor: Option<Arc<dyn FixProcessor>>,
    /// Flight detection processor for flight tracking
    flight_detection_enabled: bool,
}

impl Default for AircraftPositionProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl AircraftPositionProcessor {
    /// Create a new AircraftPositionProcessor
    pub fn new() -> Self {
        Self {
            fix_processor: None,
            flight_detection_enabled: false,
        }
    }

    /// Add a fix processor for database storage
    pub fn with_fix_processor(mut self, processor: Arc<dyn FixProcessor>) -> Self {
        self.fix_processor = Some(processor);
        self
    }

    /// Enable flight detection processing
    pub fn with_flight_detection(mut self) -> Self {
        self.flight_detection_enabled = true;
        self
    }

    /// Process an aircraft position packet
    pub fn process_aircraft_position(&self, packet: &AprsPacket) {
        // Convert to Fix and process with fix processor if available
        if let Some(fix_proc) = &self.fix_processor {
            match Fix::from_aprs_packet(packet.clone()) {
                Ok(Some(fix)) => {
                    // TODO: Get raw message - for now use a placeholder
                    let raw_message = format!("{:?}", packet); // Placeholder
                    fix_proc.process_fix(fix, &raw_message);
                }
                Ok(None) => {
                    trace!("No position fix extracted from aircraft packet");
                }
                Err(e) => {
                    debug!("Failed to extract fix from aircraft packet: {}", e);
                }
            }
        }

        // TODO: Invoke flight detection processor if enabled
        if self.flight_detection_enabled {
            trace!("Flight detection processing not yet implemented");
        }
    }
}

/// Processor for handling receiver status packets
pub struct ReceiverStatusProcessor {
    /// Repository for storing receiver status data
    status_repo: ReceiverStatusRepository,
}

impl ReceiverStatusProcessor {
    /// Create a new ReceiverStatusProcessor
    pub fn new(status_repo: ReceiverStatusRepository) -> Self {
        Self { status_repo }
    }

    /// Process a status packet from a receiver
    pub fn process_status_packet(&self, packet: &AprsPacket) {
        // Ensure this is from a receiver
        match packet.position_source_type() {
            PositionSourceType::Receiver => {
                if let AprsData::Status(status) = &packet.data {
                    // Extract receiver ID from packet source (callsign)
                    // For now, use a placeholder implementation
                    let receiver_id = self.extract_receiver_id(&packet.from.to_string());

                    if let Some(recv_id) = receiver_id {
                        // Create NewReceiverStatus from status comment if available
                        // The status.comment is already a StatusComment structure
                        let new_status = NewReceiverStatus::from_status_comment(
                            recv_id,
                            &status.comment,
                            chrono::Utc::now(), // packet timestamp (placeholder)
                            chrono::Utc::now(), // received_at
                        );

                        // Store in database (async call would need runtime context)
                        tokio::spawn({
                            let repo = self.status_repo.clone();
                            async move {
                                if let Err(e) = repo.insert(&new_status).await {
                                    error!("Failed to insert receiver status: {}", e);
                                }
                            }
                        });
                    } else {
                        debug!("Could not extract receiver ID from source: {}", packet.from);
                    }
                } else {
                    warn!("Expected status packet but got different packet type");
                }
            }
            source_type => {
                trace!(
                    "Status packet from non-receiver source {:?} - ignoring",
                    source_type
                );
            }
        }
    }

    /// Extract receiver ID from callsign
    /// TODO: Implement proper lookup from receivers table
    fn extract_receiver_id(&self, callsign: &str) -> Option<i32> {
        // Placeholder implementation - in real code, this should query the receivers table
        trace!("Would look up receiver ID for callsign: {}", callsign);
        None // For now, return None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_config_builder() {
        let config = AprsClientConfigBuilder::new()
            .server("test.aprs.net")
            .port(14580)
            .callsign("TEST123")
            .password(Some("12345"))
            .filter(Some("r/47.0/-122.0/100"))
            .max_retries(3)
            .retry_delay_seconds(10)
            .build();

        assert_eq!(config.server, "test.aprs.net");
        assert_eq!(config.port, 14580);
        assert_eq!(config.callsign, "TEST123");
        assert_eq!(config.password, Some("12345".to_string()));
        assert_eq!(config.filter, Some("r/47.0/-122.0/100".to_string()));
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay_seconds, 10);
    }

    #[test]
    fn test_login_command_with_password() {
        let config = AprsClientConfig {
            server: "test.aprs.net".to_string(),
            port: 14580,
            callsign: "TEST123".to_string(),
            password: Some("12345".to_string()),
            filter: Some("r/47.0/-122.0/100".to_string()),
            max_retries: 3,
            retry_delay_seconds: 5,
            archive_base_dir: None,
            unparsed_log_path: None,
        };

        let login_cmd = AprsClient::build_login_command(&config);
        assert_eq!(
            login_cmd,
            "user TEST123 pass 12345 vers soar-aprs-client 1.0 filter r/47.0/-122.0/100\r\n"
        );
    }

    #[test]
    fn test_login_command_without_password() {
        let config = AprsClientConfig {
            server: "test.aprs.net".to_string(),
            port: 14580,
            callsign: "TEST123".to_string(),
            password: None,
            filter: None,
            max_retries: 3,
            retry_delay_seconds: 5,
            archive_base_dir: None,
            unparsed_log_path: None,
        };

        let login_cmd = AprsClient::build_login_command(&config);
        assert_eq!(
            login_cmd,
            "user TEST123 pass -1 vers soar-aprs-client 1.0\r\n"
        );
    }

    struct TestPacketProcessor {
        counter: Arc<AtomicUsize>,
    }

    impl PacketProcessor for TestPacketProcessor {
        fn process_packet(&self, _packet: AprsPacket) {
            self.counter.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn test_packet_processor() {
        let counter = Arc::new(AtomicUsize::new(0));
        let processor: Arc<dyn PacketProcessor> = Arc::new(TestPacketProcessor {
            counter: Arc::clone(&counter),
        });

        let config = AprsClientConfig::default();
        let processors = AprsProcessors::new(processor);

        // Simulate processing a message
        AprsClient::process_message("TEST>APRS:>Test message", &processors, &config).await;

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_config_builder_with_archive() {
        let config = AprsClientConfigBuilder::new()
            .server("test.aprs.net")
            .port(14580)
            .callsign("TEST123")
            .archive_base_dir(Some("/tmp/aprs_archive"))
            .build();

        assert_eq!(config.server, "test.aprs.net");
        assert_eq!(config.port, 14580);
        assert_eq!(config.callsign, "TEST123");
        assert_eq!(
            config.archive_base_dir,
            Some("/tmp/aprs_archive".to_string())
        );
    }
}
