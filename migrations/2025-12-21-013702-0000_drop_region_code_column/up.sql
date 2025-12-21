-- Drop region_code column from locations table
-- This is legacy FAA data that's derived from state and not independently meaningful
ALTER TABLE locations DROP COLUMN region_code;
