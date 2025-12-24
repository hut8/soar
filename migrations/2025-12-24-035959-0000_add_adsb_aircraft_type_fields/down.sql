-- Remove indexes
DROP INDEX IF EXISTS idx_aircraft_category;
DROP INDEX IF EXISTS idx_aircraft_engine_type;
DROP INDEX IF EXISTS idx_aircraft_faa_pia;
DROP INDEX IF EXISTS idx_aircraft_faa_ladd;
DROP INDEX IF EXISTS idx_aircraft_owner_operator;

-- Remove columns from aircraft table
ALTER TABLE aircraft
DROP COLUMN IF EXISTS aircraft_category,
DROP COLUMN IF EXISTS engine_count,
DROP COLUMN IF EXISTS engine_type,
DROP COLUMN IF EXISTS faa_pia,
DROP COLUMN IF EXISTS faa_ladd,
DROP COLUMN IF EXISTS owner_operator,
DROP COLUMN IF EXISTS from_adsbx_ddb;

-- Rename from_ogn_ddb back to from_ddb
ALTER TABLE aircraft RENAME COLUMN from_ogn_ddb TO from_ddb;

-- Drop enum types
DROP TYPE IF EXISTS engine_type;
DROP TYPE IF EXISTS aircraft_category;
