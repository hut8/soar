// Socket client for ingesters
//
// This client connects to the soar-run socket server and sends
// length-prefixed protobuf messages. It handles reconnection with
// exponential backoff.
//
// Supports both Unix domain sockets (local) and TCP sockets (remote).

use anyhow::{Context, Result};
use std::time::Duration;
use tokio::io::{AsyncWrite, AsyncWriteExt, BufWriter};
use tokio::net::UnixStream;
use tracing::{info, warn};

use crate::SocketTarget;
use crate::protocol::{IngestSource, new_envelope, serialize_envelope};

/// Socket client for ingesters
pub struct SocketClient {
    target: SocketTarget,
    stream: Option<BufWriter<Box<dyn AsyncWrite + Unpin + Send>>>,
    source: IngestSource,
}

impl SocketClient {
    /// Create a new socket client in disconnected state
    ///
    /// Use this when soar-run is not yet available. The client will attempt
    /// to connect when send() is called or when reconnect() is called explicitly.
    ///
    /// # Arguments
    /// * `target` - Socket target (Unix path or TCP address)
    /// * `source` - Source of messages (OGN, Beast, or SBS)
    pub fn new(target: SocketTarget, source: IngestSource) -> Self {
        metrics::gauge!("socket.client.connected").set(0.0);

        Self {
            target,
            stream: None,
            source,
        }
    }

    /// Connect to the soar-run socket server
    ///
    /// # Arguments
    /// * `target` - Socket target (Unix path or TCP address)
    /// * `source` - Source of messages (OGN, Beast, or SBS)
    pub async fn connect(target: SocketTarget, source: IngestSource) -> Result<Self> {
        let stream = connect_to_target(&target).await?;

        info!(
            "Connected to soar-run at {} (source: {:?})",
            target,
            source.as_str_name()
        );
        metrics::gauge!("socket.client.connected").set(1.0);

        Ok(Self {
            target,
            stream: Some(stream),
            source,
        })
    }

    /// Send a message to soar-run
    ///
    /// The message is wrapped in an envelope with source and timestamp,
    /// serialized to protobuf, and sent with a length prefix.
    ///
    /// NOTE: This timestamps the message NOW. If you have pre-serialized
    /// envelope data (e.g., from a queue), use `send_serialized` instead.
    pub async fn send(&mut self, data: Vec<u8>) -> Result<()> {
        // Create envelope with current timestamp
        let envelope = new_envelope(self.source, data);

        // Serialize and send
        let payload = serialize_envelope(&envelope)?;
        self.send_serialized(&payload).await
    }

    /// Send a pre-serialized envelope to soar-run
    ///
    /// Use this when you have already-serialized protobuf data (e.g., from a queue
    /// where the envelope was created at ingest time to preserve the receive timestamp).
    pub async fn send_serialized(&mut self, payload: &[u8]) -> Result<()> {
        let start = std::time::Instant::now();

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
            .write_all(payload)
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

        info!("Attempting to reconnect to {}", self.target);

        let mut delay = Duration::from_secs(1);
        let max_delay = Duration::from_secs(60);

        loop {
            match connect_to_target(&self.target).await {
                Ok(stream) => {
                    self.stream = Some(stream);
                    info!("Reconnected to soar-run at {}", self.target);
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

/// Connect to the target and return a boxed async writer
async fn connect_to_target(
    target: &SocketTarget,
) -> Result<BufWriter<Box<dyn AsyncWrite + Unpin + Send>>> {
    match target {
        SocketTarget::Unix(path) => {
            let stream = UnixStream::connect(path)
                .await
                .with_context(|| format!("Failed to connect to Unix socket: {:?}", path))?;
            Ok(BufWriter::new(Box::new(stream)))
        }
        SocketTarget::Tcp(addr) => {
            let stream = tokio::net::TcpStream::connect(addr)
                .await
                .with_context(|| format!("Failed to connect to TCP socket: {}", addr))?;

            // Set TCP_NODELAY for low latency (disable Nagle's algorithm)
            stream.set_nodelay(true).ok();

            Ok(BufWriter::new(Box::new(stream)))
        }
    }
}
