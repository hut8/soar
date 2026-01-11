-- Drop indexes first
DROP INDEX IF EXISTS idx_runways_le_location_geom;
DROP INDEX IF EXISTS idx_runways_he_location_geom;

-- Drop generated columns
ALTER TABLE runways
    DROP COLUMN IF EXISTS le_location_geom,
    DROP COLUMN IF EXISTS he_location_geom;
