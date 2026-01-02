# Grafana TODO 2: Code Changes for Missing Metrics

**Files:** `src/metrics.rs`, `src/flight_tracker/*.rs`, dashboard JSON files

## 1. Add Coalesce Speed Metric (NEW METRIC)

### Problem
When flights coalesce (resume after timeout), we track distance but not speed. This is critical for detecting anomalies.

### Metric Definition
**When:** Flight coalesces (resumes after timeout)
**Calculate:** `coalesce_distance_km / time_gap_hours = speed_mph`
**Example:** Flight disappears, reappears 1 hour later, 10km away = 6.2 mph

### Implementation Steps

#### A. Add metric initialization in `src/metrics.rs`
Find the flight coalesce metrics section (around line 360) and add:
```rust
// Flight coalescing metrics
metrics::counter!("flight_tracker.coalesce.resumed_total").absolute(0);
metrics::counter!("flight_tracker.coalesce.callsign_mismatch_total").absolute(0);
metrics::counter!("flight_tracker.coalesce.no_timeout_flight_total").absolute(0);
metrics::counter!("flight_tracker.coalesce.rejected.callsign_total").absolute(0);
metrics::counter!("flight_tracker.coalesce.rejected.probable_landing_total").absolute(0);
metrics::histogram!("flight_tracker.coalesce.rejected.distance_km").record(0.0);
metrics::histogram!("flight_tracker.coalesce.resumed.distance_km").record(0.0);

// ADD THIS NEW METRIC:
metrics::histogram!("flight_tracker.coalesce.speed_mph").record(0.0);
```

#### B. Record metric when coalescing occurs
Find where `flight_tracker.coalesce.resumed.distance_km` is recorded.

**File to check:** `src/flight_tracker/mod.rs` or `src/flight_tracker/coalesce.rs`
**Search for:** `flight_tracker.coalesce.resumed.distance_km`

Add the speed calculation:
```rust
// When recording coalesce distance:
metrics::histogram!("flight_tracker.coalesce.resumed.distance_km").record(distance_km);

// ADD THIS:
if let Some(time_gap_seconds) = time_gap_seconds {
    if time_gap_seconds > 0.0 {
        let time_gap_hours = time_gap_seconds / 3600.0;
        let speed_kmh = distance_km / time_gap_hours;
        let speed_mph = speed_kmh * 0.621371;
        metrics::histogram!("flight_tracker.coalesce.speed_mph").record(speed_mph);
    }
}
```

**Note:** You'll need to ensure `time_gap_seconds` is available in scope. Check the coalesce function signature.

#### C. Configure histogram buckets
In `src/metrics.rs`, add buckets for the new metric (around line 78-95):
```rust
builder
    .set_buckets_for_metric(
        metrics_exporter_prometheus::Matcher::Full("flight_tracker_coalesce_speed_mph".to_string()),
        &[0.0, 5.0, 10.0, 20.0, 30.0, 50.0, 100.0, 200.0, 500.0, 1000.0],
    )
    .expect("failed to set buckets for flight_tracker_coalesce_speed_mph")
```

### Testing
```bash
cargo build
# Run soar and check metrics endpoint
curl localhost:9092/metrics | grep coalesce_speed
```

## 2. Fix Missing Coalesce Distance P50

### Problem
Dashboard panel "Coalesce Distance (P50)" shows "No data"

### Investigation Steps
1. Check if metric exists: `curl localhost:9092/metrics | grep coalesce.*distance`
2. Verify histogram is being recorded (not just initialized)
3. Check dashboard query uses correct metric name

### Likely Fix
The metric exists (`flight_tracker.coalesce.resumed.distance_km`) but:
- May need environment filter: `{component="run",environment="$environment"}`
- May need longer time range to have data
- Check query uses histogram_quantile correctly:
  ```promql
  histogram_quantile(0.50,
    rate(flight_tracker_coalesce_resumed_distance_km_bucket{
      component="run",
      environment="$environment"
    }[5m])
  )
  ```

**File:** `infrastructure/grafana-dashboard-run-flights.json`
**Panel:** Search for "Coalesce Distance"

## 3. Fix Missing Aircraft Processing Latency

### Problem
"Aircraft Processing Latency Breakdown (P50)" and "(P95)" show "No data"

### Metrics to check
These should exist (from `src/metrics.rs`):
- `aprs.aircraft.lookup_ms`
- `aprs.aircraft.fix_creation_ms`
- `aprs.aircraft.process_fix_internal_ms`
- `aprs.aircraft.total_processing_ms`

### Investigation
1. Check metrics are being recorded: `curl localhost:9092/metrics | grep aircraft.*_ms`
2. Verify dashboard uses correct names and component filter
3. Ensure histogram buckets are configured

**File:** `infrastructure/grafana-dashboard-run-core.json`
**Search for:** "Aircraft Processing Latency"

### Likely Fix
Add component and environment filters to query:
```promql
histogram_quantile(0.50,
  rate(aprs_aircraft_total_processing_ms_bucket{
    component="run",
    environment="$environment"
  }[$__rate_interval])
)
```

## Testing All Metrics
```bash
# Build with changes
cargo build

# Start soar run (staging)
./target/debug/soar run

# Check metrics
curl localhost:9092/metrics | grep -E "(coalesce_speed|aircraft.*_ms|coalesce.*distance)"

# Verify in Grafana after a few minutes of data collection
```
