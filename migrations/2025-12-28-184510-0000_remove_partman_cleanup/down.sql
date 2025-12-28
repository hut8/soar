-- ================================================================================
-- ROLLBACK: REINSTALL PG_PARTMAN (DATA LOSS WARNING)
-- ================================================================================
-- WARNING: This rollback CANNOT restore the dropped tables and data.
-- It can only reinstall the pg_partman extension.
--
-- The dropped tables (fixes_partman, raw_messages_partman, and all their child
-- partitions) contained historical data that is now gone. However, the current
-- data is safe in the TimescaleDB hypertables (fixes, raw_messages).
--
-- This rollback is provided for completeness, but in practice you should:
-- 1. Restore from backup if you need the old partitioned data
-- 2. Use the TimescaleDB migration rollback (2025-12-27-025500-0000_convert_to_timescaledb)
--    instead, which does have the old tables available
-- ================================================================================

-- Step 1: Reinstall pg_partman extension
CREATE EXTENSION IF NOT EXISTS pg_partman;

-- Step 2: Add warning notice
DO $$
BEGIN
    RAISE WARNING '========================================';
    RAISE WARNING 'PARTMAN REINSTALLED - DATA LOSS OCCURRED';
    RAISE WARNING '========================================';
    RAISE WARNING 'The pg_partman extension has been reinstalled.';
    RAISE WARNING '';
    RAISE WARNING 'CRITICAL: The following tables were dropped and CANNOT be recovered:';
    RAISE WARNING '  - fixes_partman and all child partitions';
    RAISE WARNING '  - raw_messages_partman and all child partitions';
    RAISE WARNING '  - fixes_old';
    RAISE WARNING '';
    RAISE WARNING 'Current state:';
    RAISE WARNING '  - fixes and raw_messages are still TimescaleDB hypertables';
    RAISE WARNING '  - pg_partman is installed but not managing any tables';
    RAISE WARNING '';
    RAISE WARNING 'To fully rollback to partman:';
    RAISE WARNING '  1. Rollback the TimescaleDB migration: 2025-12-27-025500';
    RAISE WARNING '  2. Then rollback this migration';
    RAISE WARNING '========================================';
END $$;

-- ================================================================================
-- ROLLBACK INCOMPLETE
-- ================================================================================
-- This rollback only reinstalls pg_partman. The old partitioned tables are gone.
-- If you need to restore the old partman setup, you must:
--
-- 1. First, rollback this migration (reinstalls pg_partman)
-- 2. Then, rollback the TimescaleDB migration:
--    diesel migration revert --migration-dir=migrations/2025-12-27-025500-0000_convert_to_timescaledb
--
-- That will restore the old partitioned tables from *_partman which were kept
-- during the TimescaleDB migration.
-- ================================================================================
