-- Remove total distance and maximum displacement columns from flights table

ALTER TABLE flights DROP COLUMN IF EXISTS total_distance_meters;
ALTER TABLE flights DROP COLUMN IF EXISTS maximum_displacement_meters;
