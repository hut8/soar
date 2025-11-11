-- Reverse the time_gap_seconds column addition

DROP INDEX IF EXISTS idx_fixes_time_gap_seconds;
ALTER TABLE fixes DROP COLUMN time_gap_seconds;
