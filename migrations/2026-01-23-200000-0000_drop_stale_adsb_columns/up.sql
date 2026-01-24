-- Drop stale columns and types that were created by an earlier migration
-- but never used. These exist in staging but not in the codebase.
-- All data in these columns is NULL.

-- Drop index first
DROP INDEX IF EXISTS idx_aircraft_engine_type_adsb;

-- Drop columns
ALTER TABLE aircraft DROP COLUMN IF EXISTS engine_type_adsb;
ALTER TABLE aircraft DROP COLUMN IF EXISTS num_engines;

-- Drop the stale enum type
DROP TYPE IF EXISTS engine_type_adsb;
