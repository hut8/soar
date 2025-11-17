-- Remove analytics triggers and functions from flights table

-- Drop triggers
DROP TRIGGER IF EXISTS trigger_flight_analytics_daily ON flights;
DROP TRIGGER IF EXISTS trigger_flight_analytics_hourly ON flights;
DROP TRIGGER IF EXISTS trigger_flight_duration_buckets ON flights;
DROP TRIGGER IF EXISTS trigger_device_analytics ON flights;
DROP TRIGGER IF EXISTS trigger_club_analytics_daily ON flights;
DROP TRIGGER IF EXISTS trigger_airport_analytics_daily ON flights;

-- Drop trigger functions
DROP FUNCTION IF EXISTS update_airport_analytics_daily();
DROP FUNCTION IF EXISTS update_club_analytics_daily();
DROP FUNCTION IF EXISTS update_device_analytics();
DROP FUNCTION IF EXISTS update_flight_analytics_daily();
DROP FUNCTION IF EXISTS update_flight_analytics_hourly();
DROP FUNCTION IF EXISTS update_flight_duration_buckets();
