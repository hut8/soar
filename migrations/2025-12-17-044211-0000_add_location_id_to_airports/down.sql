-- Rollback: Remove location_id from airports table
DROP INDEX IF EXISTS idx_airports_location_id;
ALTER TABLE airports DROP COLUMN IF EXISTS location_id;
