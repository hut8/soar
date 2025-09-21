use crate::Fix;
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

/// Trait for processing APRS messages
/// Implementors can define custom logic for handling received APRS messages
pub trait MessageProcessor: Send + Sync {
    /// Process a received APRS message
    ///
    /// # Arguments
    /// * `message` - The parsed APRS packet
    fn process_message(&self, _message: AprsPacket) {
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
    /// Processor for general APRS messages and raw message handling
    pub message_processor: Arc<dyn MessageProcessor>,
    /// Optional processor for position fixes (backward compatibility)
    pub fix_processor: Option<Arc<dyn FixProcessor>>,
    /// Optional processor for position messages
    pub position_processor: Option<Arc<dyn PositionProcessor>>,
    /// Optional processor for status messages
    pub status_processor: Option<Arc<dyn StatusProcessor>>,
}

impl AprsProcessors {
    /// Create a new AprsProcessors with just a message processor
    pub fn new(message_processor: Arc<dyn MessageProcessor>) -> Self {
        Self {
            message_processor,
            fix_processor: None,
            position_processor: None,
            status_processor: None,
        }
    }

    /// Create a new AprsProcessors with message and fix processors
    pub fn with_fix_processor(
        message_processor: Arc<dyn MessageProcessor>,
        fix_processor: Arc<dyn FixProcessor>,
    ) -> Self {
        Self {
            message_processor,
            fix_processor: Some(fix_processor),
            position_processor: None,
            status_processor: None,
        }
    }

    /// Create a new AprsProcessors with message, position, and status processors
    pub fn with_processors(
        message_processor: Arc<dyn MessageProcessor>,
        position_processor: Option<Arc<dyn PositionProcessor>>,
        status_processor: Option<Arc<dyn StatusProcessor>>,
    ) -> Self {
        Self {
            message_processor,
            fix_processor: None,
            position_processor,
            status_processor,
        }
    }

    /// Create a new AprsProcessors with all processor types
    pub fn with_all_processors(
        message_processor: Arc<dyn MessageProcessor>,
        fix_processor: Option<Arc<dyn FixProcessor>>,
        position_processor: Option<Arc<dyn PositionProcessor>>,
        status_processor: Option<Arc<dyn StatusProcessor>>,
    ) -> Self {
        Self {
            message_processor,
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
    pub fn add_position_processor(mut self, position_processor: Arc<dyn PositionProcessor>) -> Self {
        self.position_processor = Some(position_processor);
        self
    }

    /// Add a status processor to existing processors
    pub fn add_status_processor(mut self, status_processor: Arc<dyn StatusProcessor>) -> Self {
        self.status_processor = Some(status_processor);
        self
    }
}

/// Type alias for boxed message processor trait objects
pub type BoxedMessageProcessor = Box<dyn MessageProcessor>;

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

    /// Create a new APRS client with just a message processor (backward compatibility)
    pub fn new_with_message_processor(config: AprsClientConfig, message_processor: Arc<dyn MessageProcessor>) -> Self {
        Self {
            config,
            processors: AprsProcessors::new(message_processor),
            shutdown_tx: None,
        }
    }

    /// Create a new APRS client with both message and fix processors (backward compatibility)
    pub fn new_with_fix_processor(
        config: AprsClientConfig,
        message_processor: Arc<dyn MessageProcessor>,
        fix_processor: Arc<dyn FixProcessor>,
    ) -> Self {
        Self {
            config,
            processors: AprsProcessors::with_fix_processor(message_processor, fix_processor),
            shutdown_tx: None,
        }
    }

    /// Create a new APRS client with position and status processors (backward compatibility)
    pub fn new_with_processors(
        config: AprsClientConfig,
        message_processor: Arc<dyn MessageProcessor>,
        position_processor: Option<Arc<dyn PositionProcessor>>,
        status_processor: Option<Arc<dyn StatusProcessor>>,
    ) -> Self {
        Self {
            config,
            processors: AprsProcessors::with_processors(message_processor, position_processor, status_processor),
            shutdown_tx: None,
        }
    }

    /// Create a new APRS client with all processor types (backward compatibility)
    pub fn new_with_all_processors(
        config: AprsClientConfig,
        message_processor: Arc<dyn MessageProcessor>,
        fix_processor: Option<Arc<dyn FixProcessor>>,
        position_processor: Option<Arc<dyn PositionProcessor>>,
        status_processor: Option<Arc<dyn StatusProcessor>>,
    ) -> Self {
        Self {
            config,
            processors: AprsProcessors::with_all_processors(message_processor, fix_processor, position_processor, status_processor),
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
    async fn process_message(
        message: &str,
        processors: &AprsProcessors,
        config: &AprsClientConfig,
    ) {
        // Always call process_raw_message first (for logging/archiving)
        processors.message_processor.process_raw_message(message);

        // Try to parse the message using ogn-parser
        match ogn_parser::parse(message) {
            Ok(parsed) => {
                // Call the general message processor with the parsed message
                processors.message_processor.process_message(parsed.clone());

                // Process specific message types with their dedicated processors
                match &parsed.data {
                    AprsData::Position(pos) => {
                        // Log unparsed fragments if present
                        if let Some(unparsed) = &pos.comment.unparsed {
                            error!(
                                "Unparsed position fragment: {unparsed} from message: {message}"
                            );

                            // Log to CSV if configured
                            if let Some(log_path) = &config.unparsed_log_path
                                && let Err(e) = Self::log_unparsed_to_csv(
                                    log_path, "position", unparsed, message,
                                )
                                .await
                            {
                                warn!("Failed to write to unparsed log: {}", e);
                            }
                        }

                        // Process with position processor if available
                        if let Some(pos_proc) = &processors.position_processor {
                            pos_proc.process_position(&parsed, message);
                        } else {
                            trace!("No position processor configured, skipping position message");
                        }

                        // Also process with fix processor if available (backward compatibility)
                        // Only process aircraft position sources for fixes
                        if let Some(fix_proc) = &processors.fix_processor {
                            if parsed.position_source_type() == PositionSourceType::Aircraft {
                                match Fix::from_aprs_packet(parsed) {
                                    Ok(Some(fix)) => {
                                        fix_proc.process_fix(fix, message);
                                    }
                                    Ok(None) => {
                                        trace!("No position fix in APRS position packet");
                                    }
                                    Err(e) => {
                                        debug!(
                                            "Failed to extract fix from APRS position packet: {}",
                                            e
                                        );
                                    }
                                }
                            } else {
                                trace!(
                                    "Skipping fix processing for non-aircraft position source: {:?}",
                                    parsed.position_source_type()
                                );
                            }
                        }
                    }
                    AprsData::Status(status) => {
                        // Log unparsed fragments if present
                        if let Some(unparsed) = &status.comment.unparsed {
                            error!("Unparsed status fragment: {unparsed} from message: {message}");

                            // Log to CSV if configured
                            if let Some(log_path) = &config.unparsed_log_path
                                && let Err(e) =
                                    Self::log_unparsed_to_csv(log_path, "status", unparsed, message)
                                        .await
                            {
                                warn!("Failed to write to unparsed log: {}", e);
                            }
                        }

                        // Process with status processor if available
                        if let Some(status_proc) = &processors.status_processor {
                            status_proc.process_status(&parsed, message);
                        } else {
                            trace!("No status processor configured, skipping status message");
                        }
                    }
                    _ => {
                        trace!(
                            "Received non-position/non-status message, only general message processor called"
                        );
                    }
                }
            }
            Err(e) => {
                error!("Failed to parse APRS message '{message}': {e}");
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

    struct TestMessageProcessor {
        counter: Arc<AtomicUsize>,
    }

    impl MessageProcessor for TestMessageProcessor {
        fn process_message(&self, _message: AprsPacket) {
            self.counter.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn test_message_processor() {
        let counter = Arc::new(AtomicUsize::new(0));
        let processor: Arc<dyn MessageProcessor> = Arc::new(TestMessageProcessor {
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
