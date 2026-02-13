-- Clean up duplicate active flights before creating unique index.
-- For each aircraft_id, keep the most recent active flight (by last_fix_at) as active
-- and mark all older concurrent active flights as completed (landing_time = last_fix_at).
-- This is required because existing data may violate the unique constraint.
WITH ranked AS (
    SELECT id, aircraft_id, last_fix_at,
           ROW_NUMBER() OVER (PARTITION BY aircraft_id ORDER BY last_fix_at DESC) AS rn
    FROM flights
    WHERE landing_time IS NULL
      AND timed_out_at IS NULL
      AND aircraft_id IS NOT NULL
)
UPDATE flights f
SET landing_time = f.last_fix_at,
    updated_at = NOW()
FROM ranked r
WHERE f.id = r.id
  AND r.rn > 1;

-- Prevent multiple simultaneous active flights per aircraft.
-- An "active" flight is one with no landing_time and no timed_out_at.
-- This partial unique index ensures at most one such flight exists per aircraft_id.
CREATE UNIQUE INDEX CONCURRENTLY idx_flights_one_active_per_aircraft
ON flights (aircraft_id)
WHERE landing_time IS NULL AND timed_out_at IS NULL;
