-- Create triggers to attach analytics trigger functions to the flights table
-- These triggers maintain real-time analytics aggregations

-- Trigger for daily flight analytics
CREATE TRIGGER trigger_flight_analytics_daily
    AFTER INSERT OR UPDATE OR DELETE ON flights
    FOR EACH ROW
    EXECUTE FUNCTION update_flight_analytics_daily();

-- Trigger for hourly flight analytics
CREATE TRIGGER trigger_flight_analytics_hourly
    AFTER INSERT OR UPDATE OR DELETE ON flights
    FOR EACH ROW
    EXECUTE FUNCTION update_flight_analytics_hourly();

-- Trigger for flight duration buckets
CREATE TRIGGER trigger_flight_duration_buckets
    AFTER INSERT OR UPDATE OR DELETE ON flights
    FOR EACH ROW
    EXECUTE FUNCTION update_flight_duration_buckets();

-- Trigger for device analytics
CREATE TRIGGER trigger_device_analytics
    AFTER INSERT OR UPDATE OR DELETE ON flights
    FOR EACH ROW
    EXECUTE FUNCTION update_device_analytics();

-- Trigger for club analytics (daily)
CREATE TRIGGER trigger_club_analytics_daily
    AFTER INSERT OR UPDATE OR DELETE ON flights
    FOR EACH ROW
    EXECUTE FUNCTION update_club_analytics_daily();

-- Trigger for airport analytics (daily)
CREATE TRIGGER trigger_airport_analytics_daily
    AFTER INSERT OR UPDATE OR DELETE ON flights
    FOR EACH ROW
    EXECUTE FUNCTION update_airport_analytics_daily();
