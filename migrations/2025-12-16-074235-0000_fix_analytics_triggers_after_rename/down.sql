-- Revert analytics trigger to use old device_analytics table name
-- Note: This down migration assumes the device_analytics table exists
-- (i.e., that the 2025-12-11-165518 rename migration was also reverted)

CREATE OR REPLACE FUNCTION public.update_device_analytics()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
    old_device UUID;
    new_device UUID;
    flight_duration INT;
    old_duration INT;
BEGIN
    -- OPTIMIZATION: Skip if UPDATE only changed non-analytics fields
    IF TG_OP = 'UPDATE' AND
       OLD.takeoff_time IS NOT DISTINCT FROM NEW.takeoff_time AND
       OLD.landing_time IS NOT DISTINCT FROM NEW.landing_time AND
       OLD.aircraft_id IS NOT DISTINCT FROM NEW.aircraft_id AND
       OLD.total_distance_meters IS NOT DISTINCT FROM NEW.total_distance_meters
    THEN
        RETURN NEW;  -- Skip analytics update
    END IF;

    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        new_device := NEW.aircraft_id;
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        INSERT INTO device_analytics (device_id, registration, aircraft_model, flight_count_total, last_flight_at, total_distance_meters)
        SELECT
            NEW.aircraft_id,
            d.registration,
            d.aircraft_model,
            1,
            NEW.takeoff_time,
            COALESCE(NEW.total_distance_meters, 0)
        FROM devices d
        WHERE d.id = NEW.aircraft_id
        ON CONFLICT (device_id) DO UPDATE SET
            flight_count_total = device_analytics.flight_count_total + 1,
            last_flight_at = GREATEST(device_analytics.last_flight_at, NEW.takeoff_time),
            total_distance_meters = device_analytics.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
            avg_flight_duration_seconds = CASE WHEN device_analytics.flight_count_total + 1 > 0
                THEN ((device_analytics.avg_flight_duration_seconds * device_analytics.flight_count_total) + flight_duration) / (device_analytics.flight_count_total + 1)
                ELSE 0 END,
            updated_at = NOW();

    -- Handle UPDATE (only if analytics-relevant fields changed, checked above)
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_device := OLD.aircraft_id;
        new_device := NEW.aircraft_id;
        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        -- If device changed, update both
        IF old_device != new_device THEN
            -- Remove from old device
            UPDATE device_analytics SET
                flight_count_total = GREATEST(0, flight_count_total - 1),
                total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0)),
                updated_at = NOW()
            WHERE device_id = old_device;

            -- Add to new device
            INSERT INTO device_analytics (device_id, registration, aircraft_model, flight_count_total, last_flight_at, total_distance_meters)
            SELECT
                NEW.aircraft_id,
                d.registration,
                d.aircraft_model,
                1,
                NEW.takeoff_time,
                COALESCE(NEW.total_distance_meters, 0)
            FROM devices d
            WHERE d.id = NEW.aircraft_id
            ON CONFLICT (device_id) DO UPDATE SET
                flight_count_total = device_analytics.flight_count_total + 1,
                last_flight_at = GREATEST(device_analytics.last_flight_at, NEW.takeoff_time),
                total_distance_meters = device_analytics.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
                avg_flight_duration_seconds = CASE WHEN device_analytics.flight_count_total + 1 > 0
                    THEN ((device_analytics.avg_flight_duration_seconds * device_analytics.flight_count_total) + flight_duration) / (device_analytics.flight_count_total + 1)
                    ELSE 0 END,
                updated_at = NOW();
        ELSE
            -- Same device, just update distance if changed
            IF OLD.total_distance_meters IS DISTINCT FROM NEW.total_distance_meters THEN
                UPDATE device_analytics SET
                    total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0) + COALESCE(NEW.total_distance_meters, 0)),
                    updated_at = NOW()
                WHERE device_id = new_device;
            END IF;
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        old_device := OLD.aircraft_id;

        UPDATE device_analytics SET
            flight_count_total = GREATEST(0, flight_count_total - 1),
            total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0)),
            updated_at = NOW()
        WHERE device_id = old_device;
    END IF;

    RETURN NEW;
END;
$function$;
