-- Rollback timeout_phase column and enum type
DROP INDEX IF EXISTS idx_flights_timeout_phase;
ALTER TABLE flights DROP COLUMN timeout_phase;
DROP TYPE timeout_phase;
