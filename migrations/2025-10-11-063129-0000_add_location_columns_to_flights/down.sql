-- Drop indexes
DROP INDEX idx_flights_landing_location_id;
DROP INDEX idx_flights_takeoff_location_id;

-- Drop columns
ALTER TABLE flights DROP COLUMN landing_location_id;
ALTER TABLE flights DROP COLUMN takeoff_location_id;
