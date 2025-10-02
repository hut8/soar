-- Add altitude offset columns to flights table for tracking the difference
-- between reported altitude and known ground elevation at takeoff/landing

ALTER TABLE flights
ADD COLUMN IF NOT EXISTS takeoff_altitude_offset_ft INTEGER,
ADD COLUMN IF NOT EXISTS landing_altitude_offset_ft INTEGER;

COMMENT ON COLUMN flights.takeoff_altitude_offset_ft IS 'Difference in feet between reported altitude and known ground elevation at takeoff location';
COMMENT ON COLUMN flights.landing_altitude_offset_ft IS 'Difference in feet between reported altitude and known ground elevation at landing location';
