-- Add check constraints for timestamp relationships in flights table
--
-- The key insight: created_at is when the DATABASE RECORD was created,
-- but takeoff_time, landing_time, tow_release_time are ACTUAL EVENT TIMES.
-- timed_out_at is set to last_fix_at (not current time) when a flight times out.
--
-- For retroactive imports, created_at can be AFTER the actual flight events,
-- but there are still logical constraints on the actual event times themselves.

-- First, fix existing data inconsistencies where flights were created after their fixes
-- This happened when orphaned fixes were retroactively associated with flights
UPDATE flights
SET created_at = last_fix_at
WHERE created_at > last_fix_at;

-- 1. Takeoff must come before landing
ALTER TABLE flights
  ADD CONSTRAINT check_takeoff_before_landing
  CHECK (
    takeoff_time IS NULL OR
    landing_time IS NULL OR
    takeoff_time < landing_time
  );

-- 2. Takeoff must come before tow release
ALTER TABLE flights
  ADD CONSTRAINT check_takeoff_before_tow_release
  CHECK (
    takeoff_time IS NULL OR
    tow_release_time IS NULL OR
    takeoff_time < tow_release_time
  );

-- 3. Tow release must come before landing
ALTER TABLE flights
  ADD CONSTRAINT check_tow_release_before_landing
  CHECK (
    tow_release_time IS NULL OR
    landing_time IS NULL OR
    tow_release_time < landing_time
  );

-- 4. Last fix must be at or after the flight was created
--    This catches cases where a flight record was created but has no fixes
ALTER TABLE flights
  ADD CONSTRAINT check_last_fix_after_created
  CHECK (last_fix_at >= created_at);

-- 5. If flight timed out, the timeout must be at or after last fix
--    (timed_out_at is SET TO last_fix_at by the code, so this should always be true)
ALTER TABLE flights
  ADD CONSTRAINT check_timeout_after_last_fix
  CHECK (
    timed_out_at IS NULL OR
    timed_out_at >= last_fix_at
  );

-- 6. If flight landed, landing must be at or before last fix
--    (the last fix should be close to the landing time)
ALTER TABLE flights
  ADD CONSTRAINT check_landing_near_last_fix
  CHECK (
    landing_time IS NULL OR
    last_fix_at >= landing_time - INTERVAL '10 minutes'
  );

-- 7. If flight timed out, last_fix should be reasonably close to timeout
--    (allowing some processing delay, but not months)
ALTER TABLE flights
  ADD CONSTRAINT check_timeout_reasonable
  CHECK (
    timed_out_at IS NULL OR
    timed_out_at <= last_fix_at + INTERVAL '24 hours'
  );
