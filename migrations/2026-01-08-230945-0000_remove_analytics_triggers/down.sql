-- Restore analytics triggers on flights table
-- WARNING: These triggers cause significant write amplification during high-throughput processing.
-- Only restore if analytics are needed and you've addressed the performance implications.

CREATE TRIGGER trigger_flight_analytics_daily AFTER INSERT OR DELETE OR UPDATE ON public.flights FOR EACH ROW EXECUTE FUNCTION update_flight_analytics_daily();
CREATE TRIGGER trigger_flight_analytics_hourly AFTER INSERT OR DELETE OR UPDATE ON public.flights FOR EACH ROW EXECUTE FUNCTION update_flight_analytics_hourly();
CREATE TRIGGER trigger_flight_duration_buckets AFTER INSERT OR DELETE OR UPDATE ON public.flights FOR EACH ROW EXECUTE FUNCTION update_flight_duration_buckets();
CREATE TRIGGER trigger_device_analytics AFTER INSERT OR DELETE OR UPDATE ON public.flights FOR EACH ROW EXECUTE FUNCTION update_device_analytics();
CREATE TRIGGER trigger_club_analytics_daily AFTER INSERT OR DELETE OR UPDATE ON public.flights FOR EACH ROW EXECUTE FUNCTION update_club_analytics_daily();
CREATE TRIGGER trigger_airport_analytics_daily AFTER INSERT OR DELETE OR UPDATE ON public.flights FOR EACH ROW EXECUTE FUNCTION update_airport_analytics_daily();
