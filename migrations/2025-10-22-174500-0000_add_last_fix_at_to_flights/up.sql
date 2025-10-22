-- Add last_fix_at column to flights table to track when the most recent fix was received
-- This is used to determine if a flight should be timed out after 8 hours of no beacons
-- and to set the timed_out_at timestamp to the time of the last fix

-- Add the column with a default value so existing rows can be populated
-- We'll use created_at as the default for existing flights
ALTER TABLE flights ADD COLUMN last_fix_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Create an index on last_fix_at for efficient timeout queries
CREATE INDEX idx_flights_last_fix_at ON flights(last_fix_at);

-- Remove the default now that existing rows are populated
ALTER TABLE flights ALTER COLUMN last_fix_at DROP DEFAULT;
