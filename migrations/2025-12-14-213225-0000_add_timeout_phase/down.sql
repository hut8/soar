-- Rollback timeout_phase column
DROP INDEX IF EXISTS idx_flights_timeout_phase;
ALTER TABLE flights DROP COLUMN timeout_phase;
