-- Migration script to move data from fixes_default into proper partitions
-- This fixes the issue where partman couldn't create new partitions due to data in DEFAULT
--
-- USAGE: psql -U soar -d soar -f scripts/migrate-default-partition.sql
--
-- This script:
-- 1. Creates missing partitions for Dec 18-20, 2025
-- 2. Detaches the DEFAULT partition (non-blocking)
-- 3. Moves data from detached DEFAULT into proper partitions
-- 4. Re-creates an empty DEFAULT partition

\timing on
\set ON_ERROR_STOP on

BEGIN;

-- Step 1: Check current state
SELECT 'Current state of fixes partitions:' as step;
SELECT tablename FROM pg_tables
WHERE tablename LIKE 'fixes_p202512%' OR tablename = 'fixes_default'
ORDER BY tablename DESC;

SELECT 'Data in fixes_default:' as step;
SELECT COUNT(*), MIN(received_at), MAX(received_at)
FROM fixes_default;

COMMIT;

-- Step 2: Detach the DEFAULT partition
-- NOTE: Cannot use CONCURRENTLY when a DEFAULT partition exists (PostgreSQL limitation)
-- This will briefly lock the parent table, but should be quick since we're just detaching
SELECT 'Detaching fixes_default partition...' as step;
ALTER TABLE fixes DETACH PARTITION fixes_default;

-- Verify detachment completed
SELECT 'Verifying detachment...' as step;
SELECT EXISTS (
    SELECT 1 FROM pg_inherits i
    JOIN pg_class child ON i.inhrelid = child.oid
    JOIN pg_class parent ON i.inhparent = parent.oid
    WHERE parent.relname = 'fixes' AND child.relname = 'fixes_default'
) as still_attached;

-- Step 3: Create missing partitions (now that DEFAULT is detached)
SELECT 'Creating missing partitions...' as step;

CREATE TABLE IF NOT EXISTS fixes_p20251218 PARTITION OF fixes
FOR VALUES FROM ('2025-12-18 01:00:00+01') TO ('2025-12-19 01:00:00+01');

CREATE TABLE IF NOT EXISTS fixes_p20251219 PARTITION OF fixes
FOR VALUES FROM ('2025-12-19 01:00:00+01') TO ('2025-12-20 01:00:00+01');

CREATE TABLE IF NOT EXISTS fixes_p20251220 PARTITION OF fixes
FOR VALUES FROM ('2025-12-20 01:00:00+01') TO ('2025-12-21 01:00:00+01');

-- Step 4: Move data from detached fixes_default into proper partitions
-- We'll do this in batches to avoid long locks
-- Note: Exclude generated columns (location, location_geom) - they'll be auto-generated

SELECT 'Migrating data for 2025-12-18...' as step;
WITH moved_rows AS (
    DELETE FROM fixes_default
    WHERE received_at >= '2025-12-18 01:00:00+01'
      AND received_at < '2025-12-19 01:00:00+01'
    RETURNING id, source, aprs_type, via, timestamp, latitude, longitude,
              altitude_msl_feet, flight_number, squawk, ground_speed_knots, track_degrees,
              climb_fpm, turn_rate_rot, flight_id, aircraft_id, received_at, is_active,
              altitude_agl_feet, receiver_id, raw_message_id, altitude_agl_valid,
              time_gap_seconds, source_metadata
)
INSERT INTO fixes (id, source, aprs_type, via, timestamp, latitude, longitude,
                   altitude_msl_feet, flight_number, squawk, ground_speed_knots, track_degrees,
                   climb_fpm, turn_rate_rot, flight_id, aircraft_id, received_at, is_active,
                   altitude_agl_feet, receiver_id, raw_message_id, altitude_agl_valid,
                   time_gap_seconds, source_metadata)
SELECT * FROM moved_rows;

SELECT 'Migrating data for 2025-12-19...' as step;
WITH moved_rows AS (
    DELETE FROM fixes_default
    WHERE received_at >= '2025-12-19 01:00:00+01'
      AND received_at < '2025-12-20 01:00:00+01'
    RETURNING id, source, aprs_type, via, timestamp, latitude, longitude,
              altitude_msl_feet, flight_number, squawk, ground_speed_knots, track_degrees,
              climb_fpm, turn_rate_rot, flight_id, aircraft_id, received_at, is_active,
              altitude_agl_feet, receiver_id, raw_message_id, altitude_agl_valid,
              time_gap_seconds, source_metadata
)
INSERT INTO fixes (id, source, aprs_type, via, timestamp, latitude, longitude,
                   altitude_msl_feet, flight_number, squawk, ground_speed_knots, track_degrees,
                   climb_fpm, turn_rate_rot, flight_id, aircraft_id, received_at, is_active,
                   altitude_agl_feet, receiver_id, raw_message_id, altitude_agl_valid,
                   time_gap_seconds, source_metadata)
SELECT * FROM moved_rows;

SELECT 'Migrating data for 2025-12-20...' as step;
WITH moved_rows AS (
    DELETE FROM fixes_default
    WHERE received_at >= '2025-12-20 01:00:00+01'
      AND received_at < '2025-12-21 01:00:00+01'
    RETURNING id, source, aprs_type, via, timestamp, latitude, longitude,
              altitude_msl_feet, flight_number, squawk, ground_speed_knots, track_degrees,
              climb_fpm, turn_rate_rot, flight_id, aircraft_id, received_at, is_active,
              altitude_agl_feet, receiver_id, raw_message_id, altitude_agl_valid,
              time_gap_seconds, source_metadata
)
INSERT INTO fixes (id, source, aprs_type, via, timestamp, latitude, longitude,
                   altitude_msl_feet, flight_number, squawk, ground_speed_knots, track_degrees,
                   climb_fpm, turn_rate_rot, flight_id, aircraft_id, received_at, is_active,
                   altitude_agl_feet, receiver_id, raw_message_id, altitude_agl_valid,
                   time_gap_seconds, source_metadata)
SELECT * FROM moved_rows;

-- Step 5: Verify migration
SELECT 'Verification after migration:' as step;
SELECT
    'fixes_default' as partition,
    COUNT(*) as row_count,
    MIN(received_at) as min_date,
    MAX(received_at) as max_date
FROM fixes_default
UNION ALL
SELECT
    'fixes_p20251218' as partition,
    COUNT(*) as row_count,
    MIN(received_at) as min_date,
    MAX(received_at) as max_date
FROM fixes_p20251218
UNION ALL
SELECT
    'fixes_p20251219' as partition,
    COUNT(*) as row_count,
    MIN(received_at) as min_date,
    MAX(received_at) as max_date
FROM fixes_p20251219
UNION ALL
SELECT
    'fixes_p20251220' as partition,
    COUNT(*) as row_count,
    MIN(received_at) as min_date,
    MAX(received_at) as max_date
FROM fixes_p20251220
ORDER BY partition;

-- Step 6: Drop the detached fixes_default (it should be empty now)
SELECT 'Dropping detached fixes_default...' as step;
DROP TABLE IF EXISTS fixes_default;

-- Step 7: Optionally re-create DEFAULT partition for future overflow
-- (partman config has ignore_default_data = true, so this might not be needed)
-- Uncomment if you want to keep a DEFAULT partition:
-- CREATE TABLE fixes_default PARTITION OF fixes DEFAULT;

SELECT 'Migration completed successfully!' as step;
SELECT 'Run partman maintenance to ensure future partitions are created:' as next_step;
SELECT 'CALL partman.run_maintenance_proc();' as command;
