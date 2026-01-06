// Unix socket client for ingesters
//
// This client connects to the soar-run socket server and sends
// length-prefixed protobuf messages. It handles reconnection with
// exponential backoff.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::net::UnixStream;
use tracing::{info, warn};

use crate::protocol::{IngestSource, new_envelope, serialize_envelope};

/// Socket client for ingesters
pub struct SocketClient {
    socket_path: PathBuf,
    stream: Option<BufWriter<UnixStream>>,
    source: IngestSource,
}

impl SocketClient {
    /// Create a new socket client in disconnected state
    ///
    /// Use this when soar-run is not yet available. The client will attempt
    /// to connect when send() is called or when reconnect() is called explicitly.
    ///
    /// # Arguments
    /// * `socket_path` - Path to Unix socket
    /// * `source` - Source of messages (OGN, Beast, or SBS)
    pub fn new<P: AsRef<Path>>(socket_path: P, source: IngestSource) -> Self {
        let socket_path = socket_path.as_ref().to_path_buf();
        metrics::gauge!("socket.client.connected").set(0.0);

        Self {
            socket_path,
            stream: None,
            source,
        }
    }

    /// Connect to the soar-run socket server
    ///
    /// # Arguments
    /// * `socket_path` - Path to Unix socket
    /// * `source` - Source of messages (OGN, Beast, or SBS)
    pub async fn connect<P: AsRef<Path>>(socket_path: P, source: IngestSource) -> Result<Self> {
        let socket_path = socket_path.as_ref().to_path_buf();

        let stream = UnixStream::connect(&socket_path)
            .await
            .with_context(|| format!("Failed to connect to socket: {:?}", socket_path))?;

        let stream = BufWriter::new(stream);

        info!(
            "Connected to soar-run at {:?} (source: {:?})",
            socket_path,
            source.as_str_name()
        );
        metrics::gauge!("socket.client.connected").set(1.0);

        Ok(Self {
            socket_path,
            stream: Some(stream),
            source,
        })
    }

    /// Send a message to soar-run
    ///
    /// The message is wrapped in an envelope with source and timestamp,
    /// serialized to protobuf, and sent with a length prefix.
    pub async fn send(&mut self, data: Vec<u8>) -> Result<()> {
        let start = std::time::Instant::now();

        // Create envelope
        let envelope = new_envelope(self.source, data);

        // Serialize envelope
        let payload = serialize_envelope(&envelope)?;
        let length = payload.len() as u32;

        // Get stream reference
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        // Write length prefix + payload
        stream
            .write_all(&length.to_le_bytes())
            .await
            .context("Failed to write length prefix")?;
        stream
            .write_all(&payload)
            .await
            .context("Failed to write payload")?;
        stream.flush().await.context("Failed to flush socket")?;

        // Metrics
        let duration_ms = start.elapsed().as_millis() as f64;
        metrics::counter!("socket.client.messages.sent_total").increment(1);
        metrics::histogram!("socket.client.send_duration_ms").record(duration_ms);

        if duration_ms > 100.0 {
            metrics::counter!("socket.client.slow_sends_total").increment(1);
        }

        Ok(())
    }

    /// Reconnect to the server with exponential backoff
    ///
    /// This function will retry forever until a connection is established.
    pub async fn reconnect(&mut self) -> Result<()> {
        self.stream = None;
        metrics::gauge!("socket.client.connected").set(0.0);

        info!("Attempting to reconnect to {:?}", self.socket_path);

        let mut delay = Duration::from_secs(1);
        let max_delay = Duration::from_secs(60);

        loop {
            match UnixStream::connect(&self.socket_path).await {
                Ok(stream) => {
                    self.stream = Some(BufWriter::new(stream));
                    info!("Reconnected to soar-run at {:?}", self.socket_path);
                    metrics::gauge!("socket.client.connected").set(1.0);
                    metrics::counter!("socket.client.reconnects_total").increment(1);
                    return Ok(());
                }
                Err(e) => {
                    warn!("Reconnect failed (will retry in {:?}): {}", delay, e);
                    metrics::counter!("socket.client.reconnect_failures_total").increment(1);
                    tokio::time::sleep(delay).await;

                    // Exponential backoff
                    delay = std::cmp::min(delay * 2, max_delay);
                }
            }
        }
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }
}
