-- Drop redundant location_geog columns from aircraft and user_fixes tables
--
-- The location_geog (geography) columns were added alongside location_geom (geometry)
-- but are never actually queried. All spatial queries use the geometry columns
-- with the && operator for fast bounding box operations.
--
-- Dropping these columns saves storage space and index maintenance overhead.

-- Drop indexes first
DROP INDEX IF EXISTS idx_aircraft_location_geog;
DROP INDEX IF EXISTS idx_user_fixes_location_geog;

-- Drop the redundant geography columns
ALTER TABLE aircraft DROP COLUMN IF EXISTS location_geog;
ALTER TABLE user_fixes DROP COLUMN IF EXISTS location_geog;
