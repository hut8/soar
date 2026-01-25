-- Migration: Deduplicate aircraft by registration
-- This migration:
-- 1. Merges FLARM+ICAO duplicates (keeping ICAO address, merging data)
-- 2. Updates FK references
-- 3. Deletes merged FLARM records
-- 4. Nulls all registrations (will be repopulated by next data load with validation)
-- 5. Adds unique partial index on registration

-- Step 1: Create merge mapping table (FLARM records to merge into ICAO records)
-- Must be done BEFORE nulling registrations
CREATE TEMP TABLE aircraft_merge AS
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

-- Log count of records to merge
DO $$
DECLARE
    merge_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO merge_count FROM aircraft_merge;
    RAISE NOTICE 'Found % FLARM+ICAO duplicate pairs to merge', merge_count;
END $$;

-- Step 2: Merge fields from FLARM to ICAO (keep ICAO values if non-null/non-empty)
UPDATE aircraft i
SET
    competition_number = COALESCE(NULLIF(i.competition_number, ''), f.competition_number),
    pilot_name = COALESCE(i.pilot_name, f.pilot_name),
    home_base_airport_ident = COALESCE(i.home_base_airport_ident, f.home_base_airport_ident),
    frequency_mhz = COALESCE(i.frequency_mhz, f.frequency_mhz),
    from_ogn_ddb = i.from_ogn_ddb OR f.from_ogn_ddb
FROM aircraft f
JOIN aircraft_merge m ON f.id = m.flarm_id
WHERE i.id = m.icao_id;

-- Step 3: Update FK references

-- 3a: Flights aircraft_id
UPDATE flights SET aircraft_id = m.icao_id
FROM aircraft_merge m WHERE flights.aircraft_id = m.flarm_id;

-- 3b: Flights towed_by_aircraft_id
UPDATE flights SET towed_by_aircraft_id = m.icao_id
FROM aircraft_merge m WHERE flights.towed_by_aircraft_id = m.flarm_id;

-- 3c: Watchlist
UPDATE watchlist SET aircraft_id = m.icao_id
FROM aircraft_merge m WHERE watchlist.aircraft_id = m.flarm_id;

-- 3d: Aircraft registrations
UPDATE aircraft_registrations SET aircraft_id = m.icao_id
FROM aircraft_merge m WHERE aircraft_registrations.aircraft_id = m.flarm_id;

-- 3e: Fixes - BATCHED by day to avoid TimescaleDB issues with large updates
-- The fixes table is a hypertable and updating millions of rows at once can cause issues
DO $$
DECLARE
    batch_start TIMESTAMP WITH TIME ZONE;
    batch_end TIMESTAMP WITH TIME ZONE;
    min_ts TIMESTAMP WITH TIME ZONE;
    max_ts TIMESTAMP WITH TIME ZONE;
    updated_count INTEGER;
    total_updated INTEGER := 0;
BEGIN
    -- Get time range of fixes that need updating
    SELECT MIN(fx.timestamp), MAX(fx.timestamp)
    INTO min_ts, max_ts
    FROM fixes fx
    JOIN aircraft_merge m ON fx.aircraft_id = m.flarm_id;

    IF min_ts IS NULL THEN
        RAISE NOTICE 'No fixes to update';
        RETURN;
    END IF;

    RAISE NOTICE 'Updating fixes from % to %', min_ts, max_ts;

    -- Process in daily batches
    batch_start := DATE_TRUNC('day', min_ts);
    WHILE batch_start <= max_ts LOOP
        batch_end := batch_start + INTERVAL '1 day';

        UPDATE fixes fx
        SET aircraft_id = m.icao_id
        FROM aircraft_merge m
        WHERE fx.aircraft_id = m.flarm_id
          AND fx.timestamp >= batch_start
          AND fx.timestamp < batch_end;

        GET DIAGNOSTICS updated_count = ROW_COUNT;
        total_updated := total_updated + updated_count;

        IF updated_count > 0 THEN
            RAISE NOTICE 'Updated % fixes for %', updated_count, batch_start::date;
        END IF;

        batch_start := batch_end;
    END LOOP;

    RAISE NOTICE 'Total fixes updated: %', total_updated;
END $$;

-- Step 4: Delete merged FLARM records
DELETE FROM aircraft WHERE id IN (SELECT flarm_id FROM aircraft_merge);

-- Log count of deleted records
DO $$
DECLARE
    deleted_count INTEGER;
BEGIN
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RAISE NOTICE 'Deleted % merged FLARM records', deleted_count;
END $$;

-- Clean up temp table
DROP TABLE aircraft_merge;

-- Step 5: Null ALL registrations
-- The next data load will repopulate valid registrations using flydent validation
-- This is safer than trying to validate in SQL
UPDATE aircraft SET registration = NULL WHERE registration IS NOT NULL;

-- Step 6: Drop existing non-unique index and add unique partial index
DROP INDEX IF EXISTS idx_aircraft_registration;

CREATE UNIQUE INDEX idx_aircraft_registration_unique
ON aircraft (registration)
WHERE registration IS NOT NULL;
