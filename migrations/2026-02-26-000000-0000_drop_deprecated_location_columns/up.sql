-- Drop deprecated takeoff_location_id and landing_location_id columns from flights table.
-- These columns were replaced by start_location_id and end_location_id in December 2025
-- and have not been populated since then.

-- Drop indexes concurrently to avoid blocking queries
DROP INDEX CONCURRENTLY IF EXISTS idx_flights_takeoff_location_id;
DROP INDEX CONCURRENTLY IF EXISTS idx_flights_landing_location_id;

-- Drop columns from flights table
ALTER TABLE flights DROP COLUMN takeoff_location_id;
ALTER TABLE flights DROP COLUMN landing_location_id;

-- Drop columns from spurious_flights table (mirrors flights schema)
ALTER TABLE spurious_flights DROP COLUMN takeoff_location_id;
ALTER TABLE spurious_flights DROP COLUMN landing_location_id;
