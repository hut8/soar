-- Revert towing detection fields

DROP INDEX IF EXISTS idx_flights_towed_by_flight;
DROP INDEX IF EXISTS idx_flights_towed_by_device;

ALTER TABLE flights DROP COLUMN IF EXISTS tow_release_time;
ALTER TABLE flights DROP COLUMN IF EXISTS tow_release_altitude_msl_ft;
ALTER TABLE flights DROP COLUMN IF EXISTS towed_by_flight_id;
ALTER TABLE flights DROP COLUMN IF EXISTS towed_by_device_id;
