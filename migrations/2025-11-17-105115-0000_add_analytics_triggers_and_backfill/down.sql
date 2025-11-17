-- Rollback analytics triggers and clear backfilled data

BEGIN;

-- Drop all triggers
DROP TRIGGER IF EXISTS trigger_update_airport_analytics_daily ON flights;
DROP TRIGGER IF EXISTS trigger_update_club_analytics_daily ON flights;
DROP TRIGGER IF EXISTS trigger_update_device_analytics ON flights;
DROP TRIGGER IF EXISTS trigger_update_flight_analytics_daily ON flights;
DROP TRIGGER IF EXISTS trigger_update_flight_analytics_hourly ON flights;
DROP TRIGGER IF EXISTS trigger_update_flight_duration_buckets ON flights;

-- Drop all trigger functions
DROP FUNCTION IF EXISTS update_airport_analytics_daily();
DROP FUNCTION IF EXISTS update_club_analytics_daily();
DROP FUNCTION IF EXISTS update_device_analytics();
DROP FUNCTION IF EXISTS update_flight_analytics_daily();
DROP FUNCTION IF EXISTS update_flight_analytics_hourly();
DROP FUNCTION IF EXISTS update_flight_duration_buckets();

-- Clear backfilled data (keep tables for next migration attempt)
TRUNCATE flight_analytics_daily;
TRUNCATE flight_analytics_hourly;
TRUNCATE device_analytics;
TRUNCATE airport_analytics_daily;
TRUNCATE club_analytics_daily;

-- Reset flight_duration_buckets counts
UPDATE flight_duration_buckets SET flight_count = 0, updated_at = NOW();

COMMIT;
