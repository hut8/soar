-- Remove analytics triggers from flights table

DROP TRIGGER IF EXISTS trigger_flight_analytics_daily ON flights;
DROP TRIGGER IF EXISTS trigger_flight_analytics_hourly ON flights;
DROP TRIGGER IF EXISTS trigger_flight_duration_buckets ON flights;
DROP TRIGGER IF EXISTS trigger_device_analytics ON flights;
DROP TRIGGER IF EXISTS trigger_club_analytics_daily ON flights;
DROP TRIGGER IF EXISTS trigger_airport_analytics_daily ON flights;
