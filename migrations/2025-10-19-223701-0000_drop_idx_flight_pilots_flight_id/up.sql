-- Drop the idx_flight_pilots_flight_id index
-- This index is redundant because the UNIQUE constraint on (flight_id, pilot_id)
-- already creates an index that can efficiently handle flight_id lookups
DROP INDEX IF EXISTS idx_flight_pilots_flight_id;
