-- Drop indexes
DROP INDEX IF EXISTS idx_aircraft_lat_lon;
DROP INDEX IF EXISTS idx_aircraft_location_geog;
DROP INDEX IF EXISTS idx_aircraft_location_geom;

-- Drop cached location columns from aircraft table
-- Note: Generated columns (location_geom, location_geog) are automatically dropped with their source columns
ALTER TABLE aircraft
    DROP COLUMN IF EXISTS location_geog,
    DROP COLUMN IF EXISTS location_geom,
    DROP COLUMN IF EXISTS longitude,
    DROP COLUMN IF EXISTS latitude;
