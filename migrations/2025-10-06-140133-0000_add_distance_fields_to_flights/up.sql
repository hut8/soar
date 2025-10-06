-- Add total distance and maximum displacement columns to flights table
-- These values are computed upon landing and stored in meters

ALTER TABLE flights ADD COLUMN total_distance_meters DOUBLE PRECISION;
ALTER TABLE flights ADD COLUMN maximum_displacement_meters DOUBLE PRECISION;

-- Add comments to explain the columns
COMMENT ON COLUMN flights.total_distance_meters IS 'Total distance flown during the flight in meters, computed from consecutive fixes';
COMMENT ON COLUMN flights.maximum_displacement_meters IS 'Maximum displacement from departure airport in meters (only for local flights where departure == arrival)';
