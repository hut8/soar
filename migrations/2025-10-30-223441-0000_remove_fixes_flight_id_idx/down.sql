-- This file should undo anything in `up.sql`
-- Recreate the fixes_flight_id_idx index
CREATE INDEX fixes_flight_id_idx ON fixes (flight_id);
