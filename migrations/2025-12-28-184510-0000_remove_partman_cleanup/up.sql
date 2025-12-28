-- ================================================================================
-- REMOVE PG_PARTMAN AND CLEANUP OLD PARTITIONED TABLES
-- ================================================================================
-- This migration completes the transition from pg_partman to TimescaleDB by
-- removing all pg_partman remnants, old partitioned tables, and the extension.
--
-- Background:
-- - Migration 2025-12-27-025500-0000_convert_to_timescaledb converted fixes and
--   raw_messages from pg_partman-managed partitions to TimescaleDB hypertables
-- - The old partitioned tables (*_partman) were kept for safety verification
-- - Now that TimescaleDB is proven working, we can remove the old infrastructure
--
-- This migration:
-- 1. Removes incorrect pg_partman config entries (causing systemd failures)
-- 2. Drops the empty fixes_old table
-- 3. Drops old partman parent tables and all their child partitions
-- 4. Drops partman template tables
-- 5. Removes the pg_partman extension completely
-- ================================================================================

-- ================================================================================
-- STEP 1: REMOVE PARTMAN CONFIGURATION
-- ================================================================================
-- The TimescaleDB migration incorrectly left partman.part_config entries for
-- 'public.fixes' and 'public.raw_messages' (which are now TimescaleDB hypertables).
-- This causes the partman maintenance cron job to fail.
-- Remove these entries to prevent failures.

DELETE FROM partman.part_config WHERE parent_table = 'public.fixes';
DELETE FROM partman.part_config WHERE parent_table = 'public.raw_messages';

-- ================================================================================
-- STEP 2: DROP EMPTY LEGACY TABLE
-- ================================================================================
-- The fixes_old table is from a previous schema migration and is now empty.

DROP TABLE IF EXISTS fixes_old CASCADE;

-- ================================================================================
-- STEP 3: DROP OLD PARTMAN PARENT TABLES
-- ================================================================================
-- These tables are no longer receiving data since the TimescaleDB migration.
-- DROP CASCADE will automatically drop all child partitions:
--   - fixes_p20251209 through fixes_p20260103 (27 partitions)
--   - raw_messages_p20251209 through raw_messages_p20260103 (27 partitions)

DROP TABLE IF EXISTS fixes_partman CASCADE;
DROP TABLE IF EXISTS raw_messages_partman CASCADE;

-- ================================================================================
-- STEP 4: DROP PARTMAN TEMPLATE TABLES
-- ================================================================================
-- pg_partman uses template tables to define structure for new partitions.
-- These are no longer needed.

DROP TABLE IF EXISTS partman.template_public_fixes CASCADE;
DROP TABLE IF EXISTS partman.template_public_raw_messages CASCADE;

-- ================================================================================
-- STEP 5: VERIFY PARTMAN IS NO LONGER IN USE
-- ================================================================================
-- Before dropping the extension, verify no other tables are using it.

DO $$
DECLARE
    remaining_count integer;
BEGIN
    SELECT COUNT(*) INTO remaining_count FROM partman.part_config;

    IF remaining_count > 0 THEN
        RAISE EXCEPTION 'Cannot remove pg_partman: % tables still configured. Check partman.part_config', remaining_count;
    END IF;

    RAISE NOTICE 'pg_partman is not managing any tables. Safe to remove.';
END $$;

-- ================================================================================
-- STEP 6: DROP PG_PARTMAN EXTENSION
-- ================================================================================
-- Remove pg_partman extension and its schema completely.
-- CASCADE will drop all partman functions, tables, and dependent objects.

DROP SCHEMA IF EXISTS partman CASCADE;
DROP EXTENSION IF EXISTS pg_partman CASCADE;

-- ================================================================================
-- STEP 7: ADD COMPLETION NOTICE
-- ================================================================================

DO $$
BEGIN
    RAISE NOTICE '========================================';
    RAISE NOTICE 'Partman cleanup complete!';
    RAISE NOTICE '========================================';
    RAISE NOTICE 'Removed:';
    RAISE NOTICE '  - fixes_old table';
    RAISE NOTICE '  - fixes_partman and 27 child partitions';
    RAISE NOTICE '  - raw_messages_partman and 27 child partitions';
    RAISE NOTICE '  - pg_partman extension and schema';
    RAISE NOTICE '';
    RAISE NOTICE 'Active tables (TimescaleDB hypertables):';
    RAISE NOTICE '  - fixes (managed by TimescaleDB)';
    RAISE NOTICE '  - raw_messages (managed by TimescaleDB)';
    RAISE NOTICE '';
    RAISE NOTICE 'Next steps:';
    RAISE NOTICE '  1. Stop/disable partman systemd services';
    RAISE NOTICE '  2. Regenerate Diesel schema: diesel print-schema > src/schema.rs';
    RAISE NOTICE '========================================';
END $$;

-- ================================================================================
-- MIGRATION COMPLETE
-- ================================================================================
-- All pg_partman infrastructure has been removed. The database now uses only
-- TimescaleDB for time-series partitioning.
--
-- Manual cleanup required on servers:
--   sudo systemctl stop partman-maintenance-staging.timer
--   sudo systemctl disable partman-maintenance-staging.timer
--   sudo rm /etc/systemd/system/partman-maintenance-staging.*
--   sudo systemctl daemon-reload
-- ================================================================================
