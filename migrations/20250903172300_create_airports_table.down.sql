-- Drop the airports table and associated objects
DROP TRIGGER IF EXISTS update_airports_updated_at ON airports;
DROP TRIGGER IF EXISTS update_airport_location_trigger ON airports;
DROP FUNCTION IF EXISTS update_airport_location();
DROP TABLE IF EXISTS airports;
