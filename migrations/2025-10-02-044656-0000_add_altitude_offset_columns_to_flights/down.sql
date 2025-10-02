-- Remove altitude offset columns from flights table

ALTER TABLE flights
DROP COLUMN IF EXISTS takeoff_altitude_offset_ft,
DROP COLUMN IF EXISTS landing_altitude_offset_ft;
