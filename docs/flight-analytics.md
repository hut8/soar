# Flight Analytics Triggers (Archived)

This document describes the analytics triggers that were attached to the `flights` table. These triggers were removed to reduce database write amplification and WAL pressure during high-throughput processing.

## Overview

Six triggers fired on every INSERT, UPDATE, and DELETE operation on the `flights` table:

| Trigger | Target Table | Purpose |
|---------|--------------|---------|
| `trigger_flight_analytics_daily` | `flight_analytics_daily` | Daily flight statistics |
| `trigger_flight_analytics_hourly` | `flight_analytics_hourly` | Hourly flight statistics |
| `trigger_flight_duration_buckets` | `flight_duration_buckets` | Flight duration distribution |
| `trigger_device_analytics` | `device_analytics` | Per-aircraft flight stats |
| `trigger_club_analytics_daily` | `club_analytics_daily` | Per-club daily stats |
| `trigger_airport_analytics_daily` | `airport_analytics_daily` | Per-airport daily stats |

## Why They Were Removed

During high-throughput processing (600+ fixes/second), these triggers caused:

1. **Write Amplification**: Each flight operation triggered up to 46 additional SQL statements
2. **WAL Bottleneck**: Database connections waiting on `WALWrite` (28+ connections blocked)
3. **Latency Spike**: Fix insert latency increased from 12ms to 1,100ms+ during load
4. **Cascade Backpressure**: Slow DB writes caused upstream queues to fill, jamming the entire pipeline

## Trigger Details

### trigger_flight_analytics_daily

**Target Table**: `flight_analytics_daily`

**Purpose**: Maintains daily aggregated flight statistics.

**Fields Updated**:
- `flight_count` - Number of flights per day
- `total_duration_seconds` - Sum of all flight durations
- `total_distance_meters` - Sum of all flight distances
- `tow_flight_count` - Number of towed flights
- `cross_country_count` - Flights where departure != arrival airport
- `avg_duration_seconds` - Average flight duration

**Behavior**:
- INSERT: Adds flight stats to the day's totals
- UPDATE: Removes old values, adds new values (handles date changes)
- DELETE: Subtracts flight stats from the day's totals
- Skips flights without `takeoff_time`
- Skips updates that don't change analytics-relevant fields

### trigger_flight_analytics_hourly

**Target Table**: `flight_analytics_hourly`

**Purpose**: Maintains hourly aggregated flight statistics for recent trend analysis.

**Fields Updated**:
- `flight_count` - Number of flights per hour
- `total_duration_seconds` - Sum of flight durations
- `avg_duration_seconds` - Average flight duration

**Behavior**:
- Similar to daily but aggregates by hour
- Uses `date_trunc('hour', takeoff_time)` for bucketing

### trigger_flight_duration_buckets

**Target Table**: `flight_duration_buckets`

**Purpose**: Categorizes flights into duration buckets for distribution analysis.

**Buckets**:
- 0-15 minutes
- 15-30 minutes
- 30-60 minutes
- 1-2 hours
- 2-4 hours
- 4+ hours

**Behavior**:
- INSERT: Increments the appropriate bucket counter
- UPDATE: Moves count from old bucket to new bucket if duration changed
- DELETE: Decrements the appropriate bucket counter

### trigger_device_analytics

**Target Table**: `device_analytics`

**Purpose**: Tracks flight statistics per aircraft device.

**Fields Updated**:
- `flight_count` - Total flights for this device
- `total_duration_seconds` - Total flight time
- `avg_duration_seconds` - Average flight duration
- `last_flight_date` - Most recent flight date

**Behavior**:
- Maintains per-device lifetime statistics
- Updates on any flight for the device's `aircraft_id`

### trigger_club_analytics_daily

**Target Table**: `club_analytics_daily`

**Purpose**: Tracks daily flight statistics per club.

**Fields Updated**:
- `flight_count` - Number of flights by club aircraft
- `total_duration_seconds` - Total flight time
- `unique_aircraft_count` - Distinct aircraft flown

**Behavior**:
- Only triggers for flights with a `club_id`
- Aggregates by club and date

### trigger_airport_analytics_daily

**Target Table**: `airport_analytics_daily`

**Purpose**: Tracks daily departure/arrival counts per airport.

**Fields Updated**:
- `departure_count` - Flights departing from this airport
- `arrival_count` - Flights arriving at this airport
- `airport_ident` - Airport identifier (cached)
- `airport_name` - Airport name (cached)

**Behavior**:
- Updates both departure and arrival airport records
- Only triggers when `departure_airport_id` or `arrival_airport_id` is set

## Alternative: Batch Analytics

Instead of real-time triggers, analytics can be computed via:

1. **Scheduled Jobs**: Run hourly/daily aggregation queries
2. **Materialized Views**: Refresh periodically with `REFRESH MATERIALIZED VIEW`
3. **Application-Level Batching**: Queue analytics updates and process in batches

Example batch query for daily flight analytics:
```sql
INSERT INTO flight_analytics_daily (date, flight_count, total_duration_seconds, ...)
SELECT
    DATE(takeoff_time) as date,
    COUNT(*) as flight_count,
    SUM(EXTRACT(EPOCH FROM (landing_time - takeoff_time))) as total_duration_seconds,
    ...
FROM flights
WHERE takeoff_time >= $1 AND takeoff_time < $2
GROUP BY DATE(takeoff_time)
ON CONFLICT (date) DO UPDATE SET
    flight_count = EXCLUDED.flight_count,
    total_duration_seconds = EXCLUDED.total_duration_seconds,
    ...;
```

## Migration History

- **Created**: Migration `2025-11-17-210459-0000_add_analytics_triggers`
- **Removed**: Migration `2026-01-08-XXXXXX-0000_remove_analytics_triggers` (this removal)
