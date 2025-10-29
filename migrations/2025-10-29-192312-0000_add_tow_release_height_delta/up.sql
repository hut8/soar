-- Add tow release height delta column
-- This is the altitude gain during tow: (release altitude - takeoff altitude)
ALTER TABLE flights
ADD COLUMN tow_release_height_delta_ft INTEGER;

COMMENT ON COLUMN flights.tow_release_height_delta_ft IS 'Altitude gain during tow in feet (tow_release_altitude_msl_ft - towplane takeoff altitude MSL)';
