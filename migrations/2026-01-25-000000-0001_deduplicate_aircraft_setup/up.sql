-- Migration 1 of 2: Deduplicate aircraft - Setup and fast FK updates
--
-- This migration:
-- 1. Creates a permanent merge mapping table (FLARM→ICAO)
-- 2. Merges aircraft fields from FLARM to ICAO records
-- 3. Updates small FK tables (flights, watchlist, aircraft_registrations)
--
-- After this migration, run the manual script to update fixes.aircraft_id:
--   psql -d soar_staging -f scripts/update_fixes_aircraft_id.sql
--
-- Then run migration 2 to complete the process.

-- Step 1: Create permanent merge mapping table
-- This table persists so the manual fixes script and migration 2 can use it
CREATE TABLE aircraft_merge_mapping (
    flarm_id UUID PRIMARY KEY,
    icao_id UUID NOT NULL,
    registration TEXT NOT NULL
);

-- Populate with FLARM+ICAO duplicate pairs
INSERT INTO aircraft_merge_mapping (flarm_id, icao_id, registration)
SELECT
    f.id as flarm_id,
    i.id as icao_id,
    f.registration
FROM aircraft f
JOIN aircraft i ON f.registration = i.registration
WHERE f.address_type = 'flarm'
  AND i.address_type = 'icao'
  AND f.registration IS NOT NULL
  AND f.registration != ''
  AND f.id != i.id;

-- Log count
DO $$
DECLARE
    merge_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO merge_count FROM aircraft_merge_mapping;
    RAISE NOTICE 'Created merge mapping with % FLARM→ICAO pairs', merge_count;
END $$;

-- Create index for efficient lookups during fixes update
CREATE INDEX idx_aircraft_merge_mapping_flarm_id ON aircraft_merge_mapping(flarm_id);

-- Step 2: Merge fields from FLARM to ICAO (keep ICAO values if non-null/non-empty)
UPDATE aircraft i
SET
    competition_number = COALESCE(NULLIF(i.competition_number, ''), f.competition_number),
    pilot_name = COALESCE(i.pilot_name, f.pilot_name),
    home_base_airport_ident = COALESCE(i.home_base_airport_ident, f.home_base_airport_ident),
    frequency_mhz = COALESCE(i.frequency_mhz, f.frequency_mhz),
    from_ogn_ddb = i.from_ogn_ddb OR f.from_ogn_ddb
FROM aircraft f
JOIN aircraft_merge_mapping m ON f.id = m.flarm_id
WHERE i.id = m.icao_id;

-- Step 3: Update small FK tables

-- 3a: Flights aircraft_id
UPDATE flights SET aircraft_id = m.icao_id
FROM aircraft_merge_mapping m WHERE flights.aircraft_id = m.flarm_id;

-- 3b: Flights towed_by_aircraft_id
UPDATE flights SET towed_by_aircraft_id = m.icao_id
FROM aircraft_merge_mapping m WHERE flights.towed_by_aircraft_id = m.flarm_id;

-- 3c: Watchlist
UPDATE watchlist SET aircraft_id = m.icao_id
FROM aircraft_merge_mapping m WHERE watchlist.aircraft_id = m.flarm_id;

-- 3d: Aircraft registrations
UPDATE aircraft_registrations SET aircraft_id = m.icao_id
FROM aircraft_merge_mapping m WHERE aircraft_registrations.aircraft_id = m.flarm_id;

-- Done! Next step: run scripts/update_fixes_aircraft_id.sql manually
DO $$
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '=== Migration 1 complete ===';
    RAISE NOTICE 'Next: Run the fixes update script manually:';
    RAISE NOTICE '  psql -d soar_staging -f scripts/update_fixes_aircraft_id.sql';
    RAISE NOTICE 'Then run migration 2 to complete the process.';
    RAISE NOTICE '';
END $$;
