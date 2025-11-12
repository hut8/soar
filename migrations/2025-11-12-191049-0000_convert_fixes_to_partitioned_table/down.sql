-- ================================================================================
-- REVERT PARTITIONING - Restore Original fixes and aprs_messages Tables
-- ================================================================================
-- This reverts the partitioning migration by restoring the original tables.
--
-- PREREQUISITE: fixes_old and aprs_messages_old tables must still exist
-- If they were dropped, this rollback cannot be performed.
-- ================================================================================

-- Step 1: Verify old tables exist
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'fixes_old') THEN
        RAISE EXCEPTION 'Cannot rollback fixes: fixes_old table does not exist. Manual recovery required.';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'aprs_messages_old') THEN
        RAISE EXCEPTION 'Cannot rollback aprs_messages: aprs_messages_old table does not exist. Manual recovery required.';
    END IF;
END $$;

-- Step 2: Remove pg_partman configuration
DELETE FROM partman.part_config WHERE parent_table = 'public.fixes';
DELETE FROM partman.part_config WHERE parent_table = 'public.aprs_messages';

-- Step 3: Drop partitioned tables and all child partitions
DROP TABLE IF EXISTS fixes CASCADE;
DROP TABLE IF EXISTS aprs_messages CASCADE;

-- Step 4: Rename old tables back to their original names
ALTER TABLE fixes_old RENAME TO fixes;
ALTER TABLE aprs_messages_old RENAME TO aprs_messages;

-- Step 5: Recreate indexes with original names on fixes
DROP INDEX IF EXISTS fixes_pkey CASCADE;
CREATE UNIQUE INDEX IF NOT EXISTS fixes_pkey ON fixes (id);
ALTER TABLE fixes ADD CONSTRAINT fixes_pkey PRIMARY KEY USING INDEX fixes_pkey;

-- Step 6: Recreate indexes with original names on aprs_messages
DROP INDEX IF EXISTS aprs_messages_pkey CASCADE;
CREATE UNIQUE INDEX IF NOT EXISTS aprs_messages_pkey ON aprs_messages (id);
ALTER TABLE aprs_messages ADD CONSTRAINT aprs_messages_pkey PRIMARY KEY USING INDEX aprs_messages_pkey;

-- Step 7: Verify restoration
DO $$
DECLARE
    fixes_count bigint;
    aprs_count bigint;
BEGIN
    SELECT COUNT(*) INTO fixes_count FROM fixes;
    SELECT COUNT(*) INTO aprs_count FROM aprs_messages;

    RAISE NOTICE 'Rollback complete: % rows in restored fixes table', fixes_count;
    RAISE NOTICE 'Rollback complete: % rows in restored aprs_messages table', aprs_count;
END $$;

COMMENT ON TABLE fixes IS 'Non-partitioned fixes table (partitioning rolled back)';
COMMENT ON TABLE aprs_messages IS 'Non-partitioned aprs_messages table (partitioning rolled back)';
