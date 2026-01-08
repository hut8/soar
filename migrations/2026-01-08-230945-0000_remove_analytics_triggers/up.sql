-- Remove analytics triggers from flights table
-- These triggers cause significant write amplification (up to 46 additional SQL statements per flight operation)
-- and create WAL bottlenecks during high-throughput processing.
-- See docs/flight-analytics.md for details on what each trigger did.

-- Drop the triggers (the functions can remain for potential future use or manual analytics)
DROP TRIGGER IF EXISTS trigger_flight_analytics_daily ON flights;
DROP TRIGGER IF EXISTS trigger_flight_analytics_hourly ON flights;
DROP TRIGGER IF EXISTS trigger_flight_duration_buckets ON flights;
DROP TRIGGER IF EXISTS trigger_device_analytics ON flights;
DROP TRIGGER IF EXISTS trigger_club_analytics_daily ON flights;
DROP TRIGGER IF EXISTS trigger_airport_analytics_daily ON flights;
