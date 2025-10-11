-- Add timed_out_at column to flights table to track when a flight was timed out
-- due to no beacon being received for 5+ minutes
ALTER TABLE flights ADD COLUMN timed_out_at TIMESTAMPTZ;
