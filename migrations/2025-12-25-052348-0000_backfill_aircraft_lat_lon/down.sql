-- Revert backfill by setting latitude/longitude to NULL
-- Note: This will cause location_geom to become NULL as well (it's a generated column)
UPDATE aircraft
SET
    latitude = NULL,
    longitude = NULL;
