# SOAR Performance Fix - Flight Tracker Bottleneck

## Problem Identified

During high traffic periods, the PacketRouter internal queue was filling up (1000/1000 messages) causing severe backpressure. Investigation revealed:

### Root Cause
- **32-second flight INSERT delays** during `create_flight()`
- **25-second state transition delays** during `complete_flight()`

### Bottlenecks
1. **Runway Detection** (src/flight_tracker/runway.rs:88)
   - Queries `fixes` table for 40-second time window
   - During high traffic: lock contention on TimescaleDB hypertable
   - Blocks fix processing pipeline for 25+ seconds

2. **Pelias Geocoding** (src/flight_tracker/location.rs:183)
   - HTTP API call to reverse geocode takeoff/landing locations
   - External dependency blocks processing
   - Adds several seconds per flight operation

## Solution Implemented

### 1. Database Pool Increase ✅
- **Before**: 20 connections (too conservative after Dec 2025 incident)
- **After**: 30 connections
- **File**: `src/main.rs:487`

### 2. PgBouncer Installation Script ✅
- **File**: `scripts/setup-pgbouncer`
- **Features**:
  - Transaction-level pooling mode
  - 50 default pool size, 10 min pool, 10 reserve
  - Separate systemd service per environment
  - Auto-configuration from `/etc/soar/env`
  - Production port: 6432, Staging port: 6433

### 3. Async Flight Operations ⚠️ IN PROGRESS

#### Created Fast-Path Functions:
- `create_flight_fast()` - Returns immediately, enriches in background
- `spawn_flight_enrichment_on_creation()` - Background task for runway/location

#### New Repository Methods:
- `update_flight_takeoff_enrichment()` - Updates flight after background enrichment
- `update_flight_landing_enrichment()` - Updates flight after landing enrichment

#### Still TODO:
1. Add `complete_flight_fast()` function (similar pattern to `create_flight_fast`)
2. Add `spawn_flight_enrichment_on_completion()` background task
3. Update call sites in `state_transitions.rs` to use fast versions:
   - Line 179: Replace `create_flight()` → `create_flight_fast()`
   - Line 170: Replace `complete_flight()` → `complete_flight_fast()`
4. Add flight tracker stats logging (per user request)

## Expected Performance Improvement

### Before:
- Fix processing blocked for 25-32 seconds during flight operations
- PacketRouter queue fills up: 1000/1000 messages
- Envelope intake queue fills: 10000/10000 messages
- Only 3 Beast ops/sec during spikes

### After:
- Fix processing returns immediately (< 100ms)
- Flight enrichment happens in background (no blocking)
- Queues stay mostly empty
- Full throughput maintained during high traffic

## Deployment Steps

### 1. Install PgBouncer (on staging first)
```bash
sudo ./scripts/setup-pgbouncer staging
```

### 2. Update DATABASE_URL in /etc/soar/env-staging
```bash
# Before:
DATABASE_URL=postgresql://user:pass@localhost:5432/soar_staging

# After:
DATABASE_URL=postgresql://user:pass@localhost:6433/soar_staging
```

### 3. Restart SOAR services
```bash
sudo systemctl restart soar-run-staging
sudo systemctl restart soar-ingest-ogn-staging
sudo systemctl restart soar-ingest-adsb-staging
```

### 4. Monitor PgBouncer stats
```bash
# View connection pools
psql -h localhost -p 6433 -U soar_user pgbouncer -c 'SHOW POOLS'

# View statistics
psql -h localhost -p 6433 -U soar_user pgbouncer -c 'SHOW STATS'

# Monitor logs
tail -f /var/log/pgbouncer/pgbouncer.log
```

### 5. Deploy to Production (after staging validation)
```bash
sudo ./scripts/setup-pgbouncer production
# Update /etc/soar/env with port 6432
# Restart production services
```

## Metrics to Monitor

### Flight Tracker Performance:
- `flight_tracker.create_flight_fast.latency_ms` (should be <100ms)
- `flight_tracker.enrich_flight_on_creation.latency_ms` (can be 25-32s, but non-blocking)
- `aprs.router.internal_queue_depth` (should stay below 500)

### PgBouncer Health:
- Connection pool utilization (target: 60-80%)
- Client wait time (should be near 0)
- Query wait time (should be near 0)

### Overall System:
- Beast processing rate (should increase to 100+ ops/sec)
- Fix insertion rate (should match intake rate)
- No more "queue building up" warnings

## Rollback Plan

If issues occur:
1. Stop SOAR services
2. Revert DATABASE_URL to direct PostgreSQL connection
3. Stop pgbouncer: `systemctl stop pgbouncer-{staging|production}`
4. Restart SOAR services
5. Database pool increase (30 connections) can stay - it's safe

## Files Modified

1. `src/main.rs` - Database pool size 20 → 30
2. `src/flights_repo.rs` - Added enrichment update methods
3. `src/flight_tracker/flight_lifecycle.rs` - Added fast-path functions
4. `scripts/setup-pgbouncer` - New installation script
5. `infrastructure/grafana-dashboard-ingest-ogn.json` - Fixed environment filters

## Next Steps

1. Complete async flight operations implementation:
   - Add `complete_flight_fast()` and background enrichment
   - Update call sites in `state_transitions.rs`
2. Add flight tracker stats logging
3. Test on staging
4. Deploy to production
5. Monitor metrics and adjust pool sizes if needed
