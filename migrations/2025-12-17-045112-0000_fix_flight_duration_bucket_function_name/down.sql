-- Revert to the incorrect function name (this will break the trigger)
-- This down migration is provided for completeness, but rolling back this fix
-- will cause flights to fail to be created with the error:
-- "function get_flight_duration_bucket(integer) does not exist"

CREATE OR REPLACE FUNCTION public.update_flight_duration_buckets()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
    flight_duration INT;
    old_duration INT;
    bucket VARCHAR(20);
    old_bucket VARCHAR(20);
BEGIN
    -- OPTIMIZATION: Skip if UPDATE only changed non-analytics fields
    IF TG_OP = 'UPDATE' AND
       OLD.takeoff_time IS NOT DISTINCT FROM NEW.takeoff_time AND
       OLD.landing_time IS NOT DISTINCT FROM NEW.landing_time
    THEN
        RETURN NEW;  -- Skip analytics update
    END IF;

    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);
        bucket := get_flight_duration_bucket(flight_duration);

        UPDATE flight_duration_buckets SET
            flight_count = flight_count + 1,
            updated_at = NOW()
        WHERE bucket_name = bucket;

    -- Handle UPDATE (only if analytics-relevant fields changed, checked above)
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);
        old_bucket := get_flight_duration_bucket(old_duration);
        bucket := get_flight_duration_bucket(flight_duration);

        -- If bucket changed, remove from old and add to new
        IF old_bucket != bucket THEN
            UPDATE flight_duration_buckets SET
                flight_count = GREATEST(0, flight_count - 1),
                updated_at = NOW()
            WHERE bucket_name = old_bucket;

            UPDATE flight_duration_buckets SET
                flight_count = flight_count + 1,
                updated_at = NOW()
            WHERE bucket_name = bucket;
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        old_bucket := get_flight_duration_bucket(old_duration);

        UPDATE flight_duration_buckets SET
            flight_count = GREATEST(0, flight_count - 1),
            updated_at = NOW()
        WHERE bucket_name = old_bucket;
    END IF;

    RETURN NEW;
END;
$function$;
