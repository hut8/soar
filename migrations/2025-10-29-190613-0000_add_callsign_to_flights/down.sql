-- Revert callsign column from flights table
DROP INDEX IF EXISTS idx_flights_callsign;
ALTER TABLE flights DROP COLUMN callsign;
