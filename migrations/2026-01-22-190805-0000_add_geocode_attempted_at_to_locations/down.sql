DROP INDEX IF EXISTS idx_locations_needs_geocoding;
ALTER TABLE locations DROP COLUMN IF EXISTS geocode_attempted_at;
