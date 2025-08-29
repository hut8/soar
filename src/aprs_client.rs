use anyhow::Result;
use chrono::Utc;
use ogn_parser::AprsPacket;
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Trait for processing APRS messages
/// Implementors can define custom logic for handling received APRS messages
pub trait MessageProcessor: Send + Sync {
    /// Process a received APRS message
    ///
    /// # Arguments
    /// * `message` - The raw APRS message string received from the server
    fn process_message(&self, message: AprsPacket);
}

/// Type alias for boxed message processor trait objects
pub type BoxedMessageProcessor = Box<dyn MessageProcessor>;

/// Message archive for logging APRS messages to daily files
struct MessageArchive {
    base_dir: String,
    current_file: Mutex<Option<std::fs::File>>,
    current_date: Mutex<String>,
}

impl MessageArchive {
    fn new(base_dir: String) -> Self {
        Self {
            base_dir,
            current_file: Mutex::new(None),
            current_date: Mutex::new(String::new()),
        }
    }

    fn log_message(&self, message: &str) {
        let now = Utc::now();
        let date_str = now.format("%Y-%m-%d").to_string();

        let mut current_date = self.current_date.lock().unwrap();
        let mut current_file = self.current_file.lock().unwrap();

        // Check if we need to create a new file (new day or first time)
        if *current_date != date_str {
            // Close the current file if it exists
            *current_file = None;

            // Create the archive directory if it doesn't exist
            let archive_path = PathBuf::from(&self.base_dir);
            if let Err(e) = create_dir_all(&archive_path) {
                error!(
                    "Failed to create archive directory {}: {}",
                    archive_path.display(),
                    e
                );
                return;
            }

            // Create the new log file
            let log_file_path = archive_path.join(format!("{date_str}.log"));
            match OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file_path)
            {
                Ok(file) => {
                    info!("Opened new archive log file: {}", log_file_path.display());
                    *current_file = Some(file);
                    *current_date = date_str;
                }
                Err(e) => {
                    error!(
                        "Failed to open archive log file {}: {}",
                        log_file_path.display(),
                        e
                    );
                    return;
                }
            }
        }

        // Write the message to the current file
        if let Some(ref mut file) = *current_file {
            let timestamp = now.format("%H:%M:%S").to_string();
            if let Err(e) = writeln!(file, "[{timestamp}] {message}") {
                error!("Failed to write to archive log file: {}", e);
            } else if let Err(e) = file.flush() {
                error!("Failed to flush archive log file: {}", e);
            }
        }
    }
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
    processor: Arc<dyn MessageProcessor>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl AprsClient {
    /// Create a new APRS client with the given configuration and message processor
    pub fn new(config: AprsClientConfig, processor: Arc<dyn MessageProcessor>) -> Self {
        Self {
            config,
            processor,
            shutdown_tx: None,
        }
    }

    /// Start the APRS client
    /// This will connect to the server and begin processing messages
    pub async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let config = self.config.clone();
        let processor = Arc::clone(&self.processor);

        tokio::spawn(async move {
            let mut retry_count = 0;

            loop {
                // Check if shutdown was requested
                if shutdown_rx.try_recv().is_ok() {
                    info!("Shutdown requested, stopping APRS client");
                    break;
                }

                match Self::connect_and_run(&config, Arc::clone(&processor)).await {
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
        processor: Arc<dyn MessageProcessor>,
    ) -> Result<()> {
        info!(
            "Connecting to APRS server {}:{}",
            config.server, config.port
        );

        // Create message archive if configured
        let archive = config.archive_base_dir.as_ref().map(|base_dir| {
            info!("Message archive enabled, base directory: {}", base_dir);
            Arc::new(MessageArchive::new(base_dir.clone()))
        });

        // Connect to the APRS server
        let stream = TcpStream::connect(format!("{}:{}", config.server, config.port)).await?;
        info!("Connected to APRS server");

        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);

        // Send login command
        let login_cmd = Self::build_login_command(config);
        writer.write_all(login_cmd.as_bytes()).await?;
        writer.flush().await?;
        debug!("Sent login command: {}", login_cmd.trim());

        // Read and process messages
        let mut line = String::new();
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
                        debug!("Received: {}", trimmed_line);
                        // Log to archive before processing (if archive is enabled)
                        if let Some(ref archive) = archive {
                            archive.log_message(trimmed_line);
                        }
                        // Skip server messages (lines starting with #)
                        if !trimmed_line.starts_with('#') {
                            Self::process_message(trimmed_line, Arc::clone(&processor)).await;
                        } else {
                            debug!("Server message: {}", trimmed_line);
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
        }

        login_cmd.push_str("\r\n");
        login_cmd
    }

    /// Process a received APRS message
    async fn process_message(message: &str, processor: Arc<dyn MessageProcessor>) {
        // Try to parse the message using ogn-parser
        match ogn_parser::parse(message) {
            Ok(parsed) => {
                debug!("Successfully parsed APRS message: {:?}", parsed);
                // Call the processor with the original message
                // Note: ogn-parser returns different types, so we pass the raw message
                // The processor can decide how to handle it
                processor.process_message(parsed);
            }
            Err(e) => {
                debug!("Failed to parse APRS message '{}': {}", message, e);
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
        AprsClient::process_message("TEST>APRS:>Test message", Arc::clone(&processor)).await;

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
    fn test_message_archive() {
        use std::fs;
        use std::path::Path;

        let temp_dir = "/tmp/test_aprs_archive";
        let archive = MessageArchive::new(temp_dir.to_string());

        // Log a test message
        archive.log_message("TEST>APRS:>Test archive message");

        // Check that the directory was created
        assert!(Path::new(temp_dir).exists());

        // Check that a log file was created with today's date
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let log_file_path = Path::new(temp_dir).join(format!("{today}.log"));
        assert!(log_file_path.exists());

        // Read the file content and verify the message was logged
        let content = fs::read_to_string(&log_file_path).expect("Failed to read log file");
        assert!(content.contains("TEST>APRS:>Test archive message"));

        // Clean up
        let _ = fs::remove_dir_all(temp_dir);
    }
}
