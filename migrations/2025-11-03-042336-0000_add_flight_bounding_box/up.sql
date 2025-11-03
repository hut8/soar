-- Add bounding box columns to flights table
-- These will be populated when flights are completed (landed or timed out)
-- NULL values indicate flight is still active or bbox hasn't been calculated yet

ALTER TABLE flights
    ADD COLUMN min_latitude DOUBLE PRECISION,
    ADD COLUMN max_latitude DOUBLE PRECISION,
    ADD COLUMN min_longitude DOUBLE PRECISION,
    ADD COLUMN max_longitude DOUBLE PRECISION;

-- Create index for bounding box overlap queries
-- This enables fast spatial queries without joining to the fixes table
CREATE INDEX idx_flights_bounding_box ON flights (
    min_latitude,
    max_latitude,
    min_longitude,
    max_longitude
) WHERE min_latitude IS NOT NULL;

-- Note: Existing flights will have NULL values
-- Run the backfill query manually to populate historical data
