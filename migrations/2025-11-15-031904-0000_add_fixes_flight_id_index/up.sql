-- Create index on fixes.flight_id for faster flight-related queries
-- This improves performance when looking up all fixes for a specific flight
CREATE INDEX IF NOT EXISTS fixes_flight_id_idx ON fixes (flight_id);
