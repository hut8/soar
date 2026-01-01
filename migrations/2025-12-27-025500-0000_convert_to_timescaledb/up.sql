-- ================================================================================
-- CONVERT FROM PG_PARTMAN TO TIMESCALEDB HYPERTABLES
-- ================================================================================
-- This migration converts fixes and raw_messages tables from pg_partman-managed
-- native PostgreSQL partitioning to TimescaleDB hypertables.
--
-- Benefits:
-- - Automatic partition (chunk) management
-- - Built-in compression (90%+ space savings on old data)
-- - Continuous aggregates for analytics
-- - Better query optimization for time-series data
-- - Simplified operations (no cron jobs needed)
--
-- Strategy:
-- 1. Enable TimescaleDB extension
-- 2. Rename existing partitioned tables
-- 3. Create new regular tables with same schema
-- 4. Convert to hypertables
-- 5. Copy all data
-- 6. Recreate indexes and constraints
-- 7. Set up retention and compression policies
-- 8. Remove pg_partman configuration
-- ================================================================================

-- Step 1: Enable TimescaleDB extension
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- ================================================================================
-- MIGRATE RAW_MESSAGES TABLE
-- ================================================================================

-- Step 2: Rename existing partitioned raw_messages table
ALTER TABLE raw_messages RENAME TO raw_messages_partman;

-- Step 3: Create new non-partitioned raw_messages table
CREATE TABLE raw_messages (
    id uuid NOT NULL DEFAULT gen_random_uuid(),
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL,
    raw_message bytea NOT NULL,
    source message_source NOT NULL DEFAULT 'ogn'::message_source
);

-- Add table comment
COMMENT ON COLUMN raw_messages.raw_message IS 'Raw message data as bytes. For APRS: UTF-8 encoded ASCII text. For Beast: Raw binary ADS-B frames.';

-- Step 4: Convert raw_messages to hypertable
-- chunk_time_interval = 1 day matches our previous pg_partman setup
SELECT create_hypertable(
    'raw_messages',
    'received_at',
    chunk_time_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

-- Step 5: Copy data from old partitioned table
-- This will automatically distribute data into appropriate chunks
INSERT INTO raw_messages
SELECT * FROM raw_messages_partman;

-- Step 6: Add primary key (must include partition key for hypertables)
ALTER TABLE raw_messages ADD PRIMARY KEY (id, received_at);

-- Step 7: Recreate indexes on raw_messages
-- Note: TimescaleDB automatically creates index on partition key (received_at)
CREATE INDEX idx_raw_messages_receiver_id ON raw_messages (receiver_id);

-- Step 8: Recreate foreign key constraints on raw_messages
ALTER TABLE raw_messages
    ADD CONSTRAINT raw_messages_receiver_id_fkey
    FOREIGN KEY (receiver_id) REFERENCES receivers(id) ON DELETE CASCADE;

-- Step 9: Verify raw_messages migration
DO $$
DECLARE
    old_count bigint;
    new_count bigint;
BEGIN
    SELECT COUNT(*) INTO old_count FROM raw_messages_partman;
    SELECT COUNT(*) INTO new_count FROM raw_messages;

    IF old_count != new_count THEN
        RAISE EXCEPTION 'raw_messages migration failed: count mismatch. Old: %, New: %', old_count, new_count;
    END IF;

    RAISE NOTICE 'raw_messages migration successful: % rows migrated', new_count;
END $$;

-- ================================================================================
-- MIGRATE FIXES TABLE
-- ================================================================================

-- Step 10: Rename existing partitioned fixes table
ALTER TABLE fixes RENAME TO fixes_partman;

-- Step 11: Create new non-partitioned fixes table
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
    flight_id uuid,
    aircraft_id uuid NOT NULL,
    received_at timestamp with time zone NOT NULL,
    is_active boolean NOT NULL DEFAULT true,
    altitude_agl_feet integer,
    receiver_id uuid NOT NULL,
    raw_message_id uuid NOT NULL,
    altitude_agl_valid boolean NOT NULL DEFAULT false,
    location_geom geometry(Point,4326) GENERATED ALWAYS AS (st_setsrid(st_makepoint(longitude, latitude), 4326)) STORED,
    time_gap_seconds integer,
    source_metadata jsonb
);

-- Add table comment
COMMENT ON COLUMN fixes.source_metadata IS 'Protocol-specific metadata stored as JSONB. For OGN/APRS (protocol=aprs): snr_db, bit_errors_corrected, freq_offset_khz, gnss_*_resolution. For ADS-B (protocol=adsb): nic, nac_p, nac_v, sil, emergency_status, on_ground, etc.';

-- Step 12: Convert fixes to hypertable
SELECT create_hypertable(
    'fixes',
    'received_at',
    chunk_time_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

-- Step 13: Copy data from old partitioned table
-- Exclude generated columns (location, location_geom) - they will be auto-generated
INSERT INTO fixes (
    id, source, aprs_type, via, timestamp, latitude, longitude,
    altitude_msl_feet, flight_number, squawk, ground_speed_knots,
    track_degrees, climb_fpm, turn_rate_rot, flight_id, aircraft_id,
    received_at, is_active, altitude_agl_feet, receiver_id,
    raw_message_id, altitude_agl_valid, time_gap_seconds, source_metadata
)
SELECT
    id, source, aprs_type, via, timestamp, latitude, longitude,
    altitude_msl_feet, flight_number, squawk, ground_speed_knots,
    track_degrees, climb_fpm, turn_rate_rot, flight_id, aircraft_id,
    received_at, is_active, altitude_agl_feet, receiver_id,
    raw_message_id, altitude_agl_valid, time_gap_seconds, source_metadata
FROM fixes_partman;

-- Step 14: Add primary key (must include partition key for hypertables)
ALTER TABLE fixes ADD PRIMARY KEY (id, received_at);

-- Step 15: Recreate indexes on fixes
-- Note: TimescaleDB automatically creates index on partition key (received_at)
CREATE INDEX IF NOT EXISTS idx_fixes_aircraft_received_at ON fixes (aircraft_id, received_at DESC);
CREATE INDEX IF NOT EXISTS idx_fixes_location ON fixes USING GIST (location);
CREATE INDEX IF NOT EXISTS idx_fixes_location_geom ON fixes USING GIST (location_geom);
CREATE INDEX IF NOT EXISTS idx_fixes_source ON fixes (source);
CREATE INDEX IF NOT EXISTS idx_fixes_protocol ON fixes ((source_metadata ->> 'protocol')) WHERE source_metadata IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_fixes_source_metadata ON fixes USING GIN (source_metadata);
CREATE INDEX IF NOT EXISTS idx_fixes_flight_id ON fixes (flight_id);

-- Step 16: Recreate foreign key constraints on fixes
-- IMPORTANT: TimescaleDB has a limitation - hypertables CANNOT be foreign key
-- references of other hypertables. This means we cannot create the FK from
-- fixes.raw_message_id -> raw_messages.id because both are hypertables.
-- See: https://docs.timescale.com/use-timescale/latest/schema-management/constraints/
--
-- Regular tables CAN have FKs to hypertables, so other tables can still
-- reference raw_messages. We'll rely on application-level referential integrity
-- for the fixes -> raw_messages relationship.

ALTER TABLE fixes
    ADD CONSTRAINT fixes_aircraft_id_fkey
    FOREIGN KEY (aircraft_id) REFERENCES aircraft(id) ON DELETE SET NULL;

ALTER TABLE fixes
    ADD CONSTRAINT fixes_flight_id_fkey
    FOREIGN KEY (flight_id) REFERENCES flights(id) ON DELETE RESTRICT;

-- NOTE: Cannot create fixes_raw_message_id_fkey due to TimescaleDB limitation
-- COMMENT: fixes.raw_message_id should reference raw_messages.id but FK not supported
-- Application code must maintain this referential integrity

ALTER TABLE fixes
    ADD CONSTRAINT fixes_receiver_id_fkey
    FOREIGN KEY (receiver_id) REFERENCES receivers(id) ON DELETE SET NULL;

-- Add a comment documenting the missing FK constraint
COMMENT ON COLUMN fixes.raw_message_id IS 'References raw_messages.id - FK not enforced due to TimescaleDB hypertable limitation. Application must ensure referential integrity.';

-- Step 17: Verify fixes migration
DO $$
DECLARE
    old_count bigint;
    new_count bigint;
BEGIN
    SELECT COUNT(*) INTO old_count FROM fixes_partman;
    SELECT COUNT(*) INTO new_count FROM fixes;

    IF old_count != new_count THEN
        RAISE EXCEPTION 'fixes migration failed: count mismatch. Old: %, New: %', old_count, new_count;
    END IF;

    RAISE NOTICE 'fixes migration successful: % rows migrated', new_count;
END $$;

-- ================================================================================
-- UPDATE REFERENCING TABLES
-- ================================================================================

-- Step 18: Update receiver_statuses foreign key to point to new raw_messages
ALTER TABLE receiver_statuses DROP CONSTRAINT IF EXISTS receiver_statuses_raw_message_id_fkey;
ALTER TABLE receiver_statuses
    ADD CONSTRAINT receiver_statuses_raw_message_id_fkey
    FOREIGN KEY (raw_message_id, received_at) REFERENCES raw_messages(id, received_at);

-- ================================================================================
-- CONFIGURE RETENTION POLICIES
-- ================================================================================

-- Step 19: Add retention policy to raw_messages (30 days)
-- This will automatically drop chunks older than 30 days
SELECT add_retention_policy('raw_messages', INTERVAL '30 days');

-- Step 20: Add retention policy to fixes (30 days)
SELECT add_retention_policy('fixes', INTERVAL '30 days');

-- ================================================================================
-- CONFIGURE COMPRESSION POLICIES (OPTIONAL - HUGE SPACE SAVINGS)
-- ================================================================================

-- Step 21: Enable compression on raw_messages
-- Compress chunks older than 7 days to save ~90% disk space
ALTER TABLE raw_messages SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'receiver_id',
    timescaledb.compress_orderby = 'received_at DESC'
);

-- Add compression policy (compress chunks older than 7 days)
SELECT add_compression_policy('raw_messages', INTERVAL '7 days');

-- Step 22: Enable compression on fixes
-- Compress chunks older than 7 days to save ~90% disk space
ALTER TABLE fixes SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'aircraft_id',
    timescaledb.compress_orderby = 'received_at DESC'
);

-- Add compression policy (compress chunks older than 7 days)
SELECT add_compression_policy('fixes', INTERVAL '7 days');

-- ================================================================================
-- CLEANUP PG_PARTMAN CONFIGURATION
-- ================================================================================

-- Step 23: Remove pg_partman configuration for old tables
-- This prevents pg_partman cron jobs from trying to manage these tables
DELETE FROM partman.part_config WHERE parent_table = 'public.raw_messages_partman';
DELETE FROM partman.part_config WHERE parent_table = 'public.fixes_partman';

-- ================================================================================
-- ADD HELPFUL COMMENTS
-- ================================================================================

-- Step 24: Add comments documenting the new setup
COMMENT ON TABLE raw_messages IS 'TimescaleDB hypertable partitioned by received_at (1 day chunks). Automatic retention: 30 days. Compression enabled for chunks older than 7 days.';
COMMENT ON TABLE fixes IS 'TimescaleDB hypertable partitioned by received_at (1 day chunks). Automatic retention: 30 days. Compression enabled for chunks older than 7 days.';

-- ================================================================================
-- MIGRATION COMPLETE
-- ================================================================================
-- The old tables (raw_messages_partman, fixes_partman) are kept for safety.
-- They can be dropped manually after verifying the migration was successful:
--
--   DROP TABLE fixes_partman CASCADE;
--   DROP TABLE raw_messages_partman CASCADE;
--
-- To view TimescaleDB hypertable information:
--   SELECT * FROM timescaledb_information.hypertables;
--   SELECT * FROM timescaledb_information.chunks;
--   SELECT * FROM timescaledb_information.compression_settings;
--
-- To manually compress a chunk:
--   SELECT compress_chunk('_timescaledb_internal._hyper_X_Y_chunk');
--
-- To view chunk compression status:
--   SELECT * FROM timescaledb_information.compressed_chunk_stats;
-- ================================================================================
