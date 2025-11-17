-- Clear backfilled analytics data
-- Note: Does not drop triggers or functions (those are managed by other migrations)

TRUNCATE flight_analytics_daily;
TRUNCATE flight_analytics_hourly;
TRUNCATE device_analytics;
TRUNCATE airport_analytics_daily;
TRUNCATE club_analytics_daily;

-- Reset flight_duration_buckets counts
UPDATE flight_duration_buckets SET flight_count = 0, updated_at = NOW();
