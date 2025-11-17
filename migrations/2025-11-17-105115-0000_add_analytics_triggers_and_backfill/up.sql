-- Backfill analytics tables with historical data from existing flights
-- This migration assumes:
--   - Analytics tables exist (from 2025-11-16-015418-0000_create_analytics_tables)
--   - Trigger functions exist (from 2025-11-17-145657-0000_fix_analytics_triggers_null_handling)
--   - Triggers are attached (from 2025-11-17-210459-0000_add_analytics_triggers)
--
-- This migration is idempotent and safe to run multiple times.

-- ============================================================================
-- BACKFILL ANALYTICS DATA (idempotent - uses ON CONFLICT)
-- ============================================================================

-- Temporarily disable triggers for bulk backfill to avoid redundant work
ALTER TABLE flights DISABLE TRIGGER ALL;

-- Backfill flight_analytics_daily
INSERT INTO flight_analytics_daily (date, flight_count, total_duration_seconds, avg_duration_seconds, total_distance_meters, tow_flight_count, cross_country_count)
SELECT
    DATE(takeoff_time) as date,
    COUNT(*) as flight_count,
    SUM(EXTRACT(EPOCH FROM (COALESCE(landing_time, takeoff_time) - takeoff_time))::BIGINT) as total_duration_seconds,
    AVG(EXTRACT(EPOCH FROM (COALESCE(landing_time, takeoff_time) - takeoff_time))::INT) as avg_duration_seconds,
    SUM(COALESCE(total_distance_meters, 0)) as total_distance_meters,
    SUM(CASE WHEN towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END) as tow_flight_count,
    SUM(CASE WHEN departure_airport_id IS DISTINCT FROM arrival_airport_id THEN 1 ELSE 0 END) as cross_country_count
FROM flights
WHERE takeoff_time IS NOT NULL
GROUP BY DATE(takeoff_time)
ON CONFLICT (date) DO UPDATE SET
    flight_count = EXCLUDED.flight_count,
    total_duration_seconds = EXCLUDED.total_duration_seconds,
    avg_duration_seconds = EXCLUDED.avg_duration_seconds,
    total_distance_meters = EXCLUDED.total_distance_meters,
    tow_flight_count = EXCLUDED.tow_flight_count,
    cross_country_count = EXCLUDED.cross_country_count,
    updated_at = NOW();

-- Backfill flight_duration_buckets
WITH duration_data AS (
    SELECT
        CASE
            WHEN duration_minutes < 5 THEN '0-5min'
            WHEN duration_minutes < 10 THEN '5-10min'
            WHEN duration_minutes < 15 THEN '10-15min'
            WHEN duration_minutes < 30 THEN '15-30min'
            WHEN duration_minutes < 60 THEN '30-60min'
            WHEN duration_minutes < 90 THEN '60-90min'
            WHEN duration_minutes < 120 THEN '90-120min'
            WHEN duration_minutes < 150 THEN '120-150min'
            WHEN duration_minutes < 180 THEN '150-180min'
            WHEN duration_minutes < 210 THEN '180-210min'
            WHEN duration_minutes < 240 THEN '210-240min'
            WHEN duration_minutes < 270 THEN '240-270min'
            WHEN duration_minutes < 300 THEN '270-300min'
            WHEN duration_minutes < 330 THEN '300-330min'
            WHEN duration_minutes < 360 THEN '330-360min'
            ELSE '360+min'
        END as bucket_name,
        COUNT(*) as count
    FROM (
        SELECT EXTRACT(EPOCH FROM (COALESCE(landing_time, takeoff_time) - takeoff_time))::INT / 60 as duration_minutes
        FROM flights
        WHERE takeoff_time IS NOT NULL
        AND EXTRACT(EPOCH FROM (COALESCE(landing_time, takeoff_time) - takeoff_time)) > 0
    ) durations
    GROUP BY bucket_name
)
UPDATE flight_duration_buckets fdb
SET flight_count = dd.count,
    updated_at = NOW()
FROM duration_data dd
WHERE fdb.bucket_name = dd.bucket_name;

-- Backfill flight_analytics_hourly (last 7 days only)
INSERT INTO flight_analytics_hourly (hour, flight_count, active_devices, active_clubs)
SELECT
    DATE_TRUNC('hour', takeoff_time) as hour,
    COUNT(*) as flight_count,
    COUNT(DISTINCT device_id) as active_devices,
    COUNT(DISTINCT club_id) FILTER (WHERE club_id IS NOT NULL) as active_clubs
FROM flights
WHERE takeoff_time >= NOW() - INTERVAL '7 days'
    AND takeoff_time IS NOT NULL
GROUP BY DATE_TRUNC('hour', takeoff_time)
ON CONFLICT (hour) DO UPDATE SET
    flight_count = EXCLUDED.flight_count,
    active_devices = EXCLUDED.active_devices,
    active_clubs = EXCLUDED.active_clubs,
    updated_at = NOW();

-- Backfill device_analytics
INSERT INTO device_analytics (device_id, registration, aircraft_model, flight_count_total,
                              flight_count_30d, flight_count_7d, last_flight_at,
                              avg_flight_duration_seconds, total_distance_meters)
SELECT
    f.device_id,
    d.registration,
    d.aircraft_model,
    COUNT(*) as flight_count_total,
    COUNT(*) FILTER (WHERE f.takeoff_time >= CURRENT_DATE - 30) as flight_count_30d,
    COUNT(*) FILTER (WHERE f.takeoff_time >= CURRENT_DATE - 7) as flight_count_7d,
    MAX(f.takeoff_time) as last_flight_at,
    AVG(EXTRACT(EPOCH FROM (COALESCE(f.landing_time, f.takeoff_time) - f.takeoff_time))::INT) as avg_flight_duration_seconds,
    SUM(COALESCE(f.total_distance_meters, 0)) as total_distance_meters
FROM flights f
JOIN devices d ON d.id = f.device_id
WHERE f.takeoff_time IS NOT NULL
GROUP BY f.device_id, d.registration, d.aircraft_model
ON CONFLICT (device_id) DO UPDATE SET
    registration = EXCLUDED.registration,
    aircraft_model = EXCLUDED.aircraft_model,
    flight_count_total = EXCLUDED.flight_count_total,
    flight_count_30d = EXCLUDED.flight_count_30d,
    flight_count_7d = EXCLUDED.flight_count_7d,
    last_flight_at = EXCLUDED.last_flight_at,
    avg_flight_duration_seconds = EXCLUDED.avg_flight_duration_seconds,
    total_distance_meters = EXCLUDED.total_distance_meters,
    updated_at = NOW();

-- Calculate z-scores for device analytics
WITH stats AS (
    SELECT
        AVG(flight_count_30d) as mean,
        STDDEV(flight_count_30d) as stddev
    FROM device_analytics
    WHERE flight_count_30d > 0
)
UPDATE device_analytics
SET z_score_30d = CASE
    WHEN (SELECT stddev FROM stats) > 0
    THEN (flight_count_30d - (SELECT mean FROM stats)) / (SELECT stddev FROM stats)
    ELSE 0
END,
    updated_at = NOW()
WHERE flight_count_30d > 0;

-- Backfill airport_analytics_daily
INSERT INTO airport_analytics_daily (airport_id, date, airport_ident, airport_name, departure_count, arrival_count)
SELECT
    airport_id,
    date,
    MAX(airport_ident) as airport_ident,
    MAX(airport_name) as airport_name,
    SUM(departure_count) as departure_count,
    SUM(arrival_count) as arrival_count
FROM (
    SELECT
        departure_airport_id as airport_id,
        DATE(takeoff_time) as date,
        a.ident as airport_ident,
        a.name as airport_name,
        COUNT(*) as departure_count,
        0 as arrival_count
    FROM flights f
    JOIN airports a ON a.id = f.departure_airport_id
    WHERE f.departure_airport_id IS NOT NULL
        AND f.takeoff_time IS NOT NULL
    GROUP BY f.departure_airport_id, DATE(f.takeoff_time), a.ident, a.name

    UNION ALL

    SELECT
        arrival_airport_id as airport_id,
        DATE(takeoff_time) as date,
        a.ident as airport_ident,
        a.name as airport_name,
        0 as departure_count,
        COUNT(*) as arrival_count
    FROM flights f
    JOIN airports a ON a.id = f.arrival_airport_id
    WHERE f.arrival_airport_id IS NOT NULL
        AND f.takeoff_time IS NOT NULL
    GROUP BY f.arrival_airport_id, DATE(f.takeoff_time), a.ident, a.name
) combined
GROUP BY airport_id, date
ON CONFLICT (airport_id, date) DO UPDATE SET
    airport_ident = EXCLUDED.airport_ident,
    airport_name = EXCLUDED.airport_name,
    departure_count = EXCLUDED.departure_count,
    arrival_count = EXCLUDED.arrival_count,
    updated_at = NOW();

-- Backfill club_analytics_daily
INSERT INTO club_analytics_daily (club_id, date, club_name, flight_count, active_devices, total_airtime_seconds, tow_count)
SELECT
    f.club_id,
    DATE(f.takeoff_time) as date,
    c.name as club_name,
    COUNT(*) as flight_count,
    COUNT(DISTINCT f.device_id) as active_devices,
    SUM(EXTRACT(EPOCH FROM (COALESCE(f.landing_time, f.takeoff_time) - f.takeoff_time))::BIGINT) as total_airtime_seconds,
    SUM(CASE WHEN f.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END) as tow_count
FROM flights f
JOIN clubs c ON c.id = f.club_id
WHERE f.club_id IS NOT NULL
    AND f.takeoff_time IS NOT NULL
GROUP BY f.club_id, DATE(f.takeoff_time), c.name
ON CONFLICT (club_id, date) DO UPDATE SET
    club_name = EXCLUDED.club_name,
    flight_count = EXCLUDED.flight_count,
    active_devices = EXCLUDED.active_devices,
    total_airtime_seconds = EXCLUDED.total_airtime_seconds,
    tow_count = EXCLUDED.tow_count,
    updated_at = NOW();

-- Re-enable triggers
ALTER TABLE flights ENABLE TRIGGER ALL;
