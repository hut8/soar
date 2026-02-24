// Unix socket server for soar-run
//
// This server listens on a Unix domain socket and accepts connections
// from ingesters (ingest-ogn, ingest-adsb). It reads length-prefixed
// protobuf messages and sends them to the intake queue.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
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

                    info!("Accepted connection #{} from {:?}", id, addr);
                    metrics::gauge!("socket.connections.active").increment(1.0);
                    metrics::counter!("socket.connections.accepted_total").increment(1);

                    let intake_tx = intake_tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, intake_tx, id).await {
                            error!(connection_id = id, error = %e, "Connection error");
                        }
                        metrics::gauge!("socket.connections.active").decrement(1.0);
                        metrics::counter!("socket.connections.closed_total").increment(1);
                        info!("Connection #{} closed", id);
                    });
                }
                Err(e) => {
                    error!(error = %e, "Accept error");
                    metrics::counter!("socket.errors.accept_total").increment(1);
                }
            }
        }
    }

    /// Get socket path
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }
}

/// Handle a single connection
///
/// Reads length-prefixed messages from the socket and sends them to the intake queue.
async fn handle_connection(
    stream: UnixStream,
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
