-- Revert the last_fix_at changes

-- Drop the index
DROP INDEX IF EXISTS idx_flights_last_fix_at;

-- Drop the column
ALTER TABLE flights DROP COLUMN last_fix_at;
