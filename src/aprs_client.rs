use crate::Fix;
use anyhow::Result;
use ogn_parser::AprsPacket;
use regex::Regex;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;
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
    fn process_fix(&self, fix: Fix);
}

/// Type alias for boxed message processor trait objects
pub type BoxedMessageProcessor = Box<dyn MessageProcessor>;

/// Sanitize APRS message to fix invalid SSID values
/// APRS SSIDs must be in range 0-15, but some stations use invalid values like -1347
fn sanitize_aprs_message(message: &str) -> String {
    static INVALID_SSID_REGEX: OnceLock<Regex> = OnceLock::new();
    let regex = INVALID_SSID_REGEX.get_or_init(|| {
        // Match callsigns with SSIDs and check them programmatically
        // Pattern: CALLSIGN-DIGITS
        Regex::new(r"([A-Z0-9]+)-(\d+)").unwrap()
    });

    let sanitized = regex.replace_all(message, |caps: &regex::Captures| {
        let callsign = &caps[1];
        let ssid_str = &caps[2];

        // Parse the SSID and check if it's valid (0-15)
        if let Ok(ssid) = ssid_str.parse::<u32>()
            && ssid > 15
        {
            // Convert to valid range using modulo
            let sanitized_ssid = ssid % 16;
            debug!(
                "Sanitized invalid SSID: {}-{} -> {}-{}",
                callsign, ssid, callsign, sanitized_ssid
            );
            return format!("{}-{}", callsign, sanitized_ssid);
        }

        // SSID is valid or couldn't be parsed, return as-is
        format!("{}-{}", callsign, ssid_str)
    });

    sanitized.to_string()
}

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
        }
    }
}

/// APRS client that connects to an APRS-IS server via TCP
pub struct AprsClient {
    config: AprsClientConfig,
    message_processor: Arc<dyn MessageProcessor>,
    fix_processor: Option<Arc<dyn FixProcessor>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl AprsClient {
    /// Create a new APRS client with the given configuration and message processor
    pub fn new(config: AprsClientConfig, message_processor: Arc<dyn MessageProcessor>) -> Self {
        Self {
            config,
            message_processor,
            fix_processor: None,
            shutdown_tx: None,
        }
    }

    /// Create a new APRS client with both message and fix processors
    pub fn new_with_fix_processor(
        config: AprsClientConfig,
        message_processor: Arc<dyn MessageProcessor>,
        fix_processor: Arc<dyn FixProcessor>,
    ) -> Self {
        Self {
            config,
            message_processor,
            fix_processor: Some(fix_processor),
            shutdown_tx: None,
        }
    }

    /// Start the APRS client
    /// This will connect to the server and begin processing messages
    pub async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let config = self.config.clone();
        let message_processor = Arc::clone(&self.message_processor);
        let fix_processor = self.fix_processor.as_ref().map(Arc::clone);

        tokio::spawn(async move {
            let mut retry_count = 0;

            loop {
                // Check if shutdown was requested
                if shutdown_rx.try_recv().is_ok() {
                    info!("Shutdown requested, stopping APRS client");
                    break;
                }

                match Self::connect_and_run(
                    &config,
                    Arc::clone(&message_processor),
                    fix_processor.as_ref().map(Arc::clone),
                )
                .await
                {
                    Ok(_) => {
                        info!("APRS client connection ended normally");
                        retry_count = 0; // Reset retry count on successful connection
                    }
                    Err(e) => {
                        error!("APRS client error: {}", e);
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
        message_processor: Arc<dyn MessageProcessor>,
        fix_processor: Option<Arc<dyn FixProcessor>>,
    ) -> Result<()> {
        info!(
            "Connecting to APRS server {}:{}",
            config.server, config.port
        );

        // Connect to the APRS server
        let stream = TcpStream::connect(format!("{}:{}", config.server, config.port)).await?;
        info!("Connected to APRS server");

        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);

        // Send login command
        let login_cmd = Self::build_login_command(config);
        info!("Sending login command: {}", login_cmd.trim());
        writer.write_all(login_cmd.as_bytes()).await?;
        writer.flush().await?;
        info!("Login command sent successfully");

        // Read and process messages
        let mut line = String::new();
        let mut first_message = true;
        loop {
            line.clear();
            match buf_reader.read_line(&mut line).await? {
                0 => {
                    warn!("Connection closed by server");
                    break;
                }
                _ => {
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
                            Self::process_message(
                                trimmed_line,
                                Arc::clone(&message_processor),
                                fix_processor.as_ref().map(Arc::clone),
                            )
                            .await;
                        } else {
                            info!("Server message: {}", trimmed_line);
                        }
                    }
                }
            }
        }

        Ok(())
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
        message_processor: Arc<dyn MessageProcessor>,
        fix_processor: Option<Arc<dyn FixProcessor>>,
    ) {
        // Always call process_raw_message first (for logging/archiving)
        message_processor.process_raw_message(message);

        // Sanitize the message to fix invalid SSIDs
        let sanitized_message = sanitize_aprs_message(message);

        // Try to parse the sanitized message using ogn-parser
        match ogn_parser::parse(&sanitized_message) {
            Ok(parsed) => {
                // Call the message processor with the parsed message
                message_processor.process_message(parsed.clone());

                // If we have a fix processor, try to extract a position fix
                if let Some(fix_proc) = fix_processor {
                    match Fix::from_aprs_packet(parsed) {
                        Ok(Some(fix)) => {
                            fix_proc.process_fix(fix);
                        }
                        Ok(None) => {
                            trace!("No position fix in APRS packet");
                        }
                        Err(e) => {
                            debug!("Failed to extract fix from APRS packet: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to parse APRS message '{}': {}",
                    sanitized_message, e
                );
            }
        }
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

        // Simulate processing a message
        AprsClient::process_message("TEST>APRS:>Test message", Arc::clone(&processor), None).await;

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

    #[test]
    fn test_ssid_sanitization() {
        // Test the original failing message
        let invalid_message = "ICAA8871E>OGADSB,qAS,SRP-1347:/025403h3315.58N\\11149.60W^232/075 !W19! id21A8871E +896fpm FL016.94 A1:ASI66";
        let sanitized = sanitize_aprs_message(invalid_message);
        assert!(sanitized.contains("SRP-3")); // 1347 % 16 = 3

        // Test multiple invalid SSIDs
        let multi_invalid = "TEST-999>APRS,qAS,GATE-1234:test message";
        let sanitized = sanitize_aprs_message(multi_invalid);
        assert!(sanitized.contains("TEST-7")); // 999 % 16 = 7
        assert!(sanitized.contains("GATE-2")); // 1234 % 16 = 2

        // Test valid SSIDs should remain unchanged
        let valid_message = "N0CALL-15>APRS,qAS,GATE-1:test message";
        let sanitized = sanitize_aprs_message(valid_message);
        assert_eq!(sanitized, valid_message);

        // Test edge cases
        let edge_cases = "CALL-16>APRS,qAS,TEST-0:message"; // 16 should become 0
        let sanitized = sanitize_aprs_message(edge_cases);
        assert!(sanitized.contains("CALL-0"));
        assert!(sanitized.contains("TEST-0"));
    }
}
