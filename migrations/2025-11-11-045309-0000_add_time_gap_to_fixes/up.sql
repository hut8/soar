-- Add time_gap_seconds column to fixes table
-- This column stores the number of seconds elapsed since the previous fix within the same flight
-- NULL for the first fix in a flight or for fixes without a flight_id

ALTER TABLE fixes ADD COLUMN time_gap_seconds INTEGER;

-- Create index for efficient queries on time gaps
-- Useful for finding significant gaps or analyzing coverage patterns
CREATE INDEX idx_fixes_time_gap_seconds ON fixes (time_gap_seconds) WHERE time_gap_seconds IS NOT NULL;
