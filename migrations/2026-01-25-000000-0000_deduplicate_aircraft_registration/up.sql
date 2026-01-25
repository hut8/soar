-- Migration: Deduplicate aircraft by registration
-- This migration:
-- 1. Merges FLARM+ICAO duplicates (keeping ICAO address, merging data)
-- 2. Updates FK references
-- 3. Deletes merged FLARM records
-- 4. Nulls all registrations (will be repopulated by next data load with validation)
-- 5. Adds unique partial index on registration

-- CRITICAL: Disable TimescaleDB decompression limit for this migration
-- The fixes table has compressed chunks, and updating aircraft_id across 16M+ rows
-- would exceed the default 100K tuple decompression limit. Setting to 0 = unlimited.
SET timescaledb.max_tuples_decompressed_per_dml_transaction = 0;

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

-- 3e: Fixes - BATCHED by hour on received_at (the hypertable partition column)
-- The fixes table is a hypertable partitioned by received_at with compressed chunks.
-- Batching by received_at aligns with chunk boundaries for efficient updates.
DO $$
DECLARE
    batch_start TIMESTAMP WITH TIME ZONE;
    batch_end TIMESTAMP WITH TIME ZONE;
    min_ts TIMESTAMP WITH TIME ZONE;
    max_ts TIMESTAMP WITH TIME ZONE;
    updated_count INTEGER;
    total_updated INTEGER := 0;
    current_day DATE := NULL;
    day_count INTEGER := 0;
BEGIN
    -- Get time range of fixes that need updating (using received_at, the partition column)
    SELECT MIN(fx.received_at), MAX(fx.received_at)
    INTO min_ts, max_ts
    FROM fixes fx
    JOIN aircraft_merge m ON fx.aircraft_id = m.flarm_id;

    IF min_ts IS NULL THEN
        RAISE NOTICE 'No fixes to update';
        RETURN;
    END IF;

    RAISE NOTICE 'Updating fixes from % to % (hourly batches by received_at)', min_ts, max_ts;

    -- Process in hourly batches aligned with chunk boundaries
    batch_start := DATE_TRUNC('hour', min_ts);
    WHILE batch_start <= max_ts LOOP
        batch_end := batch_start + INTERVAL '1 hour';

        -- Check for day change BEFORE updating (to log previous day's total)
        IF batch_start::date != current_day THEN
            IF current_day IS NOT NULL AND day_count > 0 THEN
                RAISE NOTICE 'Updated % fixes for %', day_count, current_day;
            END IF;
            current_day := batch_start::date;
            day_count := 0;
        END IF;

        UPDATE fixes fx
        SET aircraft_id = m.icao_id
        FROM aircraft_merge m
        WHERE fx.aircraft_id = m.flarm_id
          AND fx.received_at >= batch_start
          AND fx.received_at < batch_end;

        GET DIAGNOSTICS updated_count = ROW_COUNT;
        total_updated := total_updated + updated_count;
        day_count := day_count + updated_count;

        batch_start := batch_end;
    END LOOP;

    -- Log final day
    IF day_count > 0 THEN
        RAISE NOTICE 'Updated % fixes for %', day_count, current_day;
    END IF;

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
