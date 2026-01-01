-- Remove indexes
DROP INDEX IF EXISTS idx_aircraft_year;
DROP INDEX IF EXISTS idx_aircraft_is_military;

-- Remove columns from aircraft table
ALTER TABLE aircraft
DROP COLUMN IF EXISTS year,
DROP COLUMN IF EXISTS is_military;
