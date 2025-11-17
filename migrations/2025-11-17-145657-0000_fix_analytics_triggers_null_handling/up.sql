-- Fix analytics trigger functions to handle NULL takeoff_time
-- This prevents "null value in column 'date' violates not-null constraint" errors
-- when flights are updated without a takeoff_time

-- Recreate all trigger functions with NULL checks
CREATE OR REPLACE FUNCTION public.update_airport_analytics_daily()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
    affected_date DATE;
    old_date DATE;
BEGIN
    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        affected_date := DATE(NEW.takeoff_time);

        -- Update departure airport
        IF NEW.departure_airport_id IS NOT NULL THEN
            INSERT INTO airport_analytics_daily (airport_id, date, airport_ident, airport_name, departure_count, arrival_count)
            SELECT
                NEW.departure_airport_id,
                affected_date,
                a.ident,
                a.name,
                1,
                0
            FROM airports a
            WHERE a.id = NEW.departure_airport_id
            ON CONFLICT (airport_id, date) DO UPDATE SET
                departure_count = airport_analytics_daily.departure_count + 1,
                updated_at = NOW();
        END IF;

        -- Update arrival airport
        IF NEW.arrival_airport_id IS NOT NULL THEN
            INSERT INTO airport_analytics_daily (airport_id, date, airport_ident, airport_name, departure_count, arrival_count)
            SELECT
                NEW.arrival_airport_id,
                affected_date,
                a.ident,
                a.name,
                0,
                1
            FROM airports a
            WHERE a.id = NEW.arrival_airport_id
            ON CONFLICT (airport_id, date) DO UPDATE SET
                arrival_count = airport_analytics_daily.arrival_count + 1,
                updated_at = NOW();
        END IF;

    -- Handle UPDATE
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_date := DATE(OLD.takeoff_time);
        affected_date := DATE(NEW.takeoff_time);

        -- Remove old departure
        IF OLD.departure_airport_id IS NOT NULL THEN
            UPDATE airport_analytics_daily SET
                departure_count = GREATEST(0, departure_count - 1),
                updated_at = NOW()
            WHERE airport_id = OLD.departure_airport_id AND date = old_date;
        END IF;

        -- Remove old arrival
        IF OLD.arrival_airport_id IS NOT NULL THEN
            UPDATE airport_analytics_daily SET
                arrival_count = GREATEST(0, arrival_count - 1),
                updated_at = NOW()
            WHERE airport_id = OLD.arrival_airport_id AND date = old_date;
        END IF;

        -- Add new departure
        IF NEW.departure_airport_id IS NOT NULL THEN
            INSERT INTO airport_analytics_daily (airport_id, date, airport_ident, airport_name, departure_count, arrival_count)
            SELECT
                NEW.departure_airport_id,
                affected_date,
                a.ident,
                a.name,
                1,
                0
            FROM airports a
            WHERE a.id = NEW.departure_airport_id
            ON CONFLICT (airport_id, date) DO UPDATE SET
                departure_count = airport_analytics_daily.departure_count + 1,
                updated_at = NOW();
        END IF;

        -- Add new arrival
        IF NEW.arrival_airport_id IS NOT NULL THEN
            INSERT INTO airport_analytics_daily (airport_id, date, airport_ident, airport_name, departure_count, arrival_count)
            SELECT
                NEW.arrival_airport_id,
                affected_date,
                a.ident,
                a.name,
                0,
                1
            FROM airports a
            WHERE a.id = NEW.arrival_airport_id
            ON CONFLICT (airport_id, date) DO UPDATE SET
                arrival_count = airport_analytics_daily.arrival_count + 1,
                updated_at = NOW();
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        old_date := DATE(OLD.takeoff_time);

        -- Remove departure
        IF OLD.departure_airport_id IS NOT NULL THEN
            UPDATE airport_analytics_daily SET
                departure_count = GREATEST(0, departure_count - 1),
                updated_at = NOW()
            WHERE airport_id = OLD.departure_airport_id AND date = old_date;
        END IF;

        -- Remove arrival
        IF OLD.arrival_airport_id IS NOT NULL THEN
            UPDATE airport_analytics_daily SET
                arrival_count = GREATEST(0, arrival_count - 1),
                updated_at = NOW()
            WHERE airport_id = OLD.arrival_airport_id AND date = old_date;
        END IF;
    END IF;

    RETURN NEW;
END;
$function$;
CREATE OR REPLACE FUNCTION public.update_club_analytics_daily()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
    affected_club UUID;
    affected_date DATE;
    old_club UUID;
    old_date DATE;
    flight_duration INT;
    old_duration INT;
BEGIN
    -- Handle INSERT
    IF TG_OP = 'INSERT' AND NEW.club_id IS NOT NULL THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        affected_club := NEW.club_id;
        affected_date := DATE(NEW.takeoff_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        INSERT INTO club_analytics_daily (club_id, date, club_name, flight_count, total_airtime_seconds, tow_count)
        SELECT
            NEW.club_id,
            affected_date,
            c.name,
            1,
            flight_duration,
            CASE WHEN NEW.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END
        FROM clubs c
        WHERE c.id = NEW.club_id
        ON CONFLICT (club_id, date) DO UPDATE SET
            flight_count = club_analytics_daily.flight_count + 1,
            total_airtime_seconds = club_analytics_daily.total_airtime_seconds + flight_duration,
            tow_count = club_analytics_daily.tow_count + CASE WHEN NEW.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END,
            updated_at = NOW();

    -- Handle UPDATE
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_club := OLD.club_id;
        old_date := DATE(OLD.takeoff_time);
        affected_club := NEW.club_id;
        affected_date := DATE(NEW.takeoff_time);
        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        -- Remove from old club/date
        IF old_club IS NOT NULL THEN
            UPDATE club_analytics_daily SET
                flight_count = GREATEST(0, flight_count - 1),
                total_airtime_seconds = GREATEST(0, total_airtime_seconds - old_duration),
                tow_count = GREATEST(0, tow_count - CASE WHEN OLD.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END),
                updated_at = NOW()
            WHERE club_id = old_club AND date = old_date;
        END IF;

        -- Add to new club/date
        IF affected_club IS NOT NULL THEN
            INSERT INTO club_analytics_daily (club_id, date, club_name, flight_count, total_airtime_seconds, tow_count)
            SELECT
                NEW.club_id,
                affected_date,
                c.name,
                1,
                flight_duration,
                CASE WHEN NEW.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END
            FROM clubs c
            WHERE c.id = NEW.club_id
            ON CONFLICT (club_id, date) DO UPDATE SET
                flight_count = club_analytics_daily.flight_count + 1,
                total_airtime_seconds = club_analytics_daily.total_airtime_seconds + flight_duration,
                tow_count = club_analytics_daily.tow_count + CASE WHEN NEW.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END,
                updated_at = NOW();
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' AND OLD.club_id IS NOT NULL THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        old_club := OLD.club_id;
        old_date := DATE(OLD.takeoff_time);
        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);

        UPDATE club_analytics_daily SET
            flight_count = GREATEST(0, flight_count - 1),
            total_airtime_seconds = GREATEST(0, total_airtime_seconds - old_duration),
            tow_count = GREATEST(0, tow_count - CASE WHEN OLD.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END),
            updated_at = NOW()
        WHERE club_id = old_club AND date = old_date;
    END IF;

    RETURN NEW;
END;
$function$;
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
    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        new_device := NEW.device_id;
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        INSERT INTO device_analytics (device_id, registration, aircraft_model, flight_count_total, last_flight_at, total_distance_meters)
        SELECT
            NEW.device_id,
            d.registration,
            d.aircraft_model,
            1,
            NEW.takeoff_time,
            COALESCE(NEW.total_distance_meters, 0)
        FROM devices d
        WHERE d.id = NEW.device_id
        ON CONFLICT (device_id) DO UPDATE SET
            flight_count_total = device_analytics.flight_count_total + 1,
            last_flight_at = GREATEST(device_analytics.last_flight_at, NEW.takeoff_time),
            total_distance_meters = device_analytics.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
            avg_flight_duration_seconds = CASE WHEN device_analytics.flight_count_total + 1 > 0
                THEN ((device_analytics.avg_flight_duration_seconds * device_analytics.flight_count_total) + flight_duration) / (device_analytics.flight_count_total + 1)
                ELSE 0 END,
            updated_at = NOW();

    -- Handle UPDATE
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_device := OLD.device_id;
        new_device := NEW.device_id;
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
                NEW.device_id,
                d.registration,
                d.aircraft_model,
                1,
                NEW.takeoff_time,
                COALESCE(NEW.total_distance_meters, 0)
            FROM devices d
            WHERE d.id = NEW.device_id
            ON CONFLICT (device_id) DO UPDATE SET
                flight_count_total = device_analytics.flight_count_total + 1,
                last_flight_at = GREATEST(device_analytics.last_flight_at, NEW.takeoff_time),
                total_distance_meters = device_analytics.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
                updated_at = NOW();
        ELSE
            -- Same device, just update the difference
            UPDATE device_analytics SET
                total_distance_meters = total_distance_meters - COALESCE(OLD.total_distance_meters, 0) + COALESCE(NEW.total_distance_meters, 0),
                last_flight_at = GREATEST(last_flight_at, NEW.takeoff_time),
                updated_at = NOW()
            WHERE device_id = new_device;
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        old_device := OLD.device_id;
        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);

        UPDATE device_analytics SET
            flight_count_total = GREATEST(0, flight_count_total - 1),
            total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0)),
            avg_flight_duration_seconds = CASE WHEN flight_count_total - 1 > 0
                THEN ((avg_flight_duration_seconds * flight_count_total) - old_duration) / (flight_count_total - 1)
                ELSE 0 END,
            updated_at = NOW()
        WHERE device_id = old_device;
    END IF;

    RETURN NEW;
END;
$function$;
CREATE OR REPLACE FUNCTION public.update_flight_analytics_daily()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
    affected_date DATE;
    old_date DATE;
    flight_duration INT;
    old_duration INT;
BEGIN
    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        affected_date := DATE(NEW.takeoff_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        INSERT INTO flight_analytics_daily (date, flight_count, total_duration_seconds, total_distance_meters, tow_flight_count, cross_country_count)
        VALUES (
            affected_date,
            1,
            flight_duration,
            COALESCE(NEW.total_distance_meters, 0),
            CASE WHEN NEW.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END,
            CASE WHEN NEW.departure_airport_id IS DISTINCT FROM NEW.arrival_airport_id THEN 1 ELSE 0 END
        )
        ON CONFLICT (date) DO UPDATE SET
            flight_count = flight_analytics_daily.flight_count + 1,
            total_duration_seconds = flight_analytics_daily.total_duration_seconds + flight_duration,
            total_distance_meters = flight_analytics_daily.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
            tow_flight_count = flight_analytics_daily.tow_flight_count + CASE WHEN NEW.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END,
            cross_country_count = flight_analytics_daily.cross_country_count + CASE WHEN NEW.departure_airport_id IS DISTINCT FROM NEW.arrival_airport_id THEN 1 ELSE 0 END,
            avg_duration_seconds = CASE WHEN flight_analytics_daily.flight_count + 1 > 0
                THEN (flight_analytics_daily.total_duration_seconds + flight_duration) / (flight_analytics_daily.flight_count + 1)
                ELSE 0 END,
            updated_at = NOW();

    -- Handle UPDATE
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_date := DATE(OLD.takeoff_time);
        affected_date := DATE(NEW.takeoff_time);
        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        -- Remove old values
        UPDATE flight_analytics_daily SET
            flight_count = GREATEST(0, flight_count - 1),
            total_duration_seconds = GREATEST(0, total_duration_seconds - old_duration),
            total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0)),
            tow_flight_count = GREATEST(0, tow_flight_count - CASE WHEN OLD.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END),
            cross_country_count = GREATEST(0, cross_country_count - CASE WHEN OLD.departure_airport_id IS DISTINCT FROM OLD.arrival_airport_id THEN 1 ELSE 0 END),
            avg_duration_seconds = CASE WHEN flight_count - 1 > 0
                THEN (total_duration_seconds - old_duration) / (flight_count - 1)
                ELSE 0 END,
            updated_at = NOW()
        WHERE date = old_date;

        -- Add new values
        INSERT INTO flight_analytics_daily (date, flight_count, total_duration_seconds, total_distance_meters, tow_flight_count, cross_country_count)
        VALUES (
            affected_date,
            1,
            flight_duration,
            COALESCE(NEW.total_distance_meters, 0),
            CASE WHEN NEW.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END,
            CASE WHEN NEW.departure_airport_id IS DISTINCT FROM NEW.arrival_airport_id THEN 1 ELSE 0 END
        )
        ON CONFLICT (date) DO UPDATE SET
            flight_count = flight_analytics_daily.flight_count + 1,
            total_duration_seconds = flight_analytics_daily.total_duration_seconds + flight_duration,
            total_distance_meters = flight_analytics_daily.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
            tow_flight_count = flight_analytics_daily.tow_flight_count + CASE WHEN NEW.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END,
            cross_country_count = flight_analytics_daily.cross_country_count + CASE WHEN NEW.departure_airport_id IS DISTINCT FROM NEW.arrival_airport_id THEN 1 ELSE 0 END,
            avg_duration_seconds = CASE WHEN flight_analytics_daily.flight_count + 1 > 0
                THEN (flight_analytics_daily.total_duration_seconds + flight_duration) / (flight_analytics_daily.flight_count + 1)
                ELSE 0 END,
            updated_at = NOW();

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        affected_date := DATE(OLD.takeoff_time);
        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);

        UPDATE flight_analytics_daily SET
            flight_count = GREATEST(0, flight_count - 1),
            total_duration_seconds = GREATEST(0, total_duration_seconds - old_duration),
            total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0)),
            tow_flight_count = GREATEST(0, tow_flight_count - CASE WHEN OLD.towed_by_device_id IS NOT NULL THEN 1 ELSE 0 END),
            cross_country_count = GREATEST(0, cross_country_count - CASE WHEN OLD.departure_airport_id IS DISTINCT FROM OLD.arrival_airport_id THEN 1 ELSE 0 END),
            avg_duration_seconds = CASE WHEN flight_count - 1 > 0
                THEN (total_duration_seconds - old_duration) / (flight_count - 1)
                ELSE 0 END,
            updated_at = NOW()
        WHERE date = affected_date;
    END IF;

    RETURN NEW;
END;
$function$;
CREATE OR REPLACE FUNCTION public.update_flight_analytics_hourly()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
    affected_hour TIMESTAMPTZ;
    old_hour TIMESTAMPTZ;
BEGIN
    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        affected_hour := DATE_TRUNC('hour', NEW.takeoff_time);

        INSERT INTO flight_analytics_hourly (hour, flight_count, active_devices, active_clubs)
        VALUES (
            affected_hour,
            1,
            1,
            CASE WHEN NEW.club_id IS NOT NULL THEN 1 ELSE 0 END
        )
        ON CONFLICT (hour) DO UPDATE SET
            flight_count = flight_analytics_hourly.flight_count + 1,
            updated_at = NOW();

    -- Handle UPDATE
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_hour := DATE_TRUNC('hour', OLD.takeoff_time);
        affected_hour := DATE_TRUNC('hour', NEW.takeoff_time);

        -- Remove from old hour
        UPDATE flight_analytics_hourly SET
            flight_count = GREATEST(0, flight_count - 1),
            updated_at = NOW()
        WHERE hour = old_hour;

        -- Add to new hour
        INSERT INTO flight_analytics_hourly (hour, flight_count, active_devices, active_clubs)
        VALUES (
            affected_hour,
            1,
            1,
            CASE WHEN NEW.club_id IS NOT NULL THEN 1 ELSE 0 END
        )
        ON CONFLICT (hour) DO UPDATE SET
            flight_count = flight_analytics_hourly.flight_count + 1,
            updated_at = NOW();

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        affected_hour := DATE_TRUNC('hour', OLD.takeoff_time);

        UPDATE flight_analytics_hourly SET
            flight_count = GREATEST(0, flight_count - 1),
            updated_at = NOW()
        WHERE hour = affected_hour;
    END IF;

    RETURN NEW;
END;
$function$;
CREATE OR REPLACE FUNCTION public.update_flight_duration_buckets()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
DECLARE
    old_bucket VARCHAR;
    new_bucket VARCHAR;
    old_duration INT;
    new_duration INT;
BEGIN
    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        new_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);
        IF new_duration > 0 THEN
            new_bucket := get_duration_bucket(new_duration);
            UPDATE flight_duration_buckets SET
                flight_count = flight_count + 1,
                updated_at = NOW()
            WHERE bucket_name = new_bucket;
        END IF;

    -- Handle UPDATE
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        new_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        IF old_duration > 0 THEN
            old_bucket := get_duration_bucket(old_duration);
            UPDATE flight_duration_buckets SET
                flight_count = GREATEST(0, flight_count - 1),
                updated_at = NOW()
            WHERE bucket_name = old_bucket;
        END IF;

        IF new_duration > 0 THEN
            new_bucket := get_duration_bucket(new_duration);
            UPDATE flight_duration_buckets SET
                flight_count = flight_count + 1,
                updated_at = NOW()
            WHERE bucket_name = new_bucket;
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        IF old_duration > 0 THEN
            old_bucket := get_duration_bucket(old_duration);
            UPDATE flight_duration_buckets SET
                flight_count = GREATEST(0, flight_count - 1),
                updated_at = NOW()
            WHERE bucket_name = old_bucket;
        END IF;
    END IF;

    RETURN NEW;
END;
$function$;
