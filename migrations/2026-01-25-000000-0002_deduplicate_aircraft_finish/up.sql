-- Migration 2 of 2: Deduplicate aircraft - Finish and create unique index
--
-- Prerequisites:
-- 1. Migration 1 (deduplicate_aircraft_setup) must be run
-- 2. scripts/update_fixes_aircraft_id.sh must be run to completion
--
-- This migration:
-- 1. Verifies mapping table exists
-- 2. Drops FK constraint temporarily (avoids expensive validation scan on compressed fixes)
-- 3. Deletes merged FLARM aircraft records
-- 4. Recreates FK constraint with NOT VALID (skip validation scan)
-- 5. Nulls all registrations (will be repopulated by data load)
-- 6. Creates unique index on registration
-- 7. Cleans up the merge mapping table

-- Step 1: Verify prerequisites (fast check - mapping table existence only)
-- The fixes update script verifies completion separately, and checking fixes
-- here is too slow due to transparent decompression
DO $$
BEGIN
    -- Check merge mapping table exists
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'aircraft_merge_mapping') THEN
        RAISE EXCEPTION 'aircraft_merge_mapping table not found. Run migration 1 first.';
    END IF;

    RAISE NOTICE 'Prerequisite check passed: aircraft_merge_mapping table exists';
    RAISE NOTICE 'IMPORTANT: Ensure scripts/update_fixes_aircraft_id.sh completed successfully!';
END $$;

-- Step 2: Drop FK constraint temporarily to avoid expensive validation scan
-- The fixes table is a compressed hypertable; FK validation would trigger
-- transparent decompression of 300M+ rows even though no fixes reference these aircraft
ALTER TABLE fixes DROP CONSTRAINT fixes_aircraft_id_fkey;

-- Step 3: Delete merged FLARM aircraft records (fast without FK validation)
DELETE FROM aircraft WHERE id IN (SELECT flarm_id FROM aircraft_merge_mapping);

-- Step 4: Recreate FK constraint with NOT VALID (skips expensive validation scan)
-- The fixes update script already verified all fixes point to valid aircraft IDs
-- Validation can be run manually later if needed after decompressing chunks:
--   ALTER TABLE fixes VALIDATE CONSTRAINT fixes_aircraft_id_fkey;
ALTER TABLE fixes ADD CONSTRAINT fixes_aircraft_id_fkey
    FOREIGN KEY (aircraft_id) REFERENCES aircraft(id) NOT VALID;

-- Step 5: Null ALL registrations
-- The next data load will repopulate valid registrations using flydent validation
UPDATE aircraft SET registration = NULL WHERE registration IS NOT NULL;

-- Step 6: Drop existing non-unique index and add unique partial index
DROP INDEX IF EXISTS idx_aircraft_registration;

CREATE UNIQUE INDEX idx_aircraft_registration_unique
ON aircraft (registration)
WHERE registration IS NOT NULL;

-- Step 7: Clean up merge mapping table
DROP TABLE aircraft_merge_mapping;

DO $$
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '=== Aircraft deduplication complete ===';
    RAISE NOTICE 'Unique index on registration created.';
    RAISE NOTICE '';
END $$;
