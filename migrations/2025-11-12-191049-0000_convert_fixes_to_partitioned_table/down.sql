-- ================================================================================
-- REVERT PARTITIONING - Restore Original fixes Table
-- ================================================================================
-- This reverts the partitioning migration by restoring the original fixes table.
--
-- PREREQUISITE: fixes_old table must still exist (up.sql leaves it for safety)
-- If fixes_old was dropped, this rollback cannot be performed.
-- ================================================================================

-- Step 1: Verify fixes_old exists
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'fixes_old') THEN
        RAISE EXCEPTION 'Cannot rollback: fixes_old table does not exist. Manual recovery required.';
    END IF;
END $$;

-- Step 2: Remove pg_partman configuration
DELETE FROM partman.part_config WHERE parent_table = 'public.fixes';

-- Step 3: Drop partitioned fixes table and all child partitions
DROP TABLE IF EXISTS fixes CASCADE;

-- Step 4: Rename fixes_old back to fixes
ALTER TABLE fixes_old RENAME TO fixes;

-- Step 5: Recreate indexes with original names (they were on fixes_old, may have different names)
-- Most of these should already exist from the original table
DROP INDEX IF EXISTS fixes_pkey CASCADE;
CREATE UNIQUE INDEX IF NOT EXISTS fixes_pkey ON fixes (id);
ALTER TABLE fixes ADD CONSTRAINT fixes_pkey PRIMARY KEY USING INDEX fixes_pkey;

-- Verify restoration
DO $$
DECLARE
    row_count bigint;
BEGIN
    SELECT COUNT(*) INTO row_count FROM fixes;
    RAISE NOTICE 'Rollback complete: % rows in restored fixes table', row_count;
END $$;

COMMENT ON TABLE fixes IS 'Non-partitioned fixes table (partitioning rolled back)';
