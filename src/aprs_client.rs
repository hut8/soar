use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Type alias for the message processor function
/// Takes a parsed APRS message and processes it
pub type MessageProcessor = Arc<dyn Fn(String) + Send + Sync>;

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
        }
    }
}

/// APRS client that connects to an APRS-IS server via TCP
pub struct AprsClient {
    config: AprsClientConfig,
    processor: MessageProcessor,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl AprsClient {
    /// Create a new APRS client with the given configuration and message processor
    pub fn new(config: AprsClientConfig, processor: MessageProcessor) -> Self {
        Self {
            config,
            processor,
            shutdown_tx: None,
        }
    }

    /// Start the APRS client
    /// This will connect to the server and begin processing messages
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        processor: MessageProcessor,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Connecting to APRS server {}:{}", config.server, config.port);

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
    async fn process_message(message: &str, processor: MessageProcessor) {
        // Try to parse the message using ogn-parser
        match ogn_parser::parse(message) {
            Ok(parsed) => {
                debug!("Successfully parsed APRS message: {:?}", parsed);
                // Call the processor with the original message
                // Note: ogn-parser returns different types, so we pass the raw message
                // The processor can decide how to handle it
                processor(message.to_string());
            }
            Err(e) => {
                debug!("Failed to parse APRS message '{}': {}", message, e);
                // Still call the processor with the raw message
                // Some processors might want to handle unparseable messages
                processor(message.to_string());
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
        };

        let login_cmd = AprsClient::build_login_command(&config);
        assert_eq!(login_cmd, "user TEST123 pass -1 vers soar-aprs-client 1.0\r\n");
    }

    #[tokio::test]
    async fn test_message_processor() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let processor: MessageProcessor = Arc::new(move |_message: String| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        // Simulate processing a message
        AprsClient::process_message("TEST>APRS:>Test message", Arc::clone(&processor)).await;

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
