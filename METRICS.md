# SOAR Metrics Reference

This document provides a comprehensive reference of all Prometheus metrics exposed by SOAR services.

**Last Updated:** 2026-01-02
**Metrics Endpoint:** `http://localhost:6970/metrics` (or respective service port)

## Metric Naming Conventions

- **Dots to Underscores:** Metrics are defined in code with dots (e.g., `aprs.connection.established_total`) but appear in Prometheus with underscores (e.g., `aprs_connection_established_total`)
- **Counter Suffixes:** Counters automatically get `_total` suffix in Prometheus
- **Histogram Suffixes:** Histograms generate multiple time series: `_count`, `_sum`, `_bucket`
- **Gauge:** No automatic suffix

## Service Overview

| Service | Port | Description |
|---------|------|-------------|
| `soar run` | 6970 | Main processing service (consumes from NATS, tracks flights) |
| `soar ingest-ogn` | 6971 | OGN/APRS-IS ingestion service |
| `soar ingest-adsb` | 6972 | ADS-B Beast protocol ingestion service |
| `soar web` | 6969 | Web API and WebSocket server |

---

## Process Metrics

**Service:** All services

| Metric | Type | Description |
|--------|------|-------------|
| `process.uptime.seconds` | Gauge | Process uptime in seconds since start |
| `process.is_up` | Gauge | Binary indicator (1.0 = process is running) |
| `process.memory.bytes` | Gauge | Process resident memory (RSS) in bytes (Linux only) |

---

## APRS/OGN Ingestion Metrics

**Service:** `soar ingest-ogn`

### Connection Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `aprs.connection.established_total` | Counter | Total APRS-IS connection establishments |
| `aprs.connection.ended_total` | Counter | Total APRS-IS connection terminations |
| `aprs.connection.failed_total` | Counter | Total APRS-IS connection failures |
| `aprs.connection.operation_failed_total` | Counter | Total APRS-IS operation failures |
| `aprs.connection.timeout_total` | Counter | Total APRS-IS connection timeouts |
| `aprs.connection.connected` | Gauge | Current connection status (1.0 = connected, 0.0 = disconnected) |

### Message Receiving Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `aprs.keepalive.sent_total` | Counter | Total keepalive messages sent to APRS-IS |
| `aprs.raw_message.received.server_total` | Counter | Server messages received from APRS-IS |
| `aprs.raw_message.received.aprs_total` | Counter | APRS messages received from APRS-IS |
| `aprs.raw_message.queued.server_total` | Counter | Server messages queued for processing |
| `aprs.raw_message.queued.aprs_total` | Counter | APRS messages queued for processing |

### NATS Publishing Metrics (Ingest-OGN → NATS)

| Metric | Type | Description |
|--------|------|-------------|
| `aprs.nats.published_total` | Counter | Total messages published to NATS |
| `aprs.nats.publish_error_total` | Counter | Total NATS publish errors |
| `aprs.nats.slow_publish_total` | Counter | Total slow publish operations (>100ms warning threshold) |
| `aprs.nats.publish_duration_ms` | Histogram | NATS publish latency in milliseconds |

### Queue Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `aprs.raw_message_queue.full_total` | Counter | Times raw message queue was full (messages dropped) |
| `aprs.aircraft_queue.full_total` | Counter | Times aircraft queue was full (messages dropped) |
| `aprs.aircraft_queue.closed_total` | Counter | Times aircraft queue send failed (channel closed) |
| `aprs.receiver_status_queue.full_total` | Counter | Times receiver status queue was full |
| `aprs.receiver_status_queue.closed_total` | Counter | Times receiver status queue send failed |
| `aprs.receiver_position_queue.full_total` | Counter | Times receiver position queue was full |
| `aprs.receiver_position_queue.closed_total` | Counter | Times receiver position queue send failed |
| `aprs.server_status_queue.full_total` | Counter | Times server status queue was full |
| `aprs.server_status_queue.closed_total` | Counter | Times server status queue send failed |

---

## APRS/OGN Processing Metrics

**Service:** `soar run`

### NATS Consumer Metrics (NATS → soar run)

| Metric | Type | Description |
|--------|------|-------------|
| `aprs.nats.intake_queue_depth` | Gauge | Current depth of NATS message intake queue |
| `aprs.nats.consumed_total` | Counter | Total APRS messages consumed from NATS |
| `aprs.nats.process_error_total` | Counter | Total message processing errors |
| `aprs.nats.decode_error_total` | Counter | Total message decode errors |
| `aprs.nats.ack_error_total` | Counter | Total NATS acknowledgment errors |
| `aprs.nats.receive_error_total` | Counter | Total NATS receive errors |
| `aprs.nats.acked_immediately_total` | Counter | Messages acknowledged without processing (optimization) |
| `aprs.nats.intake_queue_full_total` | Counter | Times intake queue was full |

### Message Processing by Type

| Metric | Type | Description |
|--------|------|-------------|
| `aprs.messages.processed.aircraft_total` | Counter | Aircraft position messages processed |
| `aprs.messages.processed.receiver_status_total` | Counter | Receiver status messages processed |
| `aprs.messages.processed.receiver_position_total` | Counter | Receiver position messages processed |
| `aprs.messages.processed.server_total` | Counter | Server messages processed |
| `aprs.messages.processed.total_total` | Counter | All messages processed (sum of above) |

### Aircraft Processing Latency

| Metric | Type | Description |
|--------|------|-------------|
| `aprs.aircraft.lookup_ms` | Histogram | Time to look up aircraft in database |
| `aprs.aircraft.fix_creation_ms` | Histogram | Time to create fix object from APRS message |
| `aprs.aircraft.process_fix_internal_ms` | Histogram | Internal fix processing time |
| `aprs.aircraft.total_processing_ms` | Histogram | Total end-to-end processing time for aircraft message |
| `aprs.aircraft.flight_insert_ms` | Histogram | Time to insert/update flight record |
| `aprs.aircraft.callsign_update_ms` | Histogram | Time to update aircraft callsign |
| `aprs.aircraft.elevation_queue_ms` | Histogram | Time to queue elevation lookup |
| `aprs.aircraft.nats_publish_ms` | Histogram | Time to publish fix to NATS for live broadcast |

### Granular Flight Insert Breakdown

| Metric | Type | Description |
|--------|------|-------------|
| `aprs.aircraft.state_transition_ms` | Histogram | Time for flight state machine transitions |
| `aprs.aircraft.fix_db_insert_ms` | Histogram | Time to insert fix into database |
| `aprs.aircraft.device_lookup_ms` | Histogram | Time to lookup/cache device record |
| `aprs.aircraft.flight_update_last_fix_ms` | Histogram | Time to update flight's last_fix timestamp |

### Queue Depth

| Metric | Type | Description |
|--------|------|-------------|
| `aprs.aircraft_queue.depth` | Gauge | Current depth of aircraft processing queue |
| `aprs.elevation_queue.depth` | Gauge | Current depth of elevation processing queue |

---

## ADS-B Beast Ingestion Metrics

**Service:** `soar ingest-adsb`

### Connection Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `beast.connection.established_total` | Counter | Total Beast protocol connection establishments |
| `beast.connection.failed_total` | Counter | Total Beast connection failures |
| `beast.operation.failed_total` | Counter | Total Beast operation failures |
| `beast.timeout_total` | Counter | Total Beast connection timeouts |
| `beast.connection.connected` | Gauge | Current connection status (1.0 = connected) |

### Data Ingestion Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `beast.bytes.received_total` | Counter | Total bytes received from Beast protocol |
| `beast.frames.published_total` | Counter | Total Beast frames published to NATS |
| `beast.frames.dropped_total` | Counter | Total Beast frames dropped (queue full) |
| `beast.message_rate` | Gauge | Current message receive rate (messages/sec) |
| `beast.ingest_failed_total` | Counter | Total ingestion failures |

### NATS Publishing Metrics (Ingest-ADS-B → NATS)

| Metric | Type | Description |
|--------|------|-------------|
| `beast.nats.published_total` | Counter | Total messages published to NATS |
| `beast.nats.publish_error_total` | Counter | Total NATS publish errors |
| `beast.nats.slow_publish_total` | Counter | Total slow publish operations (>100ms) |
| `beast.nats.publish_timeout_total` | Counter | Total publish timeouts |
| `beast.nats.connection_failed_total` | Counter | Total NATS connection failures |
| `beast.nats.stream_setup_failed_total` | Counter | Total NATS stream setup failures |
| `beast.nats.queue_depth` | Gauge | Current NATS publish queue depth |
| `beast.nats.in_flight` | Gauge | Current in-flight publish operations |
| `beast.nats.publish_duration_ms` | Histogram | NATS publish latency in milliseconds |

---

## ADS-B Beast Processing Metrics

**Service:** `soar run`

### NATS Consumer Metrics (NATS → soar run)

| Metric | Type | Description |
|--------|------|-------------|
| `beast.run.nats.consumed_total` | Counter | Total Beast messages consumed from NATS |
| `beast.run.nats.connection_failed_total` | Counter | Total NATS connection failures |
| `beast.run.nats.subscription_failed_total` | Counter | Total NATS subscription failures |
| `beast.run.nats.subscription_ended_total` | Counter | Total NATS subscription terminations |
| `beast.run.nats.lag_seconds` | Gauge | Current NATS consumer lag in seconds |
| `beast.run.nats.intake_queue_depth` | Gauge | Current depth of Beast message intake queue |

### Message Processing

| Metric | Type | Description |
|--------|------|-------------|
| `beast.run.process_beast_message.called_total` | Counter | Total Beast messages processed |
| `beast.run.invalid_message_total` | Counter | Total invalid Beast messages |
| `beast.run.decode.success_total` | Counter | Total successful ADS-B decodes |
| `beast.run.decode.failed_total` | Counter | Total ADS-B decode failures |
| `beast.run.icao_extraction_failed_total` | Counter | Total ICAO address extraction failures |
| `beast.run.aircraft_lookup_failed_total` | Counter | Total aircraft lookup failures |
| `beast.run.raw_message_stored_total` | Counter | Total raw Beast messages stored |
| `beast.run.raw_message_store_failed_total` | Counter | Total raw message store failures |
| `beast.run.adsb_to_fix_failed_total` | Counter | Total ADS-B to fix conversion failures |
| `beast.run.fixes_processed_total` | Counter | Total fixes successfully processed |
| `beast.run.fix_processing_failed_total` | Counter | Total fix processing failures |
| `beast.run.no_fix_created_total` | Counter | Messages that didn't create a fix (e.g., no position) |
| `beast.run.intake.processed_total` | Counter | Total messages processed from intake queue |
| `beast.run.message_processing_latency_ms` | Histogram | End-to-end Beast message processing latency |

---

## Flight Tracking Metrics

**Service:** `soar run`

### Active Tracking

| Metric | Type | Description |
|--------|------|-------------|
| `flight_tracker_active_aircraft` | Gauge | Number of aircraft currently being tracked |
| `flight_tracker_timeouts_detected_total` | Counter | Total flight timeout events detected |

### Flight Creation

| Metric | Type | Description |
|--------|------|-------------|
| `flight_tracker.flight_created.takeoff_total` | Counter | Flights created from detected takeoff |
| `flight_tracker.flight_created.airborne_total` | Counter | Flights created from airborne detection (no takeoff seen) |

### Flight Ending

| Metric | Type | Description |
|--------|------|-------------|
| `flight_tracker.flight_ended.landed_total` | Counter | Flights ended by detected landing |
| `flight_tracker.flight_ended.timed_out_total` | Counter | Flights ended by timeout (no recent position updates) |

### Flight Timeout Phase Tracking

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `flight_tracker.timeout.phase_total` | Counter | Flight timeouts by phase of flight | `phase`: climbing, cruising, descending, unknown |

### Flight Coalescing (Resume Detection)

| Metric | Type | Description |
|--------|------|-------------|
| `flight_tracker.coalesce.resumed_total` | Counter | Flights successfully resumed after gap |
| `flight_tracker.coalesce.callsign_mismatch_total` | Counter | Resume rejected due to callsign mismatch |
| `flight_tracker.coalesce.no_timeout_flight_total` | Counter | No timed-out flight found to resume |
| `flight_tracker.coalesce.rejected.callsign_total` | Counter | Resume rejected: callsign changed |
| `flight_tracker.coalesce.rejected.probable_landing_total` | Counter | Resume rejected: probable landing detected |
| `flight_tracker.coalesce.rejected.hard_limit_18h_total` | Counter | Resume rejected: gap exceeds 18 hour hard limit |
| `flight_tracker.coalesce.rejected.speed_distance_total` | Counter | Resume rejected: implied speed too high |
| `flight_tracker.coalesce.rejected.distance_km` | Histogram | Distance of rejected resume attempts (km) |
| `flight_tracker.coalesce.rejected.gap_hours` | Histogram | Time gap of rejected resume attempts (hours) |
| `flight_tracker.coalesce.resumed.distance_km` | Histogram | Distance of successful resume (km) |
| `flight_tracker.coalesce.speed_mph` | Histogram | Implied speed between last fix and resume (mph) |

### Location Tracking (Reverse Geocoding)

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `flight_tracker.location.created_total` | Counter | Flight locations created | `type`: start_takeoff, start_airborne, end_landing, end_timeout |

#### Pelias Geocoding

| Metric | Type | Description |
|--------|------|-------------|
| `flight_tracker.location.pelias.success_total` | Counter | Successful Pelias reverse geocoding queries |
| `flight_tracker.location.pelias.failure_total` | Counter | Failed Pelias queries |
| `flight_tracker.location.pelias.no_structured_data_total` | Counter | Pelias returned no structured location data |
| `flight_tracker.location.pelias.latency_ms` | Histogram | Pelias API response latency |

#### Photon Geocoding (Legacy)

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `flight_tracker.location.photon.success_total` | Counter | Successful Photon reverse geocoding queries |
| `flight_tracker.location.photon.failure_total` | Counter | Failed Photon queries |
| `flight_tracker.location.photon.no_structured_data_total` | Counter | Photon returned no structured location data |
| `flight_tracker.location.photon.latency_ms` | Histogram | Photon API response latency |
| `flight_tracker.location.photon.retry_total` | Counter | Photon retry attempts with expanded radius | `radius_km`: exact, 1, 5, 10 |

---

## Elevation Metrics

**Service:** `soar run`

### Cache Performance

| Metric | Type | Description |
|--------|------|-------------|
| `elevation_cache_hits_total` | Counter | Elevation cache hits (coordinate level) |
| `elevation_cache_misses_total` | Counter | Elevation cache misses (coordinate level) |
| `elevation_cache_entries` | Gauge | Current number of cached elevation coordinates |
| `elevation_tile_cache_hits_total` | Counter | Elevation tile cache hits |
| `elevation_tile_cache_misses_total` | Counter | Elevation tile cache misses |
| `elevation_tile_cache_entries` | Gauge | Current number of cached elevation tiles |

---

## NATS Publisher Metrics

**Service:** `soar run`

| Metric | Type | Description |
|--------|------|-------------|
| `nats_publisher_fixes_published_total` | Counter | Total fixes published to NATS for live streaming |
| `nats_publisher_queue_depth` | Gauge | Current depth of NATS publish queue |
| `nats_publisher_errors_total` | Counter | Total NATS publish errors |

---

## Receiver Metrics

**Service:** `soar run`

| Metric | Type | Description |
|--------|------|-------------|
| `receiver_status_updates_total` | Counter | Total receiver status updates processed |
| `generic_processor.receiver_cache.hit_total` | Counter | Receiver cache hits |
| `generic_processor.receiver_cache.miss_total` | Counter | Receiver cache misses |

---

## Analytics Metrics

**Service:** `soar web`

### API Endpoint Requests

| Metric | Type | Description |
|--------|------|-------------|
| `analytics.api.daily_flights.requests_total` | Counter | Requests to /data/analytics/flights/daily |
| `analytics.api.hourly_flights.requests_total` | Counter | Requests to /data/analytics/flights/hourly |
| `analytics.api.duration_distribution.requests_total` | Counter | Requests to /data/analytics/flights/duration-distribution |
| `analytics.api.aircraft_outliers.requests_total` | Counter | Requests to /data/analytics/devices/outliers |
| `analytics.api.top_aircraft.requests_total` | Counter | Requests to /data/analytics/devices/top |
| `analytics.api.club_analytics.requests_total` | Counter | Requests to /data/analytics/clubs/daily |
| `analytics.api.airport_activity.requests_total` | Counter | Requests to /data/analytics/airports/activity |
| `analytics.api.summary.requests_total` | Counter | Requests to /data/analytics/summary |
| `analytics.api.errors_total` | Counter | Total analytics API errors |

### Cache Performance

| Metric | Type | Description |
|--------|------|-------------|
| `analytics.cache.hit_total` | Counter | Analytics cache hits (60s TTL) |
| `analytics.cache.miss_total` | Counter | Analytics cache misses |
| `analytics.cache.size` | Gauge | Current number of cached analytics queries |

### Query Performance

| Metric | Type | Description |
|--------|------|-------------|
| `analytics.query.daily_flights_ms` | Histogram | Daily flights query latency |
| `analytics.query.hourly_flights_ms` | Histogram | Hourly flights query latency |
| `analytics.query.duration_distribution_ms` | Histogram | Duration distribution query latency |
| `analytics.query.aircraft_outliers_ms` | Histogram | Aircraft outliers query latency |
| `analytics.query.top_aircraft_ms` | Histogram | Top aircraft query latency |
| `analytics.query.club_analytics_ms` | Histogram | Club analytics query latency |
| `analytics.query.airport_activity_ms` | Histogram | Airport activity query latency |
| `analytics.query.summary_ms` | Histogram | Summary query latency |

### Analytics Data Gauges

| Metric | Type | Description |
|--------|------|-------------|
| `analytics.flights.today` | Gauge | Flight count for today (updated every 60s) |
| `analytics.flights.last_7d` | Gauge | Flight count for last 7 days |
| `analytics.flights.last_30d` | Gauge | Flight count for last 30 days |
| `analytics.aircraft.active_7d` | Gauge | Active aircraft count (last 7 days) |
| `analytics.aircraft.outliers` | Gauge | Aircraft with anomalous patterns (z-score > 3.0) |

---

## Coverage Map Metrics

**Service:** `soar web`

| Metric | Type | Description |
|--------|------|-------------|
| `coverage.api.hexes.requests_total` | Counter | Requests to coverage hexes API |
| `coverage.api.hexes.success_total` | Counter | Successful coverage hexes responses |
| `coverage.api.errors_total` | Counter | Coverage API errors |
| `coverage.api.hexes.count` | Histogram | Number of hexes returned per request |
| `coverage.query.hexes_ms` | Histogram | Coverage query latency |
| `coverage.cache.hit_total` | Counter | Coverage cache hits |
| `coverage.cache.miss_total` | Counter | Coverage cache misses |

---

## Airspace Metrics

**Service:** `soar pull-airspaces`, `soar web`

### Sync Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `airspace_sync.total_fetched_total` | Counter | Total airspaces fetched from OpenAIP |
| `airspace_sync.total_inserted_total` | Counter | Total airspaces inserted/updated in database |
| `airspace_sync.last_run_timestamp` | Gauge | Unix timestamp of last sync run |
| `airspace_sync.success` | Gauge | Last sync success status (1.0 = success, 0.0 = failure) |

### API Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `api.airspaces.requests_total` | Counter | Requests to airspace API |
| `api.airspaces.errors_total` | Counter | Airspace API errors |
| `api.airspaces.duration_ms` | Histogram | Airspace API query latency |
| `api.airspaces.results_count` | Gauge | Number of airspaces returned per request |

---

## Web Server Metrics

**Service:** `soar web`

### HTTP Request Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `http_request_duration_seconds` | Histogram | HTTP request latency (all endpoints) |
| `http_requests_total` | Counter | Total HTTP requests |

### WebSocket Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `websocket_connections` | Gauge | Current active WebSocket connections |
| `websocket_active_subscriptions` | Gauge | Current active subscriptions (all types) |
| `websocket_queue_depth` | Gauge | WebSocket message queue depth |
| `websocket_messages_sent_total` | Counter | Total messages sent to WebSocket clients |
| `websocket_send_errors_total` | Counter | Total WebSocket send errors |
| `websocket_serialization_errors_total` | Counter | Total message serialization errors |

#### Aircraft Subscriptions

| Metric | Type | Description |
|--------|------|-------------|
| `websocket_aircraft_subscribes_total` | Counter | Aircraft subscription requests |
| `websocket_aircraft_unsubscribes_total` | Counter | Aircraft unsubscription requests |

#### Area Subscriptions

| Metric | Type | Description |
|--------|------|-------------|
| `websocket_area_subscribes_total` | Counter | Single area subscription requests |
| `websocket_area_unsubscribes_total` | Counter | Single area unsubscription requests |
| `websocket_area_bulk_subscribes_total` | Counter | Bulk area subscription requests |
| `websocket_area_bulk_unsubscribes_total` | Counter | Bulk area unsubscription requests |
| `websocket_area_bulk_squares_per_subscription` | Histogram | Number of grid squares per bulk subscription |
| `websocket_area_bulk_validation_errors_total` | Counter | Bulk subscription validation errors |

---

## Watchlist Metrics

**Service:** `soar web`

| Metric | Type | Description |
|--------|------|-------------|
| `watchlist.notifications.sent_total` | Counter | Total watchlist notifications sent |
| `watchlist.notifications.errors_total` | Counter | Total notification send errors |
| `watchlist.matches.detected_total` | Counter | Total watchlist matches detected |

---

## Data Loading Metrics

**Service:** Various data import commands

| Metric | Type | Description |
|--------|------|-------------|
| `data_load.aircraft_registry.processed_total` | Counter | Aircraft registry records processed |
| `data_load.aircraft_registry.inserted_total` | Counter | Aircraft registry records inserted |
| `data_load.ddb.processed_total` | Counter | DDB records processed |
| `data_load.ddb.inserted_total` | Counter | DDB records inserted |

---

## Metric Best Practices

### Querying Tips

1. **Counter Rates:** Use `rate()` or `irate()` for counters
   ```promql
   rate(aprs_nats_published_total[5m])
   ```

2. **Histogram Percentiles:** Use `histogram_quantile()`
   ```promql
   histogram_quantile(0.95, rate(aprs_aircraft_total_processing_ms_bucket[5m]))
   ```

3. **Gauge Aggregations:** Use `avg()`, `min()`, `max()`
   ```promql
   avg(flight_tracker_active_aircraft)
   ```

### Dashboard Design

- **Use appropriate time ranges:** 5m for rates, 1m for instant values
- **Label filtering:** Use labels to filter metrics (e.g., `phase="climbing"`)
- **Group related metrics:** Organize panels by functional area
- **Set appropriate thresholds:** Use Grafana alerting for critical metrics

### Monitoring Recommendations

**Critical Metrics to Alert On:**
- `process.is_up` - Service availability
- `aprs.connection.connected` - APRS-IS connection health
- `beast.connection.connected` - Beast connection health
- `nats_publisher_errors_total` - NATS publishing failures
- `*_queue.full_total` - Queue overflow events
- `analytics.api.errors_total` - API error rate

---

## Metric Definitions in Code

All metrics are initialized to zero on startup in `src/metrics.rs`:
- `initialize_aprs_ingest_metrics()` - APRS ingestion metrics
- `initialize_beast_ingest_metrics()` - Beast ingestion metrics
- `initialize_beast_consumer_metrics()` - Beast consumer metrics
- `initialize_run_metrics()` - Flight tracking metrics
- `initialize_analytics_metrics()` - Analytics metrics
- `initialize_airspace_metrics()` - Airspace metrics
- `initialize_coverage_metrics()` - Coverage map metrics

This ensures all metrics are present in Prometheus even if no events have occurred.

---

## Prometheus Query Examples

### Flight Tracking Performance

```promql
# 95th percentile processing time for APRS messages
histogram_quantile(0.95, rate(aprs_aircraft_total_processing_ms_bucket[5m]))

# Active aircraft over time
flight_tracker_active_aircraft

# Flight creation rate (takeoffs per minute)
rate(flight_tracker_flight_created_takeoff_total[5m]) * 60
```

### Data Ingestion Health

```promql
# APRS message ingestion rate
rate(aprs_nats_published_total[5m])

# Beast frame ingestion rate
rate(beast_frames_published_total[5m])

# Combined ingestion rate
rate(aprs_nats_published_total[5m]) + rate(beast_frames_published_total[5m])
```

### Cache Performance

```promql
# Elevation cache hit rate
rate(elevation_cache_hits_total[5m]) /
(rate(elevation_cache_hits_total[5m]) + rate(elevation_cache_misses_total[5m]))

# Analytics cache hit rate
rate(analytics_cache_hit_total[5m]) /
(rate(analytics_cache_hit_total[5m]) + rate(analytics_cache_miss_total[5m]))
```

### Error Rates

```promql
# NATS publishing error rate
rate(aprs_nats_publish_error_total[5m])

# Total queue overflow events
sum(rate(aprs_aircraft_queue_full_total[5m]))

# WebSocket error rate
rate(websocket_send_errors_total[5m])
```

---

**For more information:**
- Grafana dashboards: `infrastructure/grafana-dashboard-*.json`
- Metrics initialization: `src/metrics.rs`
- Prometheus documentation: https://prometheus.io/docs/
