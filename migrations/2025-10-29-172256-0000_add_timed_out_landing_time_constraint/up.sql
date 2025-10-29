-- Delete invalid flights where both timed_out_at and landing_time are set
-- These represent invalid state and must be removed before adding the constraint
DELETE FROM flights
WHERE timed_out_at IS NOT NULL AND landing_time IS NOT NULL;

-- Add check constraint to ensure timed_out_at and landing_time are mutually exclusive
-- Both can be NULL, but both cannot be NOT NULL at the same time
ALTER TABLE flights
ADD CONSTRAINT check_timed_out_or_landed
CHECK (
    (timed_out_at IS NULL OR landing_time IS NULL)
);
