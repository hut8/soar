// Socket server for soar-run
//
// This server listens on Unix domain sockets and/or TCP sockets and accepts
// connections from ingesters (ingest-ogn, ingest-adsb). It reads length-prefixed
// protobuf messages and sends them to the intake queue.

use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncRead, AsyncReadExt, BufReader};
use tokio::net::{TcpListener, UnixListener};
use tracing::{error, info, warn};

use crate::protocol::{Envelope, deserialize_envelope};

/// Maximum message size (1 MB)
const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

/// Unix socket server
pub struct SocketServer {
    socket_path: PathBuf,
    listener: UnixListener,
}

impl SocketServer {
    /// Start a new socket server
    ///
    /// Creates the socket at the specified path and sets permissions.
    /// Removes any existing socket file first.
    pub async fn start<P: AsRef<Path>>(socket_path: P) -> Result<Self> {
        let socket_path = socket_path.as_ref().to_path_buf();

        // Remove old socket if exists
        if socket_path.exists() {
            std::fs::remove_file(&socket_path)
                .with_context(|| format!("Failed to remove old socket: {:?}", socket_path))?;
        }

        // Create parent directory
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create socket directory: {:?}", parent))?;
        }

        // Bind listener
        let listener = UnixListener::bind(&socket_path)
            .with_context(|| format!("Failed to bind Unix socket: {:?}", socket_path))?;

        // Set permissions (readable/writable by owner and group)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o660))
                .with_context(|| format!("Failed to set socket permissions: {:?}", socket_path))?;
        }

        info!("Socket server listening on {:?}", socket_path);
        metrics::gauge!("socket.server.started").set(1.0);

        Ok(Self {
            socket_path,
            listener,
        })
    }

    /// Accept connections in a loop
    ///
    /// For each connection, spawns a handler task that reads messages
    /// and sends them to the intake queue.
    pub async fn accept_loop(self, intake_tx: flume::Sender<Envelope>) {
        let mut connection_id = 0u64;

        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    connection_id += 1;
                    let id = connection_id;

                    info!("Accepted unix connection #{} from {:?}", id, addr);
                    // Emit both labeled (for per-transport breakdown) and unlabeled
                    // (backward compat with existing dashboards) metrics
                    metrics::gauge!("socket.connections.active").increment(1.0);
                    metrics::gauge!("socket.connections.active", "transport" => "unix")
                        .increment(1.0);
                    metrics::counter!("socket.connections.accepted_total").increment(1);
                    metrics::counter!("socket.connections.accepted_total", "transport" => "unix")
                        .increment(1);

                    let intake_tx = intake_tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, intake_tx, id).await {
                            error!(connection_id = id, error = %e, "Unix connection error");
                        }
                        metrics::gauge!("socket.connections.active").decrement(1.0);
                        metrics::gauge!("socket.connections.active", "transport" => "unix")
                            .decrement(1.0);
                        metrics::counter!("socket.connections.closed_total").increment(1);
                        metrics::counter!("socket.connections.closed_total", "transport" => "unix")
                            .increment(1);
                        info!("Unix connection #{} closed", id);
                    });
                }
                Err(e) => {
                    error!(error = %e, "Unix accept error");
                    metrics::counter!("socket.errors.accept_total").increment(1);
                    metrics::counter!("socket.errors.accept_total", "transport" => "unix")
                        .increment(1);
                }
            }
        }
    }

    /// Get socket path
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }
}

/// Bind a TCP listener and return it, so the caller can detect bind failures
/// before spawning the accept loop.
///
/// This allows remote ingest instances (e.g., on a Raspberry Pi) to send data
/// over TCP to the soar-run process.
///
/// # Trust boundary
///
/// This listener does not perform authentication — any client that can reach
/// the bound address can send envelopes. In production, bind only to trusted
/// interfaces (localhost, Tailscale) rather than public-facing addresses.
/// Tailscale provides mutual authentication and encryption at the network layer.
///
/// # Usage
/// ```ignore
/// let listener = bind_tcp_listener(addr).await?;
/// tokio::spawn(run_tcp_accept_loop(listener, intake_tx));
/// ```
pub async fn bind_tcp_listener(addr: SocketAddr) -> Result<TcpListener> {
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind TCP listener on {}", addr))?;

    info!("TCP socket server listening on {}", addr);
    metrics::gauge!("socket.server.started", "transport" => "tcp").set(1.0);

    Ok(listener)
}

/// Run the TCP accept loop. Intended to be spawned after [`bind_tcp_listener`].
pub async fn run_tcp_accept_loop(listener: TcpListener, intake_tx: flume::Sender<Envelope>) {
    let mut connection_id = 0u64;

    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                connection_id += 1;
                let id = connection_id;

                // Set TCP_NODELAY for low latency
                stream.set_nodelay(true).ok();

                info!("Accepted tcp connection #{} from {}", id, peer_addr);
                metrics::gauge!("socket.connections.active", "transport" => "tcp").increment(1.0);
                metrics::counter!("socket.connections.accepted_total", "transport" => "tcp")
                    .increment(1);

                let intake_tx = intake_tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, intake_tx, id).await {
                        error!(connection_id = id, peer = %peer_addr, error = %e, "TCP connection error");
                    }
                    metrics::gauge!("socket.connections.active", "transport" => "tcp")
                        .decrement(1.0);
                    metrics::counter!("socket.connections.closed_total", "transport" => "tcp")
                        .increment(1);
                    info!("TCP connection #{} from {} closed", id, peer_addr);
                });
            }
            Err(e) => {
                error!(error = %e, "TCP accept error");
                metrics::counter!("socket.errors.accept_total", "transport" => "tcp").increment(1);
            }
        }
    }
}

/// Handle a single connection (transport-agnostic)
///
/// Reads length-prefixed messages from the reader and sends them to the intake queue.
async fn handle_connection<R: AsyncRead + Unpin>(
    stream: R,
    intake_tx: flume::Sender<Envelope>,
    connection_id: u64,
) -> Result<()> {
    let mut reader = BufReader::new(stream);

    loop {
        // Read length prefix (u32 little-endian)
        let length = match reader.read_u32_le().await {
            Ok(len) => len as usize,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                // Connection closed gracefully
                return Ok(());
            }
            Err(e) => {
                return Err(e).context("Failed to read message length");
            }
        };

        // Validate length
        if length > MAX_MESSAGE_SIZE {
            metrics::counter!("socket.errors.message_too_large_total").increment(1);
            return Err(anyhow::anyhow!(
                "Message too large: {} bytes (max: {})",
                length,
                MAX_MESSAGE_SIZE
            ));
        }

        if length == 0 {
            warn!(
                "Received zero-length message on connection #{}",
                connection_id
            );
            metrics::counter!("socket.errors.zero_length_total").increment(1);
            continue;
        }

        // Read payload
        let mut payload = vec![0u8; length];
        reader
            .read_exact(&mut payload)
            .await
            .context("Failed to read message payload")?;

        // Deserialize envelope
        let envelope = match deserialize_envelope(&payload) {
            Ok(env) => env,
            Err(e) => {
                error!(error = %e, "Failed to deserialize envelope");
                metrics::counter!("socket.errors.parse_total").increment(1);
                continue;
            }
        };

        // Send to intake queue
        if let Err(e) = intake_tx.send_async(envelope).await {
            error!(error = %e, "Failed to send to intake queue");
            metrics::counter!("socket.errors.queue_send_total").increment(1);
            return Err(e.into());
        }

        metrics::counter!("socket.messages.received_total").increment(1);
        metrics::histogram!("socket.message_size_bytes").record(length as f64);
    }
}
