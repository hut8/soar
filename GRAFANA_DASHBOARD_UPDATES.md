# Grafana Dashboard Updates for Per-Processor Queue Architecture

## Summary
The APRS processing architecture has been refactored to move queues from AprsClient to PacketRouter, with dedicated queues and worker pools per processor type. The Grafana dashboard needs to be updated to reflect these changes.

## Metrics Changes

### REMOVED Metrics (old single queue)
- `aprs.worker.processed` (total, with worker_id label)
- `aprs.queue.full` (single queue)
- `aprs.queue.closed` (single queue)
- `aprs.processing.duration_ms` (total processing time)

### NEW Metrics (per-processor queues)

#### Queue Full Metrics
- `aprs.aircraft_queue.full` - Aircraft queue full drops
- `aprs.receiver_status_queue.full` - Receiver status queue full drops
- `aprs.receiver_position_queue.full` - Receiver position queue full drops
- `aprs.server_status_queue.full` - Server status queue full drops

#### Queue Closed Metrics
- `aprs.aircraft_queue.closed` - Aircraft queue closed
- `aprs.receiver_status_queue.closed` - Receiver status queue closed
- `aprs.receiver_position_queue.closed` - Receiver position queue closed
- `aprs.server_status_queue.closed` - Server status queue closed

#### Processing Throughput Metrics
- `aprs.aircraft.processed` - Aircraft positions processed
- `aprs.receiver_status.processed` - Receiver status messages processed
- `aprs.receiver_position.processed` - Receiver positions processed
- `aprs.server_status.processed` - Server status messages processed

#### Processing Duration Metrics (histograms)
- `aprs.aircraft.duration_ms` - Aircraft processing duration
- `aprs.receiver_status.duration_ms` - Receiver status processing duration
- `aprs.receiver_position.duration_ms` - Receiver position processing duration
- `aprs.server_status.duration_ms` - Server status processing duration

## Dashboard Panel Updates

### Panel: "APRS Queue" (around line 686)
**Current queries:**
```promql
sum(rate(aprs_worker_processed[1m]))  # Enqueued/sec
rate(aprs_queue_full[1m])             # Queue Full/sec
rate(aprs_queue_closed[1m])           # Queue Closed/sec
```

**UPDATE TO:**
```promql
# Queue Drops by Type
rate(aprs_aircraft_queue_full[1m])          # Aircraft Queue Full/sec
rate(aprs_receiver_status_queue_full[1m])   # Receiver Status Queue Full/sec
rate(aprs_receiver_position_queue_full[1m]) # Receiver Position Queue Full/sec
rate(aprs_server_status_queue_full[1m])     # Server Status Queue Full/sec
```

**Panel title:** Change from "APRS Queue" to "APRS Queue Drops by Processor"

### Panel: "APRS Packet Processing Duration" (around line 792)
**Current queries:**
```promql
aprs_processing_duration_ms{quantile="0.5"}
aprs_processing_duration_ms{quantile="0.95"}
aprs_processing_duration_ms{quantile="0.99"}
```

**UPDATE TO:**
```promql
# Aircraft Processing Duration
aprs_aircraft_duration_ms{quantile="0.5"}
aprs_aircraft_duration_ms{quantile="0.95"}
aprs_aircraft_duration_ms{quantile="0.99"}

# Receiver Status Processing Duration
aprs_receiver_status_duration_ms{quantile="0.5"}
aprs_receiver_status_duration_ms{quantile="0.95"}
aprs_receiver_status_duration_ms{quantile="0.99"}

# Receiver Position Processing Duration
aprs_receiver_position_duration_ms{quantile="0.5"}
aprs_receiver_position_duration_ms{quantile="0.95"}
aprs_receiver_position_duration_ms{quantile="0.99"}

# Server Status Processing Duration
aprs_server_status_duration_ms{quantile="0.5"}
aprs_server_status_duration_ms{quantile="0.95"}
aprs_server_status_duration_ms{quantile="0.99"}
```

**Panel title:** Keep as "APRS Packet Processing Duration" but add processor type to legend

### Panel: "APRS Worker Processing Rate" (around line 913)
**Current query:**
```promql
sum(rate(aprs_worker_processed[5m]))
```

**UPDATE TO:**
```promql
# Processing Rate by Type
rate(aprs_aircraft_processed[5m])          # Aircraft/sec
rate(aprs_receiver_status_processed[5m])   # Receiver Status/sec
rate(aprs_receiver_position_processed[5m]) # Receiver Position/sec
rate(aprs_server_status_processed[5m])     # Server Status/sec
```

**Panel title:** Change from "APRS Worker Processing Rate" to "APRS Processing Rate by Type"

### NEW Panel: "Processor Worker Counts" (add after "APRS Processing Rate by Type")
Add a new stat panel showing:
- Aircraft workers: 10
- Receiver status workers: 3
- Receiver position workers: 2
- Server status workers: 1

This can be a simple stat panel or annotation.

## Implementation Steps

1. Open `infrastructure/grafana-dashboard-run.json` in Grafana UI or text editor
2. Find the "APRS Queue" panel (search for `"title": "APRS Queue"`)
3. Update the panel queries as shown above
4. Find the "APRS Packet Processing Duration" panel
5. Update with per-processor metrics
6. Find the "APRS Worker Processing Rate" panel
7. Update with per-processor throughput metrics
8. Optionally add new panels for:
   - Queue depths (if we expose queue length metrics)
   - Per-processor worker utilization
9. Export the updated dashboard JSON and save to `infrastructure/grafana-dashboard-run.json`

## Testing

After updating:
1. Import the dashboard to Grafana
2. Run the `soar run` command
3. Verify all metrics are populating correctly
4. Check that queue full events show up in the appropriate panels
5. Verify processing duration histograms show data for each processor type

## Notes

- The elevation processing metrics remain unchanged (they already had their own dedicated queue)
- Connection metrics (established, failed, keepalive) remain unchanged
- Server message metrics remain unchanged
