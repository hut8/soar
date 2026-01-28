-- Create the aircraft merge mapping table for out-of-band fixes update
-- This identifies FLARM+ICAO duplicate pairs that need to be merged
--
-- Run this BEFORE running update_fixes_decompress_first.sh or update_fixes_parallel.py
--
-- Usage:
--   psql -d soar -f scripts/create-aircraft-merge-mapping.sql

-- Drop if exists (in case of re-run)
DROP TABLE IF EXISTS aircraft_merge_mapping;

-- Create mapping table: FLARM records to merge into ICAO records
CREATE TABLE aircraft_merge_mapping AS
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

-- Add index for fast lookups during UPDATE
CREATE INDEX idx_aircraft_merge_mapping_flarm_id ON aircraft_merge_mapping(flarm_id);

-- Show count
SELECT COUNT(*) as duplicate_pairs_to_merge FROM aircraft_merge_mapping;
