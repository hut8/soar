-- Remove the old latitude, longitude, and location columns from receivers
-- These have been replaced by the location_id foreign key to the locations table

-- Drop the spatial index on the generated location column
DROP INDEX IF EXISTS idx_receivers_location;

-- Remove the generated location column first (depends on latitude/longitude)
ALTER TABLE receivers
DROP COLUMN IF EXISTS location;

-- Remove the latitude and longitude columns
ALTER TABLE receivers
DROP COLUMN IF EXISTS latitude,
DROP COLUMN IF EXISTS longitude;
