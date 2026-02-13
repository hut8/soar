-- Prevent multiple simultaneous active flights per aircraft.
-- An "active" flight is one with no landing_time and no timed_out_at.
-- This partial unique index ensures at most one such flight exists per aircraft_id.
CREATE UNIQUE INDEX CONCURRENTLY idx_flights_one_active_per_aircraft
ON flights (aircraft_id)
WHERE landing_time IS NULL AND timed_out_at IS NULL;
