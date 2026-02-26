use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use uuid::Uuid;

use crate::aprs_client::{AprsClient, AprsClientConfigBuilder};
use crate::beast::{BeastClient, BeastClientConfig};
use crate::ingest_config::{DataStream, IngestConfigFile, StreamFormat};
use crate::metrics::{AprsIngestHealth, BeastIngestHealth};
use crate::persistent_queue::PersistentQueue;
use crate::sbs::{SbsClient, SbsClientConfig};

/// A running stream task with its config snapshot and cancellation token
struct RunningStream {
    config: DataStream,
    cancel: CancellationToken,
    handle: JoinHandle<()>,
}

/// Shared resources that all stream tasks need access to
pub struct SharedResources {
    pub queue: Arc<PersistentQueue<Vec<u8>>>,
    pub aprs_health: Arc<RwLock<AprsIngestHealth>>,
    pub beast_health: Arc<RwLock<BeastIngestHealth>>,
    pub sbs_health: Arc<RwLock<BeastIngestHealth>>,
    pub stats_ogn_received: Arc<AtomicU64>,
    pub stats_beast_received: Arc<AtomicU64>,
    pub stats_sbs_received: Arc<AtomicU64>,
}

/// Manages the lifecycle of data stream client tasks.
///
/// Compares running streams against config to start/stop/restart as needed.
pub struct StreamManager {
    running: HashMap<Uuid, RunningStream>,
    resources: Arc<SharedResources>,
    retry_delay: u64,
}

impl StreamManager {
    pub fn new(resources: Arc<SharedResources>, retry_delay: u64) -> Self {
        Self {
            running: HashMap::new(),
            resources,
            retry_delay,
        }
    }

    /// Apply a new config, starting/stopping/restarting streams as needed.
    pub fn apply_config(&mut self, config: &IngestConfigFile) {
        self.retry_delay = config.retry_delay;

        let desired: HashMap<Uuid, DataStream> = config
            .data_streams()
            .into_iter()
            .filter(|s| s.enabled)
            .map(|s| (s.id, s))
            .collect();

        // Stop streams no longer in config or now disabled
        let to_remove: Vec<Uuid> = self
            .running
            .keys()
            .filter(|id| !desired.contains_key(id))
            .copied()
            .collect();
        for id in to_remove {
            self.stop_stream(id);
        }

        // Restart streams whose config changed, start new ones
        for (id, stream) in &desired {
            if let Some(running) = self.running.get(id) {
                if stream_config_changed(&running.config, stream) {
                    info!(
                        stream_id = %id,
                        name = %stream.name,
                        "Stream config changed, restarting"
                    );
                    self.stop_stream(*id);
                    self.start_stream(stream.clone());
                }
            } else {
                self.start_stream(stream.clone());
            }
        }
    }

    /// Start a single stream client task.
    fn start_stream(&mut self, stream: DataStream) {
        let id = stream.id;
        let name = stream.name.clone();
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        let retry_delay = self.retry_delay;
        let resources = self.resources.clone();

        info!(
            stream_id = %id,
            name = %name,
            format = %stream.format,
            host = %stream.host,
            port = %stream.port,
            "Starting stream"
        );

        let stream_for_task = stream.clone();
        let handle = tokio::spawn(async move {
            run_stream_task(stream_for_task, resources, retry_delay, cancel_clone).await;
        });

        self.running.insert(
            id,
            RunningStream {
                config: stream,
                cancel,
                handle,
            },
        );
    }

    /// Stop a running stream by ID.
    fn stop_stream(&mut self, id: Uuid) {
        if let Some(running) = self.running.remove(&id) {
            info!(
                stream_id = %id,
                name = %running.config.name,
                "Stopping stream"
            );
            running.cancel.cancel();
            running.handle.abort();
        }
    }

    /// Returns the number of currently running streams.
    pub fn running_count(&self) -> usize {
        self.running.len()
    }

    /// Returns the IDs and names of currently running streams for stats reporting.
    pub fn running_streams(&self) -> Vec<(Uuid, String, StreamFormat)> {
        self.running
            .values()
            .map(|r| (r.config.id, r.config.name.clone(), r.config.format.clone()))
            .collect()
    }
}

/// Check if stream config fields that affect the connection have changed.
fn stream_config_changed(old: &DataStream, new: &DataStream) -> bool {
    old.host != new.host
        || old.port != new.port
        || old.format != new.format
        || old.callsign != new.callsign
        || old.filter != new.filter
}

/// Run a stream client task with cancellation support.
async fn run_stream_task(
    stream: DataStream,
    resources: Arc<SharedResources>,
    retry_delay: u64,
    cancel: CancellationToken,
) {
    match stream.format {
        StreamFormat::Aprs => {
            run_aprs_stream(stream, resources, retry_delay, cancel).await;
        }
        StreamFormat::Adsb => {
            run_beast_stream(stream, resources, retry_delay, cancel).await;
        }
        StreamFormat::Sbs => {
            run_sbs_stream(stream, resources, retry_delay, cancel).await;
        }
    }
}

async fn run_aprs_stream(
    stream: DataStream,
    resources: Arc<SharedResources>,
    retry_delay: u64,
    cancel: CancellationToken,
) {
    let callsign = stream
        .callsign
        .clone()
        .unwrap_or_else(|| "N0CALL".to_string());

    // OGN auto-port-switching: use port 10152 for full feed if no filter specified
    let port = if stream.filter.is_none() && stream.port == 14580 {
        info!(
            stream_id = %stream.id,
            name = %stream.name,
            "No filter specified, switching from port 14580 to 10152 for full feed"
        );
        10152
    } else {
        stream.port
    };

    let config = AprsClientConfigBuilder::new()
        .server(&stream.host)
        .port(port)
        .callsign(callsign)
        .filter(stream.filter.clone())
        .retry_delay_seconds(retry_delay)
        .build();

    let mut client = AprsClient::new(config);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!(stream_id = %stream.id, name = %stream.name, "APRS stream cancelled");
                return;
            }
            result = client.start(
                resources.queue.clone(),
                resources.aprs_health.clone(),
                Some(resources.stats_ogn_received.clone()),
            ) => {
                match result {
                    Ok(_) => {
                        info!(stream_id = %stream.id, name = %stream.name, "APRS client stopped normally");
                        return;
                    }
                    Err(e) => {
                        error!(stream_id = %stream.id, name = %stream.name, error = %e, "APRS client failed");
                    }
                }
            }
        }
    }
}

async fn run_beast_stream(
    stream: DataStream,
    resources: Arc<SharedResources>,
    retry_delay: u64,
    cancel: CancellationToken,
) {
    let config = BeastClientConfig {
        server: stream.host.clone(),
        port: stream.port,
        retry_delay_seconds: retry_delay,
        max_retry_delay_seconds: 60,
    };

    let mut client = BeastClient::new(config);

    tokio::select! {
        _ = cancel.cancelled() => {
            info!(stream_id = %stream.id, name = %stream.name, "Beast stream cancelled");
        }
        result = client.start(
            resources.queue.clone(),
            resources.beast_health.clone(),
            Some(resources.stats_beast_received.clone()),
        ) => {
            match result {
                Ok(_) => {
                    info!(stream_id = %stream.id, name = %stream.name, "Beast client stopped normally");
                }
                Err(e) => {
                    error!(stream_id = %stream.id, name = %stream.name, error = %e, "Beast client failed");
                }
            }
        }
    }
}

async fn run_sbs_stream(
    stream: DataStream,
    resources: Arc<SharedResources>,
    retry_delay: u64,
    cancel: CancellationToken,
) {
    let config = SbsClientConfig {
        server: stream.host.clone(),
        port: stream.port,
        retry_delay_seconds: retry_delay,
        max_retry_delay_seconds: 60,
    };

    let mut client = SbsClient::new(config);

    tokio::select! {
        _ = cancel.cancelled() => {
            info!(stream_id = %stream.id, name = %stream.name, "SBS stream cancelled");
        }
        result = client.start(
            resources.queue.clone(),
            resources.sbs_health.clone(),
            Some(resources.stats_sbs_received.clone()),
        ) => {
            match result {
                Ok(_) => {
                    info!(stream_id = %stream.id, name = %stream.name, "SBS client stopped normally");
                }
                Err(e) => {
                    error!(stream_id = %stream.id, name = %stream.name, error = %e, "SBS client failed");
                }
            }
        }
    }
}

/// Spawn a file watcher that reloads config and applies changes on modification.
///
/// The `manager` is wrapped in `Arc<tokio::sync::Mutex<>>` so it can be shared
/// between the watcher task and other consumers (like stats reporting).
pub fn spawn_config_watcher(
    path: PathBuf,
    manager: Arc<tokio::sync::Mutex<StreamManager>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        use notify::{Event, EventKind, RecursiveMode, Watcher};

        let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);

        // Watch the parent directory since the file may be atomically replaced
        let watch_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let config_path = path.clone();

        let mut watcher = match notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        // Check if the event is for our config file
                        if event
                            .paths
                            .iter()
                            .any(|p| p.ends_with(config_path.file_name().unwrap_or_default()))
                        {
                            let _ = tx.try_send(());
                        }
                    }
                    _ => {}
                }
            }
        }) {
            Ok(w) => w,
            Err(e) => {
                error!(error = %e, "Failed to create file watcher");
                return;
            }
        };

        if let Err(e) = watcher.watch(&watch_dir, RecursiveMode::NonRecursive) {
            error!(error = %e, path = ?watch_dir, "Failed to watch directory");
            return;
        }

        info!(path = ?path, "Config file watcher started");

        // Debounce: wait 500ms after last event before reloading
        loop {
            if rx.recv().await.is_none() {
                break;
            }

            // Debounce: drain any additional events within 500ms
            tokio::time::sleep(Duration::from_millis(500)).await;
            while rx.try_recv().is_ok() {}

            info!(path = ?path, "Config file changed, reloading...");

            match IngestConfigFile::load(&path) {
                Ok(config) => {
                    let mut mgr = manager.lock().await;
                    mgr.apply_config(&config);
                    info!(
                        running = mgr.running_count(),
                        "Config reloaded successfully"
                    );
                }
                Err(e) => {
                    error!(error = %e, "Failed to reload config file");
                }
            }
        }
    })
}
