# Tokio Spawned Tasks Inventory

Audit of all `tokio::spawn` and `tokio::task::spawn_blocking` usage across the codebase.
Focus is on **long-lived background tasks** that, if they panic, would silently die and degrade the system.

## Legend

- **Fire-and-forget**: JoinHandle is not stored or awaited — panic goes undetected
- **Awaited**: JoinHandle is `.await`ed — panic propagates as `JoinError`
- **Short-lived**: Completes quickly (DB query, one-shot work) — less critical
- **Long-lived**: Runs for the lifetime of the process — critical if it dies

---

## `run` Command (soar-run)

The main real-time processing pipeline. Most critical tasks live here.

### Long-lived, fire-and-forget (CRITICAL)

| File | Line | Task | Workers | Description |
|------|------|------|---------|-------------|
| `src/commands/run/mod.rs` | 88 | Metrics server | 1 | Serves Prometheus `/metrics` endpoint |
| `src/commands/run/mod.rs` | 366 | Socket accept loop | 1 | Accepts ingest connections on Unix socket |
| `src/commands/run/mod.rs` | 376 | Envelope router | 1 | Routes envelopes from socket to per-source intake queues |
| `src/commands/run/workers.rs` | 21 | OGN intake worker | 1 | Reads raw OGN/APRS messages from intake queue, routes through PacketRouter |
| `src/commands/run/workers.rs` | 66 | Beast intake workers | 200 | Process raw Beast binary frames from intake queue |
| `src/commands/run/workers.rs` | 114 | SBS intake workers | 50 | Process raw SBS CSV messages from intake queue |
| `src/commands/run/workers.rs` | 177 | Aircraft position workers | 80 | Process aircraft positions (heaviest — FixProcessor + flight tracking) |
| `src/commands/run/workers.rs` | 203 | Receiver status workers | 6 | Process receiver status packets |
| `src/commands/run/workers.rs` | 229 | Receiver position workers | 4 | Process receiver position packets |
| `src/commands/run/workers.rs` | 251 | Server status workers | 2 | Process server status messages |
| `src/commands/run/monitoring.rs` | 27 | Queue depth reporter | 1 | Reports queue depths + DB pool state every 10s |
| `src/commands/run/shutdown.rs` | 22 | Shutdown handler | 1 | Listens for Ctrl+C, drains queues before exit |
| `src/packet_processors/router.rs` | 87 | PacketRouter workers | N (configurable) | Parse APRS packets and route to per-type queues |
| `src/nats_publisher.rs` | 114 | NATS fix publisher | 1 | Publishes fixes to NATS for live WebSocket clients |
| `src/archive_service.rs` | 63 | Archive file writer | 1 | Writes raw messages to daily log files |
| `src/flight_tracker/mod.rs` | 398 | Flight timeout checker | 1 | Periodically checks for timed-out flights (every 60s) |
| `src/flight_tracker/mod.rs` | 418 | Aircraft state cleanup | 1 | Removes stale aircraft states (every hour) |

### Short-lived, fire-and-forget (lower risk)

| File | Line | Task | Description |
|------|------|------|-------------|
| `src/fix_processor.rs` | 407 | Receiver timestamp update | Updates receiver's `latest_packet_at` after each fix |
| `src/flight_tracker/flight_lifecycle.rs` | 139 | Flight creation enrichment | Geocodes departure airport after flight created |
| `src/flight_tracker/flight_lifecycle.rs` | 247 | Airborne enrichment | Creates start location via geocoding when airborne |
| `src/flight_tracker/flight_lifecycle.rs` | 417 | Flight completion background | Full flight enrichment on landing |
| `src/flight_tracker/flight_lifecycle.rs` | 720 | Flight email notification | Sends email when a tracked flight completes |
| `src/flight_tracker/flight_lifecycle.rs` | 1100 | Completion enrichment | Post-completion airport/runway matching |
| `src/flight_tracker/geofence_alerts.rs` | 127 | Geofence exit alert | Sends email alerts for geofence exits |
| `src/archive_service.rs` | 86 | Archive file compression | Compresses previous day's log file |
| `src/socket_server.rs` | 83 | Per-connection handler | Handles a single ingest connection (lives as long as connection) |
| `src/actions/auth.rs` | 73 | Signup notification | Sends admin email about new user signup |

### Startup, fire-and-forget

| File | Line | Task | Description |
|------|------|------|-------------|
| `src/flight_tracker/mod.rs` | 331 | Startup enrichment coordinator | Enriches orphaned flights found during init |
| `src/flight_tracker/mod.rs` | 356 | Per-flight startup enrichment | Individual flight enrichment (up to 10 concurrent) |

---

## `ingest` Command (soar-ingest)

Connects to OGN/Beast/SBS sources and forwards messages to soar-run via Unix socket.

### Long-lived, fire-and-forget (CRITICAL)

| File | Line | Task | Description |
|------|------|------|-------------|
| `src/commands/ingest.rs` | 137 | Metrics server | Serves Prometheus `/metrics` endpoint |
| `src/commands/ingest.rs` | 237 | Connection status monitor | Checks source health every 1s, publishes changes to NATS |
| `src/commands/ingest.rs` | 331 | Unified publisher | Reads from persistent queue, sends to Unix socket |
| `src/commands/ingest.rs` | 422 | Stats reporter | Logs throughput stats every 30s, updates Prometheus gauges |
| `src/commands/ingest.rs` | 637 | OGN client | Connects to OGN/APRS server, enqueues messages |
| `src/commands/ingest.rs` | 680 | Beast client(s) | Connects to Beast ADS-B server(s), enqueues messages (one per server) |
| `src/commands/ingest.rs` | 713 | SBS client(s) | Connects to SBS server(s), enqueues messages (one per server) |
| `src/connection_status.rs` | 124 | Periodic status publish | Publishes connection status to NATS every 60s |

### Inside APRS/Beast/SBS clients (spawned internally)

| File | Line | Task | Description |
|------|------|------|-------------|
| `src/aprs_client.rs` | 99 | APRS queue feeder | Reads lines from TCP, enqueues to persistent queue |
| `src/aprs_client.rs` | 153 | APRS connection manager | Manages TCP reconnection with retry logic |
| `src/beast/client.rs` | 107 | Beast TCP reader | Reads Beast binary frames from TCP |
| `src/beast/client.rs` | 116 | Beast queue publisher | Publishes decoded frames to persistent queue |
| `src/beast/client.rs` | 138 | Beast frame processor | Decodes individual Beast frames |
| `src/beast/client.rs` | 325 | Beast queue feeder (envelope mode) | Reads Beast frames, creates protobuf envelopes |
| `src/beast/client.rs` | 456 | Beast file feeder (envelope mode) | Reads Beast frames from file, creates envelopes |
| `src/sbs/client.rs` | 99 | SBS queue publisher | Publishes SBS frames to persistent queue |
| `src/sbs/client.rs` | 104 | SBS TCP reader | Reads SBS CSV lines from TCP |
| `src/sbs/client.rs` | 188 | SBS queue feeder (envelope mode) | Reads SBS lines, creates protobuf envelopes |
| `src/sbs/client.rs` | 322 | SBS file feeder (envelope mode) | Reads SBS lines from file, creates envelopes |

---

## `web` Command (soar-web)

HTTP server and WebSocket handlers.

### Long-lived, fire-and-forget

| File | Line | Task | Description |
|------|------|------|-------------|
| `src/web.rs` | 583 | Process metrics task | Reports RSS/CPU metrics periodically |
| `src/web.rs` | 586 | Analytics metrics task | Updates analytics summary gauges every 60s |
| `src/live_fixes.rs` | 108 | NATS status subscription | Listens for connection status updates from NATS |
| `src/live_fixes.rs` | 178 | Per-aircraft NATS subscription | Forwards NATS fixes for a specific aircraft to WebSocket broadcast |
| `src/live_fixes.rs` | 234 | Per-area NATS subscription | Forwards NATS fixes for a geographic area to WebSocket broadcast |

### Per-request, short-lived

| File | Line | Task | Description |
|------|------|------|-------------|
| `src/actions/fixes.rs` | 292 | WebSocket read task | Reads subscription messages from WebSocket client |
| `src/actions/fixes.rs` | 297 | WebSocket write task | Writes live fix messages to WebSocket client |
| `src/actions/fixes.rs` | 302 | WebSocket subscription manager | Manages NATS subscriptions for a WebSocket session |

---

## `pull-data` / `load-data` Commands

Short-lived batch operations.

| File | Line | Task | Description |
|------|------|------|-------------|
| `src/commands/pull_data.rs` | 162 | Metrics server | Serves metrics during data pull operation |
| `src/commands/run_aggregates.rs` | 140 | Per-day aggregation | Parallel coverage aggregation per day×resolution |

---

## `spawn_blocking` (Diesel DB operations)

All `tokio::task::spawn_blocking` calls are **short-lived and awaited** — they exist to run synchronous Diesel database queries off the async runtime. These are safe because:

1. The JoinHandle is immediately `.await`ed (panic propagates)
2. They complete quickly (single DB query)
3. There are ~200+ of these across `*_repo.rs` files

**Files with spawn_blocking:** `analytics_repo.rs`, `aircraft_repo.rs`, `airports_repo.rs`, `airspaces_repo.rs`, `clubs_repo.rs`, `club_tow_fees_repo.rs`, `coverage_repo.rs`, `faa/aircraft_model_repo.rs`, `fixes_repo.rs`, `flights_repo.rs`, `geofence_repo.rs`, `locations_repo.rs`, `pilots_repo.rs`, `raw_messages_repo.rs`, `receiver_repo.rs`, `receiver_status_repo.rs`, `runways_repo.rs`, `server_messages_repo.rs`, `user_fixes_repo.rs`, `users_repo.rs`, `watchlist_repo.rs`, `elevation/hgt.rs`, `commands/sitemap.rs`, `commands/run_aggregates.rs`, `commands/archive/*.rs`, `commands/load_data/*.rs`

---

## `beast_consumer_task.rs` (NATS Beast consumer)

| File | Line | Task | Description |
|------|------|------|-------------|
| `src/beast_consumer_task.rs` | 56 | Batch writer | Batches Beast messages and writes to DB (awaited) |

---

## `metrics.rs` (duplicated spawn)

| File | Line | Task | Description |
|------|------|------|-------------|
| `src/metrics.rs` | 1066 | Process metrics task | Spawned inside `start_metrics_server()` — may duplicate with explicit spawns in `run`/`web` |

---

## Panic Detection Gaps

### No JoinHandle monitoring
The vast majority of `tokio::spawn()` calls in `run` and `ingest` are fire-and-forget — the `JoinHandle` is never stored or awaited. If any of these tasks panic, the task dies silently and the process continues running in a degraded state.

### Only two places check JoinHandle for panic
1. `src/aprs_client.rs:225` — `tokio::join!` on queue feeder + connection handle, propagates via `?`
2. `src/elevation/hgt.rs:61` — `spawn_blocking` result checked with `??`

### Sentry captures panics (partially)
The `sentry-panic` feature installs a global `std::panic::set_hook` that fires before tokio catches the panic. So panics ARE reported to Sentry, but:
- No email notification is sent
- The process continues running with a dead task
- No automatic restart of the failed task
