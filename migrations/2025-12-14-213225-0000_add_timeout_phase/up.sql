-- Add timeout_phase column to track flight state when timeout occurs
ALTER TABLE flights
ADD COLUMN timeout_phase TEXT
CHECK (timeout_phase IN ('climbing', 'cruising', 'descending', 'unknown'));

-- Add index for timeout phase queries
CREATE INDEX idx_flights_timeout_phase
ON flights(timeout_phase)
WHERE timeout_phase IS NOT NULL;
