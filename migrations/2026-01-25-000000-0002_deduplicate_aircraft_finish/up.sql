-- Migration 2 of 2: Deduplicate aircraft - Finish and create unique index
--
-- Prerequisites:
-- 1. Migration 1 (deduplicate_aircraft_setup) must be run
-- 2. scripts/update_fixes_aircraft_id.sh must be run to completion
--
-- This migration:
-- 1. Verifies all fixes have been updated (fails if not)
-- 2. Deletes merged FLARM aircraft records
-- 3. Nulls all registrations (will be repopulated by data load)
-- 4. Creates unique index on registration
-- 5. Cleans up the merge mapping table

-- Step 1: Verify prerequisites - FAIL FAST if fixes aren't updated
DO $$
DECLARE
    remaining_count BIGINT;
BEGIN
    -- Check merge mapping table exists
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'aircraft_merge_mapping') THEN
        RAISE EXCEPTION 'aircraft_merge_mapping table not found. Run migration 1 first.';
    END IF;

    -- Check all fixes have been updated
    SELECT COUNT(*) INTO remaining_count
    FROM fixes fx
    JOIN aircraft_merge_mapping m ON fx.aircraft_id = m.flarm_id;

    IF remaining_count > 0 THEN
        RAISE EXCEPTION 'Cannot proceed: % fixes still reference old FLARM aircraft IDs. Run scripts/update_fixes_aircraft_id.sh first.', remaining_count;
    END IF;

    RAISE NOTICE 'Prerequisite check passed: all fixes have been updated';
END $$;

-- Step 2: Delete merged FLARM aircraft records
DELETE FROM aircraft WHERE id IN (SELECT flarm_id FROM aircraft_merge_mapping);

DO $$
DECLARE
    deleted_count INTEGER;
BEGIN
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RAISE NOTICE 'Deleted % merged FLARM aircraft records', deleted_count;
END $$;

-- Step 3: Null ALL registrations
-- The next data load will repopulate valid registrations using flydent validation
UPDATE aircraft SET registration = NULL WHERE registration IS NOT NULL;

DO $$
DECLARE
    updated_count INTEGER;
BEGIN
    GET DIAGNOSTICS updated_count = ROW_COUNT;
    RAISE NOTICE 'Nulled registrations for % aircraft', updated_count;
END $$;

-- Step 4: Drop existing non-unique index and add unique partial index
DROP INDEX IF EXISTS idx_aircraft_registration;

CREATE UNIQUE INDEX idx_aircraft_registration_unique
ON aircraft (registration)
WHERE registration IS NOT NULL;

-- Step 5: Clean up merge mapping table
DROP TABLE aircraft_merge_mapping;

DO $$
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '=== Aircraft deduplication complete ===';
    RAISE NOTICE 'Unique index on registration created.';
    RAISE NOTICE '';
END $$;
