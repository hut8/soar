-- Drop deprecated tow_release_height_msl column
-- This field has been replaced by tow_release_altitude_msl_ft
ALTER TABLE flights DROP COLUMN tow_release_height_msl;
