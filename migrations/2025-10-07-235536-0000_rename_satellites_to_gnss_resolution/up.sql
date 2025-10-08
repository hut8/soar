-- Rename satellite fields to GNSS resolution fields
-- These represent horizontal and vertical resolution, not satellite counts

ALTER TABLE fixes RENAME COLUMN satellites_used TO gnss_horizontal_resolution;
ALTER TABLE fixes RENAME COLUMN satellites_visible TO gnss_vertical_resolution;
