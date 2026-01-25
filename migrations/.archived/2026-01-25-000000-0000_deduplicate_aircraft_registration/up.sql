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
-- Using SET LOCAL to ensure it applies within this transaction.
SET LOCAL timescaledb.max_tuples_decompressed_per_dml_transaction = 0;

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

-- 3e: Fixes - Decompress affected chunks, update, then recompress
-- Updating compressed chunks is slow because each update requires decompress-update-recompress.
-- Much faster to decompress first, do one big update, then recompress.
DO $$
DECLARE
    chunk_record RECORD;
    chunk_count INTEGER := 0;
    min_ts TIMESTAMP WITH TIME ZONE;
    max_ts TIMESTAMP WITH TIME ZONE;
    updated_count INTEGER;
BEGIN
    -- Get time range of fixes that need updating
    SELECT MIN(fx.received_at), MAX(fx.received_at)
    INTO min_ts, max_ts
    FROM fixes fx
    JOIN aircraft_merge m ON fx.aircraft_id = m.flarm_id;

    IF min_ts IS NULL THEN
        RAISE NOTICE 'No fixes to update';
        RETURN;
    END IF;

    RAISE NOTICE 'Fixes time range: % to %', min_ts, max_ts;

    -- Step 1: Decompress all compressed chunks in the affected time range
    RAISE NOTICE 'Decompressing affected chunks...';
    FOR chunk_record IN
        SELECT chunk_schema, chunk_name
        FROM timescaledb_information.chunks
        WHERE hypertable_name = 'fixes'
          AND is_compressed = true
          AND range_start <= max_ts
          AND range_end >= min_ts
    LOOP
        EXECUTE format('SELECT decompress_chunk(%L)',
            chunk_record.chunk_schema || '.' || chunk_record.chunk_name);
        chunk_count := chunk_count + 1;
        RAISE NOTICE 'Decompressed chunk %', chunk_record.chunk_name;
    END LOOP;
    RAISE NOTICE 'Decompressed % chunks', chunk_count;

    -- Step 2: Single UPDATE on uncompressed data (fast!)
    RAISE NOTICE 'Updating fixes...';
    UPDATE fixes fx
    SET aircraft_id = m.icao_id
    FROM aircraft_merge m
    WHERE fx.aircraft_id = m.flarm_id;

    GET DIAGNOSTICS updated_count = ROW_COUNT;
    RAISE NOTICE 'Updated % fixes', updated_count;

    -- Step 3: Recompress the chunks we decompressed
    RAISE NOTICE 'Recompressing chunks...';
    chunk_count := 0;
    FOR chunk_record IN
        SELECT chunk_schema, chunk_name
        FROM timescaledb_information.chunks
        WHERE hypertable_name = 'fixes'
          AND is_compressed = false
          AND range_start <= max_ts
          AND range_end >= min_ts
          -- Only recompress chunks older than 1 day (recent chunks stay uncompressed)
          AND range_end < NOW() - INTERVAL '1 day'
    LOOP
        EXECUTE format('SELECT compress_chunk(%L)',
            chunk_record.chunk_schema || '.' || chunk_record.chunk_name);
        chunk_count := chunk_count + 1;
    END LOOP;
    RAISE NOTICE 'Recompressed % chunks', chunk_count;
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
