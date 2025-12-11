-- Remove check constraints for timestamp relationships in flights table

ALTER TABLE flights DROP CONSTRAINT IF EXISTS check_timeout_reasonable;
ALTER TABLE flights DROP CONSTRAINT IF EXISTS check_landing_near_last_fix;
ALTER TABLE flights DROP CONSTRAINT IF EXISTS check_timeout_after_last_fix;
ALTER TABLE flights DROP CONSTRAINT IF EXISTS check_last_fix_after_created;
ALTER TABLE flights DROP CONSTRAINT IF EXISTS check_tow_release_before_landing;
ALTER TABLE flights DROP CONSTRAINT IF EXISTS check_takeoff_before_tow_release;
ALTER TABLE flights DROP CONSTRAINT IF EXISTS check_takeoff_before_landing;
