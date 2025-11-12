-- ================================================================================
-- CONVERT FIXES TABLE TO PARTITIONED TABLE WITH PG_PARTMAN
-- ================================================================================
-- This migration converts the fixes table to use native PostgreSQL partitioning
-- with daily partitions managed by pg_partman extension.
--
-- IMPORTANT: This migration is SLOW on production (225M rows)
-- Estimated time: 30-60 minutes
-- Run during maintenance window!
--
-- Strategy:
-- 1. Install pg_partman extension
-- 2. Rename existing fixes table to fixes_old
-- 3. Create new partitioned fixes table
-- 4. Use pg_partman to create partitions and migrate data
-- 5. Recreate indexes on partitions
-- 6. Configure automatic partition management
-- ================================================================================

-- Step 1: Install pg_partman extension
CREATE EXTENSION IF NOT EXISTS pg_partman SCHEMA partman;

-- Step 2: Rename existing table
ALTER TABLE fixes RENAME TO fixes_old;

-- Step 3: Create new partitioned table with same structure
CREATE TABLE fixes (
    id uuid NOT NULL DEFAULT gen_random_uuid(),
    source character varying(9) NOT NULL,
    aprs_type character varying(9) NOT NULL,
    via text[] NOT NULL,
    timestamp timestamp with time zone NOT NULL,
    latitude double precision NOT NULL,
    longitude double precision NOT NULL,
    location geography(Point,4326) GENERATED ALWAYS AS (st_point(longitude, latitude)::geography) STORED,
    altitude_msl_feet integer,
    flight_number character varying(20),
    squawk character varying(4),
    ground_speed_knots real,
    track_degrees real CHECK (track_degrees >= 0 AND track_degrees < 360),
    climb_fpm integer,
    turn_rate_rot real,
    snr_db real,
    bit_errors_corrected integer,
    freq_offset_khz real,
    flight_id uuid,
    device_id uuid NOT NULL,
    received_at timestamp with time zone NOT NULL,
    is_active boolean NOT NULL DEFAULT true,
    altitude_agl_feet integer,
    receiver_id uuid NOT NULL,
    gnss_horizontal_resolution smallint,
    gnss_vertical_resolution smallint,
    aprs_message_id uuid NOT NULL,
    altitude_agl_valid boolean NOT NULL DEFAULT false,
    location_geom geometry(Point,4326) GENERATED ALWAYS AS (st_setsrid(st_makepoint(longitude, latitude), 4326)) STORED,
    time_gap_seconds integer
) PARTITION BY RANGE (received_at);

-- Step 4: Create partitions using pg_partman
-- Configure partman to create daily partitions
SELECT partman.create_parent(
    p_parent_table := 'public.fixes',
    p_control := 'received_at',
    p_type := 'native',
    p_interval := 'daily',
    p_premake := 3,  -- Create 3 days ahead
    p_start_partition := (SELECT date_trunc('day', MIN(received_at))::text FROM fixes_old)
);

-- Step 5: Migrate data from old table to new partitioned table
-- This uses pg_partman's partition_data_time which handles data migration
-- This is the SLOW part - it will take 30-60 minutes on production
SELECT partman.partition_data_time(
    p_parent_table := 'public.fixes',
    p_batch_count := 10,  -- Process in batches
    p_batch_interval := interval '100000 rows',  -- Batch size
    p_lock_wait := 2  -- Wait 2 seconds for locks
);

-- Step 6: Add primary key constraint
-- Note: In partitioned tables, PK must include partition key
ALTER TABLE fixes ADD PRIMARY KEY (id, received_at);

-- Step 7: Recreate indexes on parent table (will cascade to partitions)
-- Note: Some indexes will be created automatically by pg_partman
CREATE INDEX idx_fixes_device_received_at ON fixes (device_id, received_at DESC);
CREATE INDEX idx_fixes_location_geom ON fixes USING GIST (location_geom);
CREATE INDEX idx_fixes_location ON fixes USING GIST (location);
CREATE INDEX idx_fixes_source ON fixes (source);
CREATE INDEX idx_fixes_timestamp ON fixes (timestamp DESC);
CREATE INDEX idx_fixes_altitude_agl_feet ON fixes (altitude_agl_feet);
CREATE INDEX idx_fixes_altitude_agl_valid ON fixes (altitude_agl_valid) WHERE altitude_agl_valid = false;
CREATE INDEX idx_fixes_aprs_message_id ON fixes (aprs_message_id);
CREATE INDEX idx_fixes_backfill_optimized ON fixes (timestamp) WHERE altitude_agl_valid = false AND altitude_msl_feet IS NOT NULL AND is_active = true;
CREATE INDEX idx_fixes_device_id_timestamp ON fixes (device_id, timestamp);
CREATE INDEX idx_fixes_flight_id_timestamp ON fixes (flight_id, timestamp);
CREATE INDEX idx_fixes_ground_speed_knots ON fixes (ground_speed_knots);
CREATE INDEX idx_fixes_is_active ON fixes (is_active);
CREATE INDEX idx_fixes_receiver_id ON fixes (receiver_id);
CREATE INDEX idx_fixes_time_gap_seconds ON fixes (time_gap_seconds) WHERE time_gap_seconds IS NOT NULL;

-- Step 8: Recreate foreign key constraints
ALTER TABLE fixes ADD CONSTRAINT fixes_aprs_message_id_fkey FOREIGN KEY (aprs_message_id) REFERENCES aprs_messages(id) ON DELETE SET NULL;
ALTER TABLE fixes ADD CONSTRAINT fixes_device_id_fkey FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE SET NULL;
ALTER TABLE fixes ADD CONSTRAINT fixes_flight_id_fkey FOREIGN KEY (flight_id) REFERENCES flights(id) ON DELETE SET NULL;
ALTER TABLE fixes ADD CONSTRAINT fixes_receiver_id_fkey FOREIGN KEY (receiver_id) REFERENCES receivers(id) ON DELETE SET NULL;

-- Step 9: Configure pg_partman for automatic partition management
-- This sets retention to keep only 8 days of data
UPDATE partman.part_config SET
    retention = '8 days',
    retention_keep_table = false,  -- Drop old partitions completely
    retention_keep_index = false,
    infinite_time_partitions = true
WHERE parent_table = 'public.fixes';

-- Step 10: Verify migration was successful
DO $$
DECLARE
    old_count bigint;
    new_count bigint;
BEGIN
    SELECT COUNT(*) INTO old_count FROM fixes_old;
    SELECT COUNT(*) INTO new_count FROM fixes;

    IF old_count != new_count THEN
        RAISE EXCEPTION 'Migration failed: row count mismatch. Old: %, New: %', old_count, new_count;
    END IF;

    RAISE NOTICE 'Migration successful: % rows migrated', new_count;
END $$;

-- Step 11: Drop old table (uncomment after verifying)
-- DROP TABLE fixes_old;

-- Add helpful comment
COMMENT ON TABLE fixes IS 'Partitioned by received_at (daily). Managed by pg_partman. Retention: 8 days.';
