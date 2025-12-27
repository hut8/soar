-- ================================================================================
-- ROLLBACK TIMESCALEDB MIGRATION - RESTORE PG_PARTMAN SETUP
-- ================================================================================
-- This migration rolls back the TimescaleDB conversion and restores the
-- pg_partman-managed partitioned tables.
--
-- WARNING: This will drop the TimescaleDB hypertables and restore the old
-- partitioned tables. Only run this if the migration failed or needs to be
-- reverted during testing.
-- ================================================================================

-- Step 1: Drop foreign keys from referencing tables
ALTER TABLE receiver_statuses DROP CONSTRAINT IF EXISTS receiver_statuses_raw_message_id_fkey;

-- ================================================================================
-- RESTORE FIXES TABLE
-- ================================================================================

-- Step 2: Drop the TimescaleDB fixes hypertable
DROP TABLE IF EXISTS fixes CASCADE;

-- Step 3: Rename the old partitioned table back
ALTER TABLE fixes_partman RENAME TO fixes;

-- Step 4: Restore foreign key on receiver_statuses (if it existed before)
-- This will reconnect to the restored partitioned raw_messages table
-- We'll add this after raw_messages is restored

-- ================================================================================
-- RESTORE RAW_MESSAGES TABLE
-- ================================================================================

-- Step 5: Drop the TimescaleDB raw_messages hypertable
DROP TABLE IF EXISTS raw_messages CASCADE;

-- Step 6: Rename the old partitioned table back
ALTER TABLE raw_messages_partman RENAME TO raw_messages;

-- ================================================================================
-- RESTORE FOREIGN KEYS FROM REFERENCING TABLES
-- ================================================================================

-- Step 7: Restore receiver_statuses foreign key to raw_messages
ALTER TABLE receiver_statuses
    ADD CONSTRAINT receiver_statuses_raw_message_id_fkey
    FOREIGN KEY (raw_message_id, received_at) REFERENCES raw_messages(id, received_at);

-- ================================================================================
-- RESTORE PG_PARTMAN CONFIGURATION
-- ================================================================================

-- Step 8: Restore pg_partman configuration for fixes
-- Note: This assumes pg_partman is still installed and configured
INSERT INTO partman.part_config (
    parent_table,
    control,
    partition_interval,
    partition_type,
    premake,
    retention,
    retention_keep_table,
    retention_keep_index,
    infinite_time_partitions
) VALUES (
    'public.fixes',
    'received_at',
    '1 day',
    'native',
    3,
    '30 days',
    true,
    false,
    true
) ON CONFLICT (parent_table) DO UPDATE SET
    control = EXCLUDED.control,
    partition_interval = EXCLUDED.partition_interval,
    partition_type = EXCLUDED.partition_type,
    premake = EXCLUDED.premake,
    retention = EXCLUDED.retention,
    retention_keep_table = EXCLUDED.retention_keep_table,
    retention_keep_index = EXCLUDED.retention_keep_index,
    infinite_time_partitions = EXCLUDED.infinite_time_partitions;

-- Step 9: Restore pg_partman configuration for raw_messages
INSERT INTO partman.part_config (
    parent_table,
    control,
    partition_interval,
    partition_type,
    premake,
    retention,
    retention_keep_table,
    retention_keep_index,
    infinite_time_partitions
) VALUES (
    'public.raw_messages',
    'received_at',
    '1 day',
    'native',
    3,
    '30 days',
    true,
    false,
    true
) ON CONFLICT (parent_table) DO UPDATE SET
    control = EXCLUDED.control,
    partition_interval = EXCLUDED.partition_interval,
    partition_type = EXCLUDED.partition_type,
    premake = EXCLUDED.premake,
    retention = EXCLUDED.retention,
    retention_keep_table = EXCLUDED.retention_keep_table,
    retention_keep_index = EXCLUDED.retention_keep_index,
    infinite_time_partitions = EXCLUDED.infinite_time_partitions;

-- ================================================================================
-- RESTORE TABLE COMMENTS
-- ================================================================================

-- Step 10: Restore original comments
COMMENT ON TABLE raw_messages IS 'Partitioned by received_at (daily). Managed by pg_partman. Retention: 30 days (detached, not dropped).';
COMMENT ON TABLE fixes IS 'Partitioned by received_at (daily). Managed by pg_partman. Retention: 30 days (detached, not dropped).';

-- ================================================================================
-- ROLLBACK COMPLETE
-- ================================================================================
-- The TimescaleDB hypertables have been removed and the original pg_partman
-- partitioned tables have been restored.
--
-- NOTE: If you want to completely remove TimescaleDB:
--   DROP EXTENSION IF EXISTS timescaledb CASCADE;
--
-- However, this is NOT recommended if you plan to attempt the migration again.
-- ================================================================================
