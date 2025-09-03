-- Drop the runways table and associated objects
DROP TRIGGER IF EXISTS update_runways_updated_at ON runways;
DROP TRIGGER IF EXISTS update_runway_locations_trigger ON runways;
DROP FUNCTION IF EXISTS update_runway_locations();
DROP TABLE IF EXISTS runways;
