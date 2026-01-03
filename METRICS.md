# New Metrics for Queue + Unix Socket Architecture

This document lists the new metrics introduced by replacing NATS with persistent queues and Unix sockets.

## PersistentQueue Metrics

All queue metrics follow the pattern `queue.{name}.*` where `{name}` is the queue identifier (e.g., "ogn", "adsb-beast", "adsb-sbs").

### Gauges
- `queue.{name}.state` - Queue state (0=disconnected, 1=connected, 2=draining)

### Counters
- `queue.{name}.messages.sent_total` - Messages sent through fast path (directly to consumer)
- `queue.{name}.messages.buffered_total` - Messages buffered to disk
- `queue.{name}.messages.received_total` - Messages received by consumer
- `queue.{name}.messages.drained_total` - Messages drained from backlog file
- `queue.{name}.corruption_total` - Corrupted messages detected (CRC32 failures)

### Histograms
- `queue.{name}.drain_duration_seconds` - Time taken to drain backlog

## Socket Server Metrics (soar-run)

### Gauges
- `socket.server.started` - Server startup indicator (1.0 when running)
- `socket.connections.active` - Number of active client connections

### Counters
- `socket.connections.accepted_total` - Total connections accepted
- `socket.connections.closed_total` - Total connections closed
- `socket.messages.received_total` - Total messages received from all clients

### Histograms
- `socket.message_size_bytes` - Size distribution of received messages

### Error Counters
- `socket.errors.accept_total` - Connection accept errors
- `socket.errors.message_too_large_total` - Messages exceeding size limit
- `socket.errors.zero_length_total` - Zero-length message errors
- `socket.errors.parse_total` - Protobuf parsing errors
- `socket.errors.queue_send_total` - Queue send errors

## Socket Client Metrics (ingesters)

### Gauges
- `socket.client.connected` - Connection status (1.0=connected, 0.0=disconnected)

### Counters
- `socket.client.messages.sent_total` - Total messages sent to server
- `socket.client.reconnects_total` - Successful reconnections
- `socket.client.reconnect_failures_total` - Failed reconnection attempts
- `socket.client.slow_sends_total` - Sends that took >100ms

### Histograms
- `socket.client.send_duration_ms` - Message send latency distribution

## Grafana Dashboard Updates Needed

### grafana-dashboard-ingest-ogn.json
Add panels for:
- Queue state for OGN queue (`queue.ogn.state`)
- Messages buffered vs sent rate (`queue.ogn.messages.buffered_total`, `queue.ogn.messages.sent_total`)
- Socket client connection status (`socket.client.connected`)
- Socket client reconnection rate (`socket.client.reconnects_total`)
- Send latency (`socket.client.send_duration_ms`)

### grafana-dashboard-ingest-adsb.json
Add panels for:
- Queue state for both queues (`queue.adsb-beast.state`, `queue.adsb-sbs.state`)
- Messages buffered vs sent rate for both queues
- Socket client metrics (same as OGN)

### grafana-dashboard-run-core.json
Add panels for:
- Socket server status (`socket.server.started`)
- Active connections (`socket.connections.active`)
- Connection rate (`socket.connections.accepted_total`)
- Message receive rate (`socket.messages.received_total`)

### grafana-dashboard-run-ingestion.json
Add panels for:
- Queue depth/state for all queues
- Drain operations (when queues transition to draining state)
- Message flow: ingester → queue → socket → soar-run

## Removed Metrics

The following NATS-related metrics from ingesters are no longer emitted:
- `aprs.nats.queue_depth` (replaced by `queue.ogn.state` and message counters)
- Any fire-and-forget publish metrics from NATS publishers

Note: `nats_publisher.*` metrics for external fix publishing (NatsFixPublisher) are still emitted as that functionality remains unchanged.
