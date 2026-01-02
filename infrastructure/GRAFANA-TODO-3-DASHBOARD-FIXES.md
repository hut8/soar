# Grafana TODO 3: Individual Dashboard Fixes

## 1. BEAST/ADSB Ingest Dashboard
**File:** `infrastructure/grafana-dashboard-ingest-adsb.json`

### Changes Needed
- Replace all instances of "BEAST" with "ADS-B" in panel titles and descriptions
- Verify all metrics exist in `src/metrics.rs` (check `initialize_beast_ingest_metrics()`)
- Metrics to verify:
  ```
  beast.connection.established_total
  beast.connection.failed_total
  beast.bytes.received_total
  beast.frames.published_total
  beast.nats.published_total
  beast.nats.publish_error_total
  beast.nats.publish_duration_ms
  beast.nats.queue_depth
  ```

### Implementation
```bash
cd infrastructure
sed -i 's/BEAST/ADS-B/g' grafana-dashboard-ingest-adsb.json
sed -i 's/Beast/ADS-B/g' grafana-dashboard-ingest-adsb.json
# Manual review: some "beast" metric names should stay lowercase
```

## 2. Coverage Map Dashboard
**File:** `infrastructure/grafana-dashboard-coverage.json`

### Already Fixed
- ✅ Removed duplicate `_total_total` suffixes

### Additional Needed
- Add environment selector (staging/production)
- Add postgres datasource template variable
- Update datasource UID from `soar-postgres` to `${postgres_datasource}`

### Template Variables
```json
{
  "templating": {
    "list": [
      {
        "name": "environment",
        "type": "custom",
        "query": "production,staging",
        "current": {"value": "staging"}
      },
      {
        "name": "postgres_datasource",
        "type": "datasource",
        "query": "postgres",
        "regex": "/soar-postgres-${environment}/",
        "hide": 2
      }
    ]
  }
}
```

## 3. Run - Core System Dashboard
**File:** `infrastructure/grafana-dashboard-run-core.json`

### A. Fix Aircraft Processing Latency Panels
**Panels:** "Aircraft Processing Latency Breakdown (P50)" and "(P95)"

**Problem:** Showing "No data"

**Fix:** Add proper component/environment filters (see TODO-2-CODE-CHANGES.md)

### B. Add Documentation Panel for Queue Closed Errors
**Panel:** "Queue Closed Errors" (shows "No data")

**Action:** Add text panel below explaining what queue closed errors mean:

```markdown
# Queue Closed Errors

Queue closed errors occur when internal message channels are shut down, typically during:

1. **Graceful shutdown** - Application is stopping
2. **Component failure** - Upstream component crashed or restarted
3. **Resource exhaustion** - System under extreme load

**Metrics:**
- `aprs.aircraft_queue.closed_total` - Aircraft processing queue closed
- `aprs.receiver_status_queue.closed_total` - Receiver status queue closed
- `aprs.receiver_position_queue.closed_total` - Receiver position queue closed
- `aprs.server_status_queue.closed_total` - Server status queue closed

**Normal behavior:** Should be 0 during normal operation
**Alert threshold:** > 0 indicates abnormal shutdown or component failure
```

**Implementation:** Add text panel with `"type": "text"`, `"mode": "markdown"`

## 4. Run - Elevation Processing Dashboard
**File:** `infrastructure/grafana-dashboard-run-elevation.json`

### Problem
"Receiver latest_packet_at Cache" panel does NOT belong in elevation dashboard

### Action
1. **Remove from elevation dashboard:**
   - Find panel with title containing "receiver" and "cache"
   - Delete entire panel object

2. **Move to Run - Core System dashboard:**
   - Add to `grafana-dashboard-run-core.json`
   - Place in appropriate section (likely near database metrics)

### Panel to Move
Search for panel with:
- Metrics: `generic_processor.receiver_cache.hit_total`, `generic_processor.receiver_cache.miss_total`
- Title: Something like "Receiver Cache" or "Receiver latest_packet_at Cache"

## 5. Run - Geocoding Dashboard
**File:** `infrastructure/grafana-dashboard-run-geocoding.json`

### Already Fixed
- ✅ Removed duplicate `_total_total` suffixes
- ✅ Added Pelias metrics initialization in code

### Verify
- Check that panels now show data (metrics start with `flight_tracker.location.pelias.*`)
- Ensure component and environment filters are present:
  ```promql
  rate(flight_tracker_location_pelias_success_total{
    component="run",
    environment="$environment"
  }[5m])
  ```

## 6. Run - Aircraft & Flight Tracking Dashboard
**File:** `infrastructure/grafana-dashboard-run-flights.json`

### A. Fix Coalesce Distance P50
See TODO-2-CODE-CHANGES.md for investigation steps

### B. Add Panel: Flight Timeout Phase Definitions
**Location:** Below "Flight Timeouts by Phase" panel

**Content:**
```markdown
# Flight Phase Definitions

Flight phases are determined by vertical speed and altitude changes:

## Climbing
- Vertical speed > 0.5 m/s
- Aircraft gaining altitude
- Typical: thermal circling, ridge lift, wave lift

## Cruising
- Vertical speed between -0.5 and +0.5 m/s
- Relatively stable altitude
- Typical: straight glides between thermals

## Descending
- Vertical speed < -0.5 m/s
- Aircraft losing altitude
- Typical: approach to landing, poor lift conditions

## Unknown
- Insufficient data to determine phase
- First few fixes of flight
- GPS quality issues

**Timeout Detection:**
When a flight times out (no position updates for 30+ minutes), the phase at timeout
helps identify flight conditions and potential issues.
```

**Implementation:** Text panel, markdown mode, place after panel ID for "Flight Timeouts by Phase"
