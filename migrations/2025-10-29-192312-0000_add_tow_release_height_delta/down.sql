-- Remove tow release height delta column
ALTER TABLE flights
DROP COLUMN tow_release_height_delta_ft;
