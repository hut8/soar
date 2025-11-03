-- Rollback: Remove bounding box columns and index

DROP INDEX IF EXISTS idx_flights_bounding_box;

ALTER TABLE flights
    DROP COLUMN IF EXISTS min_latitude,
    DROP COLUMN IF EXISTS max_latitude,
    DROP COLUMN IF EXISTS min_longitude,
    DROP COLUMN IF EXISTS max_longitude;
