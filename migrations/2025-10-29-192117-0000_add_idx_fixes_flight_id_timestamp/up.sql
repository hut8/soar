-- Add index on fixes for flight_id and timestamp
-- This index supports queries that need to fetch fixes for a specific flight
-- ordered by timestamp (e.g., retrieving a flight's track in chronological order)
--
-- NOTE: This is NOT CONCURRENTLY because Diesel migrations run in transactions
CREATE INDEX idx_fixes_flight_id_timestamp
ON fixes (flight_id, timestamp);
