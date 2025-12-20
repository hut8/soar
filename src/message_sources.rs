//! Message source abstraction for OGN/APRS messages
//!
//! This module provides a trait-based abstraction for consuming raw APRS messages
//! from different sources. This enables:
//! - Production: Real-time NATS streaming
//! - Testing: Replaying messages from files
//!
//! Message Format:
//! All messages follow the format: "YYYY-MM-DDTHH:MM:SS.SSSZ <raw_aprs_message>"
//! Example: "2025-01-15T12:34:56.789Z FLRDDA5BA>APRS,qAS,LFNM:/074548h..."
use anyhow::Result;
use async_trait::async_trait;
use futures_util::StreamExt;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{debug, warn};

/// Trait for sources of raw APRS messages
///
/// Implementors provide an async stream of timestamped APRS messages.
/// Messages should include the RFC3339 timestamp prefix.
#[async_trait]
pub trait RawMessageSource: Send + Sync {
    /// Get the next message from the source
    ///
    /// Returns:
    /// - `Ok(Some(message))` - Next message available
    /// - `Ok(None)` - End of stream (no more messages)
    /// - `Err(e)` - Error reading message
    async fn next_message(&mut self) -> Result<Option<String>>;

    /// Optional: Get a hint about how many messages remain (for progress tracking)
    fn remaining_hint(&self) -> Option<usize> {
        None
    }
}

/// NATS message source for production use
///
/// Wraps a NATS subscriber and provides messages from the stream.
/// Messages are consumed in real-time from the NATS subject.
pub struct NatsMessageSource {
    subscriber: async_nats::Subscriber,
}

impl NatsMessageSource {
    /// Create a new NATS message source from a subscriber
    pub fn new(subscriber: async_nats::Subscriber) -> Self {
        Self { subscriber }
    }

    /// Create a NATS message source by connecting and subscribing
    ///
    /// # Arguments
    /// * `nats_url` - NATS server URL (e.g., "nats://localhost:4222")
    /// * `subject` - NATS subject to subscribe to (e.g., "ogn.raw")
    /// * `client_name` - Client name for NATS connection
    pub async fn connect(nats_url: &str, subject: &str, client_name: &str) -> Result<Self> {
        let client = async_nats::ConnectOptions::new()
            .name(client_name)
            .connect(nats_url)
            .await?;

        let subscriber = client.subscribe(subject.to_string()).await?;

        Ok(Self { subscriber })
    }
}

#[async_trait]
impl RawMessageSource for NatsMessageSource {
    async fn next_message(&mut self) -> Result<Option<String>> {
        match self.subscriber.next().await {
            Some(msg) => {
                // Convert NATS message payload to String
                match String::from_utf8(msg.payload.to_vec()) {
                    Ok(message) => Ok(Some(message)),
                    Err(e) => {
                        warn!("Failed to decode NATS message as UTF-8: {}", e);
                        // Skip this message and try the next one
                        self.next_message().await
                    }
                }
            }
            None => Ok(None), // Stream ended
        }
    }
}

/// Test message source for integration testing
///
/// Reads pre-recorded APRS messages from a file, one per line.
/// Each line should contain: "YYYY-MM-DDTHH:MM:SS.SSSZ <raw_aprs_message>"
///
/// Files can be generated using the `scripts/dump-flight-messages.sh` script.
pub struct TestMessageSource {
    reader: BufReader<File>,
    line_buffer: String,
    total_messages: Option<usize>,
    messages_read: usize,
}

impl TestMessageSource {
    /// Create a test message source from a file path
    ///
    /// # Arguments
    /// * `path` - Path to the file containing timestamped APRS messages
    ///
    /// # File Format
    /// Each line should contain a timestamped APRS message:
    /// ```text
    /// 2025-01-15T12:34:56.789Z FLRDDA5BA>APRS,qAS,LFNM:/074548h...
    /// 2025-01-15T12:34:57.123Z FLRDD1234>APRS,qAS,LFNM:/074549h...
    /// ```
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref()).await?;
        let reader = BufReader::new(file);

        debug!("Opened test message source from: {:?}", path.as_ref());

        Ok(Self {
            reader,
            line_buffer: String::new(),
            total_messages: None,
            messages_read: 0,
        })
    }

    /// Create a test message source from file path with known message count
    ///
    /// Providing the total count enables progress tracking via `remaining_hint()`
    pub async fn from_file_with_count<P: AsRef<Path>>(
        path: P,
        total_messages: usize,
    ) -> Result<Self> {
        let mut source = Self::from_file(path).await?;
        source.total_messages = Some(total_messages);
        Ok(source)
    }

    /// Get the number of messages read so far
    pub fn messages_read(&self) -> usize {
        self.messages_read
    }
}

#[async_trait]
impl RawMessageSource for TestMessageSource {
    async fn next_message(&mut self) -> Result<Option<String>> {
        // Clear the buffer from the previous read
        self.line_buffer.clear();

        // Read the next line from the file
        let bytes_read = self.reader.read_line(&mut self.line_buffer).await?;

        if bytes_read == 0 {
            // End of file
            debug!(
                "Reached end of test message file after {} messages",
                self.messages_read
            );
            return Ok(None);
        }

        self.messages_read += 1;

        // Remove trailing newline if present
        let message = self.line_buffer.trim_end().to_string();

        if message.is_empty() {
            // Skip empty lines and try the next one
            debug!("Skipping empty line in test message file");
            return self.next_message().await;
        }

        Ok(Some(message))
    }

    fn remaining_hint(&self) -> Option<usize> {
        self.total_messages
            .map(|total| total.saturating_sub(self.messages_read))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_message_source_reads_file() {
        // Create a temporary test file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            "2025-01-15T12:00:00.000Z FLRDDA5BA>APRS,qAS,LFNM:/120000h"
        )
        .unwrap();
        writeln!(
            temp_file,
            "2025-01-15T12:00:01.000Z FLRDD1234>APRS,qAS,LFNM:/120001h"
        )
        .unwrap();
        writeln!(
            temp_file,
            "2025-01-15T12:00:02.000Z FLRDD5678>APRS,qAS,LFNM:/120002h"
        )
        .unwrap();
        temp_file.flush().unwrap();

        let mut source: TestMessageSource = TestMessageSource::from_file(temp_file.path())
            .await
            .unwrap();

        // Read all messages
        let msg1: Option<String> = source.next_message().await.unwrap();
        assert!(msg1.is_some());
        assert!(msg1.unwrap().contains("FLRDDA5BA"));

        let msg2: Option<String> = source.next_message().await.unwrap();
        assert!(msg2.is_some());
        assert!(msg2.unwrap().contains("FLRDD1234"));

        let msg3: Option<String> = source.next_message().await.unwrap();
        assert!(msg3.is_some());
        assert!(msg3.unwrap().contains("FLRDD5678"));

        // Should reach end of file
        let msg4: Option<String> = source.next_message().await.unwrap();
        assert!(msg4.is_none());
    }

    #[tokio::test]
    async fn test_message_source_with_count() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "2025-01-15T12:00:00.000Z FLRDDA5BA>APRS").unwrap();
        writeln!(temp_file, "2025-01-15T12:00:01.000Z FLRDD1234>APRS").unwrap();
        temp_file.flush().unwrap();

        let mut source: TestMessageSource =
            TestMessageSource::from_file_with_count(temp_file.path(), 2)
                .await
                .unwrap();

        assert_eq!(source.remaining_hint(), Some(2));

        let _msg1: Option<String> = source.next_message().await.unwrap();
        assert_eq!(source.remaining_hint(), Some(1));
        assert_eq!(source.messages_read(), 1);

        let _msg2: Option<String> = source.next_message().await.unwrap();
        assert_eq!(source.remaining_hint(), Some(0));
        assert_eq!(source.messages_read(), 2);

        let _msg3: Option<String> = source.next_message().await.unwrap();
        assert_eq!(source.remaining_hint(), Some(0));
    }

    #[tokio::test]
    async fn test_message_source_skips_empty_lines() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "2025-01-15T12:00:00.000Z FLRDDA5BA>APRS").unwrap();
        writeln!(temp_file).unwrap(); // Empty line
        writeln!(temp_file, "2025-01-15T12:00:01.000Z FLRDD1234>APRS").unwrap();
        temp_file.flush().unwrap();

        let mut source: TestMessageSource = TestMessageSource::from_file(temp_file.path())
            .await
            .unwrap();

        let msg1: Option<String> = source.next_message().await.unwrap();
        assert!(msg1.unwrap().contains("FLRDDA5BA"));

        let msg2: Option<String> = source.next_message().await.unwrap();
        assert!(msg2.unwrap().contains("FLRDD1234"));

        let msg3: Option<String> = source.next_message().await.unwrap();
        assert!(msg3.is_none());
    }
}
