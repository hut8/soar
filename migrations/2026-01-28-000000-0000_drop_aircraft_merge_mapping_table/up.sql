-- Clean up out-of-band migration prep table
-- This table was created manually to run the fixes aircraft_id update out-of-band
-- before the deduplicate_aircraft_registration migration, to avoid slow updates
-- through compressed TimescaleDB chunks.
DROP TABLE IF EXISTS aircraft_merge_mapping;
