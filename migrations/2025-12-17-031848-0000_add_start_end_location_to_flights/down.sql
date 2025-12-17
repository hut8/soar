-- Rollback: Remove start and end location columns from flights table

DROP INDEX IF EXISTS idx_flights_start_location_id;
DROP INDEX IF EXISTS idx_flights_end_location_id;

ALTER TABLE flights DROP COLUMN IF EXISTS start_location_id;
ALTER TABLE flights DROP COLUMN IF EXISTS end_location_id;
