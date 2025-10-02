-- Remove altitude offset columns from flights table

ALTER TABLE flights
DROP COLUMN takeoff_altitude_offset_ft,
DROP COLUMN landing_altitude_offset_ft;
