-- Remove the check constraint
ALTER TABLE flights
DROP CONSTRAINT check_timed_out_or_landed;
