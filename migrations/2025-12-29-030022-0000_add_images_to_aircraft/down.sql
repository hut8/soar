-- Remove images column and index
DROP INDEX IF EXISTS idx_aircraft_images;

ALTER TABLE aircraft
DROP COLUMN IF EXISTS images;
