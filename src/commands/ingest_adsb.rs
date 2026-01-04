use anyhow::{Context, Result};
use soar::beast::{BeastClient, BeastClientConfig};
use soar::instance_lock::InstanceLock;
use soar::sbs::{SbsClient, SbsClientConfig};
use std::env;
use tracing::Instrument;
use tracing::{error, info, warn};

/// Parse a "host:port" string into (hostname, port)
fn parse_server_address(addr: &str) -> Result<(String, u16)> {
    let parts: Vec<&str> = addr.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!(
            "Invalid server address '{}' - expected format 'host:port'",
            addr
        ));
    }
    let host = parts[0].to_string();
    let port = parts[1]
        .parse::<u16>()
        .context(format!("Invalid port in '{}'", addr))?;
    Ok((host, port))
}

pub async fn handle_ingest_adsb(
    beast_servers: Vec<String>,
    sbs_servers: Vec<String>,
    max_retries: u32,
    retry_delay: u64,
) -> Result<()> {
    sentry::configure_scope(|scope| {
        scope.set_tag("operation", "ingest-adsb");
    });

    // Validate that at least one server is specified
    if beast_servers.is_empty() && sbs_servers.is_empty() {
        return Err(anyhow::anyhow!(
            "No servers specified - use --beast or --sbs to specify at least one server"
        ));
    }

    // Determine environment
    let soar_env = env::var("SOAR_ENV").unwrap_or_default();
    let is_production = soar_env == "production";
    let is_staging = soar_env == "staging";

    info!(
        "Starting ADS-B ingestion service - Beast servers: {:?}, SBS servers: {:?}",
        beast_servers, sbs_servers
    );

    info!(
        "Environment: {}",
        if is_production {
            "production"
        } else if is_staging {
            "staging"
        } else {
            "development"
        }
    );

    // Initialize health state for this ingester
    let health_state = soar::metrics::init_beast_health();
    soar::metrics::set_beast_health(health_state.clone());

    // Initialize all ADS-B ingester metrics to zero so they appear in Grafana even before events occur
    // This MUST happen before starting the metrics server to avoid race conditions where
    // Prometheus scrapes before metrics are initialized
    info!("Initializing ADS-B ingester metrics...");
    soar::metrics::initialize_beast_ingest_metrics();
    info!("ADS-B ingester metrics initialized");

    // Start metrics server in production/staging mode (AFTER metrics are initialized)
    if is_production || is_staging {
        // Allow overriding metrics port via METRICS_PORT env var (for blue-green deployment)
        // Auto-assign default based on environment: production=9094, staging=9096
        let default_port = if is_staging { 9096 } else { 9094 };
        let metrics_port = env::var("METRICS_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(default_port);

        info!("Starting metrics server on port {}", metrics_port);
        tokio::spawn(
            async move {
                soar::metrics::start_metrics_server(metrics_port).await;
            }
            .instrument(tracing::info_span!("metrics_server")),
        );
    }

    // Acquire instance lock to prevent multiple ingest instances from running
    let lock_name = if is_production {
        "adsb-ingest-production"
    } else {
        "adsb-ingest-dev"
    };
    let _lock = InstanceLock::new(lock_name)
        .context("Failed to acquire instance lock - is another adsb-ingest instance running?")?;
    info!("Instance lock acquired for {}", lock_name);

    // Set up signal handling for immediate shutdown
    info!("Setting up shutdown handlers...");
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Spawn signal handler task for both SIGINT and SIGTERM
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};

            let mut sigterm =
                signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");
            let mut sigint =
                signal(SignalKind::interrupt()).expect("Failed to register SIGINT handler");

            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, exiting immediately...");
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT (Ctrl+C), exiting immediately...");
                }
            }
        }

        #[cfg(not(unix))]
        {
            match tokio::signal::ctrl_c().await {
                Ok(()) => {
                    info!("Received SIGINT (Ctrl+C), exiting immediately...");
                }
                Err(err) => {
                    error!("Failed to listen for SIGINT signal: {}", err);
                    return;
                }
            }
        }

        // Signal shutdown
        let _ = shutdown_tx.send(());
    });

    // Create persistent queues for buffering messages
    let beast_queue_path = std::path::PathBuf::from("/var/lib/soar/queues/adsb-beast.queue");
    let beast_queue = std::sync::Arc::new(
        soar::persistent_queue::PersistentQueue::<Vec<u8>>::new(
            "adsb-beast".to_string(),
            beast_queue_path,
            Some(10 * 1024 * 1024 * 1024), // 10 GB max
            1000,                          // Memory capacity
        )
        .expect("Failed to create Beast persistent queue"),
    );

    let sbs_queue_path = std::path::PathBuf::from("/var/lib/soar/queues/adsb-sbs.queue");
    let sbs_queue = std::sync::Arc::new(
        soar::persistent_queue::PersistentQueue::<Vec<u8>>::new(
            "adsb-sbs".to_string(),
            sbs_queue_path,
            Some(10 * 1024 * 1024 * 1024), // 10 GB max
            1000,                          // Memory capacity
        )
        .expect("Failed to create SBS persistent queue"),
    );

    info!("Created persistent queues at /var/lib/soar/queues/adsb-*.queue");

    // Create socket clients for sending to soar-run
    let socket_path = std::path::PathBuf::from("/var/run/soar/run.sock");
    let mut beast_socket_client = match soar::socket_client::SocketClient::connect(
        &socket_path,
        soar::protocol::IngestSource::Beast,
    )
    .await
    {
        Ok(client) => {
            info!("Beast socket connected to soar-run at {:?}", socket_path);
            client
        }
        Err(e) => {
            warn!(
                "Failed to connect Beast socket to soar-run (will buffer to queue): {}",
                e
            );
            // Create disconnected client - it will connect later when soar-run is available
            soar::socket_client::SocketClient::new(
                &socket_path,
                soar::protocol::IngestSource::Beast,
            )
        }
    };

    let mut sbs_socket_client = match soar::socket_client::SocketClient::connect(
        &socket_path,
        soar::protocol::IngestSource::Sbs,
    )
    .await
    {
        Ok(client) => {
            info!("SBS socket connected to soar-run at {:?}", socket_path);
            client
        }
        Err(e) => {
            warn!(
                "Failed to connect SBS socket to soar-run (will buffer to queue): {}",
                e
            );
            // Create disconnected client - it will connect later when soar-run is available
            soar::socket_client::SocketClient::new(&socket_path, soar::protocol::IngestSource::Sbs)
        }
    };

    // Mark socket as connected in health state (reuse nats_connected field for now)
    {
        let mut health = health_state.write().await;
        health.nats_connected =
            beast_socket_client.is_connected() || sbs_socket_client.is_connected();
    }

    // Connect consumers to queues (transition from Disconnected to Connected/Draining state)
    beast_queue
        .connect_consumer("beast-publisher".to_string())
        .await
        .expect("Failed to connect Beast consumer to queue");
    sbs_queue
        .connect_consumer("sbs-publisher".to_string())
        .await
        .expect("Failed to connect SBS consumer to queue");

    // Spawn Beast publisher task: reads from queue → sends to socket
    let beast_queue_for_publisher = beast_queue.clone();
    let beast_publisher_handle = tokio::spawn(async move {
        info!("Beast publisher task started");
        loop {
            match beast_queue_for_publisher.recv().await {
                Ok(message) => {
                    // Send to socket
                    if let Err(e) = beast_socket_client.send(message).await {
                        error!("Failed to send Beast message to socket: {}", e);
                        metrics::counter!("beast.socket.send_error_total").increment(1);

                        // Reconnect and retry
                        if let Err(e) = beast_socket_client.reconnect().await {
                            error!("Failed to reconnect Beast socket: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to receive from Beast queue: {}", e);
                    break;
                }
            }
        }
        info!("Beast publisher task stopped");
    });

    // Spawn SBS publisher task: reads from queue → sends to socket
    let sbs_queue_for_publisher = sbs_queue.clone();
    let sbs_publisher_handle = tokio::spawn(async move {
        info!("SBS publisher task started");
        loop {
            match sbs_queue_for_publisher.recv().await {
                Ok(message) => {
                    // Send to socket
                    if let Err(e) = sbs_socket_client.send(message).await {
                        error!("Failed to send SBS message to socket: {}", e);
                        metrics::counter!("sbs.socket.send_error_total").increment(1);

                        // Reconnect and retry
                        if let Err(e) = sbs_socket_client.reconnect().await {
                            error!("Failed to reconnect SBS socket: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to receive from SBS queue: {}", e);
                    break;
                }
            }
        }
        info!("SBS publisher task stopped");
    });

    // Create broadcast channel for shutdown signal (multiple tasks need to receive it)
    let (shutdown_broadcast_tx, _) = tokio::sync::broadcast::channel::<()>(1);

    // Spawn signal handler task
    let shutdown_tx_clone = shutdown_broadcast_tx.clone();
    tokio::spawn(async move {
        let _ = shutdown_rx.await;
        info!("Shutdown signal received, broadcasting to all client tasks...");
        let _ = shutdown_tx_clone.send(());
    });

    // Spawn tasks for each Beast server
    let mut client_handles = vec![];

    for beast_addr in &beast_servers {
        let (server, port) = parse_server_address(beast_addr)?;
        let config = BeastClientConfig {
            server: server.clone(),
            port,
            max_retries,
            retry_delay_seconds: retry_delay,
            max_retry_delay_seconds: 60,
        };

        let queue = beast_queue.clone();
        let health = health_state.clone();
        let shutdown_rx = shutdown_broadcast_tx.subscribe();
        let (client_shutdown_tx, client_shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        // Bridge broadcast to oneshot
        tokio::spawn(async move {
            let mut rx = shutdown_rx;
            if rx.recv().await.is_ok() {
                let _ = client_shutdown_tx.send(());
            }
        });

        info!("Spawning Beast client for {}:{}", server, port);
        let server_clone = server.clone();
        let handle = tokio::spawn(
            async move {
                let mut client = BeastClient::new(config);
                match client
                    .start_with_queue(queue, client_shutdown_rx, health)
                    .await
                {
                    Ok(_) => info!("Beast client {}:{} stopped normally", server, port),
                    Err(e) => error!("Beast client {}:{} failed: {}", server, port, e),
                }
            }
            .instrument(tracing::info_span!("beast_client", server = %server_clone, port = %port)),
        );
        client_handles.push(handle);
    }

    // Spawn tasks for each SBS server
    for sbs_addr in &sbs_servers {
        let (server, port) = parse_server_address(sbs_addr)?;
        let config = SbsClientConfig {
            server: server.clone(),
            port,
            max_retries,
            retry_delay_seconds: retry_delay,
            max_retry_delay_seconds: 60,
        };

        let queue = sbs_queue.clone();
        let health = health_state.clone();
        let shutdown_rx = shutdown_broadcast_tx.subscribe();
        let (client_shutdown_tx, client_shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        // Bridge broadcast to oneshot
        tokio::spawn(async move {
            let mut rx = shutdown_rx;
            if rx.recv().await.is_ok() {
                let _ = client_shutdown_tx.send(());
            }
        });

        info!("Spawning SBS client for {}:{}", server, port);
        let server_clone = server.clone();
        let handle = tokio::spawn(
            async move {
                let mut client = SbsClient::new(config);
                match client
                    .start_with_queue(queue, client_shutdown_rx, health)
                    .await
                {
                    Ok(_) => info!("SBS client {}:{} stopped normally", server, port),
                    Err(e) => error!("SBS client {}:{} failed: {}", server, port, e),
                }
            }
            .instrument(tracing::info_span!("sbs_client", server = %server_clone, port = %port)),
        );
        client_handles.push(handle);
    }

    info!(
        "Started {} Beast client(s) and {} SBS client(s)",
        beast_servers.len(),
        sbs_servers.len()
    );

    // Wait for all client tasks to complete
    for handle in client_handles {
        let _ = handle.await;
    }

    info!("All client tasks completed, waiting for publisher tasks...");

    // Wait for publisher tasks to complete
    let _ = beast_publisher_handle.await;
    let _ = sbs_publisher_handle.await;

    info!("All tasks completed, shutting down");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_server_address_valid() {
        let result = parse_server_address("localhost:30005");
        assert!(result.is_ok());
        let (host, port) = result.unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 30005);
    }

    #[test]
    fn test_parse_server_address_ipv4() {
        let result = parse_server_address("192.168.1.100:8080");
        assert!(result.is_ok());
        let (host, port) = result.unwrap();
        assert_eq!(host, "192.168.1.100");
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_parse_server_address_hostname() {
        let result = parse_server_address("data.adsbhub.org:5002");
        assert!(result.is_ok());
        let (host, port) = result.unwrap();
        assert_eq!(host, "data.adsbhub.org");
        assert_eq!(port, 5002);
    }

    #[test]
    fn test_parse_server_address_with_dash() {
        let result = parse_server_address("my-server.example.com:12345");
        assert!(result.is_ok());
        let (host, port) = result.unwrap();
        assert_eq!(host, "my-server.example.com");
        assert_eq!(port, 12345);
    }

    #[test]
    fn test_parse_server_address_missing_port() {
        let result = parse_server_address("localhost");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("expected format 'host:port'")
        );
    }

    #[test]
    fn test_parse_server_address_invalid_port() {
        let result = parse_server_address("localhost:invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid port"));
    }

    #[test]
    fn test_parse_server_address_port_out_of_range() {
        let result = parse_server_address("localhost:99999");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_server_address_extra_colons() {
        let result = parse_server_address("localhost:30005:extra");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("expected format 'host:port'")
        );
    }

    #[test]
    fn test_parse_server_address_empty_string() {
        let result = parse_server_address("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_server_address_only_colon() {
        let result = parse_server_address(":");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_server_address_empty_host() {
        let result = parse_server_address(":30005");
        assert!(result.is_ok()); // Empty host is technically valid (means bind to all interfaces)
        let (host, port) = result.unwrap();
        assert_eq!(host, "");
        assert_eq!(port, 30005);
    }

    #[test]
    fn test_parse_server_address_empty_port() {
        let result = parse_server_address("localhost:");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid port"));
    }

    #[test]
    fn test_parse_server_address_port_zero() {
        let result = parse_server_address("localhost:0");
        assert!(result.is_ok());
        let (host, port) = result.unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 0);
    }

    #[test]
    fn test_parse_server_address_max_port() {
        let result = parse_server_address("localhost:65535");
        assert!(result.is_ok());
        let (host, port) = result.unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 65535);
    }

    #[test]
    fn test_multiple_beast_servers() {
        // This test verifies the structure for handling multiple servers
        let beast_servers = vec![
            "server1.example.com:30005".to_string(),
            "server2.example.com:30005".to_string(),
            "192.168.1.100:30005".to_string(),
        ];

        let sbs_servers: Vec<String> = vec![];

        // Verify all servers can be parsed
        for addr in &beast_servers {
            let result = parse_server_address(addr);
            assert!(
                result.is_ok(),
                "Failed to parse address: {}",
                result.unwrap_err()
            );
        }

        // Verify we have the expected number of servers
        assert_eq!(beast_servers.len(), 3);
        assert_eq!(sbs_servers.len(), 0);
    }

    #[test]
    fn test_multiple_sbs_servers() {
        let beast_servers: Vec<String> = vec![];
        let sbs_servers = vec![
            "data.adsbhub.org:5002".to_string(),
            "backup.adsbhub.org:5002".to_string(),
        ];

        // Verify all servers can be parsed
        for addr in &sbs_servers {
            let result = parse_server_address(addr);
            assert!(
                result.is_ok(),
                "Failed to parse address: {}",
                result.unwrap_err()
            );
        }

        assert_eq!(beast_servers.len(), 0);
        assert_eq!(sbs_servers.len(), 2);
    }

    #[test]
    fn test_mixed_beast_and_sbs_servers() {
        let beast_servers = vec![
            "radar.example.com:30005".to_string(),
            "192.168.1.50:30005".to_string(),
        ];
        let sbs_servers = vec!["data.adsbhub.org:5002".to_string()];

        // Verify all servers can be parsed
        for addr in &beast_servers {
            assert!(parse_server_address(addr).is_ok());
        }
        for addr in &sbs_servers {
            assert!(parse_server_address(addr).is_ok());
        }

        assert_eq!(beast_servers.len(), 2);
        assert_eq!(sbs_servers.len(), 1);
    }

    #[test]
    fn test_realistic_server_configurations() {
        // Production-like configuration
        let configs = vec![
            // Single Beast server
            (vec!["radar:41365"], vec![]),
            // Multiple Beast servers for redundancy
            (vec!["radar1:30005", "radar2:30005", "radar3:30005"], vec![]),
            // SBS server only
            (vec![], vec!["data.adsbhub.org:5002"]),
            // Mixed configuration
            (vec!["radar:41365"], vec!["data.adsbhub.org:5002"]),
        ];

        for (beast, sbs) in configs {
            for addr in &beast {
                assert!(
                    parse_server_address(addr).is_ok(),
                    "Failed to parse Beast address: {}",
                    addr
                );
            }
            for addr in &sbs {
                assert!(
                    parse_server_address(addr).is_ok(),
                    "Failed to parse SBS address: {}",
                    addr
                );
            }
        }
    }
}
